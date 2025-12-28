# Turbo Code Execution Engine

Turbo is a high-performance, secure code execution engine built in Rust. It provides a sandboxed environment to compile and execute code in various programming languages, making it suitable for competitive programming platforms, online IDEs, and educational tools.

## 🚀 Key Features

### Performance ⚡
Turbo is designed for speed and efficiency:
- **Asynchronous Architecture**: Built on top of **Tokio** and **Axum**, enabling high-concurrency handling of execution requests without blocking system resources.
- **Smart Caching**: Implements compilation caching to avoid recompiling identical code submissions, significantly reducing latency for popular problems.
- **Efficient Resource Management**: Uses an LRU (Least Recently Used) eviction policy to manage disk space for cached artifacts.

### Security 🔒
Security is paramount when running untrusted user code. Turbo leverages modern Linux kernel features to ensure robust isolation:
- **Cgroups V2**: strictly limits resource usage (CPU, Memory, PIDs) for each execution to prevent DoS attacks.
- **Linux Namespaces**: Uses `unshare` to create isolated environments for:
    - **Network** (`CLONE_NEWNET`): Completely disables network access.
    - **Process IDs** (`CLONE_NEWPID`): Hides other system processes.
    - **Mounts** (`CLONE_NEWNS`): Provides a restricted file system view.
    - **IPC** (`CLONE_NEWIPC`): Prevents inter-process communication.
- **Resource Limits**: Enforces `RLIMIT_NOFILE` and other limits via `setrlimit`.
- **Swap Disabled**: Prevents swapping to allow accurate memory usage tracking and prevent system thrashing.
- **Output Capping**: Prevents log flooding by enforcing strict limits on `stdout` and `stderr` size.

## 🛠️ Architecture

- **`turbo-server`**: The HTTP API server handling requests and job queuing.
- **`turbo-core`**: Core data models and traits.
- **`turbo-box`**: The sandboxing implementation using Linux primitives.
- **`turbo-db`**: Database layer for job state management.
- **`turbo-pkg`**: Package manager for handling language runtimes.

## 🏁 Getting Started

### Prerequisites
- Linux OS with Cgroups v2 enabled (Unified Hierarchy).
- Rust (latest stable).
- Root privileges (required for creating cgroups and namespaces).

### Running the Server

1. **Build the project:**
   ```bash
   cargo build --release -p turbo-server
   ```

2. **Run the server (requires sudo):**
   ```bash
   sudo ./target/release/turbo-server
   ```
   *Note: Sudo is strictly required to initialize the sandbox environment.*

3. **Check Status:**
   The server listens on `0.0.0.0:3000` by default.

## 📖 Documentation

- [API Documentation](docs/api.md) - Detailed guide to the REST API endpoints.
- [Design Architecture](docs/TURBO_DESIGN.md) - Deep dive into system design.
- [Code Styles](docs/CODE_STYLES.md) - Contribution guidelines.

## 📦 Project Structure

```
.
├── apps
│   ├── turbo-cli       # Command-line interface tool
│   └── turbo-server    # Main API server
├── crates
│   ├── turbo-box       # Low-level sandboxing logic
│   ├── turbo-core      # Shared types and interfaces
│   ├── turbo-db        # Data persistence layer
│   └── turbo-pkg       # Package management
├── docs                # Project documentation
└── packages            # Language runtime definitions
```
