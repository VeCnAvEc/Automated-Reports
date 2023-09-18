use serde::{Deserialize, Serialize};
use crate::r#trait::api_request::ApiRequest;
use crate::r#type::types::ResponseError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: Option<String>,
    username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    email: Option<String>,
    birthday: Option<String>,
    gender: Option<String>,
    pictureurl: Option<String>,
    merchant_id: Option<String>,
    provider_id: Option<String>,
    commission: Option<String>,
    #[serde(rename = "type")]
    pub user_type: Option<String>,
    status: Option<String>,
    parent_id: Option<String>,
    region_id: Option<String>,
    vendor: Option<Vec<bool>>,
}



impl ApiRequest for UserInfo {}

impl UserInfo {
    pub fn get_pub_fields(field: &Option<String>) -> String {
        let error_field = "None".to_string();

        return match field {
            None => error_field,
            Some(string) => string.clone(),
        }
    }

    pub fn check_on_error(user_id: String) -> Result<String, ResponseError> {
        if user_id == "None".to_string() {
            Err((1334302, "Не удалось создать или найти папку для текущего пользователя".to_string()))
        } else {
            Ok(user_id)
        }
    }
}