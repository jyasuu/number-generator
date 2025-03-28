use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrefixRule {
    pub prefix_key: String,
    pub format: String,
    pub seq_length: u32,
    pub initial_seq: u64,
}

pub trait PrefixRuleManager {
    fn register_prefix_rule(&self, prefix_rule: PrefixRule) -> Result<(), Box<dyn Error>>;
    fn get_prefix_rule(&self, prefix_key: &str) -> Result<Option<PrefixRule>, Box<dyn Error>>;
}

#[async_trait]
pub trait AsyncPrefixRuleManager {
    async fn register_prefix_rule(&self, prefix_rule: PrefixRule) -> Result<(), Box<dyn Error>>;
    async fn get_prefix_rule(&self, prefix_key: &str) -> Result<Option<PrefixRule>, Box<dyn Error>>;
}
