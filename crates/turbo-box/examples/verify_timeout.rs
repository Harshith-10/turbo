use turbo_box::linux::LinuxSandbox;
use turbo_box::traits::Sandbox;
use turbo_core::models::StageStatus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let box_impl = LinuxSandbox::new("/tmp/turbo-root".to_string());

    let id = "verify_timeout";
    box_impl.init(id).await?;

    let mut limits = turbo_core::models::ExecutionLimits::default();
    limits.timeout_ms = 3000; // 3s

    println!("Running 'sleep 10' (Expect TimeLimitExceeded)...");
    let result = box_impl
        .run(id, "sleep", &["10".to_string()], &[], Some(limits))
        .await?;

    println!("Result:\n{}", result);

    if result.status == StageStatus::TimeLimitExceeded {
        println!("PASS: Status is TimeLimitExceeded.");
    } else {
        println!("FAIL: Expected TimeLimitExceeded, got {:?}", result.status);
    }

    box_impl.cleanup(id).await?;

    Ok(())
}
