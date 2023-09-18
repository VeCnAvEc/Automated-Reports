/// Модуль для работы с асинхронными задачами токенов
pub mod tokio_tasks {
    use std::sync::Arc;
    use actix_web::web::Data;
    use mysql_async::Conn;
    use tokio::sync::{RwLock as TokioRwLock, Mutex as TokioMutex};
    use crate::args::Settings;
    use crate::r#type::types::ReportsStorage;
    use crate::server::tokens_storage::TokensStorage;

    /// Запускает таски по токенам
    pub async fn launch_token_handlers(token_storage: Data<TokioRwLock<TokensStorage>>) {
        token_tasks::clean_up_old_token(token_storage).await;
    }

    /// Запускает таски по шеру
    pub async fn launch_share_handlers(share: ReportsStorage) {
        share_tasks::remove_report(share).await;
    }

    /// Запускает таски по базе данных
    pub async fn launch_database_handlers(db_conn: Data<Arc<TokioMutex<Conn>>>, settings: Data<Settings>) {
        database_task::check_database_connection(db_conn, settings).await;
    }

    pub mod token_tasks {
        use std::time::Duration;
        use actix_web::web::Data;
        use tokio::sync::RwLock as TokioRwLock;
        use tokio::time::interval;
        use crate::server::tokens_storage::TokensStorage;

        /// 3600 секунд == 1 час
        pub const INTERVAL_CLEAN_UP_TOKENS_STORAGE: u64 = 15;

        /// Очищает старые токены каждые [INTERVAL_CLEAN_UP_TOKENS_STORAGE] секунд
        pub async fn clean_up_old_token(token_storage: Data<TokioRwLock<TokensStorage>>) {
            let mut interval_clean_up = interval(Duration::from_secs(INTERVAL_CLEAN_UP_TOKENS_STORAGE));

            // Запускаем таймер и вызываем функцию очистки токенов для каждого пользователя
            tokio::spawn(async move {
                loop {
                    interval_clean_up.tick().await;
                    let mut token_state = token_storage.write().await;
                    token_state.clean_up();
                }
            });
        }
    }

    pub mod share_tasks {
        use std::time::Duration;
        use chrono::Utc;
        use tokio::time::interval;
        use tracing::info;
        use crate::r#type::types::ReportsStorage;

        const INTERVAL_TIME_REMOVE_REPORTS: u64 = 1800;

        pub async fn remove_report(share: ReportsStorage) {
            let mut interval = interval(Duration::from_secs(INTERVAL_TIME_REMOVE_REPORTS));

            tokio::spawn(async move {
                loop {
                    interval.tick().await;
                    let mut report_keys = Vec::new();

                    let share_reader =  share.read().await;

                    for key in share_reader.reports.data.read().await.keys() {
                        report_keys.push(key.to_string());
                    }

                    let mut share_writer =  share_reader.reports.data.write().await;

                    for rp_key in report_keys.iter() {
                        let mut interval_time_creating_report = 0;

                        if let Some(report) = share_writer.get(rp_key) {
                            let now_time = Utc::now().timestamp();
                            let report = report.0.read().await;

                            let time_of_report_creation = report.create_at;

                            interval_time_creating_report = now_time - time_of_report_creation;
                        }

                        if interval_time_creating_report >= INTERVAL_TIME_REMOVE_REPORTS as i64 {
                            share_writer.remove(rp_key);
                            info!("Отчет {} был удален", rp_key);
                        }
                    }
                }
            });
        }
    }

    pub mod database_task {
        use std::sync::{Arc};
        use std::time::Duration;

        use actix_web::web::Data;
        use mysql_async::Conn;
        use mysql_async::prelude::Queryable;
        use tokio::sync::{Mutex as TokioMutex, MutexGuard as TokioMutexGuard};
        use tokio::time::interval;
        use tracing::{error, info};
        use crate::args::Settings;
        use crate::db::connect::connect_to_database;

        const INTERVAL_CHECK_DATABASE_CONNECTION: u64 = 10;

        pub async fn check_database_connection(mut db_conn: Data<Arc<TokioMutex<Conn>>>, settings: Data<Settings>) {
            let mut interval = interval(Duration::from_secs(INTERVAL_CHECK_DATABASE_CONNECTION));

            tokio::spawn(async move {
                loop {
                    interval.tick().await;
                    let ping_db = db_conn.lock().await.ping().await;

                    if let Err(error) = ping_db {
                        error!("Не удалось пропинговать базу данных: {}", error);
                        let settings_clone = Data::clone(&settings);

                        let conn = connect_to_database(&**settings_clone).await;
                        if let Err(error) = conn {
                            error!("Не удалось повторно подключиться к базе данных - code: {} message: {}", error.0, error.1);
                            panic!("Приложение не может корректно работать из-за отсутствие подключения к базе данных ");
                        }
                        let new_conn = Data::new(Arc::new(TokioMutex::new(conn.unwrap())));
                        db_conn = new_conn;
                        info!("Было успешно создано новое подключение к базе данных!");
                    }
                }
            });
        }
    }
}
