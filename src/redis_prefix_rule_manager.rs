use async_trait::async_trait;
use redis::{Client, Commands, ConnectionLike};
use serde_json;
use std::error::Error;

use crate::prefix_rule_manager::{AsyncPrefixRuleManager, PrefixRule, PrefixRuleManager};

#[derive(Clone, Debug)]
pub struct RedisPrefixRuleManager {
    client: Client,
    redis_url: String,
}

impl RedisPrefixRuleManager {
    pub fn new(redis_url: String) -> Result<Self, Box<dyn Error>> {
        let client = Client::open(redis_url.clone())?;
        client.get_connection()?;
        Ok(RedisPrefixRuleManager {
            client,
            redis_url,
        })
    }
}

impl PrefixRuleManager for RedisPrefixRuleManager {
    fn register_prefix_rule(&self, prefix_rule: PrefixRule) -> Result<(), Box<dyn Error>> {
        let mut conn = self.client.get_connection()?;
        let key = format!("prefix_rule:{}", prefix_rule.prefix_key);
        let value = serde_json::to_string(&prefix_rule)?;
        conn.set(key, value)?;
        Ok(())
    }

    fn get_prefix_rule(&self, prefix_key: &str) -> Result<Option<PrefixRule>, Box<dyn Error>> {
        let mut conn = self.client.get_connection()?;
        let key = format!("prefix_rule:{}", prefix_key);
        let result: Option<String> = conn.get(key)?;
        match result {
            Some(value) => {
                let prefix_rule: PrefixRule = serde_json::from_str(&value)?;
                Ok(Some(prefix_rule))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redis::Commands;
    use crate::prefix_rule_manager::PrefixRuleManager;

    #[test]
    fn test_register_and_get_prefix_rule() -> Result<(), Box<dyn Error>> {
        let redis_url = "redis://127.0.0.1/";
        let manager = RedisPrefixRuleManager::new(redis_url.to_string())?;

        let prefix_rule = PrefixRule {
            prefix_key: "TEST".to_string(),
            format: "TEST-{SEQ:4}".to_string(),
            seq_length: 4,
            initial_seq: 1,
        };

        PrefixRuleManager::register_prefix_rule(&manager, prefix_rule.clone())?;

        let retrieved_prefix_rule = PrefixRuleManager::get_prefix_rule(&manager, "TEST")?.unwrap();
        assert_eq!(prefix_rule.prefix_key, retrieved_prefix_rule.prefix_key);
        assert_eq!(prefix_rule.format, retrieved_prefix_rule.format);
        assert_eq!(prefix_rule.seq_length, retrieved_prefix_rule.seq_length);
        assert_eq!(prefix_rule.initial_seq, retrieved_prefix_rule.initial_seq);

        // Clean up the test data
        let mut conn = manager.client.get_connection()?;
        let key = format!("prefix_rule:{}", prefix_rule.prefix_key);
        conn.del(key.as_str())?;

        Ok(())
    }

    #[test]
    fn test_get_non_existent_prefix_rule() -> Result<(), Box<dyn Error>> {
        let redis_url = "redis://127.0.0.1/";
        let manager = RedisPrefixRuleManager::new(redis_url.to_string())?;

        let retrieved_prefix_rule = PrefixRuleManager::get_prefix_rule(&manager, "NON_EXISTENT")?;
        assert!(retrieved_prefix_rule.is_none());

        Ok(())
    }

    #[test]
    fn test_redis_connection_error() {
        let redis_url = "redis://127.0.0.1:1234/";
        let result = RedisPrefixRuleManager::new(redis_url.to_string());
        assert!(!result.is_ok());
    }
}

#[async_trait]
impl AsyncPrefixRuleManager for RedisPrefixRuleManager {
    async fn register_prefix_rule(&self, prefix_rule: PrefixRule) -> Result<(), Box<dyn Error>> {
        let client = redis::Client::open(self.redis_url.clone())?;
        let mut conn = client.get_async_connection().await?;
        let key = format!("prefix_rule:{}", prefix_rule.prefix_key);
        let value = serde_json::to_string(&prefix_rule)?;
        redis::cmd("SET").arg(&[&key, &value]).query_async(&mut conn).await?;
        Ok(())
    }

    async fn get_prefix_rule(&self, prefix_key: &str) -> Result<Option<PrefixRule>, Box<dyn Error>> {
        let client = redis::Client::open(self.redis_url.clone())?;
        let mut conn = client.get_async_connection().await?;
        let key = format!("prefix_rule:{}", prefix_key);
        let result: Option<String> = redis::cmd("GET").arg(&[&key]).query_async(&mut conn).await?;

        match result {
            Some(value) => {
                let prefix_rule: PrefixRule = serde_json::from_str(&value)?;
                Ok(Some(prefix_rule))
            }
            None => Ok(None),
        }
    }
}
