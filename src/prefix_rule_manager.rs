use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Debug, Clone)]
pub struct PrefixConfig {
    pub format: String,
    pub seq_length: u32,
    pub initial_seq: u64,
}

pub trait PrefixRuleManager {
    fn register_prefix(&self, prefix_key: String, config: PrefixConfig) -> Result<(), String>;
    fn get_prefix_config(&self, prefix_key: &str) -> Result<Option<PrefixConfig>, String>;
}

pub struct InMemoryPrefixRuleManager {
    configs: RwLock<HashMap<String, PrefixConfig>>,
}

impl InMemoryPrefixRuleManager {
    pub fn new() -> Self {
        InMemoryPrefixRuleManager {
            configs: RwLock::new(HashMap::new()),
        }
    }
}

impl PrefixRuleManager for InMemoryPrefixRuleManager {
    fn register_prefix(&self, prefix_key: String, config: PrefixConfig) -> Result<(), String> {
        let mut configs = self.configs.write().map_err(|e| e.to_string())?;
        configs.insert(prefix_key, config);
        Ok(())
    }

    fn get_prefix_config(&self, prefix_key: &str) -> Result<Option<PrefixConfig>, String> {
        let configs = self.configs.read().map_err(|e| e.to_string())?;
        Ok(configs.get(prefix_key).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get_prefix() {
        let manager = InMemoryPrefixRuleManager::new();
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
}
