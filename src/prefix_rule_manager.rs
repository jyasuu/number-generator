use async_trait::async_trait;
use std::error::Error;
use std::fmt::Debug;

use crate::prefix_rule::PrefixRule;

#[async_trait]
pub trait PrefixRuleManager: Debug + Send + Sync {
    async fn register_prefix(&self, prefix_rule: PrefixRule) -> Result<(), Box<dyn Error>>;
    async fn get_prefix(&self, prefix_key: &str) -> Result<Option<PrefixRule>, Box<dyn Error>>;
}
