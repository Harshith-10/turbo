use crate::traits::Sandbox;
use async_trait::async_trait;
use std::fs;
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tracing::{info, instrument, warn};
use turbo_core::{models::StageStatus, Result, StageResult, TurboError};

const CGROUP_ROOT: &str = "/sys/fs/cgroup";
const MANAGER_DIR: &str = "turbo_executor";

/// Sandbox implementation for Linux utilizing Cgroups V2 and Namespaces.
///
/// This implementation relies on:
/// - `cgroup_no_v1=all` or unified cgroup hierarchy.
/// - Root privileges to create cgroups and use `unshare` for namespaces.
pub struct LinuxSandbox {
    /// Root path where the sandbox environment (temp dirs) will be created (not used for cgroups).
    pub root_path: String,
}

impl LinuxSandbox {
    /// Create a new LinuxSandbox instance.
    pub fn new(root_path: String) -> Self {
        Self { root_path }
    }

    fn get_manager_path() -> PathBuf {
        Path::new(CGROUP_ROOT).join(MANAGER_DIR)
    }

    fn get_job_path(id: &str) -> PathBuf {
        Self::get_manager_path().join(format!("turbo-box-{}", id))
    }

    // Helper to handle simple file writes
    fn write_cgroup_file(path: &Path, content: &str) -> Result<()> {
        let mut file = fs::OpenOptions::new().write(true).open(path).map_err(|e| {
            TurboError::Sandbox(format!("Failed to open cgroup file {:?}: {}", path, e))
        })?;
        file.write_all(content.as_bytes()).map_err(|e| {
            TurboError::Sandbox(format!("Failed to write to cgroup file {:?}: {}", path, e))
        })?;
        Ok(())
    }

    fn read_cgroup_file(path: &Path) -> Result<String> {
        fs::read_to_string(path).map_err(|e| {
            TurboError::Sandbox(format!("Failed to read cgroup file {:?}: {}", path, e))
        })
    }
}

#[async_trait]
impl Sandbox for LinuxSandbox {
    /// Initialize a new sandbox for the given job ID.
    ///
    /// This creates the necessary Cgroup hierarchy under `/sys/fs/cgroup/turbo_executor/turbo-box-{id}`.
    #[instrument(skip(self))]
    async fn init(&self, id: &str) -> Result<()> {
        let manager_path = Self::get_manager_path();
        info!(
            "Initializing Linux Sandbox for {} in manager {:?}",
            id, manager_path
        );

        // 1. Setup Manager Cgroup
        if !manager_path.exists() {
            fs::create_dir_all(&manager_path).map_err(|e| {
                TurboError::Sandbox(format!(
                    "Failed to create manager cgroup at {:?}: {}",
                    manager_path, e
                ))
            })?;

            // Enable Controllers in Manager
            let subtree_control = manager_path.join("cgroup.subtree_control");
            // We ignore errors here in case some controllers are not available or already enabled,
            // but for a robust implementation we should probably check.
            // For now, try to enable what we need.
            if let Err(e) = Self::write_cgroup_file(&subtree_control, "+cpu +memory +pids") {
                warn!(
                    "Failed to enable controllers in manager: {}. Continuing...",
                    e
                );
            }
        }

        // 2. Create Job Cgroup
        let job_path = Self::get_job_path(id);
        if !job_path.exists() {
            fs::create_dir(&job_path).map_err(|e| {
                TurboError::Sandbox(format!(
                    "Failed to create job cgroup at {:?}: {}",
                    job_path, e
                ))
            })?;
        }

        // 3. Set Default Limits (Can be overridden in run)
        // Memory Max: 512 MB default
        let limit = (512 * 1024 * 1024).to_string();
        Self::write_cgroup_file(&job_path.join("memory.max"), &limit)?;
        Self::write_cgroup_file(&job_path.join("memory.swap.max"), "0")?;

        // Pids Max: 256 default
        Self::write_cgroup_file(&job_path.join("pids.max"), "256")?;

        Ok(())
    }

