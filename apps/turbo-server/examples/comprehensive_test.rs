use colored::*;
use turbo_core::models::{FileRequest, JobRequest, JobResult, StageStatus, Testcase};

#[derive(Debug)]
struct TestConfig {
    name: String,
    language: String,
    version: String,
    files: Vec<FileRequest>,
    expected_status: StageStatus,
    description: String,
    stdin: Option<String>,
    args: Option<Vec<String>>,
    // Check if output contains specific string
    expected_output_contains: Option<String>,
    // Check if stderr contains specific string
    expected_stderr_contains: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let port = std::env::var("TURBO_SERVER_PORT").unwrap_or_else(|_| "3000".to_string());
    let url = format!("http://localhost:{}/api/v1/execute", port);

    let tests = vec![
        // ==========================================
        // PYTHON TESTS
        // ==========================================
        TestConfig {
            name: "Python: Hello World".to_string(),
            language: "python".to_string(),
            version: "3.14.2".to_string(),
            files: vec![FileRequest {
                name: Some("main.py".to_string()),
                content: "print('Hello Python')".to_string(),
                encoding: Some("utf8".to_string()),
            }],
            expected_status: StageStatus::Success,
            description: "Basic happy path test".to_string(),
            stdin: None,
            args: Some(vec!["main.py".to_string()]),
            expected_output_contains: Some("Hello Python".to_string()),
            expected_stderr_contains: None,
        },
        TestConfig {
            name: "Python: Syntax Error".to_string(),
            language: "python".to_string(),
            version: "3.14.2".to_string(),
            files: vec![FileRequest {
                name: Some("main.py".to_string()),
                content: "print('Missing closing quote)".to_string(),
                encoding: Some("utf8".to_string()),
            }],
            expected_status: StageStatus::RuntimeError, // Python syntax errors are often runtime errors in the sense that the script runs and fails immediately, or compilation failure if strictly compiled. For interpreted, it usually returns exit code 1. Let's see how system handles it. Actually, for python, it's usually a runtime error from the perspective of "run" stage if we consider "compile" stage as empty. Let's assume Runtime Error for now.
            description: "Code with invalid syntax".to_string(),
            stdin: None,
            args: Some(vec!["main.py".to_string()]),
            expected_output_contains: None,
            expected_stderr_contains: Some("SyntaxError".to_string()),
        },
        TestConfig {
            name: "Python: Runtime Error (ZeroDivision)".to_string(),
            language: "python".to_string(),
            version: "3.14.2".to_string(),
            files: vec![FileRequest {
                name: Some("main.py".to_string()),
                content: "print(1/0)".to_string(),
                encoding: Some("utf8".to_string()),
            }],
            expected_status: StageStatus::RuntimeError,
            description: "Runtime exception".to_string(),
            stdin: None,
            args: Some(vec!["main.py".to_string()]),
            expected_output_contains: None,
            expected_stderr_contains: Some("ZeroDivisionError".to_string()),
        },
        TestConfig {
            name: "Python: Timeout".to_string(),
            language: "python".to_string(),
            version: "3.14.2".to_string(),
            files: vec![FileRequest {
                name: Some("main.py".to_string()),
                content: "while True: pass".to_string(),
                encoding: Some("utf8".to_string()),
            }],
            expected_status: StageStatus::TimeLimitExceeded,
            description: "Infinite loop that should timeout".to_string(),
            stdin: None,
            args: Some(vec!["main.py".to_string()]),
            expected_output_contains: None,
            expected_stderr_contains: None,
        },
        TestConfig {
            name: "Python: Stdin Input".to_string(),
            language: "python".to_string(),
            version: "3.14.2".to_string(),
            files: vec![FileRequest {
                name: Some("main.py".to_string()),
                content: "import sys; print(f'Received: {sys.stdin.read().strip()}')".to_string(),
                encoding: Some("utf8".to_string()),
            }],
            expected_status: StageStatus::Success,
            description: "Reading from stdin".to_string(),
            stdin: Some("SecretMessage".to_string()),
            args: Some(vec!["main.py".to_string()]),
            expected_output_contains: Some("Received: SecretMessage".to_string()),
            expected_stderr_contains: None,
        },
        // ==========================================
        // JAVA TESTS
        // ==========================================
        TestConfig {
            name: "Java: Hello World".to_string(),
            language: "java".to_string(),
            version: "25.0.1".to_string(),
            files: vec![FileRequest {
                name: Some("Main.java".to_string()),
                content: r#"
                    public class Main {
                        public static void main(String[] args) {
                            System.out.println("Hello Java");
                        }
                    }
                "#
                .to_string(),
                encoding: Some("utf8".to_string()),
            }],
            expected_status: StageStatus::Success,
            description: "Basic happy path test".to_string(),
            stdin: None,
            args: Some(vec!["Main.java".to_string()]), // The wrapper script handles compilation
            expected_output_contains: Some("Hello Java".to_string()),
            expected_stderr_contains: None,
        },
        TestConfig {
            name: "Java: Compile Error".to_string(),
            language: "java".to_string(),
            version: "25.0.1".to_string(),
            files: vec![FileRequest {
                name: Some("Main.java".to_string()),
                content: r#"
                    public class Main {
                        public static void main(String[] args) {
                            System.out.println("Missing semicolon")
                        }
                    }
                "#
                .to_string(),
                encoding: Some("utf8".to_string()),
            }],
            // This depends on how the run.sh is implemented.
            // If run.sh compiles and runs in one go, a compile error might result in Runtime Error or just text in stderr.
            // Ideally, the system should allow separating compile stage, but for now we often treat it as 'run' failing.
            // Let's expect RuntimeError or CompilationError depending on implementation.
            // If the script fails, it's usually RuntimeError from the harness perspective unless explicitly separated.
            // However, let's assume if it fails to compile, the exit code is non-zero.
            expected_status: StageStatus::RuntimeError,
            description: "Code that fails to compile".to_string(),
            stdin: None,
            args: Some(vec!["Main.java".to_string()]),
            expected_output_contains: None,
            expected_stderr_contains: Some("error:".to_string()), // Java compiler usually says "error:"
        },
        TestConfig {
            name: "Java: Runtime Error (Exception)".to_string(),
            language: "java".to_string(),
            version: "25.0.1".to_string(),
            files: vec![FileRequest {
                name: Some("Main.java".to_string()),
                content: r#"
                    public class Main {
                        public static void main(String[] args) {
                            throw new RuntimeException("Boom");
                        }
                    }
                "#
                .to_string(),
                encoding: Some("utf8".to_string()),
            }],
            expected_status: StageStatus::RuntimeError,
            description: "Unhandled exception".to_string(),
            stdin: None,
            args: Some(vec!["Main.java".to_string()]),
            expected_output_contains: None,
            expected_stderr_contains: Some("Exception in thread".to_string()),
        },
    ];

