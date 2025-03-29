use actix_web::{web, App, HttpResponse, HttpServer, Responder, Result};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, sync::Mutex};

mod prefix_rule;
mod prefix_rule_manager;
mod sequence_generator;
mod number_assembler;
mod redis_prefix_rule_manager;

use crate::redis_prefix_rule_manager::RedisPrefixRuleManager;
use crate::sequence_generator::{SequenceGenerator, RedisSequenceGenerator};
use crate::number_assembler::NumberAssembler;
use crate::prefix_rule::PrefixRule;
use crate::prefix_rule_manager::PrefixRuleManager;

#[derive(Debug, Deserialize)]
struct PrefixConfigPayload {
    format: String,
    #[serde(rename = "seqLength")]
    seq_length: u32,
    #[serde(rename = "initialSeq")]
    initial_seq: u64,
}

impl From<PrefixConfigPayload> for PrefixRule {
    fn from(payload: PrefixConfigPayload) -> Self {
        PrefixRule {
            prefix_key: String::new(), // This will be set later
            format: payload.format,
            seq_length: payload.seq_length,
            initial_seq: payload.initial_seq,
            network_partition: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct NumberResponse {
    number: String,
}

async fn generate_number(
    prefix_key: web::Path<String>,
    prefix_rule_manager: web::Data<Arc<Mutex<dyn PrefixRuleManager + Send + Sync>>>,
    sequence_generator: web::Data<Arc<RedisSequenceGenerator>>,
    number_assembler: web::Data<Arc<NumberAssembler>>,
) -> Result<impl Responder> {
    let prefix_key = prefix_key.into_inner();

        let prefix_rule = {
            let prefix_rule_manager_clone = prefix_rule_manager.clone();
            let manager = prefix_rule_manager_clone.lock().unwrap();
            manager.get_prefix_rule(prefix_key.clone()).await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?
        };

    match prefix_rule {
        Some(config) => {
            let sequence = sequence_generator.generate(&prefix_key).await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

            let number = number_assembler.assemble_number(&prefix_key, &config, sequence)
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

            Ok(web::Json(NumberResponse { number }))
        }
        None => Err(actix_web::error::ErrorBadRequest("Prefix not registered")),
    }
}

async fn register_prefix(
    prefix_key: web::Path<String>,
    payload: web::Json<PrefixConfigPayload>,
    prefix_rule_manager: web::Data<Arc<Mutex<dyn PrefixRuleManager + Send + Sync>>>,
) -> Result<impl Responder> {
    let prefix_key = prefix_key.into_inner();
    let mut prefix_rule: PrefixRule = payload.into_inner().into();
    prefix_rule.prefix_key = prefix_key.clone();

    let prefix_rule_manager_clone = prefix_rule_manager.clone();
    let mut manager = prefix_rule_manager_clone.lock().unwrap();
    if !is_valid_format(&prefix_rule.format) {
        return Err(actix_web::error::ErrorBadRequest("Invalid prefix format"));
    }
    let result = manager.register_prefix_rule(prefix_key.clone(), prefix_rule).await;
    match result {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(e) => {
            eprintln!("Failed to register prefix rule: {}", e);
            Err(actix_web::error::ErrorInternalServerError(e))
        }
    }
}

fn is_valid_format(format: &str) -> bool {
    // Check if the format contains {SEQ:N} where N is a number
    let re = regex::Regex::new(r"\{SEQ:\d+\}").unwrap();
    // Check if the format contains {SEQ:N} where N is a number
    re.is_match(format) && format.contains("{year}")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Service Statelessness:
    // The service is stateless, as it doesn't store any state within the service instance itself.
    // All state is stored in Redis. This allows for horizontal scaling and no single point of failure.
    let redis_url = "redis://localhost:6379/".to_string();
    let prefix_rule_manager: Arc<Mutex<dyn PrefixRuleManager + Send + Sync>> = {
        let redis_prefix_rule_manager = RedisPrefixRuleManager::new(redis_url.clone()).unwrap();
        Arc::new(Mutex::new(redis_prefix_rule_manager))
    };
    let sequence_generator: Arc<RedisSequenceGenerator> = {
        let redis_sequence_generator = RedisSequenceGenerator::new(redis_url.clone()).unwrap();
        Arc::new(redis_sequence_generator)
    };
    let number_assembler = Arc::new(NumberAssembler::new());

    let prefix_rule_manager_data: web::Data<Arc<Mutex<dyn PrefixRuleManager + Send + Sync>>> = web::Data::new(prefix_rule_manager.clone());
    let sequence_generator_data = web::Data::new(sequence_generator);
    let number_assembler_data = web::Data::new(number_assembler.clone());

    // Service Node Downtime:
    // Service node downtime is handled by the load balancer, which automatically
    // switches traffic to healthy nodes. Since the service is stateless, any instance
    // can handle any request.
    HttpServer::new(move || {
        App::new()
            .app_data(prefix_rule_manager_data.clone())
            .app_data(sequence_generator_data.clone())
            .app_data(number_assembler_data.clone())
            .app_data(sequence_generator_data.clone())
            .app_data(number_assembler_data.clone())
            .app_data(sequence_generator_data.clone())
            .app_data(number_assembler_data.clone())
            .route("/api/numbers/{prefixKey}", web::get().to(generate_number))
            .route("/api/prefix-configs/{prefixKey}", web::put().to(register_prefix))
            .route("/api/prefix-configs/{prefixKey}/network-partition", web::post().to(set_network_partition))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

async fn set_network_partition(
    prefix_key: web::Path<String>,
    prefix_rule_manager: web::Data<Arc<Mutex<dyn PrefixRuleManager + Send + Sync>>>,
) -> Result<impl Responder> {
    let prefix_key = prefix_key.into_inner();

    let prefix_rule_manager_clone = prefix_rule_manager.clone();
    let mut manager = prefix_rule_manager_clone.lock().unwrap();

    match manager.get_prefix_rule(prefix_key.clone()).await {
        Ok(Some(mut prefix_rule)) => {
            prefix_rule.network_partition = true;
            manager.register_prefix_rule(prefix_key.clone(), prefix_rule).await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
            Ok(HttpResponse::Ok().finish())
        }
        Ok(None) => Err(actix_web::error::ErrorBadRequest("Prefix not registered")),
        Err(e) => Err(actix_web::error::ErrorInternalServerError(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App};
    use actix_web::http::StatusCode;
    use serde_json::json;

    #[actix_web::test]
    async fn test_register_and_generate_number() {
        let redis_url = "redis://localhost:6379/".to_string();
        let client = redis::Client::open(redis_url.clone()).unwrap();
        let mut conn = client.get_connection().unwrap();
        let _ : () = redis::cmd("FLUSHDB").execute(&mut conn);

        let prefix_rule_manager: Arc<Mutex<dyn PrefixRuleManager + Send + Sync>> = {
            let redis_prefix_rule_manager = RedisPrefixRuleManager::new(redis_url.clone()).unwrap();
            Arc::new(Mutex::new(redis_prefix_rule_manager))
        };
        let sequence_generator: Arc<RedisSequenceGenerator> = {
            let redis_sequence_generator = RedisSequenceGenerator::new(redis_url.clone()).unwrap();
            Arc::new(redis_sequence_generator)
        };
        let number_assembler = Arc::new(NumberAssembler::new());

        let prefix_rule_manager_data = web::Data::new(prefix_rule_manager.clone());
        let sequence_generator_data = web::Data::new(sequence_generator.clone());
        let number_assembler_data = web::Data::new(number_assembler.clone());

        let app = test::init_service(
            App::new()
                .app_data(prefix_rule_manager_data.clone())
                .app_data(sequence_generator_data.clone())
                .app_data(number_assembler_data.clone())
                .route("/api/numbers/{prefixKey}", web::get().to(generate_number))
                .route("/api/prefix-configs/{prefixKey}", web::put().to(register_prefix))
        )
        .await;

        // Register prefix
        let register_payload = json!({
            "format": "TEST-{year}-{SEQ:4}",
            "seqLength": 4,
            "initialSeq": 1
        });

        let register_request = test::TestRequest::put()
            .uri(&"/api/prefix-configs/TEST".to_string())
            .set_json(&register_payload)
            .to_request();

        let register_response = test::call_service(&app, register_request).await;
        let status = register_response.status();
        let body = actix_web::body::to_bytes(register_response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        println!("Register response status: {}", status);
        println!("Register response body: {}", body_str);
        assert_eq!(status, StatusCode::OK);

        // Generate number
        let generate_request = test::TestRequest::get()
            .uri(&"/api/numbers/TEST".to_string())
            .to_request();

        let generate_response = test::call_service(&app, generate_request).await;
        assert_eq!(generate_response.status(), StatusCode::OK);

        let body = test::read_body(generate_response).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        println!("Response body: {}", body_str);

        let number_response: NumberResponse = serde_json::from_str(&body_str).unwrap();
        assert!(number_response.number.starts_with("TEST"));
        assert!(number_response.number.contains("-2025-"));
        assert_eq!(number_response.number.len(), 14); // TEST-2025-0001

        // Clear Redis after the test
        let client = redis::Client::open(redis_url.clone()).unwrap();
        let mut conn = client.get_connection().unwrap();
        redis::cmd("FLUSHDB").execute(&mut conn);
    }

    #[actix_web::test]
    async fn test_register_prefix_invalid_format() {
        let redis_url = "redis://localhost:6379/".to_string();
        let client = redis::Client::open(redis_url.clone()).unwrap();
        let mut conn = client.get_connection().unwrap();
        let _ : () = redis::cmd("FLUSHDB").execute(&mut conn);

        let prefix_rule_manager: Arc<Mutex<dyn PrefixRuleManager + Send + Sync>> = {
            let redis_prefix_rule_manager = RedisPrefixRuleManager::new(redis_url.clone()).unwrap();
            Arc::new(Mutex::new(redis_prefix_rule_manager))
        };
        let sequence_generator: Arc<RedisSequenceGenerator> = {
            let redis_sequence_generator = RedisSequenceGenerator::new(redis_url.clone()).unwrap();
            Arc::new(redis_sequence_generator)
        };
        let number_assembler = Arc::new(NumberAssembler::new());

        let prefix_rule_manager_data = web::Data::new(prefix_rule_manager.clone());
        let sequence_generator_data = web::Data::new(sequence_generator.clone());
        let number_assembler_data = web::Data::new(number_assembler.clone());

        let app = test::init_service(
            App::new()
                .app_data(prefix_rule_manager_data.clone())
                .app_data(sequence_generator_data.clone())
                .app_data(number_assembler_data.clone())
                .route("/api/numbers/{prefixKey}", web::get().to(generate_number))
                .route("/api/prefix-configs/{prefixKey}", web::put().to(register_prefix))
        )
        .await;

        // Register prefix with invalid format
        let register_payload = json!({
            "format": "INVALID",
            "seqLength": 4,
            "initialSeq": 1
        });

        let register_request = test::TestRequest::put()
            .uri(&"/api/prefix-configs/INVALID".to_string())
            .set_json(&register_payload)
            .to_request();

        let register_response = test::call_service(&app, register_request).await;
        assert_eq!(register_response.status(), StatusCode::BAD_REQUEST);

        // Clear Redis after the test
        let client = redis::Client::open(redis_url.clone()).unwrap();
        let mut conn = client.get_connection().unwrap();
        redis::cmd("FLUSHDB").execute(&mut conn);
    }

    #[actix_web::test]
    async fn test_generate_number_prefix_not_registered() {
        let redis_url = "redis://localhost:6379/".to_string();
        let client = redis::Client::open(redis_url.clone()).unwrap();
        let mut conn = client.get_connection().unwrap();
        let _ : () = redis::cmd("FLUSHDB").execute(&mut conn);

        let prefix_rule_manager: Arc<Mutex<dyn PrefixRuleManager + Send + Sync>> = {
            let redis_prefix_rule_manager = RedisPrefixRuleManager::new(redis_url.clone()).unwrap();
            Arc::new(Mutex::new(redis_prefix_rule_manager))
        };
        let sequence_generator: Arc<RedisSequenceGenerator> = {
            let redis_sequence_generator = RedisSequenceGenerator::new(redis_url.clone()).unwrap();
            Arc::new(redis_sequence_generator)
        };
        let number_assembler = Arc::new(NumberAssembler::new());

        let prefix_rule_manager_data = web::Data::new(prefix_rule_manager.clone());
        let sequence_generator_data = web::Data::new(sequence_generator.clone());
        let number_assembler_data = web::Data::new(number_assembler.clone());

        let app = test::init_service(
            App::new()
                .app_data(prefix_rule_manager_data.clone())
                .app_data(sequence_generator_data.clone())
                .app_data(number_assembler_data.clone())
                .route("/api/numbers/{prefixKey}", web::get().to(generate_number))
                .route("/api/prefix-configs/{prefixKey}", web::put().to(register_prefix))
        )
        .await;

        // Generate number for unregistered prefix
        let generate_request = test::TestRequest::get()
            .uri(&"/api/numbers/UNKNOWN".to_string())
            .to_request();

        let generate_response = test::call_service(&app, generate_request).await;
        assert_eq!(generate_response.status(), StatusCode::BAD_REQUEST);

        let body = test::read_body(generate_response).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        println!("Response body: {}", body_str);
        assert_eq!(body_str, "Prefix not registered");

        // Clear Redis after the test
        let client = redis::Client::open(redis_url.clone()).unwrap();
        let mut conn = client.get_connection().unwrap();
        redis::cmd("FLUSHDB").execute(&mut conn);
    }

    #[actix_web::test]
    async fn test_generate_number_network_partition() {
        let redis_url = "redis://localhost:6379/".to_string();
        let client = redis::Client::open(redis_url.clone()).unwrap();
        let mut conn = client.get_connection().unwrap();
        let _ : () = redis::cmd("FLUSHDB").execute(&mut conn);

        let prefix_rule_manager: Arc<Mutex<dyn PrefixRuleManager + Send + Sync>> = {
            let redis_prefix_rule_manager = RedisPrefixRuleManager::new(redis_url.clone()).unwrap();
            Arc::new(Mutex::new(redis_prefix_rule_manager))
        };
        let sequence_generator: Arc<RedisSequenceGenerator> = {
            let redis_sequence_generator = RedisSequenceGenerator::new(redis_url.clone()).unwrap();
            Arc::new(redis_sequence_generator)
        };
        let number_assembler = Arc::new(NumberAssembler::new());

        let prefix_rule_manager_data = web::Data::new(prefix_rule_manager.clone());
        let sequence_generator_data = web::Data::new(sequence_generator.clone());
        let number_assembler_data = web::Data::new(number_assembler.clone());

        let app = test::init_service(
            App::new()
                .app_data(prefix_rule_manager_data.clone())
                .app_data(sequence_generator_data.clone())
                .app_data(number_assembler_data.clone())
                .route("/api/numbers/{prefixKey}", web::get().to(generate_number))
                .route("/api/prefix-configs/{prefixKey}", web::put().to(register_prefix))
                .route("/api/prefix-configs/{prefixKey}/network-partition", web::post().to(set_network_partition))
        )
        .await;

        // Register prefix
        let register_payload = json!({
            "format": "TEST-{year}-{SEQ:4}",
            "seqLength": 4,
            "initialSeq": 1
        });

        let register_request = test::TestRequest::put()
            .uri(&"/api/prefix-configs/TEST".to_string())
            .set_json(&register_payload)
            .to_request();

        let register_response = test::call_service(&app, register_request).await;
        let status = register_response.status();
        assert_eq!(status, StatusCode::OK);

        // Set network partition
        let network_partition_request = test::TestRequest::post()
            .uri(&"/api/prefix-configs/TEST/network-partition".to_string())
            .to_request();

        let network_partition_response = test::call_service(&app, network_partition_request).await;
        assert_eq!(network_partition_response.status(), StatusCode::OK);

        // Generate number
        let generate_request = test::TestRequest::get()
            .uri(&"/api/numbers/TEST".to_string())
            .to_request();

        let generate_response = test::call_service(&app, generate_request).await;
        assert_eq!(generate_response.status(), StatusCode::OK);

        let body = test::read_body(generate_response).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        let number_response: NumberResponse = serde_json::from_str(&body_str).unwrap();
        assert!(number_response.number.ends_with("-NP"));

        // Clear Redis after the test
        let client = redis::Client::open(redis_url.clone()).unwrap();
        let mut conn = client.get_connection().unwrap();
        redis::cmd("FLUSHDB").execute(&mut conn);
    }
}
