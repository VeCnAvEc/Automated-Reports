pub mod err_utils {
    use crate::r#type::types::{ChunksInReport, ResponseError};

    /// Проверка на ошибки messages и codes
    pub fn is_check_on_errors_message_and_code(errors: &Vec<ResponseError>) -> bool {
        if errors.is_empty() {
            false
        } else {
            true
        }
    }

    /// Получить последнию ошибку messages и codes
    pub fn get_last_error_message_and_code(errors: &Vec<ResponseError>) -> ResponseError {
        return errors.last().unwrap().clone();
    }
    
    /// Получить последнию ошибку messages и codes
    pub fn get_first_error_message_and_code(errors: &Vec<ResponseError>) -> ResponseError {
        return errors.first().unwrap().clone();
    }

    pub fn chunk_is_empty(unwrap_chunks: &ChunksInReport, id: u32) -> Result<(), ResponseError> {
        if unwrap_chunks.is_empty() {
            let message = format!("Файл под id {} не содержит в себе нужных вам данных", id);
            return Err((4334304, message));
        } else {
            Ok(())
        }
    }
}