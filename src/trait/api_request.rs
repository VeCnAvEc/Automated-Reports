use crate::r#type::types::ResponseError;

pub trait ApiRequest {
    fn convert_error(&self, code: i32, message: String) -> ResponseError where  {
        (code, message)
    }
}

impl std::fmt::Debug for dyn ApiRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserInfo").finish()
    }
}