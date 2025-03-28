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
        manager.get_prefix(&prefix_key).await
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
    manager.register_prefix(prefix_rule).await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    Ok(HttpResponse::Ok().finish())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let redis_url = "redis://localhost:6379/".to_string();
    let prefix_rule_manager: Arc<Mutex<dyn PrefixRuleManager + Send + Sync>> = {
        let redis_prefix_rule_manager = RedisPrefixRuleManager::new(redis_url.clone()).unwrap();
        Arc::new(Mutex::new(redis_prefix_rule_manager))
    };
    let sequence_generator: Arc<RedisSequenceGenerator> = {
        let redis_sequence_generator = RedisSequenceGenerator::new(redis_url.clone(), prefix_rule_manager.clone() as Arc<Mutex<dyn PrefixRuleManager + Send + Sync>>).unwrap();
        Arc::new(redis_sequence_generator)
    };
    let number_assembler = Arc::new(NumberAssembler::new());

    let prefix_rule_manager_data: web::Data<Arc<Mutex<dyn PrefixRuleManager + Send + Sync>>> = web::Data::new(prefix_rule_manager.clone());
    let sequence_generator_data = web::Data::new(sequence_generator);
    let number_assembler_data = web::Data::new(number_assembler.clone());

    HttpServer::new(move || {
        App::new()
            .app_data(prefix_rule_manager_data.clone())
            .app_data(sequence_generator_data.clone())
            .app_data(number_assembler_data.clone())
            .route("/api/numbers/{prefixKey}", web::get().to(generate_number))
            .route("/api/prefix-configs/{prefixKey}", web::put().to(register_prefix))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
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
            let redis_sequence_generator = RedisSequenceGenerator::new(redis_url.clone(), prefix_rule_manager.clone()).unwrap();
            Arc::new(redis_sequence_generator)
        };
        let number_assembler = Arc::new(NumberAssembler::new());

        let prefix_rule_manager_data = web::Data::new(prefix_rule_manager.clone() as Arc<Mutex<dyn PrefixRuleManager + Send + Sync>>);
        let sequence_generator_data = web::Data::new(sequence_generator.clone());
        let number_assembler_data = web::Data::new(number_assembler.clone());

        let app = test::init_service(
            App::new()
                .app_data(prefix_rule_manager_data.clone())
                .app_data(sequence_generator_data.clone())
                .app_data(number_assembler_data.clone())
                .route("/api/numbers/{prefixKey}", web::get().to(generate_number))
                .route("/api/prefix-configs/{prefixKey}", web::put().to(register_prefix))
        ).await;

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

        // Clear Redis after the test
        let client = redis::Client::open(redis_url.clone()).unwrap();
        let mut conn = client.get_connection().unwrap();
        redis::cmd("FLUSHDB").execute(&mut conn);
    }
}
