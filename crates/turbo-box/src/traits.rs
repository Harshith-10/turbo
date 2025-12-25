use async_trait::async_trait;
use turbo_core::{StageResult, Result, ExecutionLimits};

#[async_trait]
pub trait Sandbox: Send + Sync {
    /// Initialize the sandbox (create files, checking resources)
    async fn init(&self, id: &str) -> Result<()>;

    /// Run a command inside the sandbox
    async fn run(&self, id: &str, cmd: &str, args: &[String], env: &[String], limits: Option<ExecutionLimits>) -> Result<StageResult>;

    /// Cleanup the sandbox resources
    async fn cleanup(&self, id: &str) -> Result<()>;
}
