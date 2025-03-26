use std::sync::{Mutex, Arc};
use std::collections::HashMap;

pub trait SequenceGenerator {
    fn generate(&self, prefix_key: &str) -> Result<u64, String>;
}

pub struct InMemorySequenceGenerator {
    sequences: Arc<Mutex<HashMap<String, u64>>>,
    local_cache: Arc<Mutex<HashMap<String, Vec<u64>>>>,
    cache_size: usize,
}

impl InMemorySequenceGenerator {
    pub fn new(cache_size: usize) -> Self {
        InMemorySequenceGenerator {
            sequences: Arc::new(Mutex::new(HashMap::new())),
            local_cache: Arc::new(Mutex::new(HashMap::new())),
            cache_size,
        }
    }

    fn fill_cache(&self, prefix_key: &str) -> Result<(), String> {
        let mut cache = self.local_cache.lock().map_err(|e| e.to_string())?;
        let mut sequences = self.sequences.lock().map_err(|e| e.to_string())?;

        let next_val = sequences.entry(prefix_key.to_string()).or_insert(0);
        let mut values = Vec::with_capacity(self.cache_size);
        for _ in 0..self.cache_size {
            *next_val += 1;
            values.push(*next_val);
        }
        cache.insert(prefix_key.to_string(), values);
        Ok(())
    }
}

impl SequenceGenerator for InMemorySequenceGenerator {
    fn generate(&self, prefix_key: &str) -> Result<u64, String> {
        let mut cache = self.local_cache.lock().map_err(|e| e.to_string())?;
        if let Some(values) = cache.get_mut(prefix_key) {
            if !values.is_empty() {
                return Ok(values.remove(0));
            }
        }

        // Cache is empty, try to fill it
        self.fill_cache(prefix_key)?;

        // Try again to get a value from the cache
        if let Some(values) = cache.get_mut(prefix_key) {
            if !values.is_empty() {
                return Ok(values.remove(0));
            }
        }

        // If we reach here, it means something is wrong
        Err("Failed to generate sequence number".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_generate_sequence() {
        let generator = InMemorySequenceGenerator::new(10);
        let prefix_key = "TEST".to_string();

        let result1 = generator.generate(&prefix_key).unwrap();
        let result2 = generator.generate(&prefix_key).unwrap();

        assert_eq!(result1, 1);
        assert_eq!(result2, 2);
    }

    #[test]
    fn test_concurrent_sequence_generation() {
        let generator = Arc::new(InMemorySequenceGenerator::new(100));
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
