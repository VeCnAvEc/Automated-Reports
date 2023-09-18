use actix_web::{HttpResponse, Result};

pub async fn handle_not_found() -> Result<HttpResponse> {
    // Возвращаем кастомный ответ с ошибкой 404
    Err(actix_web::error::ErrorNotFound("Страница не найдена"))
}
