# Turbo Project Code Styles & Practices

This document outlines the coding standards, architectural guidelines, and best practices for the Turbo project. The goal is to ensure high performance, maintainability, and consistency across the codebase.

## 1. General Philosophy
- **Simplicity**: Write code that is easy to understand. Avoid over-engineering.
- **Performance**: We are building a high-performance execution engine. Avoid unnecessary allocations and cloning.
- **Safety**: Leverage Rust's type system to ensure memory safety and correctness.

## 2. Code Formatting & Linting
- **Formatting**: All code must be formatted using `rustfmt`.
    - Run `cargo fmt` before committing.
- **Linting**: We strictly follow `clippy` suggestions.
    - Run `cargo clippy -- -D warnings` to ensure no warnings exist.
    - Treat warnings as errors in CI.

## 3. Architecture & Modularization
The project follows a **Monorepo** structure managed by a Cargo Workspace.

### Directory Structure
- `apps/`: Application binaries (e.g., `turbo-server`, `turbo-cli`).
- `crates/`: Reusable library crates (e.g., `turbo-core`, `turbo-box`).

### Rules
- **Dependency Direction**: `apps` depend on `crates`. `crates` should generally not depend on `apps`.
- **Modularity**: Break down functionality into small, focused crates.
- **Public API**: minimize `pub` visibility. Only expose what is necessary.

## 4. Testing Strategy
Testing is mandatory. **Every module must have tests.**

### Unit Tests
- Place unit tests in the same file as the code, within a `tests` module.
- Test strict logic paths and edge cases.
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_functionality() {
        // ...
    }
}
```

### Integration Tests
- Place integration tests in the `tests/` directory of the crate.
- Test the public API of the crate as an external consumer.

## 5. Error Handling
- **Libraries (`crates/`)**: Use `thiserror` to define custom, strongly-typed errors.
- **Applications (`apps/`)**: Use `anyhow` for flexible error handling in binaries.
- Avoid `unwrap()` and `expect()` in production code. Use `?` propagation or handle errors explicitly.

## 6. Asynchronous Programming
- Use `tokio` as the async runtime.
- Be mindful of blocking operations. Use `tokio::task::spawn_blocking` for CPU-intensive or blocking I/O tasks.

## 7. Documentation
- Document all public structs, enums, and functions using doc comments (`///`).
- Include examples in doc comments where complex usage is involved.
- Every crate must have a `README.md` explaining its purpose.
