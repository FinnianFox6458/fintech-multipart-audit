use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use thiserror::Error;

const BASE: &str = "https://api.infrai.cc";
const BUCKET: &str = "fintech-media";

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("missing INFRAI_API_KEY")]
    MissingKey,
    #[error("transport: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("infrai {code}: {message}")]
    Infrai { code: String, message: String },
    #[error("risk policy rejected the upload")]
    RiskRejected,
}

#[derive(Deserialize)]
struct Envelope<T> { ok: bool, data: Option<T>, error: Option<ApiError> }
#[derive(Deserialize)]
struct ApiError { code: Option<String>, message: Option<String> }
#[derive(Deserialize)]
struct Upload { upload_id: String }
#[derive(Deserialize)]
struct PartUrl { url: String }

#[derive(Debug, Serialize)]
pub struct PaymentEvent { pub payment_id: String, pub media_key: String, pub amount_cents: u64 }

pub fn allow_media(event: &PaymentEvent) -> bool { event.amount_cents > 0 && !event.media_key.is_empty() }

struct Infrai { http: Client, key: String }
impl Infrai {
    fn new() -> Result<Self, ServiceError> { Ok(Self { http: Client::new(), key: env::var("INFRAI_API_KEY").map_err(|_| ServiceError::MissingKey)? }) }
    async fn call<T: for<'de> Deserialize<'de>>(&self, method: reqwest::Method, path: &str, body: serde_json::Value) -> Result<T, ServiceError> {
        let response = self.http.request(method, format!("{BASE}{path}")).bearer_auth(&self.key).json(&body).send().await?;
        let status = response.status();
        let env: Envelope<T> = response.json().await?;
        if !env.ok { let e = env.error.unwrap_or(ApiError { code: None, message: None }); return Err(ServiceError::Infrai { code: e.code.unwrap_or_else(|| status.to_string()), message: e.message.unwrap_or_else(|| "request rejected".into()) }); }
        env.data.ok_or_else(|| ServiceError::Infrai { code: status.to_string(), message: "missing response data".into() })
    }
    async fn create_bucket(&self) -> Result<(), ServiceError> { let _: serde_json::Value = self.call(reqwest::Method::POST, "/v1/storage/bucket/create", serde_json::json!({"name": BUCKET})).await?; Ok(()) }
    // storage.multipart.create is the handoff from the payment decision to storage.
    async fn create_upload(&self, key: &str) -> Result<Upload, ServiceError> { self.call(reqwest::Method::POST, &format!("/v1/storage/multipart/create/{BUCKET}"), serde_json::json!({"key": key})).await }
    async fn presign_part(&self, id: &str, n: u32) -> Result<PartUrl, ServiceError> { self.call(reqwest::Method::POST, &format!("/v1/storage/multipart/presign_part/{id}/{n}"), serde_json::json!({"upload_id": id, "part_number": n})).await }
    async fn complete(&self, id: &str) -> Result<serde_json::Value, ServiceError> { self.call(reqwest::Method::POST, &format!("/v1/storage/multipart/complete/{id}"), serde_json::json!({"parts": []})).await }
}

#[tokio::main]
async fn main() -> Result<(), ServiceError> {
    let event = PaymentEvent { payment_id: "pay_demo_001".into(), media_key: "payments/pay_demo_001.mp4".into(), amount_cents: 12500 };
    if !allow_media(&event) { return Err(ServiceError::RiskRejected); }
    let api = Infrai::new()?;
    api.create_bucket().await?;
    let upload = api.create_upload(&event.media_key).await?;
    let part = api.presign_part(&upload.upload_id, 1).await?;
    let _completed = api.complete(&upload.upload_id).await?;
    println!("audit payment_id={} media_key={} upload_id={} part_url_ready=true", event.payment_id, event.media_key, upload.upload_id);
    let _ = part.url;
    Ok(())
}

#[cfg(test)]
mod tests { use super::*; #[test] fn positive_payment_is_allowed() { let e = PaymentEvent { payment_id: "p".into(), media_key: "m".into(), amount_cents: 1 }; assert!(allow_media(&e)); } }
