use reqwest::header::HeaderValue;

use serde_json::Value;

use crate::api_server::api_requests::{RpcRequest, RpcRequestParams};

use crate::api_server::response_handlers::response_to_json;

use crate::r#type::types::ResponseError;

#[allow(dead_code)]
impl RpcRequest {
    pub async fn get_ven_payments_tool(token: Option<&HeaderValue>, merchant_id: String) -> Result<Value, ResponseError> {
        let merchant_id = merchant_id.parse::<u32>().unwrap_or(0);
        let token = Self::check_token(token);

        if let Err(error) = token {
            return Err(error)
        }

        let token = token.unwrap();

        let request = RpcRequest::build_request(
            "get_payment_list".to_string(),
            RpcRequestParams{ merchant_id: Some(merchant_id), user_id: None, provider_id: None }
        );

        let response = Self::send(request, token.clone()).await;

        let value = response_to_json(response).await;

        value
    }
}