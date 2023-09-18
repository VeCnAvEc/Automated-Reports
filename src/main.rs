pub mod args;
mod db;
pub mod download_report_chunks;
pub mod error;
pub mod handlers;
pub mod helper;
pub mod indexing_report_struct;
pub mod routes;
pub mod share;
pub mod r#trait;
mod r#type;
mod server;
mod tokio_tasks;
mod api_server;

use std::env;
use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex};

use actix_web::web::Data;

use dotenv_codegen::dotenv;

use env_logger::Env;

use crate::share::Share;

use crate::r#trait::chunks_trait::IDownloadReportChunks;

use crate::args::Settings;

use crate::db::connect::connect_to_database;

use tokio::sync::{Mutex as TokioMutex, RwLock as TokioRwLock};

use tracing;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;
use crate::r#type::types::ReportsStorage;
use crate::server::tokens_storage::TokensStorage;
use crate::tokio_tasks::tokio_tasks::{launch_database_handlers, launch_share_handlers, launch_token_handlers};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let address = dotenv!("HOST_ADDRESS");
    let port = dotenv!("HOST_PORT").parse::<u16>().unwrap_or(8080);

    let subscriber = FmtSubscriber::builder()
        // TRACE
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

    let mut args: Vec<String> = env::args().collect();
    let mut settings = Settings::new();

    let get_proda_with_global_var = if args.len() > 1 {
        args
    } else {
        if let Ok(bol) = env::var("proda") {
            args.push(format!("proda={}", bol).to_string());
            args
        } else {
            args.push("proda=false".to_string());
            args
        }
    };

    match settings.set_prod(get_proda_with_global_var) {
        Ok(_) => {
            info!("Autotuning: everything is set up safely");
        }
        Err(error) => error!(
            "Autotuning: an error occurred in the auto settings:\nmessage: {}\ncode: {}",
            error.1, error.0
        ),
    }

    info!("{}", format!("Settings: {:#?}", settings));
    info!(
        "\n
            host address: {}\n
            host port: {}\n
        ",
        address,
        port
    );
    if settings.get_prod() {
            info!("
                \nmysql host: {}\n
                mysql port: {}\n
                mysql user: {}\n
                mysql password: {}\n
                mysql database: {}\n
            ",
                dotenv!("GLOBAL_MYSQL_HOST"),
                dotenv!("GLOBAL_MYSQL_PORT"),
                dotenv!("GLOBAL_MYSQL_USER"),
                dotenv!("GLOBAL_MYSQL_PASSWORD"),
                dotenv!("GLOBAL_MYSQL_DATABASE")
            );
    } else {
        info!(
            "\n
            mysql host: {}\n
            mysql port: {}\n
            mysql user: {}\n
            mysql password: {}\n
            mysql database: {}\n
            ",
            dotenv!("LOCAL_MYSQL_HOST"),
            dotenv!("LOCAL_MYSQL_PORT"),
            dotenv!("LOCAL_MYSQL_USER"),
            dotenv!("LOCAL_MYSQL_PASSWORD"),
            dotenv!("LOCAL_MYSQL_DATABASE")
        );
    }

    let db = connect_to_database(&settings).await;
    if let Err(error) = db {
        if settings.get_prod() {
            error!("Не удалось подключиться к базе данных: {}\naddress: {}\nport: {}", error.1, dotenv!("GLOBAL_MYSQL_HOST"), dotenv!("GLOBAL_MYSQL_PORT"));
        } else {
            error!("Не удалось подключиться к базе данных: {}\naddress: {}\nport: {}", error.1, dotenv!("LOCAL_MYSQL_HOST"), dotenv!("LOCAL_MYSQL_PORT"));
        }
        return Err(Error::new(
            ErrorKind::NotConnected,
            format!("Не удалось подключиться к базе данных: {}", error.1)
        ));
    }

    let tokens_storage = Data::new(TokioRwLock::new(TokensStorage::new()));

    let share: ReportsStorage = Data::new(TokioRwLock::new(Share::new()));
    let report_download_chunks = Data::new(Arc::new(Mutex::new(
        download_report_chunks::DownloadReportChunks::new(),
    )));

    let conn_db = Data::new(Arc::new(TokioMutex::new(
        db.unwrap(),
    )));
    let settings = Data::new(settings);

    env_logger::init_from_env(Env::default().default_filter_or("info"));
    env::set_var("RUST_LOG", "debug");

    info!("launching `database worker`...");
    let _database_worker = launch_database_handlers(Data::clone(&conn_db), Data::clone(&settings)).await;
    info!("share worker has been launched.");

    info!("launching `share worker`...");
    let _share_worker = launch_share_handlers(Data::clone(&share)).await;
    info!("share worker has been launched.");

    info!("launching `tokens worker`...");
    let _tokens_worker = launch_token_handlers(Data::clone(&tokens_storage)).await;
    info!("tokens worker has been launched.");

    info!("launching `server`");
    let _server = server::server::run(
        address,
        port,
        report_download_chunks,
        settings,
        conn_db,
        share,
        tokens_storage
    ).await;

    Ok(())
}
