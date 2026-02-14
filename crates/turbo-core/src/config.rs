use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TurboConfig {
    pub server: ServerConfig,
    pub sandbox: SandboxConfig,
    pub redis: RedisConfig,
    pub paths: PathsConfig,
}

#[derive(Debug, Deserialize)]
pub struct PathsConfig {
    /// Directory where runtimes are installed (e.g., /home/user/.turbo)
    pub turbo_home: String,
    /// Directory containing package definitions (e.g., ./packages)
    pub packages_path: String,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub log_level: String,
}

#[derive(Debug, Deserialize)]
pub struct SandboxConfig {
    pub max_concurrent_jobs: usize,
    pub memory_limit_mb: u64,
}

#[derive(Debug, Deserialize)]
pub struct RedisConfig {
    pub url: String,
}

impl TurboConfig {
    pub fn new() -> Result<Self, config::ConfigError> {
        let builder = config::Config::builder()
            // Start with defaults
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 4000)?
            .set_default("server.log_level", "INFO")?
            .set_default("sandbox.max_concurrent_jobs", 64)?
            .set_default("sandbox.memory_limit_mb", 512)?
            .set_default("redis.url", "redis://127.0.0.1:6379")?
            .set_default("paths.turbo_home", default_turbo_home())?
            .set_default("paths.packages_path", "./packages")?
            // Merge turbo.toml if exists
            .add_source(config::File::with_name("turbo").required(false))
            // Merge environment variables (TURBO_*)
            .add_source(config::Environment::with_prefix("TURBO").separator("_"));

        builder.build()?.try_deserialize()
    }
}

/// Returns a default turbo home directory.
/// Prefers TURBO_HOME env var, then $HOME/.turbo, then /var/lib/turbo as fallback.
fn default_turbo_home() -> String {
    if let Ok(turbo_home) = std::env::var("TURBO_HOME") {
        return turbo_home;
    }
    if let Ok(home) = std::env::var("HOME") {
        return format!("{}/.turbo", home);
    }
    // Fallback for when running as root with no HOME set
    "/var/lib/turbo".to_string()
}
