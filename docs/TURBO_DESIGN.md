# Turbo Project Plan: High-Performance Execution Engine in Rust

## 1. Executive Summary
**Turbo** is a cutting-edge, high-performance code execution engine built in **Rust**. It is designed as a standalone, innovative successor to Piston, abandoning legacy compatibility in favor of maximum performance, scalability, and modern features. Turbo utilizes a **monorepo** architecture and leverages native Linux technologies for sandboxing.

**Key Objectives:**
- **Cutting Edge Performance**: Native `cgroups` and namespaces via `libcontainer`.
- **Scalability**: Redis-backed job queues and SQLite for metadata.
- **Modern Package Management**: Container-image-like layered packages with incremental updates and verifying local manifests.
- **Unified API**: A brand new `v1` API consolidating execution features.

---

## 2. Technical Stack
- **Language**: Rust (2024 edition)
- **Async Runtime**: `tokio` (industry standard for async I/O)
- **Web Framework**: `axum` (high performance, ergonomic)
- **Serialization**: `serde` / `serde_json`
- **Error Handling**: `thiserror` (libs), `anyhow` (apps)
- **Sandbox**: `libcontainer` / `cgroups-rs` (Native Rust implementation)
- **Database**: 
  - **Job Queue**: `redis` (High throughput)
  - **Metadata**: `sqlite` (Structured queries for packages/runtimes)

---

## 3. Monorepo Architecture
Managed via Cargo Workspace.

```text
turbo/
├── Cargo.toml          # Workspace root
├── apps/
│   ├── turbo-server/   # Unified API Server
│   └── turbo-cli/      # Administration & Package Management Tool
└── crates/
    ├── turbo-core/     # Common types, configuration, error handling
    ├── turbo-box/      # Native Linux Sandbox implementation
    ├── turbo-pkg/      # Layered Package Management logic
    └── turbo-db/       # Database connectors (Redis/SQLite)
```

---

## 4. Module Specifications

### 4.1. `crates/turbo-core`
- **Functionality**:
  - `Job`, `Package`, `Runtime` definitions.
  - Configuration via `turbo.toml` (Env vars: `TURBO_*`).
- **Design**:
  - Strongly typed configuration.
  - Optimized data structures for zero-copy deserialization.

### 4.2. `crates/turbo-box` (Native Executor)
Implements high-performance isolation without shelling out to external binaries.
- **Functionality**:
  - Direct manipulation of Linux Namespaces and Cgroups v2.
  - Fine-grained resource control (CPU shares, Memory pages, OOM killing).
  - **Networking**: Configurable per-box networking (enabled/disabled/allowlist).
- **Innovation**:
  - **Snapshotting**: Future capability to snapshot process state for instant "hot starts".
  - **Internal Pool**: Pre-created cgroups/namespaces waiting for jobs.

### 4.3. `crates/turbo-pkg` (Modern Packaging)
A completely new package system inspired by OCI container images.
- **Format**:
  - Packages are composed of **layers** (tarballs).
  - **Multi-Version Support**: Fully supports side-by-side installation of multiple versions (e.g., Python 3.10 and 3.12). Common base layers (like shared libs) are deduplicated on disk, but each version has its own independent manifest.
  - **Incremental Updates**: Download only changed layers when updating a language.
- **Repositories**:
- **Repositories**:
  - **Definitions**: Stored in a structured directory: `packages/<language>/<version>/`.
  - **Versioning**: Users install by name (`python`), defaulting to the latest available version in the repository.
  - **Local Registry**: Installed runtimes live in `~/.turbo/runtimes`.

### 4.4. `crates/turbo-db`
- **Functionality**:
  - Abstraction layer over Redis (for queues) and SQLite (for persistence).
  - Schema migrations for SQLite.

### 4.5. `apps/turbo-server` (Unified API)
Exposes the generic `v1` API.
- **Endpoints**:
  - `POST /api/v1/execute`: 
    - Supports single run, batch run, compiled.
    - **Hybrid Streaming**:
        - **SSE (Server-Sent Events)**: Default for standard execution (stdout/stderr streaming). Lower overhead for high concurrency.
        - **WebSockets**: Used ONLY for interactive sessions (Bi-directional REPL requirements).
  - `GET /api/v1/runtimes`: List installed runtimes (grouped by language, listing all versions).
  - `GET /api/v1/packages`: List available packages (remote/local).
- **Features**:
  - **Result Caching**: Cache execution results for identical inputs (optional, Configurable).
  - **Observability**: Metrics (Prometheus) and Tracing (OpenTelemetry) built-in.

### 4.6. `apps/turbo-cli`
- **Commands**:
  - `turbo start`: Launch server.
  - `turbo pkg install <name>[:version] [--local <path>]`: Install a package. Defaults to latest if version omitted.
  - `turbo pkg list [--online]`: List installed or available remote packages.
  - `turbo pkg update`: Update all packages (incremental).
  - `turbo gc`: Prune unused layers and boxes.

---

## 5. Execution Pipeline (The "Turbo" Engine)

1.  **Ingest**: `turbo-server` receives `POST /v1/execute`.
2.  **Queue**: Job pushed to Redis queue.
3.  **Dispatch**: Worker thread pulls job.
4.  **Prepare**:
    - `turbo-box` allocates a "warm" sandbox.
    - Mounts necessary package layers (OverlayFS) into the rootfs.
5.  **Execute**:
    - "Compile Once" (if needed).
    - "Run Many" (Batch execution loops internally within the sandbox for max speed).
6.  **Report**: Results stream back via SSE (or WS if interactive).

---

## 6. Implementation Stages

### Phase 1: Core & Native Sandbox
- **Goal**: Run a simple binary inside a Rust-managed Cgroup/Namespace.
- **Deliverables**: `turbo-core`, `turbo-box` (initial implementation using `libcontainer`).

### Phase 2: The New Package Format
- **Goal**: Create the layer-based package system.
- **Deliverables**: `turbo-pkg` implementation, `turbo-cli` for creating/installing local packages.

### Phase 3: Turbo Server (API v1)
- **Goal**: HTTP/SSE/WS server processing jobs.
- **Deliverables**: `turbo-server` (Axum), Redis integration, API definition.

### Phase 4: Polish & Scale
- **Goal**: Production readiness.
- **Deliverables**: CLI "online" search, Prometheus metrics, Documentation.

---

## 7. Configuration Strategy
- **File**: `turbo.toml`
- **Environment**: `TURBO_REDIS_URL`, `TURBO_LOG_LEVEL`, etc.
- **No legacy `PISTON_` support**.
