use crate::args::Settings;

use crate::r#type::types::ResponseError;

use actix_web::web::Data;

use dotenv_codegen::dotenv;

use mysql_async::prelude::*;
use mysql_async::{Conn, Error as MysqlError, Error, Row};

use std::sync::Arc;

use tokio::sync::{ Mutex as TokioMutex };
use tracing::info;

/// code @58602
pub async fn connect_to_database(settings: &Settings) -> Result<Conn, ResponseError> {
    let url = if settings.get_prod() {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            dotenv!("GLOBAL_MYSQL_USER"),
            dotenv!("GLOBAL_MYSQL_PASSWORD"),
            dotenv!("GLOBAL_MYSQL_HOST"),
            dotenv!("GLOBAL_MYSQL_PORT"),
            dotenv!("GLOBAL_MYSQL_DATABASE"),
        )
    } else {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            dotenv!("LOCAL_MYSQL_USER"),
            dotenv!("LOCAL_MYSQL_PASSWORD"),
            dotenv!("LOCAL_MYSQL_HOST"),
            dotenv!("LOCAL_MYSQL_PORT"),
            dotenv!("LOCAL_MYSQL_DATABASE")
        )
    };

    info!(
        "\n
        mysql address: {:?}
        ",
        url
    );

    let pool = mysql_async::Pool::new(url.as_str());
    let conn = pool.get_conn().await;

    if let Err(err) = conn {
        return Err((4586020, format!("{}", err.to_string())));
    }

    return Ok(conn.unwrap());
}

pub async fn get_info_about_files_by_id(
    ids: Vec<u128>,
    conn: Data<Arc<TokioMutex<Conn>>>,
) -> Result<Vec<Row>, ResponseError> {
    let mut conn_db = conn.lock().await;

    let ids = ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<String>>()
        .join(", ");
    let id_s = ids.split(", ").collect::<Vec<&str>>();

    let result = format!(
        "SELECT file_path, `segment`, `id`, `from`, `to`, `user_id` FROM table_name WHERE id IN ({});",
        id_s.join(", ")
    )
        .as_str()
        .with(())
        .map(&mut *conn_db, |res: Row| res)
        .await;

    drop(conn_db);

    if let Err(error) = check_response(&result, 3424324) {
        return Err(error);
    }

    let result = result
        .unwrap()
        .into_iter()
        .map(|element| element)
        .collect::<Vec<Row>>();

    Ok(result)
}

pub async fn get_last_id_from_table_name(conn: Data<Arc<TokioMutex<Conn>>>) -> Result<Vec<Row>, ResponseError> {
    let mut conn_db = conn.lock().await;

    let get_max_id = format!(
        "SELECT MAX(id) from table_name",
    ).as_str()
        .with(())
        .map(&mut *conn_db, |res: Row| res)
        .await;

    drop(conn_db);

    if let Err(error) = check_response(&get_max_id, 3424325) {
        return Err(error);
    }

    return Ok(get_max_id.unwrap());
}

pub fn check_response(is_err_response_db: &Result<Vec<Row>, MysqlError>, code: i32) -> Result<(), ResponseError> {
    if let Err(error) = is_err_response_db {
        return Err((
            code,
            format!("Ошибка базы данных: {}", error.to_string())
        ));
    }

    Ok(())
}