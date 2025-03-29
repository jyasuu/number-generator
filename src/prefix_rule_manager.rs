use async_trait::async_trait;
use std::error::Error;
use crate::prefix_rule::PrefixRule;

#[async_trait]
pub trait PrefixRuleManager {
    async fn register_prefix(&self, prefix_key: String, prefix_rule: PrefixRule) -> Result<(), Box<dyn Error>>;
    async fn get_prefix(&self, prefix_key: String) -> Result<Option<PrefixRule>, Box<dyn Error>>;
}
