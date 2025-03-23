use redis::{AsyncCommands, RedisResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;
use std::future::Future;

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
}

impl NumberGenerator {
    async fn new(redis_url: &str) -> RedisResult<Self> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self {
            redis_client: client,
            prefix_configs: HashMap::new(),
        })
    }

    async fn generate_number(&self, prefix_key: &str) -> RedisResult<String> {
        println!("Checking prefix configuration for: {}", prefix_key);
        if let Some(config) = self.prefix_configs.get(prefix_key) {
            println!("Found configuration: {:?}", config);
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
            
            let number = config.format
                .replace("{SEQ}", &format!("{:0>width$}", seq, width = config.seq_length))
                .replace("{year}", &chrono::Local::now().format("%Y").to_string());
            println!("Formatted number: {}", number);
            Ok(number)
        } else {
            println!("Prefix not found in configurations");
            Err(redis::RedisError::from((
                redis::ErrorKind::TypeError,
                "Prefix not configured",
            )))
        }
    }

    async fn register_prefix(&mut self, prefix_key: String, config: PrefixConfig) {
        self.prefix_configs.insert(prefix_key, config);
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
                        let number_gen = number_gen.lock().await;
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

    let routes = generate_route.or(config_route);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030))
        .await;
}
