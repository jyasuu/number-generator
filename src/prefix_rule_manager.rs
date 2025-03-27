use redis::{Client, Commands};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefixConfig {
    pub format: String,
    pub seq_length: u32,
    pub initial_seq: u64,
}

pub trait PrefixRuleManager {
    fn register_prefix(&self, prefix_key: String, config: PrefixConfig) -> Result<(), PrefixRuleError>;
    fn get_prefix_config(&self, prefix_key: &str) -> Result<Option<PrefixConfig>, PrefixRuleError>;
}

#[derive(Debug, Clone)]
pub struct RedisPrefixRuleManager {
    redis_client: Client,
}

#[derive(Debug, thiserror::Error)]
pub enum PrefixRuleError {
    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    // #[error("Prefix not found: {0}")]
    // PrefixNotFound(String),
    #[error("Prefix already exists: {0}")]
    PrefixAlreadyExists(String),
    // #[error("Invalid prefix format: {0}")]
    // InvalidPrefixFormat(String),
}


impl RedisPrefixRuleManager {
    pub fn new(redis_url: &str) -> Result<Self, PrefixRuleError> {
        let redis_client = Client::open(redis_url)?;
        Ok(RedisPrefixRuleManager { redis_client })
    }

    fn get_redis_key(&self, prefix_key: &str) -> String {
        format!("prefix_config:{}", prefix_key)
    }
}

impl PrefixRuleManager for RedisPrefixRuleManager {
    fn register_prefix(&self, prefix_key: String, config: PrefixConfig) -> Result<(), PrefixRuleError> {
        let mut conn = self.redis_client.get_connection()?;
        let redis_key = self.get_redis_key(&prefix_key);

        // Check if the prefix already exists
        if let Ok(true) = conn.exists(&redis_key) {
            return Err(PrefixRuleError::PrefixAlreadyExists(prefix_key));
        }

        let config_json = serde_json::to_string(&config)?;
        println!("config_json: {}", config_json);
        conn.set::<_, _, ()>(&redis_key, config_json)?;
        println!("Prefix registered successfully");
        Ok(())
    }

    fn get_prefix_config(&self, prefix_key: &str) -> Result<Option<PrefixConfig>, PrefixRuleError> {
        let mut conn = self.redis_client.get_connection()?;
        let redis_key = self.get_redis_key(prefix_key);

        let config_json: Option<String> = conn.get(&redis_key)?;

        match config_json {
            Some(json) => {
                let config: PrefixConfig = serde_json::from_str(&json)?;
                Ok(Some(config))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redis::Connection;

    // Helper function to clear Redis before each test
    fn clear_redis(conn: &mut Connection) {
        redis::cmd("FLUSHDB").execute(conn);
    }

    #[test]
    fn test_register_and_get_prefix() {
        let redis_url = "redis://127.0.0.1/";
        let manager = RedisPrefixRuleManager::new(redis_url).unwrap();
        let mut conn = manager.redis_client.get_connection().unwrap();
        clear_redis(&mut conn);

        let prefix_key = "TEST".to_string();
        let config = PrefixConfig {
            format: "TEST-{SEQ:4}".to_string(),
            seq_length: 4,
            initial_seq: 1,
        };

        manager.register_prefix(prefix_key.clone(), config.clone()).unwrap();

        let retrieved_config = manager.get_prefix_config(&prefix_key).unwrap();
        assert_eq!(retrieved_config.unwrap().format, config.format);
    }

    #[test]
    fn test_get_nonexistent_prefix() {
        let redis_url = "redis://127.0.0.1/";
        let manager = RedisPrefixRuleManager::new(redis_url).unwrap();
        let mut conn = manager.redis_client.get_connection().unwrap();
        clear_redis(&mut conn);

        let prefix_key = "NONEXISTENT".to_string();
        let retrieved_config = manager.get_prefix_config(&prefix_key).unwrap();
        assert!(retrieved_config.is_none());
    }

    #[test]
    fn test_register_duplicate_prefix() {
        let redis_url = "redis://127.0.0.1/";
        let manager = RedisPrefixRuleManager::new(redis_url).unwrap();
        let mut conn = manager.redis_client.get_connection().unwrap();
        clear_redis(&mut conn);

        let prefix_key = "DUPLICATE".to_string();
        let config = PrefixConfig {
            format: "DUPLICATE-{SEQ:4}".to_string(),
            seq_length: 4,
            initial_seq: 1,
        };

        manager.register_prefix(prefix_key.clone(), config.clone()).unwrap();

        let result = manager.register_prefix(prefix_key.clone(), config.clone());
        assert!(result.is_err());
        match result.err().unwrap() {
            PrefixRuleError::PrefixAlreadyExists(_) => {},
            _ => panic!("Expected PrefixAlreadyExists error"),
        }
    }
}
