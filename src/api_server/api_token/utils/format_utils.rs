/// code @32430
pub mod token_utils {
    use actix_web::http::header::HeaderMap;
    use actix_web::web::Json;
    use crate::helper::get_token_from_header;
    use crate::r#trait::automated_report_response::Response;
    use crate::r#type::types::ResponseError;

    pub fn token_format_to_string(headers: &HeaderMap) -> Result<String, ResponseError> {
        let token_opt = match get_token_from_header(headers) {
            Ok(token) => Some(token),
            Err(_) => None
        };

        if token_opt.is_none() {
            return Err((4324302, "token не был передан".to_string()));
        }

        let token_to_string = token_opt.unwrap().to_str();

        if let Err(_) = token_to_string {
            return Err((4324303, "Не удалось привести токен к строке".to_string()));
        }

        let token = token_to_string.unwrap().to_string();

        Ok(token)
    }

    pub fn handle_token_error(token_result: &Result<String, (i32, String)>) -> Option<Json<Response>> {
        if let Err(error)  = token_result {
            return Some(Json(Response::new::<String>(
                Some(error.clone()),
                None,
                None
                )
            ))
        }

        None
    }
}