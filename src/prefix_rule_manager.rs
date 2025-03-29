use async_trait::async_trait;
use std::error::Error;

use crate::prefix_rule::PrefixRule;

#[async_trait]
pub trait PrefixRuleManager {
    async fn register_prefix_rule(&self, prefix_key: String, rule: PrefixRule) -> Result<(), Box<dyn Error + Send>>;
    async fn get_prefix_rule(&self, prefix_key: String) -> Result<Option<PrefixRule>, Box<dyn Error + Send>>;
}
