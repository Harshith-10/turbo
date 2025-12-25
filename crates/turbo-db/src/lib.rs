pub mod metadata;
pub mod queue;

pub use metadata::SqliteMetadataStore;
pub use queue::{QueueError, RedisQueue};

#[derive(Clone)]
pub struct TurboDb {
    pub queue: RedisQueue,
    pub metadata: SqliteMetadataStore,
}

impl TurboDb {
    pub async fn new(redis_url: &str, sqlite_url: &str) -> anyhow::Result<Self> {
        let queue = RedisQueue::new(redis_url)?;
        let metadata = SqliteMetadataStore::new(sqlite_url).await?;
        Ok(Self { queue, metadata })
    }
}
