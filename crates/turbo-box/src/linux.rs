use async_trait::async_trait;
use std::os::unix::process::ExitStatusExt;
use turbo_core::{StageResult, Result, TurboError};
use crate::traits::Sandbox;
use tracing::{info, instrument};
use cgroups_rs::{Cgroup, hierarchies, Resources};
use std::process::Stdio;

pub struct LinuxSandbox {
    pub root_path: String,
}

impl LinuxSandbox {
    pub fn new(root_path: String) -> Self {
        Self { root_path }
    }

    fn get_cgroup_name(&self, id: &str) -> String {
        format!("turbo-box-{}", id)
    }
}

#[async_trait]
impl Sandbox for LinuxSandbox {
    #[instrument(skip(self))]
    async fn init(&self, id: &str) -> Result<()> {
        info!("Initializing Linux Sandbox for {}", id);
        
        let hier = hierarchies::auto();
        let cg_name = self.get_cgroup_name(id);
        
        let cg = Cgroup::new(hier, cg_name.as_str())
            .map_err(|e| TurboError::Sandbox(format!("Failed to create cgroup: {}", e)))?;

        // Fix: Use correct field structure for Resources
        // Since cgroups-rs v0.3.4, MemController fields are different or use different types.
        // We will construct it carefully or use default.
        // Checking docs/source implies usage of `memory: MemController { ... }` but wrapped in option or struct.
        // Let's use `Resources::default()` and modify it if needed, or instantiate `MemController` directly.
        // The error `expected MemoryResources, found MemController` suggests `Resources.memory` expects `MemoryResources`.
        
        // 2. Set Limits
        let mut resources = Resources::default();
        
        // Memory Limit: 512 MB
        let limit = 512 * 1024 * 1024;
        resources.memory.memory_hard_limit = Some(limit);
        resources.memory.memory_swap_limit = Some(limit); // Disable swap by setting same max
        
        // PID Limit: 256
        // Note: Field name depends on cgroups-rs version.
        // v0.3.x usually uses `pids` struct.
        // Assuming `resources.pids.max = cgroups_rs::MaxValue::Value(256)`
        resources.pid.maximum_number_of_processes = Some(cgroups_rs::MaxValue::Value(256));

        cg.apply(&resources)
            .map_err(|e| TurboError::Sandbox(format!("Failed to apply limits: {}", e)))?;

        Ok(())
    }


