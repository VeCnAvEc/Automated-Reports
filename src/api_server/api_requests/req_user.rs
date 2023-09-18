use actix_web::http::header::HeaderValue;
use serde_json::Value;
use crate::api_server::api_requests::{RpcRequest, RpcRequestParams};
use crate::api_server::response_handlers::response_to_json;

use crate::r#type::types::ResponseError;

impl RpcRequest {
    pub async fn get_userinfo_by_token(token: Option<&HeaderValue>) -> Result<Value, ResponseError> {
        let token = Self::check_token(token);

        if let Err(error) = token {
            return Err(error)
        }

        let token = token.unwrap();

        let request = RpcRequest::build_request(
            "user_info".to_string(),
            RpcRequestParams { merchant_id: None, user_id: None, provider_id: None }
        );

        let response = Self::send(request, token.clone()).await;

        let value = response_to_json(response).await;

        value
    }
}