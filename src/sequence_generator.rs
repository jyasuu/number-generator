use async_trait::async_trait;
use redis::{Client, RedisError};
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
    prefix_rule_manager: Arc<Mutex<dyn PrefixRuleManager + Send + Sync>>,
}

impl RedisSequenceGenerator {
    pub fn new(redis_url: String, prefix_rule_manager: Arc<Mutex<dyn PrefixRuleManager + Send + Sync>>) -> Result<Self, SequenceGeneratorError> {
        let redis_client = Client::open(redis_url).map_err(|e| SequenceGeneratorError::Other(format!("Failed to connect to Redis: {}", e)))?;
        Ok(RedisSequenceGenerator {
            redis_client,
            prefix_rule_manager,
        })
    }
}

#[async_trait]
impl SequenceGenerator for RedisSequenceGenerator {
    async fn generate(&self, prefix_key: &str) -> Result<u64, SequenceGeneratorError> {
        let mut conn = self.redis_client.get_async_connection().await?;
        let next_sequence: u64 = redis::cmd("INCR")
            .arg(format!("seq:{}", prefix_key))
            .query_async(&mut conn)
            .await?;
        Ok(next_sequence)
    }
}
