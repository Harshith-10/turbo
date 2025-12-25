use turbo_box::linux::LinuxSandbox;
use turbo_box::traits::Sandbox;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let box_impl = LinuxSandbox::new("/tmp/turbo-root".to_string());

    let id = "verify_output";
    box_impl.init(id).await?;

    // 2. Run 'yes' to generate tons of output (Should be capped)
    println!("Running 'yes' (Expect truncated output)...");
    // "yes" runs forever, so it will hit timeout AND cap output.
    // Or we can use head to limit it but we want to test output cap on the internal reader.
    // Let's rely on timeout to kill 'yes' but check if stdout size is <= 1024.

    let mut limits = turbo_core::models::ExecutionLimits::default();
    limits.output_limit_bytes = 1024; // 1KB

    let result = box_impl.run(id, "yes", &[], &[], Some(limits)).await?;

    println!("Stdout Length: {}", result.stdout.len());

    if result.stdout.len() <= 1024 {
        println!("PASS: Output Cap working (len: {}).", result.stdout.len());
    } else {
        println!(
            "FAIL: Output Cap NOT working (len: {}).",
            result.stdout.len()
        );
    }

    box_impl.cleanup(id).await?;

    Ok(())
}
