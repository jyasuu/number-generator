use redis::{AsyncCommands, RedisResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;
use std::future::Future;
use prometheus::{Registry, IntCounter, Histogram, TextEncoder, Encoder};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PrefixConfig {
    format: String,
    seq_length: usize,
    initial_seq: u64,
}

#[derive(Debug)]
struct PrefixNotConfigured;

impl warp::reject::Reject for PrefixNotConfigured {}

#[derive(Debug)]
struct MissingPrefixKey;

impl warp::reject::Reject for MissingPrefixKey {}

#[derive(Debug, Serialize)]
struct NumberResponse {
    number: String,
}

#[derive(Clone)]
struct NumberGenerator {
    redis_client: redis::Client,
    prefix_configs: HashMap<String, PrefixConfig>,
    local_cache: Arc<Mutex<HashMap<String, PrefixConfig>>>,
    metrics: Arc<Metrics>,
}

#[allow(dead_code)]
#[derive(Clone)]
struct Metrics {
    registry: Registry,
    requests_total: IntCounter,
    successful_generations: IntCounter,
    failed_generations: IntCounter,
    lock_acquisitions: IntCounter,
    lock_failures: IntCounter,
    generation_latency: Histogram,
}

impl Metrics {
    fn new(registry: &Registry) -> Self {
        let requests_total = IntCounter::new(
            "number_generator_requests_total",
            "Total number of number generation requests"
        ).unwrap();
        
        let successful_generations = IntCounter::new(
            "number_generator_successful_generations_total",
            "Total number of successful number generations"
        ).unwrap();
        
        let failed_generations = IntCounter::new(
            "number_generator_failed_generations_total",
            "Total number of failed number generations"
        ).unwrap();
        
        let lock_acquisitions = IntCounter::new(
            "number_generator_lock_acquisitions_total",
            "Total number of successful lock acquisitions"
        ).unwrap();
        
        let lock_failures = IntCounter::new(
            "number_generator_lock_failures_total",
            "Total number of failed lock acquisitions"
        ).unwrap();
        
        let generation_latency = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "number_generator_generation_latency_seconds",
                "Latency of number generation in seconds"
            )
        ).unwrap();

        registry.register(Box::new(requests_total.clone())).unwrap();
        registry.register(Box::new(successful_generations.clone())).unwrap();
        registry.register(Box::new(failed_generations.clone())).unwrap();
        registry.register(Box::new(lock_acquisitions.clone())).unwrap();
        registry.register(Box::new(lock_failures.clone())).unwrap();
        registry.register(Box::new(generation_latency.clone())).unwrap();

        Self {
            registry: registry.clone(),
            requests_total,
            successful_generations,
            failed_generations,
            lock_acquisitions,
            lock_failures,
            generation_latency,
        }
    }
}

