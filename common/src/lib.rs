use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub id: usize,
    pub input: String,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseResult {
    pub id: usize,
    pub worker_id: Uuid,
    pub passed: bool,
    pub actual_output: String,
    pub error: String,
    pub time: String,
    pub memory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobData {
    Compile {
        language: String,
        code: String,
    },
    Execute {
        language: String,
        binary: Option<Vec<u8>>, // For compiled languages
        code: Option<String>,    // For interpreted languages
        testcases: Vec<TestCase>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub data: JobData,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobResultData {
    Compile {
        success: bool,
        message: String,
        binary: Option<Vec<u8>>,
    },
    Execute {
        results: Vec<TestCaseResult>,
    },
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub job_id: Uuid,
    pub worker_id: Uuid,
    pub data: JobResultData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Heartbeat { worker_id: Uuid, port: u16 },
    JobRequest(Job),
    JobCompleted(JobResult),
}
