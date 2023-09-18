use crate::share::Share;
use actix_web::web::{Data, Json};
use actix_web::{HttpRequest, Responder};
use tokio::sync::RwLock as TokioRwLock;
use tracing::warn;
use crate::api_server::api_token::utils::format_utils::token_utils::{handle_token_error, token_format_to_string};
use crate::r#trait::automated_report_response::Response;
use crate::server::tokens_storage::TokensStorage;
use crate::share::share_helper::ShareHelper;

pub async fn get_share(reqeust: HttpRequest, share: Data<TokioRwLock<Share>>, token_storage: Data<TokioRwLock<TokensStorage>>) -> impl Responder {
    let get_token = token_format_to_string(reqeust.headers());

    if let Some(error) = handle_token_error(&get_token) {
        return error;
    }

    if !token_storage.read().await.is_exist_token(&get_token.clone().unwrap()) {
        let mut token_storage_guard = token_storage.write().await;

        let token_result = token_storage_guard.check_token_and_get_user_info(get_token.clone().unwrap()).await;

        if let Err(error) = token_result {
            return Json(Response::new(
                Some(error),
                None::<ShareHelper>,
                None
            ));
        }

        let user_info = token_result.unwrap();

        if !token_storage_guard.set_new_token(get_token.clone().unwrap().clone(), user_info.user_type.clone()) {
            warn!("Токен {} уже существует в token_storage", get_token.clone().unwrap());
        }
    }

    let create_share_helper = ShareHelper::share_to_share_helper(Data::clone(&share)).await;

    if let Err(error) = create_share_helper {
        return Json(Response::new(
            Some(error),
            None::<ShareHelper>,
            None
        ));
    }

    Json(Response::new(
        None,
        Some(create_share_helper.unwrap()),
        Some("share")
    ))
}
