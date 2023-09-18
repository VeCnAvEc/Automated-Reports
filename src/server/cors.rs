pub mod cors {
    use actix_cors::Cors;
    use actix_web::http::header;

    pub fn cors() -> Cors {
        Cors::default()
            // От куда могут идти запросы
            // В случае allow_any_origin запросы могут прилетать от куда угодно
            // Если нужно ограничить отпровителей запросов то можно использовать метод allowed_origin
            // и передать в него источник от которого можно принимать запросы
            // пример: allowed_origin("http://localhost:8082")
            .allow_any_origin()
            // Задает список методов который может быть выполнен
            .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"])
            // Заголовки которые могут быть отправленны
            .allowed_header(header::CONTENT_TYPE)
            // Задайте максимальное время (в секундах), в течение которого этот CORS-запрос может быть кэширован.
            .max_age(3600)
    }
}