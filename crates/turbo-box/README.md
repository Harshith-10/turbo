# Turbo Box

`turbo-box` is a library crate providing secure sandboxing capabilities for the Turbo project. It leverages Linux Cgroups v2 and Namespaces to isolate processes and limit their resource usage.

## Features

- **Cgroup v2 Support**: Manages memory, generic I/O, and PID limits directly via the cgroup v2 filesystem.
- **Metric Collection**: Accurately tracks memory usage and CPU time for executed processes.
- **Namespace Isolation**: Uses `unshare` to isolate PID, Network, IPC, UTS, and Mount namespaces.
- **Resource Limits**: Configurable limits for Memory, CPU, PIDs, and File Descriptors.

## Usage

```rust
use turbo_box::{LinuxSandbox, Sandbox};

#[tokio::main]
async fn main() {
    let sandbox = LinuxSandbox::new("/var/turbo/sandbox".to_string());
    
    // Initialize sandbox for a job
    sandbox.init("job-123").await.unwrap();
    
    // Run a command
    let result = sandbox.run(
        "job-123", 
        "echo", 
        &["hello".to_string()], 
        &[], 
        None
    ).await.unwrap();
    
    println!("Output: {}", result.stdout);
    
    // Cleanup
    sandbox.cleanup("job-123").await.unwrap();
}
```

## Requirements

- Linux Kernel with Cgroup v2 enabled.
- Root privileges (to create cgroups and unshare namespaces).
