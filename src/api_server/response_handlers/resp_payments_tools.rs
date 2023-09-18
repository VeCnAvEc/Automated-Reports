pub mod handlers_payments_tools {
    use std::collections::HashMap;

    use serde_json::Value;
    use serde::{Deserialize, Serialize};

    use crate::r#type::types::ResponseError;

    #[derive(Deserialize, Serialize, Debug)]
    pub struct PaymentsTools {
        available_payment_tools: HashMap<String, Value>
    }

    #[allow(dead_code)]
    pub fn handler_payments_tools(response: Result<&Value, ResponseError>) -> Result<PaymentsTools, ResponseError> {
        let mut payments_tools = PaymentsTools { available_payment_tools: HashMap::new() };

        match response {
            Ok(available_payments_tools) => {
                match available_payments_tools.get("available_tools") {
                    Some(tools) => {
                        match tools {
                            Value::Object(parse_value) => {
                                parse_value.iter().for_each(|(k, v)| {
                                    payments_tools.available_payment_tools.insert(k.clone(), v.clone());
                                });
                            }
                            _ => {
                                return Err((2443243, "Получен не верный формат платежных инструменотов".to_string()));
                            }
                        }
                    }
                    None => {
                        return Err((2443244, "Не удалось получить поле `available_payment_tools`".to_string()))
                    }
                }
            }
            Err(error) => {
                return Err(error);
            }
        }

        Ok(payments_tools)
    }

}