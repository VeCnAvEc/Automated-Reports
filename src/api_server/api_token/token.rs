use actix_web::http::header::HeaderValue;
use crate::api_server::api_requests::RpcRequest;
use crate::r#type::types::ResponseError;


impl RpcRequest {
    pub fn check_token(token: Option<&HeaderValue>) -> Result<String, ResponseError> {
        if let None = token {
            return Err((4765430, "Токен не был получен".to_string()));
        }

        let token = token.unwrap().to_str().unwrap_or("").to_string();

        if token.is_empty() {
            return Err((4765431, "Был передан пустой `token`".to_string()));
        } else {
            Ok(token)
        }
    }
}

