use actix_web::web;

use crate::handlers::not_found::handle_not_found;
use crate::handlers::{
    download_report::Streamer, generate_report::generate_report,
    get_file_weight::get_file_weight, get_share::get_share,
};

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Что-бы сгенерировать excel
        .route("/generate_file", web::post().to(generate_report))
        // Получаем все данные которые находятся в share
        .route("/get_share", web::get().to(get_share))
        // Получить amount за все дни определенного провайдера с определнными фильтрами
        .service(
            web::scope("/download")
                // Скачать отчет /{path}
                .route("/{path}", web::get().to(Streamer::download_report))
                // Получит вес отчета /get_weight/{path}
                .route("/get_weight/{path}", web::get().to(get_file_weight)),
        )
        .route("/{any:.*}", web::get().to(handle_not_found));
}
