use regex::Regex;
use std::collections::HashMap;

use crate::prefix_rule_manager::PrefixConfig;

pub struct NumberAssembler {}

impl NumberAssembler {
    pub fn new() -> Self {
        NumberAssembler {}
    }

    pub fn assemble_number(
        &self,
        prefix: &str,
        config: &PrefixConfig,
        sequence: u64,
    ) -> Result<String, String> {
        let mut replacements: HashMap<String, String> = HashMap::new();
        replacements.insert("prefix".to_string(), prefix.to_string());

        let year = chrono::Utc::now().format("%Y").to_string();
        replacements.insert("year".to_string(), year);

        let seq_formatted = format!("{:0width$}", sequence, width = config.seq_length as usize);
        replacements.insert("SEQ".to_string(), seq_formatted);

        let mut formatted_number = config.format.clone();

        let re = Regex::new(r"\{([A-Za-z0-9_]+)(?::(\d+))?\}").unwrap();
        for capture in re.captures_iter(&config.format) {
            let full_match = capture.get(0).unwrap().as_str();
            let variable_name = capture.get(1).unwrap().as_str();

            if let Some(replacement_value) = replacements.get(variable_name) {
                formatted_number = formatted_number.replace(full_match, replacement_value);
            }
        }

        Ok(formatted_number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefix_rule_manager::PrefixConfig;

    #[test]
    fn test_assemble_number() {
        let assembler = NumberAssembler::new();
        let prefix = "TEST";
        let config = PrefixConfig {
            format: "TEST-{year}-{SEQ:4}".to_string(),
            seq_length: 4,
            initial_seq: 1,
        };
        let sequence = 123;

        let assembled_number = assembler.assemble_number(prefix, &config, sequence).unwrap();
        let expected_number = format!("TEST-{}-0123", chrono::Utc::now().format("%Y"));

        assert_eq!(assembled_number, expected_number);
    }

    #[test]
    fn test_assemble_number_with_prefix_only() {
        let assembler = NumberAssembler::new();
        let prefix = "ORDER";
        let config = PrefixConfig {
            format: "{prefix}-{SEQ:6}".to_string(),
            seq_length: 6,
            initial_seq: 1,
        };
        let sequence = 456;

        let assembled_number = assembler.assemble_number(prefix, &config, sequence).unwrap();
        assert_eq!(assembled_number, "ORDER-000456");
    }
}
