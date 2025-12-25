use turbo_box::linux::LinuxSandbox;
use turbo_box::traits::Sandbox;
use turbo_core::models::StageStatus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let box_impl = LinuxSandbox::new("/tmp/turbo-root".to_string());

    let id = "verify_memory";
    box_impl.init(id).await?;

    let cmd = "perl";
    let script = "$a = \"x\" x (2048 * 1024 * 1024);"; // 2GB, no sleep

    println!("Running Memory Hog (Expect MemoryLimitExceeded)...");

    let mut limits = turbo_core::models::ExecutionLimits::default();
    limits.memory_limit_bytes = 512 * 1024 * 1024; // 512 MB

    let result = box_impl
        .run(
            id,
            cmd,
            &["-e".to_string(), script.to_string()],
            &[],
            Some(limits),
        )
        .await;

    match result {
        Ok(res) => {
            println!("Result:\n{}", res);
            if res.status == StageStatus::MemoryLimitExceeded {
                println!("PASS: Status is MemoryLimitExceeded.");
            } else {
                println!("FAIL: Expected MemoryLimitExceeded, got {:?}", res.status);
            }
        }
        Err(e) => {
            println!("ERROR: Process failed with error: {}", e);
        }
    }

    box_impl.cleanup(id).await?;

    Ok(())
}
