use serde::{Deserialize, Serialize};

pub mod number_assembler;
pub mod prefix_rule_manager;
pub mod redis_prefix_rule_manager;
pub mod sequence_generator;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrefixRule {
    pub prefix_key: String,
    pub format: String,
    pub seq_length: u32,
    pub initial_seq: u64,
}

use std::error::Error;

pub trait PrefixRuleManager {
    fn register_prefix_rule(&self, prefix_rule: PrefixRule) -> Result<(), Box<dyn Error>>;
    fn get_prefix_rule(&self, prefix_key: &str) -> Result<Option<PrefixRule>, Box<dyn Error>>;
}
