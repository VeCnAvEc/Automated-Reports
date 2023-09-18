mod req_provider;
mod req_user;
mod req_payments_tools;

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use rand::Rng;

use serde::{Deserialize, Serialize};

use reqwest::{Error, Response};
use reqwest::header::HeaderMap;

pub const URL: &str = "https://api.lo/Api";
/// Логин для админа
pub const ADMIN_LOGIN: &str = "ApI";
/// Пароль/ключ для админа
pub const ADMIN_KEY: &str = "ApI";

#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
pub struct RpcRequest {
    pub(crate) jsonrpc: String,
    pub(crate) method: String,
    pub(crate) params: RpcRequestParams,
    pub(crate) id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct RpcRequestParams {
    provider_id: Option<String>,
    user_id: Option<u32>,
    merchant_id: Option<u32>
}

impl RpcRequest {
    /// Отправить от имени администратора
    pub async fn send(request: RpcRequest, token: String) -> Result<Response, Error> {
        let client = reqwest::Client::new();

        let value_request = serde_json::to_value(&request).unwrap();
        let mut headers = HashMap::new();

        let auth = Self::generate_admin_auth();

        headers.insert("token".to_string(), token);
        headers.insert("dev".to_string(), "1".to_string());
        headers.insert(
            "User-Agent".to_string(),
            "microapi/0.3 api.api.lo".to_string(),
        );
        headers.insert("AUTH".to_string(), auth);

        let response = client
            .get(URL)
            .body(value_request.to_string())
            .headers(HeaderMap::try_from(&headers).unwrap())
            .send()
            .await;

        response
    }

    pub fn build_request(method: String, params: RpcRequestParams) -> RpcRequest {
        RpcRequest {
            jsonrpc: "2.0".to_string(),
            method,
            params,
            id: rand::thread_rng().gen(),
        }
    }

    pub fn generate_admin_auth() -> String {
        let time = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_secs();

        format!(
            "{}-{}-{}",
            ADMIN_LOGIN,
            format!("{:X}", md5::compute(format!("{}{}", ADMIN_KEY, time))).to_lowercase(),
            time.to_string()
        )
    }
}