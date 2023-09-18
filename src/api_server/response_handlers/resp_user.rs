pub mod handlers_user {
    use serde::Deserialize;
    use serde_json::Value;
    use crate::api_server::response_handlers::RpcError;
    use crate::helper::user_info::user::UserInfo;
    use crate::r#type::types::ResponseError;

    #[derive(Debug, Deserialize)]
    pub struct AccountReplenishment {
        #[allow(dead_code)]
        pub(crate) acc: Option<String>,
        pub(crate) amount: Option<String>,
        pub(crate) comment: Option<String>,
        pub(crate) date: Option<String>,
        pub(crate) first_name: Option<String>,
        pub(crate) id: Option<String>,
        pub(crate) last_name: Option<String>,
        #[allow(dead_code)]
        pub(crate) provider_id: Option<String>,
        #[allow(dead_code)]
        pub(crate) user_id: Option<String>,
        pub(crate) username: Option<String>,
    }

    pub fn handler_user_info(response: Result<Value, ResponseError>) -> Result<UserInfo, ResponseError> {
        use serde_json::Result;

        let mut error: Option<RpcError> = None;
        let mut result: Option<UserInfo> = None;

        match response {
            Ok(result_r) => {
                let user = match result_r.get("user") {
                    Some(user_info) => user_info,
                    None => &Value::Null
                };

                if let &Value::Null = user {
                    return Err((2765430, "Не удалось получить информацию об user-е по токену.".to_string()));
                }

                let des_user_info: Result<UserInfo> = serde_json::from_value(user.clone());

                if let Err(error) = des_user_info {
                    return Err((2765431, format!("{}", error.to_string())));
                }

                result = Some(des_user_info.unwrap());
            }
            Err(error_r) => {
                let cust_error = RpcError {
                    code: error_r.0 as i64,
                    message: error_r.1
                };

                error = Some(cust_error);
            }
        }

        return if result.is_some() {
            Ok(result.unwrap())
        } else {
            let error = match error {
                Some(rpc_error) => {
                    Ok(rpc_error)
                },
                None => Err((2765433, "Ошибка от api, ответ оказалась пустой".to_string()))
            };

            if let Err(error) = error {
                return Err(error)
            }

            let error = error.unwrap();

            Err((error.code as i32, error.message))
        }
    }
}