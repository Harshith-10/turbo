use turbo_core::models::{FileRequest, JobRequest, JobResult, Testcase};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    let req = JobRequest {
        language: "python".to_string(),
        version: Some("3.14.2".to_string()),
        files: vec![FileRequest {
            name: Some("main.py".to_string()),
            // Simple python script that doubles input
            content: "import sys; print(sys.stdin.read().strip() * 2)".to_string(),
            encoding: Some("utf8".to_string()),
        }],
        testcases: Some(vec![
            Testcase {
                id: "1".into(),
                input: "A".into(),
                expected_output: Some("AA".into()),
            },
            Testcase {
                id: "2".into(),
                input: "B".into(),
                expected_output: Some("BB".into()),
            },
            Testcase {
                id: "3".into(),
                input: "Hello".into(),
                expected_output: Some("HelloHello".into()),
            },
            // This one should fail
            Testcase {
                id: "4".into(),
                input: "Fail".into(),
                expected_output: Some("Wrong".into()),
            },
        ]),
        args: Some(vec!["main.py".to_string()]),
        stdin: None,
        run_timeout: None,
        compile_timeout: None,
        run_memory_limit: None,
        compile_memory_limit: None,
    };

    println!("Submitting Batch Run Job...");
    let port = std::env::var("TURBO_SERVER_PORT").unwrap_or_else(|_| "3000".to_string());
    let url = format!("http://localhost:{}/api/v1/execute", port);
    let res = client.post(&url).json(&req).send().await?;

    if !res.status().is_success() {
        eprintln!("Error: {}", res.text().await?);
        return Ok(());
    }

    let result: JobResult = res.json().await?;

    if let Some(compile) = &result.compile {
        if compile.status != turbo_core::models::StageStatus::Success {
            println!("Compilation Failed!");
            println!("Status: {:?}", compile.status);
            println!("Stdout: {}", compile.stdout);
            println!("Stderr: {}", compile.stderr);
        }
    }

    if let Some(testcases) = result.testcases {
        println!("Testcases: {}", testcases.len());
        for tc in testcases {
            let status_str = if tc.passed { "PASSED" } else { "FAILED" };
            println!("  [{}] Testcase {}:", status_str, tc.id);
            println!("      Status: {:?}", tc.run_details.status);
            println!("      Stdout: '{}'", tc.run_details.stdout.trim());
            println!("      Stderr: '{}'", tc.run_details.stderr.trim());
            if !tc.passed {
                println!("      Expected: (hidden/unknown from here)");
                println!("      Actual:   '{}'", tc.actual_output);
            }
        }
    } else {
        println!("No testcases returned.");
        if let Some(run) = result.run {
            println!("Global Run Error (e.g. Sandbox Init):");
            println!("  Status: {:?}", run.status);
            println!("  Stderr: {}", run.stderr);
        }
    }

    Ok(())
}
