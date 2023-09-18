pub mod handlers_provider {
    use serde_json::Value;
    use tracing::error;
    use crate::api_server::response_handlers::resp_user::handlers_user::AccountReplenishment;
    use crate::api_server::response_handlers::{RpcError, ProviderInfo};
    use crate::r#type::types::ResponseError;

    pub fn handler_provider_recent_deposits(response: &Value) -> Result<Vec<AccountReplenishment>, ResponseError> {
        let mut error: Option<RpcError> = None;
        let mut result: Option<Vec<AccountReplenishment>> = None;

        let replenishment = match response.get("balance") {
            Some(balance) => {
                match balance.get("recent_deposits") {
                    None => None,
                    Some(recent_deposits) => {
                        Some(recent_deposits)
                    }
                }
            },
            None => None
        };

        if replenishment.is_none() {
            error!("Не удалось получить информацию об пополнение баланса");
            return Err((1765432, "Не удалось получить информацию об пополнение баланса".to_string()));
        }

        match replenishment {
            None => {
                error!("Не удалось получить данные с api.lo");
                error = Some(RpcError {
                    message: "Не удалось получить данные с api.lo".to_string(),
                    code: 1765433
                })
            }
            Some(account_replenishment) => {
                result = Some(serde_json::from_value::<Vec<AccountReplenishment>>(account_replenishment.clone()).unwrap())
            }
        };

        return if result.is_some() {
            Ok(result.unwrap())
        } else {
            let error = match error {
                Some(rpc_error) => {
                    Ok(rpc_error)
                },
                None => {
                    error!("code: 1375922 message: Ошибка от arg ответ оказалась пустой");
                    Err((1375922, "Ошибка от api, ответ оказалась пустой".to_string()))
                }
            };

            if let Err(error) = error {
                error!("code: {} message: {}", error.0, error.1);
                return Err(error)
            }

            let error = error.unwrap();

            Err((error.code as i32, error.message))
        }
    }

    pub fn handler_Provider_info_ser(response: &Value) -> Option<&Value> {
        let Provider_info_res = match response.get("info") {
            Some(info) => Some(info),
            None => None,
        };

        Provider_info_res
    }

    #[allow(dead_code)]
    pub fn handler_Provider_info_des(response: &Value) -> Result<ProviderInfo, ResponseError> {
        let Provider_info_res = handler_Provider_info_ser(response);

        if let None = Provider_info_res {
            return Err((3213213, "Не удалось получить Provider_info".to_string()))
        }

        let Provider_info = Provider_info_res.unwrap();
        let Provider_info_json_to_struct: Result<ProviderInfo, serde_json::Error> = serde_json::from_value(Provider_info.clone());

        if let Err(error) =  Provider_info_json_to_struct {
            return Err((3213214, format!("Не удалось распарсить Provider_info: {}", error)))
        }

        Ok(Provider_info_json_to_struct.unwrap())
    }

    #[allow(dead_code)]
    pub fn handler_Provider_merchant_id(response: &Value) -> Result<String, ResponseError> {
        let Provider_info_res = handler_Provider_info_ser(response);

        if let None = Provider_info_res {
            return Err((3213213, "Не удалось получить Provider_info".to_string()))
        }

        let merchant_id = match Provider_info_res.unwrap().get("merchant_id") {
            Some(merchant_id) => {
                Ok(serde_json::from_value(merchant_id.clone()).unwrap())
            },
            None => Err((3213215, "Не удалось получить merchant_id".to_string()))
        };

        merchant_id
    }
}