    /// Run a command in the sandbox
    #[instrument(skip(self))]
    async fn run(
        &self,
        id: &str,
        cmd: &str,
        args: &[String],
        env: &[String],
        limits: Option<turbo_core::models::ExecutionLimits>,
    ) -> Result<StageResult> {
        info!("Running command in sandbox {}: {} {:?}", id, cmd, args);

        let limits = limits.unwrap_or_default();
        let job_path = Self::get_job_path(id);

        self.apply_limits(&job_path, &limits)?;

        let mut command = self.prepare_command(cmd, args, env, &job_path, &limits);
        let mut child = command.spawn().map_err(TurboError::Io)?;

        self.monitor_child(&mut child, &job_path, &limits).await
    }

    #[instrument(skip(self))]
    async fn cleanup(&self, id: &str) -> Result<()> {
        info!("Cleaning up sandbox {}", id);
        let job_path = Self::get_job_path(id);

        if job_path.exists() {
            // In V2, we might need to kill processes first if any are lingering?
            // Usually cgroup.kill can be written to 1 to kill all.
            // But if we just waited, they should be gone.

            // Try to remove directory
            if let Err(e) = fs::remove_dir(&job_path) {
                // If failed, maybe processes are still there?
                warn!(
                    "Failed to delete cgroup {:?}: {}. Attempting to kill...",
                    job_path, e
                );
                // Try writing 1 to cgroup.kill (V2 feature)
                let kill_file = job_path.join("cgroup.kill");
                if kill_file.exists() {
                    let _ = Self::write_cgroup_file(&kill_file, "1");
                }
                // Sleep a tiny bit?
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                // Try removing again
                if let Err(e2) = fs::remove_dir(&job_path) {
                    warn!("Still failed to remove cgroup {:?}: {}", job_path, e2);
                }
            }
        }

        Ok(())
    }
}

impl LinuxSandbox {
    /// Applies resource limits to the job's cgroup based on the provided `ExecutionLimits`.
    /// This includes memory and PID limits.
    fn apply_limits(&self, job_path: &Path, limits: &turbo_core::models::ExecutionLimits) -> Result<()> {
        // Update Cgroup Limits based on execution request
        if limits.memory_limit_bytes > 0 {
            let limit = limits.memory_limit_bytes.to_string();
            Self::write_cgroup_file(&job_path.join("memory.max"), &limit)?;
            Self::write_cgroup_file(&job_path.join("memory.swap.max"), "0")?; // Keep swap disabled
        }
        if limits.pid_limit > 0 {
            Self::write_cgroup_file(&job_path.join("pids.max"), &limits.pid_limit.to_string())?;
        }
        Ok(())
    }

    /// Prepares a `tokio::process::Command` for execution within the sandbox.
    /// This includes setting arguments, environment variables, stdout/stderr piping,
    /// and the critical `pre_exec` hook for namespace isolation and cgroup attachment.
    fn prepare_command(
        &self,
        cmd: &str,
        args: &[String],
        env: &[String],
        job_path: &Path,
        limits: &turbo_core::models::ExecutionLimits,
    ) -> tokio::process::Command {
        let mut command = tokio::process::Command::new(cmd);
        command
            .args(args)
            .envs(env.iter().map(|s| {
                let parts: Vec<&str> = s.splitn(2, '=').collect();
                if parts.len() == 2 {
                    (parts[0], parts[1])
                } else {
                    (s.as_str(), "")
                }
            }))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // CRITICAL: We use unsafe pre_exec to setup isolation in the CHILD process
        unsafe {
            let file_limit = limits.file_limit;
            let uid = limits.uid;
            let gid = limits.gid;
            let job_path_clone = job_path.to_path_buf(); // PathBuf is cloneable

            command.pre_exec(move || {
                // 1. Unshare Namespaces (PID, NET, IPC, UTS, MOUNT)
                if let Err(e) = nix::sched::unshare(
                    nix::sched::CloneFlags::CLONE_NEWNET
                        | nix::sched::CloneFlags::CLONE_NEWNS
                        | nix::sched::CloneFlags::CLONE_NEWIPC
                        | nix::sched::CloneFlags::CLONE_NEWUTS,
                ) {
                    return Err(std::io::Error::other(format!("Failed to unshare: {}", e)));
                }

                // 2. Set RLIMITs
                let nofile = file_limit;
                let _ = nix::sys::resource::setrlimit(
                    nix::sys::resource::Resource::RLIMIT_NOFILE,
                    nofile,
                    nofile,
                );

                // 3. Switch User
                if let Some(g) = gid {
                    let _ = nix::unistd::setgid(nix::unistd::Gid::from_raw(g));
                }
                if let Some(u) = uid {
                    let _ = nix::unistd::setuid(nix::unistd::Uid::from_raw(u));
                }

                // 4. Attach to Cgroup (v2) by writing "0" (current process) to procs
                let procs_path = job_path_clone.join("cgroup.procs");
                let mut file = std::fs::OpenOptions::new().write(true).open(&procs_path)?;
                use std::io::Write;
                write!(file, "0")?;

                Ok(())
            });
        }
        command
    }