    println!("{}", "\nStarting Comprehensive Test Suite...".bold().blue());
    println!("Target URL: {}", url);

    let mut passed_count = 0;
    let mut failed_count = 0;

    for test in tests {
        println!("{}", "-".repeat(50));
        println!("Running Test: {}", test.name.bold());
        println!("Description: {}", test.description.italic());

        let req = JobRequest {
            language: test.language.clone(),
            version: Some(test.version.clone()),
            files: test.files.clone(),
            testcases: None, // Single run mode for these
            args: test.args.clone(),
            stdin: test.stdin.clone(),
            run_timeout: if test.expected_status == StageStatus::TimeLimitExceeded {
                Some(1000)
            } else {
                None
            }, // Short timeout for timeout tests
            compile_timeout: None,
            run_memory_limit: None,
            compile_memory_limit: None,
        };

        let res = client.post(&url).json(&req).send().await;

        match res {
            Ok(response) => {
                if !response.status().is_success() {
                    println!(
                        "{} Server returned error: {}",
                        "FAILED".red(),
                        response.status()
                    );
                    if let Ok(text) = response.text().await {
                        println!("Body: {}", text);
                    }
                    failed_count += 1;
                    continue;
                }

                let job_result: JobResult = response.json().await?;

                // Analyze results
                let run_stage = job_result.run.as_ref();
                let status = run_stage
                    .map(|r| r.status.clone())
                    .unwrap_or(StageStatus::RuntimeError);
                let stdout = run_stage.map(|r| r.stdout.clone()).unwrap_or_default();
                let stderr = run_stage.map(|r| r.stderr.clone()).unwrap_or_default();

                let mut passed = true;
                let mut reasons = Vec::new();

                // Check Status
                if status != test.expected_status {
                    passed = false;
                    reasons.push(format!(
                        "Expected status {:?}, got {:?}",
                        test.expected_status, status
                    ));
                }

                // Check Stdout
                if let Some(expected_out) = &test.expected_output_contains {
                    if !stdout.contains(expected_out) {
                        passed = false;
                        reasons.push(format!("Stdout did not contain '{}'", expected_out));
                    }
                }

                // Check Stderr
                if let Some(expected_err) = &test.expected_stderr_contains {
                    if !stderr.contains(expected_err) {
                        passed = false;
                        reasons.push(format!("Stderr did not contain '{}'", expected_err));
                    }
                }

                if passed {
                    println!("{} {}", "PASSED".green(), test.name);
                    println!("  Actual Stdout: {}", stdout.trim());
                    println!("  Actual Stderr: {}", stderr.trim());
                    passed_count += 1;
                } else {
                    println!("{} {}", "FAILED".red(), test.name);
                    for reason in reasons {
                        println!("  - {}", reason);
                    }
                    println!("  Actual Stdout: {}", stdout.trim());
                    println!("  Actual Stderr: {}", stderr.trim());
                    failed_count += 1;
                }
            }
            Err(e) => {
                println!("{} Connection failed: {}", "ERROR".red(), e);
                failed_count += 1;
            }
        }
    }