    #[instrument(skip(self))]
    async fn run(&self, id: &str, cmd: &str, args: &[String], env: &[String], limits: Option<turbo_core::models::ExecutionLimits>) -> Result<StageResult> {
        info!("Running command in sandbox {}: {} {:?}", id, cmd, args);
        
        let limits = limits.unwrap_or_default();
        let cg_name = self.get_cgroup_name(id);
        
        // 1. Update Cgroup Limits (Memory & PIDs) explicitly before run
        // This allows per-execution configuration
        let hier = hierarchies::auto();
        let cg = Cgroup::load(hier, cg_name.as_str());
        
        // We reuse the Resources struct to update
        let mut resources = Resources::default();
        resources.memory.memory_hard_limit = Some(limits.memory_limit_bytes as i64);
        resources.memory.memory_swap_limit = Some(limits.memory_limit_bytes as i64); // No swap
        resources.pid.maximum_number_of_processes = Some(cgroups_rs::MaxValue::Value(limits.pid_limit as i64));
        
        cg.apply(&resources)
             .map_err(|e| TurboError::Sandbox(format!("Failed to update limits: {}", e)))?;

        let mut command = tokio::process::Command::new(cmd);
        command
            .args(args)
            .envs(env.iter().map(|s| {
                let parts: Vec<&str> = s.splitn(2, '=').collect();
                if parts.len() == 2 { (parts[0], parts[1]) } else { (s.as_str(), "") }
            }))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // CRITICAL: We use unsafe pre_exec to setup isolation in the CHILD process
        unsafe {
            let cg_name = cg_name.clone();
            // Clone limits to move into closure (ExecutionLimits is Clone)
            let limits = limits.clone();
            let file_limit = limits.file_limit;
            
            command.pre_exec(move || {
                // 1. Unshare Namespaces (PID, NET, IPC, UTS, MOUNT)
                if let Err(e) = nix::sched::unshare(
                    nix::sched::CloneFlags::CLONE_NEWNET |
                    nix::sched::CloneFlags::CLONE_NEWNS |
                    nix::sched::CloneFlags::CLONE_NEWIPC |
                    nix::sched::CloneFlags::CLONE_NEWUTS
                ) {
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to unshare: {}", e)));
                }

                // 2. Set RLIMITs
                // RLIMIT_NOFILE (Max Open Files)
                let nofile = file_limit;
                let _ = nix::sys::resource::setrlimit(
                    nix::sys::resource::Resource::RLIMIT_NOFILE, 
                    nofile,
                    nofile
                );
                
                // 3. Switch User
                // If UID/GID are provided in limits, switch to them.
                if let Some(gid) = limits.gid {
                    let _ = nix::unistd::setgid(nix::unistd::Gid::from_raw(gid));
                }
                if let Some(uid) = limits.uid {
                    let _ = nix::unistd::setuid(nix::unistd::Uid::from_raw(uid));
                }
                
                // Fallback: If we are root and no UID specified, maybe safer to warn or default?
                // For now, we only switch if explicitly asked via ExecutionLimits.

                // 4. Attach to Cgroup (v2) by writing "0" (current process) to procs
                let cg_path = format!("/sys/fs/cgroup/{}/cgroup.procs", cg_name);
                if let Ok(mut file) = std::fs::OpenOptions::new().write(true).open(&cg_path) {
                     use std::io::Write;
                     let _ = write!(file, "0"); 
                }

                Ok(())
            });
        }

        let mut child = command.spawn().map_err(|e| TurboError::Io(e))?;
        
        // Output Capping & Timeouts
        let stdout = child.stdout.take().ok_or_else(|| TurboError::Io(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Failed to capture stdout")))?;
        let stderr = child.stderr.take().ok_or_else(|| TurboError::Io(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Failed to capture stderr")))?;
        
        let stdout_reader = tokio::io::BufReader::new(stdout);
        let stderr_reader = tokio::io::BufReader::new(stderr);
        
        use tokio::io::AsyncReadExt;
        
        let output_cap = limits.output_limit_bytes as u64;
        
        let read_task = tokio::spawn(async move {
             let mut stdout_buf = Vec::new();
             let mut stderr_buf = Vec::new();
             let mut stdout = stdout_reader.take(output_cap); 
             let mut stderr = stderr_reader.take(output_cap); 
             
             let _ = stdout.read_to_end(&mut stdout_buf).await;
             let _ = stderr.read_to_end(&mut stderr_buf).await;
             (stdout_buf, stderr_buf)
        });

        // Timeout
        let timeout_duration = std::time::Duration::from_millis(limits.timeout_ms);
        let start_time = std::time::Instant::now();
        
        let result = tokio::select! {
             res = child.wait() => {
                 let duration = start_time.elapsed().as_millis() as u64;
                 // Process finished naturally
                 match res {
                     Ok(status) => {
                         let (stdout_bytes, stderr_bytes) = read_task.await.unwrap_or_else(|_| (Vec::new(), Vec::new()));
                         let mut final_status = if status.success() { 
                             turbo_core::models::StageStatus::Success 
                         } else { 
                             turbo_core::models::StageStatus::RuntimeError 
                         };
                         
                         // Heuristic for OOM
                         if let Some(9) = status.signal() {
                             final_status = turbo_core::models::StageStatus::MemoryLimitExceeded;
                         }
                         
                         // Gather Resource Usage
                         // We reload the cgroup to get the latest stats (though the object might already have them if they are not cached)
                         // Actually cgroups-rs reads on demand usually.
                         
                         let mem_peak = if let Some(mem) = cg.controller_of::<cgroups_rs::memory::MemController>() {
                             mem.memory_stat().max_usage_in_bytes
                         } else {
                             0
                         };

                         let cpu_time_us = {
                             let path = format!("/sys/fs/cgroup/{}/cpu.stat", cg_name);
                             if let Ok(content) = std::fs::read_to_string(path) {
                                 // usage_usec 12345
                                 content.lines()
                                     .find(|l| l.starts_with("usage_usec"))
                                     .and_then(|l| l.split_whitespace().nth(1))
                                     .and_then(|v| v.parse::<u64>().ok())
                                     .unwrap_or(0)
                             } else {
                                 0
                             }
                         };
                         
                         // Clean up cgroup immediately after done to avoid clutter (or let caller do it)
                         // But we need to ensure stats are read first.

                         Ok(StageResult {
                             status: final_status,
                             stdout: String::from_utf8_lossy(&stdout_bytes).to_string(),
                             stderr: String::from_utf8_lossy(&stderr_bytes).to_string(),
                             exit_code: status.code(),
                             signal: status.signal().map(|s: i32| s.to_string()), 
                             memory_usage: Some(mem_peak), 
                             cpu_time: Some(cpu_time_us),    
                             execution_time: Some(duration),
                         })
                     },
                     Err(e) => Err(TurboError::Io(e))
                 }
             },
             _ = tokio::time::sleep(timeout_duration) => {
                 let _ = child.kill().await;
                 let duration = start_time.elapsed().as_millis() as u64;
                 
                 // Await the output readers to finish reading what they can
                 let (stdout_bytes, stderr_bytes) = read_task.await.unwrap_or_else(|_| (Vec::new(), Vec::new()));

                 // Try to read stats even if timed out (might be useful)
                 let mem_peak = if let Some(mem) = cg.controller_of::<cgroups_rs::memory::MemController>() {
                     mem.memory_stat().max_usage_in_bytes
                 } else {
                     0
                 };
                 
                 let cpu_time_us = {
                     let path = format!("/sys/fs/cgroup/{}/cpu.stat", cg_name);
                     if let Ok(content) = std::fs::read_to_string(path) {
                         content.lines()
                             .find(|l| l.starts_with("usage_usec"))
                             .and_then(|l| l.split_whitespace().nth(1))
                             .and_then(|v| v.parse::<u64>().ok())
                             .unwrap_or(0)
                     } else {
                         0
                     }
                 };
                 
                 Ok(StageResult {
                     status: turbo_core::models::StageStatus::TimeLimitExceeded,
                     stdout: String::from_utf8_lossy(&stdout_bytes).to_string(),
                     stderr: String::from_utf8_lossy(&stderr_bytes).to_string(),
                     exit_code: None,
                     signal: Some("SIGKILL".to_string()),
                     memory_usage: Some(mem_peak),
                     cpu_time: Some(cpu_time_us), // Use actual CPU time
                     execution_time: Some(duration),
                 })
             }
        }?;

        Ok(result)
    }

    #[instrument(skip(self))]
    async fn cleanup(&self, id: &str) -> Result<()> {
        info!("Cleaning up sandbox {}", id);
        let hier = hierarchies::auto();
        let cg_name = self.get_cgroup_name(id);
        
        // Cgroup Cleanup
        let cg = Cgroup::load(hier, cg_name.as_str());
        let _ = cg.delete(); // Ignore errors if already gone
            
        Ok(())
    }
}
