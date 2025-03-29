use async_trait::async_trait;
use redis::{Client, RedisError, AsyncCommands};
use std::{sync::Arc, fmt,sync::Mutex};

use crate::prefix_rule_manager::PrefixRuleManager;

#[async_trait]
pub trait SequenceGenerator {
    async fn generate(&self, prefix_key: &str) -> Result<u64, SequenceGeneratorError>;
}

#[derive(Debug)]
pub enum SequenceGeneratorError {
    RedisError(RedisError),
    PrefixNotFound,
    Other(String),
}

impl From<RedisError> for SequenceGeneratorError {
    fn from(err: RedisError) -> Self {
        SequenceGeneratorError::RedisError(err)
    }
}

impl fmt::Display for SequenceGeneratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SequenceGeneratorError::RedisError(e) => write!(f, "Redis error: {}", e),
            SequenceGeneratorError::PrefixNotFound => write!(f, "Prefix not found"),
            SequenceGeneratorError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

pub struct RedisSequenceGenerator {
    redis_client: Client,
    // prefix_rule_manager: Arc<Mutex<dyn PrefixRuleManager + Send + Sync>>, // Not used in this implementation
}

impl RedisSequenceGenerator {
    pub fn new(redis_url: String) -> Result<Self, SequenceGeneratorError> {
        let redis_client = Client::open(redis_url).map_err(|e| SequenceGeneratorError::Other(format!("Failed to connect to Redis: {}", e)))?;
        Ok(RedisSequenceGenerator {
            redis_client,
            // prefix_rule_manager,
        })
    }
}


#[async_trait]
impl SequenceGenerator for RedisSequenceGenerator {
    async fn generate(&self, prefix_key: &str) -> Result<u64, SequenceGeneratorError> {
        // Concurrency Control Strategy:
        // This implementation uses Redis atomic INCR operation for concurrency control.
        // Redis INCR provides atomic increment, ensuring that sequence numbers are generated
        // uniquely and continuously even under high concurrency. This strategy prioritizes
        // low latency and high throughput, but allows for slight number skipping in case of Redis failures.
        let mut conn = self.redis_client.get_async_connection().await?;
        let next_sequence: u64 = conn.incr(format!("seq:{}", prefix_key), 1).await?;
        Ok(next_sequence)
    }
}
