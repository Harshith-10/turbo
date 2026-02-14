use anyhow::Result;
use redis::AsyncCommands;
use turbo_core::models::Runtime;

#[derive(Clone)]
pub struct RedisMetadataStore {
    client: redis::Client,
}

impl RedisMetadataStore {
    pub fn new(client: redis::Client) -> Self {
        Self { client }
    }

    pub async fn add_runtime(&self, runtime: &Runtime) -> Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = "turbo:runtimes";
        let json = serde_json::to_string(runtime)?;
        let field_key = format!("{}:{}", runtime.language, runtime.version);
        let _: () = conn.hset(key, field_key, json).await?;
        Ok(())
    }

    pub async fn get_runtimes(&self) -> Result<Vec<Runtime>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = "turbo:runtimes";
        let map: std::collections::HashMap<String, String> = conn.hgetall(key).await?;

        let runtimes = map
            .values()
            .filter_map(|json| serde_json::from_str(json).ok())
            .collect();
        Ok(runtimes)
    }
}
