use turbo_box::linux::LinuxSandbox;
use turbo_box::traits::Sandbox;
use turbo_core::models::StageStatus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let box_impl = LinuxSandbox::new("/tmp/turbo-root".to_string());

    let id = "verify_user";
    box_impl.init(id).await?;

    // Limits with UID set to 65534 (nobody)
    let mut limits = turbo_core::models::ExecutionLimits::default();
    limits.uid = Some(65534);
    limits.gid = Some(65534);

    println!("Running 'id' as user 65534 (Expect uid=65534(nobody))...");

    let result = box_impl.run(id, "id", &[], &[], Some(limits)).await?;

    println!("Stdout: {}", result.stdout);

    if result.stdout.contains("uid=65534") {
        println!("PASS: Process ran as user 65534.");
    } else {
        println!("FAIL: Process did NOT run as user 65534.");
    }

    box_impl.cleanup(id).await?;

    Ok(())
}
