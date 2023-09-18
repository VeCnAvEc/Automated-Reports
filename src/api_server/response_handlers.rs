use reqwest::{Response, Error};

use serde_json::Value;

use serde::{Deserialize, Serialize};
use tracing::error;

use crate::r#type::types::ResponseError;

pub mod resp_provider;
pub mod resp_user;
pub mod resp_payments_tools;

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct RpcError {
    code: i64,
    message: String,
}

#[derive(Deserialize, Serialize,Debug)]
pub struct ProviderInfo {
    date: Option<String>,
    id: Option<String>,
    live: Option<String>,
    organization: Option<String>,
    field_1: Option<String>,
    field_2: Option<String>,
    server: Option<String>,
    provider_id: Option<String>,
    status: Option<String>,
    #[serde(rename = "type")]
    s_type: Option<String>,
    merchant_id: Option<String>,
    provider_type: Option<String>,
    providers: Option<Vec<String>>,
}

pub async fn response_to_json(response: Result<Response, Error>) -> Result<Value, ResponseError> {
    use serde_json::Result;

    match response {
        Ok(r) => {
            match r.text().await {
                Ok(res) => {
                    let value: Result<Value> = serde_json::from_str(&res);
                    if let Err(error) = value {
                        return Err((3765430, error.to_string()));
                    }

                    let res_unwrap: Value = value.unwrap();

                    let mut error: Option<Value> = None;
                    let mut result: Option<Value> = None;

                    if let Value::Object(obj) = res_unwrap {
                        if let Some(error_r) = obj.get("error") {
                            if error_r != &Value::Null {
                                error = Some(error_r.clone());
                            } else if let Some(result_r) = obj.get("result") {
                                if result_r != &Value::Null {
                                    result = Some(result_r.clone());
                                }
                            }
                        }
                    }

                    if result.is_some() {
                        return Ok(result.unwrap());
                    } else {
                        let error: Result<RpcError> = serde_json::from_value(error.unwrap_or(Value::Null));

                        if let Err(api_error) = error {
                            return Err((3432321, format!("{:?}", api_error)))
                        }
                        let error_unwrap = error.unwrap();

                        error!("code: {} message: {}", error_unwrap.code, error_unwrap.message);
                        Err((error_unwrap.code as i32, error_unwrap.message))
                    }
                }
                Err(error) => {
                    error!("code: {} error: {:?}", 432432, error);
                    Err((3765431, error.to_string()))
                }
            }
        },
        Err(error) => {
            error!("code: 323245 message: {}", error.to_string());
            Err((3765432, error.to_string()))
        }
    }
}