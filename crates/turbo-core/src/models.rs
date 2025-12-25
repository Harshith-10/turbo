use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRequest {
    pub language: String,
    pub version: Option<String>,
    pub files: Vec<FileRequest>,
    pub testcases: Option<Vec<Testcase>>,
    pub args: Option<Vec<String>>,
    pub stdin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRequest {
    pub name: Option<String>,
    pub content: String,
    pub encoding: Option<String>, // "base64", "hex", or "utf8" (default)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Testcase {
    pub id: String,
    pub input: String,
    pub expected_output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub language: String,
    pub version: String,
    pub run: Option<StageResult>,
    pub compile: Option<StageResult>,
    pub testcases: Option<Vec<TestcaseResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLimits {
    pub memory_limit_bytes: u64,
    pub pid_limit: u64,
    pub file_limit: u64,
    pub timeout_ms: u64,
    pub output_limit_bytes: u64,
    pub uid: Option<u32>, // User ID to switch to
    pub gid: Option<u32>, // Group ID to switch to
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            memory_limit_bytes: 512 * 1024 * 1024, // 512 MB
            pid_limit: 256,
            file_limit: 2048,
            timeout_ms: 3000, // 3s
            output_limit_bytes: 1024, // 1KB
            uid: None, // Default to no switch (or root if started as root) until configured
            gid: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StageStatus {
    Pending,
    Running,
    Success,
    RuntimeError,
    CompilationError,
    TimeLimitExceeded,
    MemoryLimitExceeded,
    OutputLimitExceeded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub status: StageStatus,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub signal: Option<String>,
    pub memory_usage: Option<u64>,
    pub cpu_time: Option<u64>,
    pub execution_time: Option<u64>, // Wall-clock time in ms
}

impl std::fmt::Display for StageResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Status: {:?}", self.status)?;
        if let Some(code) = self.exit_code {
            writeln!(f, "Exit Code: {}", code)?;
        }
        if let Some(signal) = &self.signal {
            writeln!(f, "Signal: {}", signal)?;
        }
        
        if let Some(mem) = self.memory_usage {
            let (val, unit) = if mem > 1024 * 1024 * 1024 {
                (mem as f64 / 1024.0 / 1024.0 / 1024.0, "GB")
            } else if mem > 1024 * 1024 {
                (mem as f64 / 1024.0 / 1024.0, "MB")
            } else if mem > 1024 {
                (mem as f64 / 1024.0, "KB")
            } else {
                (mem as f64, "B")
            };
            writeln!(f, "Memory Usage: {:.2} {}", val, unit)?;
        }
        
        if let Some(cpu) = self.cpu_time {
            let (val, unit) = if cpu > 1_000_000 {
                (cpu as f64 / 1_000_000.0, "s")
            } else if cpu > 1_000 {
                (cpu as f64 / 1_000.0, "ms")
            } else {
                (cpu as f64, "µs")
            };
            writeln!(f, "CPU Time: {:.2} {}", val, unit)?;
        }

        if let Some(exec) = self.execution_time {
             let (val, unit) = if exec > 1_000 {
                (exec as f64 / 1_000.0, "s")
            } else {
                (exec as f64, "ms")
            };
            writeln!(f, "Execution Time: {:.2} {}", val, unit)?;
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestcaseResult {
    pub id: String,
    pub passed: bool,
    pub actual_output: String,
    pub run_details: StageResult,
}
