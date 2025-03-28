use async_trait::async_trait;
use redis::{Client, Commands};
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