    // ==========================================
    // BATCH TESTS
    // ==========================================
    println!("{}", "-".repeat(50));
    println!("Running Test: {}", "Batch Execution (Python)".bold());
    println!(
        "Description: {}",
        "Test batch processing with passing and failing cases".italic()
    );

    let batch_req = JobRequest {
        language: "python".to_string(),
        version: Some("3.14.2".to_string()),
        files: vec![FileRequest {
            name: Some("main.py".to_string()),
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

    let batch_res = client.post(&url).json(&batch_req).send().await;

    match batch_res {
        Ok(response) => {
            if !response.status().is_success() {
                println!(
                    "{} Batch Server returned error: {}",
                    "FAILED".red(),
                    response.status()
                );
                failed_count += 1;
            } else {
                let result: JobResult = response.json().await?;
                if let Some(tcs) = result.testcases {
                    let mut batch_passed = true;
                    // Check if we got 4 testcases
                    if tcs.len() != 4 {
                        println!("{} Expected 4 testcases, got {}", "FAILED".red(), tcs.len());
                        batch_passed = false;
                    }

                    // Check individual results
                    let map_res: std::collections::HashMap<_, _> =
                        tcs.iter().map(|tc| (tc.id.clone(), tc)).collect();

                    if let Some(tc) = map_res.get("1") {
                        if !tc.passed {
                            println!("  Testcase 1 failed unexpectedly");
                            batch_passed = false;
                        }
                    }
                    if let Some(tc) = map_res.get("4") {
                        if tc.passed {
                            println!("  Testcase 4 passed unexpectedly (should fail)");
                            batch_passed = false;
                        }
                    }

                    if batch_passed {
                        println!("{} {}", "PASSED".green(), "Batch Execution (Python)");
                        passed_count += 1;
                    } else {
                        println!("{} {}", "FAILED".red(), "Batch Execution (Python)");
                        failed_count += 1;
                    }
                } else {
                    println!("{} No testcases returned in batch mode", "FAILED".red());
                    failed_count += 1;
                }
            }
        }
        Err(e) => {
            println!("{} Batch Connection failed: {}", "ERROR".red(), e);
            failed_count += 1;
        }
    }

    println!("{}", "=".repeat(50));
    println!("Test Suite Completed");
    println!("Passed: {}", passed_count.to_string().green());
    println!("Failed: {}", failed_count.to_string().red());

    Ok(())
}
