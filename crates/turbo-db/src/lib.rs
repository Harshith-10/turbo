pub mod metadata;
pub mod queue;

pub use metadata::RedisMetadataStore;
pub use queue::{QueueError, RedisQueue};

#[derive(Clone)]
pub struct TurboDb {
    pub queue: RedisQueue,
    pub metadata: RedisMetadataStore,
}

impl TurboDb {
    pub async fn new(redis_url: &str) -> anyhow::Result<Self> {
        let queue = RedisQueue::new(redis_url)?;
        let client = redis::Client::open(redis_url)?;
        let metadata = RedisMetadataStore::new(client);
        Ok(Self { queue, metadata })
    }
}
