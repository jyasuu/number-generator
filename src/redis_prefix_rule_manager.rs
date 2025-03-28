use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use redis::{Client, Commands, RedisError};
use serde_json;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::prefix_rule_manager::PrefixRuleManager;
use crate::prefix_rule::PrefixRule;

#[derive(Debug, Error)]
pub enum RedisPrefixRuleManagerError {
    #[error("Redis error: {0}")]
    RedisError(#[from] RedisError),
    #[error("Prefix rule not found: {0}")]
    PrefixRuleNotFound(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Debug)]
pub struct RedisPrefixRuleManager {
    redis_client: Client,
    prefix_rules: Arc<Mutex<HashMap<String, PrefixRule>>>,
}

impl RedisPrefixRuleManager {
    pub fn new(redis_url: String) -> Result<Self, Box<dyn std::error::Error>> {
        let redis_client = Client::open(redis_url).map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to connect to Redis: {}", e))))?;
        let prefix_rules = Arc::new(Mutex::new(HashMap::new()));
        Ok(RedisPrefixRuleManager {
            redis_client,
            prefix_rules,
        })
    }

    fn get_redis_key(prefix_key: &str) -> String {
        format!("prefix_rule:{}", prefix_key)
    }
}

#[async_trait]
impl PrefixRuleManager for RedisPrefixRuleManager {
    async fn register_prefix(&self, prefix_rule: PrefixRule) -> Result<(), Box<dyn std::error::Error>> {
        let mut conn = self.redis_client.get_connection().map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        let redis_key = Self::get_redis_key(&prefix_rule.prefix_key);
        let prefix_rule_json = serde_json::to_string(&prefix_rule).map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        conn.set(redis_key, prefix_rule_json).map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        Ok(())
    }

    async fn get_prefix(&self, prefix_key: &str) -> Result<Option<PrefixRule>, Box<dyn std::error::Error>> {
        let mut conn = self.redis_client.get_connection().map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        let redis_key = Self::get_redis_key(prefix_key);
        let prefix_rule_json: Option<String> = conn.get(redis_key).map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        match prefix_rule_json {
            Some(json) => {
                let prefix_rule: PrefixRule = serde_json::from_str(&json).map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
                Ok(Some(prefix_rule))
            }
            None => Ok(None),
        }
    }
}
