use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use redis::{Client, RedisError, AsyncCommands};
use serde_json;
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::prefix_rule_manager::PrefixRuleManager;
use crate::prefix_rule::PrefixRule;

const LOCAL_CACHE_SIZE: usize = 1000;

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
    local_cache: Arc<Mutex<HashMap<String, PrefixRule>>>,
}

impl RedisPrefixRuleManager {
    pub fn new(redis_url: String) -> Result<Self, Box<dyn std::error::Error + Send>> {
        let redis_client = Client::open(redis_url).map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to connect to Redis: {}", e))) as Box<dyn std::error::Error + Send>)?;
        let prefix_rules = Arc::new(Mutex::new(HashMap::new()));
        let local_cache = Arc::new(Mutex::new(HashMap::with_capacity(LOCAL_CACHE_SIZE)));
        Ok(RedisPrefixRuleManager {
            redis_client,
            prefix_rules,
            local_cache,
        })
    }

    fn get_redis_key(prefix_key: &str) -> String {
        format!("prefix_rule:{}", prefix_key)
    }

    async fn get_prefix_rule_from_redis(&self, prefix_key: String) -> Result<Option<PrefixRule>, Box<dyn std::error::Error + Send>> {
        let mut conn = self.redis_client.get_async_connection().await.map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send>)?;
        let redis_key = Self::get_redis_key(&prefix_key);
        let prefix_rule_json: Option<String> = conn.get(redis_key).await.map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send>)?;

        match prefix_rule_json {
            Some(json) => {
                let prefix_rule: PrefixRule = serde_json::from_str(&json).map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send>)?;
                Ok(Some(prefix_rule))
            }
            None => Ok(None),
        }
    }
}

#[async_trait]
impl PrefixRuleManager for RedisPrefixRuleManager {
    async fn register_prefix_rule(&self, prefix_key: String, prefix_rule: PrefixRule) -> Result<(), Box<dyn std::error::Error + Send>> {
        let mut conn = self.redis_client.get_async_connection().await.map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send>)?;
        let redis_key = Self::get_redis_key(&prefix_key);
        let prefix_rule_json = serde_json::to_string(&prefix_rule).map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send>)?;
        conn.set(redis_key, prefix_rule_json).await.map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send>)?;

        let mut cache = self.local_cache.lock().await;
        cache.insert(prefix_key.clone(), prefix_rule.clone());

        Ok(())
    }

    async fn get_prefix_rule(&self, prefix_key: String) -> Result<Option<PrefixRule>, Box<dyn std::error::Error + Send>> {
        // 1. Try to get from local cache
        let mut cache = self.local_cache.lock().await;
        if let Some(rule) = cache.get(&prefix_key) {
            return Ok(Some(rule.clone()));
        }

        // 2. If not in cache, try to get from Redis
        match self.get_prefix_rule_from_redis(prefix_key.clone()).await {
            Ok(Some(rule)) => {
                // 3. Store in local cache
                cache.insert(prefix_key.clone(), rule.clone());
                Ok(Some(rule))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                // Attempt to reconnect to Redis
                eprintln!("Error getting prefix from Redis: {}. Retrying...", e);
                sleep(Duration::from_secs(1)).await;
                match self.get_prefix_rule_from_redis(prefix_key.clone()).await {
                    Ok(Some(rule)) => {
                        // 3. Store in local cache
                        cache.insert(prefix_key.clone(), rule.clone());
                        Ok(Some(rule))
                    }
                    Ok(None) => Ok(None),
                    Err(e) => {
                        eprintln!("Error getting prefix from Redis after retry: {}", e);
                        Err(e)
                    }
                }
            }
        }
    }
}
