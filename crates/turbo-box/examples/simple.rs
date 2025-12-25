use turbo_box::{LinuxSandbox, Sandbox};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = LinuxSandbox::new("/tmp/turbo-box".to_string());

    println!("Initializing sandbox...");
    sandbox.init("test-01").await?;

    println!("Running echo...");
    let result = sandbox
        .run(
            "test-01",
            "echo",
            &["Hello from Turbo!".to_string()],
            &[],
            None,
        )
        .await?;

    println!("Result: {:?}", result);

    sandbox.cleanup("test-01").await?;
    Ok(())
}
