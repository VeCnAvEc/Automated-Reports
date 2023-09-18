
mod cors;
pub(crate) mod tokens_storage;

pub mod server {
    use std::sync::{Arc, Mutex};

    use actix_web::{App, HttpServer};
    use actix_web::middleware::Logger;
    use actix_web::web::Data;
    use tokio::sync::{Mutex as TokioMutex, RwLock as TokioRwLock};
    use tracing::info;
    use crate::args::Settings;
    use crate::download_report_chunks::DownloadReportChunks;
    use crate::routes::routes;
    use crate::server::cors::cors::cors;
    use crate::server::tokens_storage::TokensStorage;
    use crate::share::Share;


    pub async fn run(
        address: &str,
        port: u16,
        report_download_chunks: Data<Arc<Mutex<DownloadReportChunks>>>,
        settings: Data<Settings>,
        db: Data<Arc<TokioMutex<mysql_async::Conn>>>,
        share: Data<TokioRwLock<Share>>,
        tokens_storage: Data<TokioRwLock<TokensStorage>>
    ) -> std::io::Result<()> {
        info!("THE WEB SERVER IS RUNNING");
        HttpServer::new(move || {
            App::new()
                // Дефолтные логи актикса
                .wrap(Logger::default())
                // Логирует user-agent
                .wrap(Logger::new("%a %{User-Agent}i"))
                .wrap(
                    cors()
                )
                // Общий доступ к базе данных
                .app_data(Data::clone(&db))
                // Общий доступ к общим данным
                .app_data(Data::clone(&share))
                // Общий доступ для скаченным чанкам (Стоит ограничить доступ и оставить его только для `download_report`)
                .app_data(Data::clone(&report_download_chunks))
                // Общие настройки
                .app_data(Data::clone(&settings))
                // Хранилище токенов
                .app_data(Data::clone(&tokens_storage))
                // Роутинг
                .configure(routes)
        })
        .bind((address, port))
        .expect(format!("Не удалось подключиться по адресу {}:{}", address, port).as_str())
        .run()
        .await
    }
}
