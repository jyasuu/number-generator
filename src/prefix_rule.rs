use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefixRule {
    pub prefix_key: String,
    pub format: String,
    pub seq_length: u32,
    pub initial_seq: u64,
    pub network_partition: bool,
}
