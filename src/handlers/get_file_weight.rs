use crate::args::Settings;
use crate::error::error_response::map_io_error;
use crate::helper::file_struct::FilePath;
use actix_web::web::{Data, Json};
use actix_web::{web, Responder, HttpRequest, ResponseError};
use std::fs;
use crate::api_server::api_requests::RpcRequest;
use crate::api_server::response_handlers::resp_user::handlers_user::handler_user_info;
use crate::helper::user_info::user::UserInfo;
use crate::r#trait::automated_report_response::Response;

pub async fn get_file_weight(
    req: HttpRequest,
    path_to_file: web::Path<FilePath>,
    settings: Data<Settings>,
) -> impl Responder {
    let header = req.headers();

    let token = header.get("token");

    if let None = token {
        return Json(Response::new::<String>(
            Some((4765430, "Токен не был получен".to_string())),
            None,
            None
        ));
    }

    let user_info = handler_user_info(RpcRequest::get_userinfo_by_token(token).await);

    if let Err(error) = user_info {
        return Json(Response::new::<String>(
            Some(error),
            None,
            None
        ));
    }

    let user_id = UserInfo::get_pub_fields(&user_info.unwrap().id);

    let file_path = FilePath::get_path(&path_to_file.path, settings, user_id);

    let metadata = match fs::metadata(file_path) {
        Ok(file) => Ok(file.len()),
        Err(error) => Err(map_io_error(error)),
    };

    return if let Err(error) = metadata {
        let error_response = error.error_response().status();

        let status = error_response.to_string();

        Json(
            Response::new::<String>(
                Some((status.parse::<i32>().unwrap_or(404), error_response.to_string())),
                None,
                None
            )
        )
    } else {
        Json(
            Response::new(
                None,
                Some(metadata.unwrap().to_string()),
                Some("bytes")
            )
        )
    };
}