    /// Monitors a spawned child process, handles output capturing, applies timeouts,
    /// and gathers the final execution results including resource usage.
    async fn monitor_child(
        &self,
        child: &mut tokio::process::Child,
        job_path: &Path,
        limits: &turbo_core::models::ExecutionLimits,
    ) -> Result<StageResult> {
        // Output Capping & Timeouts
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| TurboError::Io(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Failed to capture stdout")))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| TurboError::Io(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Failed to capture stderr")))?;

        let stdout_reader = tokio::io::BufReader::new(stdout);
        let stderr_reader = tokio::io::BufReader::new(stderr);

        use tokio::io::AsyncReadExt;

        let output_cap = limits.output_limit_bytes; // No need for `as u64`

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

        tokio::select! {
             res = child.wait() => {
                 let duration = start_time.elapsed().as_millis() as u64;
                 // Process finished naturally
                 match res {
                     Ok(status) => {
                         let (stdout_bytes, stderr_bytes) = read_task.await.unwrap_or_else(|_| (Vec::new(), Vec::new()));
                         let mut final_status = if status.success() {
                             StageStatus::Success
                         } else {
                             StageStatus::RuntimeError
                         };

                         // Heuristic for OOM (SIGKILL = 9)
                         if let Some(9) = status.signal() {
                             final_status = StageStatus::MemoryLimitExceeded;
                         }

                         // Gather Resource Usage
                         let mem_peak = Self::read_cgroup_file(&job_path.join("memory.current"))
                             .ok()
                             .and_then(|v| v.trim().parse::<u64>().ok())
                             .unwrap_or(0);

                         let cpu_time_us = Self::read_cgroup_file(&job_path.join("cpu.stat"))
                             .ok()
                             .and_then(|content| {
                                content.lines()
                                    .find(|l| l.starts_with("usage_usec"))
                                    .and_then(|l| l.split_whitespace().nth(1))
                                    .and_then(|v| v.parse::<u64>().ok())
                             })
                             .unwrap_or(0);

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

                 // CRITICAL: Ensure all processes in the cgroup are killed
                 // In V2, writing "1" to cgroup.kill kills all processes in the cgroup
                 let kill_file = job_path.join("cgroup.kill");
                 if kill_file.exists() {
                     let _ = Self::write_cgroup_file(&kill_file, "1");
                 }

                 let duration = start_time.elapsed().as_millis() as u64;

                 // Await the output readers to finish reading what they can
                 let (stdout_bytes, stderr_bytes) = read_task.await.unwrap_or_else(|_| (Vec::new(), Vec::new()));

                 // Read stats
                 let mem_peak = Self::read_cgroup_file(&job_path.join("memory.current"))
                     .ok()
                     .and_then(|v| v.trim().parse::<u64>().ok())
                     .unwrap_or(0);

                 let cpu_time_us = Self::read_cgroup_file(&job_path.join("cpu.stat"))
                     .ok()
                     .and_then(|content| {
                        content.lines()
                            .find(|l| l.starts_with("usage_usec"))
                            .and_then(|l| l.split_whitespace().nth(1))
                            .and_then(|v| v.parse::<u64>().ok())
                     })
                     .unwrap_or(0);

                 Ok(StageResult {
                     status: StageStatus::TimeLimitExceeded,
                     stdout: String::from_utf8_lossy(&stdout_bytes).to_string(),
                     stderr: String::from_utf8_lossy(&stderr_bytes).to_string(),
                     exit_code: None,
                     signal: Some("SIGKILL".to_string()),
                     memory_usage: Some(mem_peak),
                     cpu_time: Some(cpu_time_us),
                     execution_time: Some(duration),
                 })
             }
        }
    }
}
