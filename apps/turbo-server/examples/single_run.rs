use turbo_core::models::{FileRequest, JobRequest, JobResult};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    let req = JobRequest {
        language: "python".to_string(),
        version: Some("3.14.2".to_string()),
        files: vec![FileRequest {
            name: Some("main.py".to_string()),
            content: "print('Hello from Single Run')".to_string(),
            encoding: Some("utf8".to_string()),
        }],
        testcases: None,
        args: Some(vec!["main.py".to_string()]),
        stdin: None,
        run_timeout: None,
        compile_timeout: None,
        run_memory_limit: None,
        compile_memory_limit: None,
    };

    println!("Submitting Single Run Job...");
    let port = std::env::var("TURBO_SERVER_PORT").unwrap_or_else(|_| "3000".to_string());
    let url = format!("http://localhost:{}/api/v1/execute", port);
    let res = client.post(&url).json(&req).send().await?;

    if !res.status().is_success() {
        eprintln!("Error: {}", res.text().await?);
        return Ok(());
    }

    let result: JobResult = res.json().await?;

    if let Some(run) = result.run {
        println!("Status: {:?}", run.status);
        println!("Stdout: {}", run.stdout);
        println!("Stderr: {}", run.stderr);
        println!("Time: {:?} ms", run.execution_time);
        println!("Memory: {:?} bytes", run.memory_usage);
    } else {
        println!("No run result.");
    }

    Ok(())
}
