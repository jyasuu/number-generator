use async_trait::async_trait;
use crate::prefix_rule::PrefixRule;

#[async_trait]
pub trait PrefixRuleManager {
    async fn register_prefix_rule(&self, prefix_rule: PrefixRule) -> Result<(), String>;
    async fn get_prefix_rule(&self, prefix_key: &str) -> Result<Option<PrefixRule>, String>;
}
