use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TurboConfig {
    pub server: ServerConfig,
    pub sandbox: SandboxConfig,
    pub redis: RedisConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
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
            .set_default("server.port", 3000)?
            .set_default("server.log_level", "INFO")?
            .set_default("sandbox.max_concurrent_jobs", 64)?
            .set_default("sandbox.memory_limit_mb", 512)?
            .set_default("redis.url", "redis://127.0.0.1:6379")?
            .set_default("database.url", "sqlite://turbo.db")?
            // Merge turbo.toml if exists
            .add_source(config::File::with_name("turbo").required(false))
            // Merge environment variables (TURBO_*)
            .add_source(config::Environment::with_prefix("TURBO").separator("_"));

        builder.build()?.try_deserialize()
    }
}
