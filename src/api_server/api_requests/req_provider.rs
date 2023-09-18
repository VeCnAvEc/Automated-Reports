use serde_json::Value;
use crate::api_server::api_requests::{RpcRequest, RpcRequestParams};
use crate::api_server::response_handlers::response_to_json;
use crate::r#type::types::ResponseError;

impl RpcRequest {
    pub async fn get_provider_info(provider_id: String, token: String) -> Result<Value, ResponseError> {
        if provider_id.is_empty() {
            return Err((1765430, "Был передан пустой `provider_id`".to_string()))
        }

        let request = RpcRequest::build_request(
            "provider.info".to_string(),
            RpcRequestParams {
                provider_id: Some(provider_id),
                user_id: None,
                merchant_id: None,
            }
        );

        let response = Self::send(request, token.clone()).await;

        let value = response_to_json(response).await;

        value
    }
}