#[allow(unused_variables)]
impl NumberGenerator {
    async fn new(redis_url: &str) -> RedisResult<Self> {
        let client = redis::Client::open(redis_url)?;
        let registry = Registry::new();
        let metrics = Arc::new(Metrics::new(&registry));
        let local_cache = Arc::new(Mutex::new(HashMap::new()));
        let mut num_retries = 5;
        let mut redis_client = None;
        while num_retries > 0 {
            match redis::Client::open(redis_url) {
                Ok(client) => {
                    redis_client = Some(client);
                    break;
                }
                Err(e) => {
                    println!("Failed to connect to Redis: {}, retrying...", e);
                    num_retries -= 1;
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }

        let redis_client = match redis_client {
            Some(client) => client,
            None => return Err(redis::RedisError::from((redis::ErrorKind::ClientError, "Failed to connect to Redis after multiple retries"))),
        };
        Ok(Self {
            redis_client: redis_client,
            prefix_configs: HashMap::new(),
            local_cache,
            metrics,
        })
    }

    async fn generate_number(&mut self, prefix_key: &str) -> RedisResult<String> {
        println!("Checking prefix configuration for: {}", prefix_key);
        let config = {
            let cache = self.local_cache.lock().await;
            match cache.get(prefix_key) {
                Some(config) => {
                    println!("Found configuration in local cache: {:?}", config);
                    config.clone()
                }
                None => {
                    println!("Prefix not found in local cache, checking Redis...");
                    let mut con = self.redis_client.get_async_connection().await?;
                    let config_json: Option<String> = con.get(format!("prefix_config:{}", prefix_key)).await?;
                    match config_json {
                        Some(json) => {
                            println!("Found configuration in Redis: {}", json);
                            let config: PrefixConfig = serde_json::from_str(&json).map_err(|e| {
                                redis::RedisError::from((redis::ErrorKind::TypeError,"", e.to_string()))
                            })?;
                            let mut cache = self.local_cache.lock().await;
                            cache.insert(prefix_key.to_string(), config.clone());
                            config
                        }
                        None => {
                            println!("Prefix not found in Redis");
                            return Err(redis::RedisError::from((
                                redis::ErrorKind::TypeError,
                                "Prefix not configured",
                            )));
                        }
                    }
                }
            }
        };

        println!("Connecting to Redis...");
        let mut con = match self.redis_client.get_async_connection().await {
            Ok(conn) => {
                println!("Successfully connected to Redis");
                conn
            }
            Err(e) => {
                println!("Failed to connect to Redis: {:?}", e);
                return Err(e);
            }
        };

        // Acquire distributed lock
        let lock_key = format!("lock:{}", prefix_key);
        let lock_id = uuid::Uuid::new_v4().to_string();
        let lock_acquired: bool = con.set_nx(&lock_key, &lock_id).await?;

        if !lock_acquired {
            println!("Failed to acquire lock for prefix: {}", prefix_key);
            return Err(redis::RedisError::from((
                redis::ErrorKind::TryAgain,
                "Failed to acquire lock",
            )));
        }

        // Set lock expiration
        con.expire(&lock_key, 5).await?;

        println!("Generating sequence...");
        let seq_future : std::pin::Pin<Box<dyn Future<Output = Result<i32, redis::RedisError>> + Send>> = con.incr(format!("seq:{}", prefix_key), 1) ;
        let seq = seq_future.await;
        let seq = match seq
        {
            Ok(seq) => {
                println!("Successfully generated sequence: {:?}", seq);
                seq
            }
            Err(e) => {
                println!("Failed to generate sequence: {:?}", e);
                return Err(e);
            }
        };

        // Release lock
        let current_lock_id: String = con.get(&lock_key).await?;
        if current_lock_id == lock_id {
            con.del(&lock_key).await?;
        }

        let number = config.format
            .replace("{SEQ}", &format!("{:0>width$}", seq, width = config.seq_length))
            .replace("{year}", &chrono::Local::now().format("%Y").to_string());
        println!("Formatted number: {}", number);
        Ok(number)
    }

    async fn register_prefix(&mut self, prefix_key: String, config: PrefixConfig) -> RedisResult<()> {
        let mut con = self.redis_client.get_async_connection().await?;
        let config_json = serde_json::to_string(&config).map_err(|e| {
            redis::RedisError::from((redis::ErrorKind::TypeError,"", e.to_string()))
        })?;
        con.set(format!("prefix_config:{}", prefix_key), config_json).await?;

        let mut cache = self.local_cache.lock().await;
        cache.insert(prefix_key.clone(), config.clone());
        self.prefix_configs.insert(prefix_key.clone(), config);
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let redis_url = "redis://127.0.0.1/";
    let number_gen = Arc::new(Mutex::new(
        NumberGenerator::new(redis_url)
            .await
            .expect("Failed to connect to Redis")
    ));

    let generate_route = warp::path!("api" / "numbers")
        .and(warp::get())
        .and(warp::query::<HashMap<String, String>>())
        .and_then({
            let number_gen = Arc::clone(&number_gen);
            move |params: HashMap<String, String>| {
                let number_gen = Arc::clone(&number_gen);
                async move {
                    if let Some(prefix_key) = params.get("prefixKey") {
                        let mut number_gen = number_gen.lock().await;
                        match number_gen.generate_number(prefix_key).await {
                            Ok(number) => Ok(warp::reply::json(&NumberResponse { number })),
                            Err(_) => Err(warp::reject::custom(PrefixNotConfigured)),
                        }
                    } else {
                        Err(warp::reject::custom(MissingPrefixKey))
                    }
                }
            }
        });

    {
        let mut number_gen = number_gen.lock().await;
        // Register a test prefix configuration
        number_gen.register_prefix("PREFIX_TEST".to_string(), PrefixConfig {
            format: "TEST-{year}-{SEQ}".to_string(),
            seq_length: 6,
            initial_seq: 1,
        }).await;
    }

    let config_route = warp::path!("api" / "prefix-configs" / String)
        .and(warp::put())
        .and(warp::body::json())
        .and_then({
            let number_gen = Arc::clone(&number_gen);
            move |prefix_key: String, config: PrefixConfig| {
                let number_gen = Arc::clone(&number_gen);
                async move {
                    let mut number_gen = number_gen.lock().await;
                    number_gen.register_prefix(prefix_key, config).await;
                    Ok::<_, warp::Rejection>(warp::reply::json(&"Prefix configuration registered"))
                }
            }
        });

    let metrics_route = warp::path!("metrics")
        .and_then(move || {
            let number_gen = Arc::clone(&number_gen);
            async move {
                let encoder = TextEncoder::new();
                let mut buffer = vec![];
                encoder.encode(&number_gen.lock().await.metrics.registry.gather(), &mut buffer).unwrap();
                Ok::<_, warp::Rejection>(String::from_utf8(buffer).unwrap())
            }
        });

    let routes = generate_route.or(config_route).or(metrics_route);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030))
        .await;
}
