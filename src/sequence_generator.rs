use std::sync::{Mutex, Arc};
use std::collections::HashMap;

pub trait SequenceGenerator {
    fn generate(&self, prefix_key: &str) -> Result<u64, String>;
}

pub struct InMemorySequenceGenerator {
    sequences: Arc<Mutex<HashMap<String, u64>>>,
}

impl InMemorySequenceGenerator {
    pub fn new() -> Self {
        InMemorySequenceGenerator {
            sequences: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl SequenceGenerator for InMemorySequenceGenerator {
    fn generate(&self, prefix_key: &str) -> Result<u64, String> {
        let mut sequences = self.sequences.lock().map_err(|e| e.to_string())?;
        let next_val = sequences.entry(prefix_key.to_string()).or_insert(0);
        *next_val += 1;
        Ok(*next_val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_generate_sequence() {
        let generator = InMemorySequenceGenerator::new();
        let prefix_key = "TEST".to_string();

        let result1 = generator.generate(&prefix_key).unwrap();
        let result2 = generator.generate(&prefix_key).unwrap();

        assert_eq!(result1, 1);
        assert_eq!(result2, 2);
    }

    #[test]
    fn test_concurrent_sequence_generation() {
        let generator = Arc::new(InMemorySequenceGenerator::new());
        let prefix_key = "TEST".to_string();
        let num_threads = 10;
        let num_iterations = 100;

        let mut handles = vec![];

        for _ in 0..num_threads {
            let generator_clone = generator.clone();
            let prefix_key_clone = prefix_key.clone();

            let handle = thread::spawn(move || {
                for _ in 0..num_iterations {
                    generator_clone.generate(&prefix_key_clone).unwrap();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let expected_count = (num_threads * num_iterations) as u64;
        let mut sequences = generator.sequences.lock().unwrap();
        let final_count = sequences.get(&prefix_key).unwrap();

        assert_eq!(*final_count, expected_count);
    }
}
