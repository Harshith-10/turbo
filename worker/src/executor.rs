use bollard::Docker;
use bollard::container::{Config, CreateContainerOptions, RemoveContainerOptions, UploadToContainerOptions, DownloadFromContainerOptions, LogOutput};
use bollard::exec::{StartExecOptions, CreateExecOptions};
use turbo_common::{Job, JobResult, JobData, JobResultData, TestCaseResult};
use futures_util::StreamExt;
use std::default::Default;
use tracing::info;
use uuid::Uuid;
use tar::{Builder, Header, Archive};
use std::io::Read;
use tokio::io::AsyncWriteExt;

fn create_tar(path: &str, content: &[u8]) -> Vec<u8> {
    let mut ar = Builder::new(Vec::new());
    let mut header = Header::new_gnu();
    header.set_path(path).unwrap();
    header.set_size(content.len() as u64);
    header.set_mode(0o777);
    header.set_cksum();
    ar.append(&header, content).unwrap();
    ar.into_inner().unwrap()
}

pub async fn execute_job(docker: &Docker, job: &Job, worker_id: Uuid) -> JobResult {
    info!("Executing job: {}", job.id);
    match &job.data {
        JobData::Compile { language, code } => {
            compile_code(docker, job.id, worker_id, language, code).await
        },
        JobData::Execute { language, binary, code, testcases } => {
            execute_testcases(docker, job.id, worker_id, language, binary, code, testcases).await
        }
    }
}

async fn compile_code(docker: &Docker, job_id: Uuid, worker_id: Uuid, language: &str, code: &str) -> JobResult {
    let (image, filename, cmd, out_file) = match language {
        "java" => ("eclipse-temurin:17-jdk-jammy", "Main.java", "javac Main.java && jar cf Main.jar *.class", "Main.jar"),
        "rust" => ("rust:latest", "main.rs", "rustc main.rs", "main"),
        _ => return JobResult {
            job_id,
            worker_id,
            data: JobResultData::Error("Unsupported language for compilation".to_string()),
        },
    };

    let container_name = format!("compile-{}", job_id);
    let config = Config {
        image: Some(image),
        cmd: Some(vec!["sleep", "60"]), // Keep alive
        ..Default::default()
    };

    if let Err(e) = docker.create_container(Some(CreateContainerOptions { name: container_name.clone(), ..Default::default() }), config).await {
        return JobResult { job_id, worker_id, data: JobResultData::Error(format!("Create container failed: {}", e)) };
    }

    if let Err(e) = docker.start_container::<String>(&container_name, None).await {
        let _ = docker.remove_container(&container_name, None::<RemoveContainerOptions>).await;
        return JobResult { job_id, worker_id, data: JobResultData::Error(format!("Start container failed: {}", e)) };
    }

    // Upload code
    let tar_data = create_tar(filename, code.as_bytes());
    if let Err(e) = docker.upload_to_container(&container_name, Some(UploadToContainerOptions { path: "/", ..Default::default() }), tar_data.into()).await {
        let _ = docker.remove_container(&container_name, None::<RemoveContainerOptions>).await;
        return JobResult { job_id, worker_id, data: JobResultData::Error(format!("Upload failed: {}", e)) };
    }

    // Exec compile
    info!("Creating exec for compile: {}", cmd);
    let exec = docker.create_exec(&container_name, CreateExecOptions {
        cmd: Some(vec!["sh", "-c", cmd]),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        attach_stdin: Some(true),
        ..Default::default()
    }).await.unwrap();

    info!("Starting exec for compile");
    let start_exec = docker.start_exec(&exec.id, None::<StartExecOptions>).await.unwrap();
    
    let mut output = String::new();
    match start_exec {
        bollard::exec::StartExecResults::Attached { output: mut stream, .. } => {
            info!("Reading exec output");
            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(LogOutput::StdOut { message }) => output.push_str(&String::from_utf8_lossy(&message)),
                    Ok(LogOutput::StdErr { message }) => output.push_str(&String::from_utf8_lossy(&message)),
                    _ => {}
                }
            }
            info!("Finished reading exec output");
        },
        _ => {}
    }

    let inspect = docker.inspect_exec(&exec.id).await.unwrap();
    if inspect.exit_code != Some(0) {
        let _ = docker.remove_container(&container_name, None::<RemoveContainerOptions>).await;
        return JobResult {
            job_id,
            worker_id,
            data: JobResultData::Compile { success: false, message: output, binary: None },
        };
    }

    // Download binary
    let mut binary_stream = docker.download_from_container(&container_name, Some(DownloadFromContainerOptions { path: out_file }));
    let mut tar_content = Vec::new();
    while let Some(chunk) = binary_stream.next().await {
        if let Ok(data) = chunk {
            tar_content.extend_from_slice(&data);
        }
    }

    let _ = docker.remove_container(&container_name, None::<RemoveContainerOptions>).await;

    // Extract binary from tar
    let mut archive = Archive::new(&tar_content[..]);
    let mut binary_data = Vec::new();
    
    for file in archive.entries().unwrap() {
        let mut file = file.unwrap();
        if file.path().unwrap().to_str().unwrap() == out_file {
            file.read_to_end(&mut binary_data).unwrap();
            break;
        }
    }

    if binary_data.is_empty() {
         return JobResult {
            job_id,
            worker_id,
            data: JobResultData::Compile { success: false, message: "Binary not found".to_string(), binary: None },
        };
    }

    JobResult {
        job_id,
        worker_id,
        data: JobResultData::Compile { success: true, message: output, binary: Some(binary_data) },
    }
}

