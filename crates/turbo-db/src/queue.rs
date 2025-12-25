use futures_util::StreamExt;
use redis::AsyncCommands;
// use serde::{Deserialize, Serialize};
use turbo_core::models::{Job, JobResult};

#[derive(thiserror::Error, Debug)]
pub enum QueueError {
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[derive(Clone)]
pub struct RedisQueue {
    client: redis::Client,
}

impl RedisQueue {
    pub fn new(redis_url: &str) -> Result<Self, QueueError> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self { client })
    }

    pub async fn push_job(&self, job: Job) -> Result<(), QueueError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let job_json = serde_json::to_string(&job)?;
        let _: () = conn.rpush("turbo:jobs", job_json).await?;
        Ok(())
    }

    pub async fn pop_job(&self) -> Result<Option<Job>, QueueError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let result: Option<(String, String)> = conn.blpop("turbo:jobs", 0.0).await?;
        match result {
            Some((_queue, job_json)) => {
                let job = serde_json::from_str(&job_json)?;
                Ok(Some(job))
            }
            None => Ok(None),
        }
    }

    pub async fn publish_result(&self, job_id: &str, result: &JobResult) -> Result<(), QueueError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let json = serde_json::to_string(result)?;
        let _: () = conn.publish(format!("turbo:job:{}", job_id), &json).await?;
        let _: () = conn
            .set_ex(format!("turbo:result:{}", job_id), json, 3600_u64)
            .await?;
        Ok(())
    }

    pub async fn wait_for_result(&self, job_id: &str) -> Result<JobResult, QueueError> {
        #[allow(deprecated)]
        let conn = self.client.get_async_connection().await?;
        let mut pubsub = conn.into_pubsub();
        pubsub.subscribe(format!("turbo:job:{}", job_id)).await?;

        // Check existing
        let mut multiplexed = self.client.get_multiplexed_async_connection().await?;
        let existing: Option<String> = multiplexed.get(format!("turbo:result:{}", job_id)).await?;
        if let Some(json) = existing {
            return Ok(serde_json::from_str(&json)?);
        }

        if let Some(msg) = pubsub.on_message().next().await {
            let payload: String = msg.get_payload()?;
            return Ok(serde_json::from_str(&payload)?);
        }

        Err(QueueError::Redis(redis::RedisError::from((
            redis::ErrorKind::IoError,
            "Stream ended",
        ))))
    }
}
