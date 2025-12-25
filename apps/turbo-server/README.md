# Turbo Server

Turbo Server is the core execution engine for the Turbo project. It provides an HTTP API to submit code execution jobs, manages the job queue, and orchestrates the execution of code within secure sandboxes.

## Features

- **Queue-based Execution**: Jobs are queued in Redis and processed by background workers.
- **Secure Sandboxing**: Uses `turbo-box` (cgroups v2, namespaces) to isolate code execution.
- **Scalable**: Designed to run multiple workers to handle high throughput.
- **Language Support**: Supports multiple languages via `turbo-pkg` definitions.

## Getting Started

### Prerequisites

- Rust (latest stable)
- Redis and SQLite (managed by `turbo-server`)
- Root privileges (for Cgroup management)

### Running the Server

```bash
# Must be run as root for sandbox capabilities
sudo TURBO_HOME=/path/to/home target/debug/turbo-server
```

## Architecture

- **`api/`**: Axum-based HTTP API for job submission (`POST /submit`).
- **`worker.rs`**: Background worker that pulls jobs from Redis, prepares the sandbox, runs the code, and tests outputs.
- **`main.rs`**: Application entry point, initializes configuration, database, and spawns the API server and worker.
