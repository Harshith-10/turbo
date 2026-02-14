use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;
use tracing::{error, info};
use turbo_box::{LinuxSandbox, Sandbox};
use turbo_core::models::{
    ExecutionLimits, Job, JobResult, StageResult, StageStatus, TestcaseResult,
};
use turbo_db::TurboDb;
use turbo_pkg::models::PackageDefinition;

fn get_runtime_path(runtimes_dir: &Path, lang: &str, ver: &str) -> PathBuf {
    runtimes_dir.join(lang).join(ver)
}

/// Starts the worker loop, polling the Redis queue for new jobs.
///
/// This function runs indefinitely, processing jobs one by one.
pub async fn start_worker(id: usize, db: TurboDb, runtimes_dir: PathBuf) {
    info!("Worker {} started", id);
    let sandbox = LinuxSandbox::new("/var/turbo/sandbox".to_string());

    loop {
        match db.queue.pop_job().await {
            Ok(Some(job)) => {
                info!("Processing job {}", job.id);
                let result = execute_job(&job, &sandbox, &runtimes_dir).await;
                if let Err(e) = db.queue.publish_result(&job.id, &result).await {
                    error!("Failed to publish result for {}: {}", job.id, e);
                }
            }
            Ok(None) => {} // Busy loop or small sleep? DB blpop blocks.
            Err(e) => {
                error!("Queue error: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

/// Executes a single job within the sandbox.
///
/// 1. Creates a temporary directory for source files.
/// 2. Resolves the runtime package (e.g., Python, C++).
/// 3. Initializes the sandbox.
/// 4. Compiles the code (if `build.sh` exists).
/// 5. Runs the code (single run or batched testcases).
/// 6. Cleans up resources.
async fn execute_job(job: &Job, sandbox: &impl Sandbox, runtimes_dir: &Path) -> JobResult {
    let job_id = &job.id;
    let req = &job.request;

    let user = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    let temp_dir = std::env::temp_dir().join(format!("turbo-{}", user)).join(job_id);
    if let Err(e) = fs::create_dir_all(&temp_dir).await {
        return fail_job(job, format!("Failed to create temp dir: {}", e));
    }

    for file in &req.files {
        let path = temp_dir.join(file.name.as_deref().unwrap_or("main"));
        if let Err(e) = fs::write(&path, &file.content).await {
            return fail_job(job, format!("Failed to write file: {}", e));
        }
    }

    let version = req.version.as_deref().unwrap_or("latest");
    let runtime_path = get_runtime_path(runtimes_dir, &req.language, version);

    // Check if runtime exists
    if !runtime_path.exists() {
        return fail_job(job, format!("Runtime not found at {:?}", runtime_path));
    }

    let pkg_def = match PackageDefinition::from_path(runtime_path.clone()) {
        Ok(d) => d,
        Err(e) => return fail_job(job, format!("Invalid runtime definition: {}", e)),
    };

    if let Err(e) = sandbox.init(job_id).await {
        return fail_job(job, format!("Sandbox init failed: {}", e));
    }

    let mut compile_result = None;
    let compile_script = pkg_def.path.join("compile.sh");
    
    // Attempt caching if compile script exists
    let user = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    let cache_dir = std::env::temp_dir().join(format!("turbo-cache-{}", user));
    let mut cache_path = None;

    if compile_script.exists() {
        // Calculate hash
        let compile_script_content = fs::read_to_string(&compile_script).await.unwrap_or_default();
        let hash = calculate_job_hash(req, &compile_script_content);
        let job_cache_path = cache_dir.join(&hash);
        
        if job_cache_path.exists() {
            info!("Cache hit for job {}, hash {}", job_id, hash);
             // Restore from cache
                if let Err(e) = hard_link_recursive(&job_cache_path, &temp_dir).await {
                error!("Failed to restore from cache: {}", e);
                // Fallback to normal compile if restore fails
            } else {
                 // Touch cache to update modification time for LRU
                 let _ = fs::set_permissions(&job_cache_path, std::fs::Permissions::from_mode(0o755)).await;
                 let _ = fs::write(job_cache_path.join(".touch"), "").await;
                 
                 compile_result = Some(StageResult {
                    status: StageStatus::Success,
                    stdout: "Restored from cache".to_string(),
                    stderr: "".to_string(),
                    ..stub_result()
                 });
            }
        }
        
        cache_path = Some(job_cache_path);
    }


    if compile_result.is_none() && compile_script.exists() {
        let wrapper_cmd = "sh";
        let mut compile_cmd = format!("cd {} && {}", temp_dir.display(), compile_script.display());
        for file in &req.files {
            let filename = file.name.as_deref().unwrap_or("main");
            compile_cmd.push_str(&format!(" \"{}\"", filename));
        }

        let wrapper_args = vec![
            "-c".to_string(),
            compile_cmd,
        ];

        let limits = ExecutionLimits {
            timeout_ms: req.compile_timeout.unwrap_or(10000),
            memory_limit_bytes: req.compile_memory_limit.unwrap_or(512 * 1024 * 1024),
            ..Default::default()
        };

        match sandbox
            .run(job_id, wrapper_cmd, &wrapper_args, &[], Some(limits))
            .await
        {
            Ok(res) => {
                let success = res.status == StageStatus::Success;
                compile_result = Some(res.clone());
                if !success {
                    let mut failed_res = res;
                    failed_res.status = StageStatus::CompilationError;
                    compile_result = Some(failed_res);
                    let _ = sandbox.cleanup(job_id).await;
                    return JobResult {
                        language: req.language.clone(),
                        version: version.to_string(),
                        run: None,
                        compile: compile_result,
                        testcases: None,
                    };
                }
                
                // Save to cache on success
                if let Some(path) = cache_path {
                     if let Err(e) = copy_dir_recursive(&temp_dir, &path).await {
                         error!("Failed to save to cache: {}", e);
                     } else {
                         // Touch newly created cache to ensure timestamp is fresh
                         let _ = fs::write(path.join(".touch"), "").await;
                     }
                }
            }
            Err(e) => {
                let _ = sandbox.cleanup(job_id).await;
                return fail_job(job, format!("Compile execution failed: {}", e));
            }
        }
    }

    let run_script = pkg_def.path.join("run.sh");
    if !run_script.exists() {
        let _ = sandbox.cleanup(job_id).await;
        return fail_job(job, format!("Run script not found at {:?}", run_script));
    }

    let mut testcase_results = Vec::new();
    let mut single_run_result = None;

    if let Some(testcases) = &req.testcases {
        for tc in testcases {
            let input_file = temp_dir.join(format!("input_{}.txt", tc.id));
            let _ = fs::write(&input_file, &tc.input).await;

            let mut cmd_str = format!(
                "cd {} && {} < {}",
                temp_dir.display(),
                run_script.display(),
                input_file.display()
            );
            if let Some(args) = &req.args {
                for arg in args {
                    cmd_str.push_str(&format!(" \"{}\"", arg));
                }
            }
            info!("Batch Exec Cmd: {}", cmd_str);
            let wrapper_args = vec!["-c".to_string(), cmd_str];

            let limits = ExecutionLimits {
                timeout_ms: req.run_timeout.unwrap_or(3000),
                memory_limit_bytes: req.run_memory_limit.unwrap_or(512 * 1024 * 1024),
                ..Default::default()
            };

            let stage_res = match sandbox
                .run(job_id, "sh", &wrapper_args, &[], Some(limits))
                .await
            {
                Ok(r) => r,
                Err(e) => StageResult {
                    status: StageStatus::RuntimeError,
                    stdout: "".to_string(),
                    stderr: format!("Sandbox error: {}", e),
                    ..stub_result()
                },
            };

            let passed = if let Some(expected) = &tc.expected_output {
                stage_res.stdout.trim() == expected.trim()
            } else {
                true
            };

            testcase_results.push(TestcaseResult {
                id: tc.id.clone(),
                passed,
                actual_output: stage_res.stdout.clone(),
                run_details: stage_res,
            });
        }
    } else {
        let input_file = temp_dir.join("input.txt");
        let _ = fs::write(&input_file, req.stdin.as_deref().unwrap_or("")).await;

        let mut cmd_str = format!(
            "cd {} && {} < {}",
            temp_dir.display(),
            run_script.display(),
            input_file.display()
        );
        if let Some(args) = &req.args {
            for arg in args {
                cmd_str.push_str(&format!(" \"{}\"", arg));
            }
        }
        let wrapper_args = vec!["-c".to_string(), cmd_str];

        let limits = ExecutionLimits {
            timeout_ms: req.run_timeout.unwrap_or(3000),
            memory_limit_bytes: req.run_memory_limit.unwrap_or(512 * 1024 * 1024),
            ..Default::default()
        };

        single_run_result = sandbox
            .run(job_id, "sh", &wrapper_args, &[], Some(limits))
            .await
            .ok();
    }

    let _ = sandbox.cleanup(job_id).await;
    let _ = fs::remove_dir_all(&temp_dir).await;

    JobResult {
        language: req.language.clone(),
        version: version.to_string(),
        compile: compile_result,
        run: single_run_result,
        testcases: if testcase_results.is_empty() {
            None
        } else {
            Some(testcase_results)
        },
    }
}

fn fail_job(job: &Job, err: String) -> JobResult {
    JobResult {
        language: job.request.language.clone(),
        version: job.request.version.clone().unwrap_or_default(),
        run: Some(StageResult {
            status: StageStatus::RuntimeError,
            stdout: "".to_string(),
            stderr: err,
            ..stub_result()
        }),
        compile: None,
        testcases: None,
    }
}

fn stub_result() -> StageResult {
    StageResult {
        status: StageStatus::Pending,
        stdout: "".into(),
        stderr: "".into(),
        exit_code: None,
        signal: None,
        memory_usage: None,
        cpu_time: None,
        execution_time: None,
    }
}

// Helper for async recursive copy
async fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst).await?;
    }
    let mut entries = fs::read_dir(src).await?;
    while let Some(entry) = entries.next_entry().await? {
        let ty = entry.file_type().await?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            Box::pin(copy_dir_recursive(&src_path, &dst_path)).await?;
        } else {
            fs::copy(&src_path, &dst_path).await?;
        }
    }
    Ok(())
}

// Helper for async recursive hard link with fallback to copy
async fn hard_link_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst).await?;
    }
    let mut entries = fs::read_dir(src).await?;
    while let Some(entry) = entries.next_entry().await? {
        let ty = entry.file_type().await?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            Box::pin(hard_link_recursive(&src_path, &dst_path)).await?;
        } else {
            if let Err(_) = fs::hard_link(&src_path, &dst_path).await {
                // Fallback to copy if hard link fails
                 fs::copy(&src_path, &dst_path).await?;
            }
        }
    }
    Ok(())
}

fn calculate_job_hash(req: &turbo_core::models::JobRequest, compile_script_content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(req.language.as_bytes());
    hasher.update(req.version.as_deref().unwrap_or("latest").as_bytes());
    hasher.update(compile_script_content.as_bytes());

    // Sort files to ensure stable hash
    let mut files = req.files.clone();
    files.sort_by(|a, b| a.name.cmp(&b.name));

    for file in files {
        hasher.update(file.name.as_deref().unwrap_or("main").as_bytes());
        hasher.update(&file.content);
    }

    hex::encode(hasher.finalize())
}
