use crate::r#type::types::ResponseError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub trait ErrorAutomatedReport {
    type AutomatedReportError;
}
pub trait ResponseAutomatedReport {
    type MicroApiResponse;
}

/// # Capu response
/// Пытался применить но пока не получилось
/// ### [`T`] ответ от [`Capu`]
/// ### [`E`] ошибка от [`Capu`]
pub enum CapuResult<T: ResponseAutomatedReport, E: ErrorAutomatedReport> {
    Ok(T),
    Err(E),
}

/// Получаем ошибку от capu и десереализуем ее вв Struct Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorData {
    pub(crate) error: Error,
    pub(crate) id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    code: i32,
    message: String,
}

/// code ошибки и message ошибки
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Info {
    code: i64,
    message: String,
}

impl ErrorAutomatedReport for ErrorData {
    type AutomatedReportError = ErrorData;
}

impl ResponseAutomatedReport for Response {
    type MicroApiResponse = Response;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub error: Value,
    pub result: Value,
}

impl Response {
    pub fn new<T>(error: Option<ResponseError>, result: Option<T>, field: Option<&str>) -> Response
        where T: Serialize + std::fmt::Debug
    {
        match error {
            None => {
                let result = result.unwrap();
                Response {
                    error: Value::Null,
                    result: serde_json::json!({ format!("{}", field.unwrap_or("data")).as_str(): result }),
                }
            }
            Some(error) => Response {
                error: serde_json::json!({
                    "code": error.0,
                    "message": error.1
                }),
                result: Default::default(),
            },
        }
    }
}