async fn execute_testcases(docker: &Docker, job_id: Uuid, worker_id: Uuid, language: &str, binary: &Option<Vec<u8>>, code: &Option<String>, testcases: &[turbo_common::TestCase]) -> JobResult {
    let (image, filename, run_cmd) = match language {
        "java" => ("eclipse-temurin:17-jdk-jammy", "Main.jar", vec!["java", "-cp", "Main.jar", "Main"]),
        "rust" => ("rust:latest", "main", vec!["./main"]),
        "python" => ("python:3.9-slim", "script.py", vec!["python", "script.py"]),
        _ => return JobResult { job_id, worker_id, data: JobResultData::Error("Unsupported language".to_string()) },
    };

    let container_name = format!("exec-{}", job_id);
    let config = Config {
        image: Some(image),
        cmd: Some(vec!["sleep", "600"]), // Keep alive
        ..Default::default()
    };

    if let Err(e) = docker.create_container(Some(CreateContainerOptions { name: container_name.clone(), ..Default::default() }), config).await {
        return JobResult { job_id, worker_id, data: JobResultData::Error(format!("Create container failed: {}", e)) };
    }

    if let Err(e) = docker.start_container::<String>(&container_name, None).await {
        let _ = docker.remove_container(&container_name, None::<RemoveContainerOptions>).await;
        return JobResult { job_id, worker_id, data: JobResultData::Error(format!("Start container failed: {}", e)) };
    }

    // Upload binary or code
    let content = if let Some(b) = binary { b.clone() } else { code.as_ref().unwrap().as_bytes().to_vec() };
    let tar_data = create_tar(filename, &content);
    if let Err(e) = docker.upload_to_container(&container_name, Some(UploadToContainerOptions { path: "/", ..Default::default() }), tar_data.into()).await {
        let _ = docker.remove_container(&container_name, None::<RemoveContainerOptions>).await;
        return JobResult { job_id, worker_id, data: JobResultData::Error(format!("Upload failed: {}", e)) };
    }

    let mut results = Vec::new();

    for tc in testcases {
        let exec = docker.create_exec(&container_name, CreateExecOptions {
            cmd: Some(run_cmd.clone()),
            attach_stdin: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        }).await.unwrap();

        let start_exec = docker.start_exec(&exec.id, None::<StartExecOptions>).await.unwrap();
        
        let mut actual_output = String::new();
        let mut error_output = String::new();

        match start_exec {
            bollard::exec::StartExecResults::Attached { mut output, mut input } => {
                // Send input
                input.write_all(tc.input.as_bytes()).await.unwrap();
                input.flush().await.unwrap();
                input.shutdown().await.unwrap();
                drop(input);

                while let Some(msg) = output.next().await {
                    match msg {
                        Ok(LogOutput::StdOut { message }) => actual_output.push_str(&String::from_utf8_lossy(&message)),
                        Ok(LogOutput::StdErr { message }) => error_output.push_str(&String::from_utf8_lossy(&message)),
                        _ => {}
                    }
                }
            },
            _ => {}
        }

        // Trim output
        let actual_trimmed = actual_output.trim();
        let expected_trimmed = tc.output.trim();
        let passed = actual_trimmed == expected_trimmed;

        results.push(TestCaseResult {
            id: tc.id,
            worker_id,
            passed,
            actual_output: actual_trimmed.to_string(),
            error: error_output,
            time: "0ms".to_string(), // Placeholder
            memory: "0MB".to_string(), // Placeholder
        });
    }

    let _ = docker.remove_container(&container_name, None::<RemoveContainerOptions>).await;

    JobResult {
        job_id,
        worker_id,
        data: JobResultData::Execute { results },
    }
}
