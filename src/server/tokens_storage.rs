use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use tracing::{info, warn};

use actix_web::http::header::HeaderValue;
use crate::api_server::api_requests::RpcRequest;
use crate::api_server::response_handlers::resp_user::handlers_user::handler_user_info;

use crate::helper::get_now_time_in_unix_sec_format;
use crate::helper::user_info::user::UserInfo;
use crate::r#type::types::ResponseError;
use crate::tokio_tasks::tokio_tasks::token_tasks::INTERVAL_CLEAN_UP_TOKENS_STORAGE;

const USER_MAX_REQUESTS_SENT: u8 = 10;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Token {
    pub requests_sent: u8,
    pub max_requests_sent: u8,
    pub limited_time_to: u64,
    pub create_at: u64,
    is_admin: bool
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TokensStorage {
    tokens: HashMap<String, Token>,
}

impl Token {
    fn new(limited_time_to: u64, user_type: Option<String>) -> Self {
        let default_template_token = Token {
            requests_sent: 0,
            max_requests_sent: USER_MAX_REQUESTS_SENT,
            limited_time_to,
            create_at: get_now_time_in_unix_sec_format(),
            is_admin: false,
        };

        let token = if user_type.is_some() {
            let user_tp = user_type.unwrap().parse::<i8>().unwrap();

            // Если user_type -1 или 1 то в таком случае поле is_admin становится true
            // Если же оно пустое или же имеет другую цифру, то is_admin становится flase
            if user_tp == -1 {
                Token {
                    requests_sent: 0,
                    max_requests_sent: 0,
                    limited_time_to: 0,
                    create_at: get_now_time_in_unix_sec_format(),
                    is_admin: true,
                }
            } else {
                default_template_token
            }
        } else {
            default_template_token
        };

        return token;
    }

    pub fn update_token_info(&mut self) {
        let now = get_now_time_in_unix_sec_format();
        if now >= self.limited_time_to {
            self.limited_time_to = now + 61;
            self.requests_sent = 0;
        }
    }

    pub fn get_is_admin(&self) -> bool {
        self.is_admin
    }
}

impl TokensStorage {
    /// Создает новую структуру [TokensStorage]
    pub fn new() -> Self {
        let token_store = TokensStorage { tokens: HashMap::new() };
        token_store
    }

    /// Ищет token_admin в [TokensStorage]
    pub fn find_admin_token(&self) -> Option<String> {
        let mut admin_token = None;

        let all_keys = self.tokens.keys();
        for key in all_keys {
            match self.tokens.get(key.as_str()) {
                Some(token_info) => {
                    if token_info.is_admin {
                        admin_token = Some(key.to_string());
                    }
                }
                None => {}
            }
        }

        admin_token
    }

    /// В этой функции идет проверка токена и проверка токена на ограничение,
    /// Если лимит ограничений превышен то мы об этом предупреждаем пользователя
    /// и просим подождать
    pub async fn check_token_and_get_user_info(&mut self, token: String) -> Result<UserInfo, ResponseError> {
        let is_exist_token = self.request_is_exist_token(&token.clone()).await;

        return if is_exist_token.0 {
            let token_info_opt = self.get_mut_token(&token);
            if let Some(info) = token_info_opt {
                let now = get_now_time_in_unix_sec_format();
                // Если текущий токен не токен админа то нужно проверить его параметры
                if !info.get_is_admin() {
                    // Проверяем токен пользователя, может ли пользователь с переданным токеном отправлять запрос
                    return if info.requests_sent < info.max_requests_sent && info.limited_time_to > now {
                        info.requests_sent += 1;
                        is_exist_token.1
                    } else if info.limited_time_to <= now {
                        // Если от текущего времени до limited_time_to прошла минута или больше то мы обновляем info
                        // Ставим limited_time_to и аннулируем поле requests_sent
                        info.update_token_info();
                        info.requests_sent += 1;
                        info!("Токен {} был обнволен", token);
                        is_exist_token.1
                    } else {
                        let max_request = info.max_requests_sent;
                        let message = format!("Количество запросрв по токену {} достигло придела. Доступно {} запросов в минуту.", token, max_request);

                        info!("{}", &message);
                        Err((1854691, message))
                    }
                }

                is_exist_token.1
            } else {
                is_exist_token.1
            }
        } else {
            is_exist_token.1
        }
    }

    /// Даем запрос в api.lo и проверяем существует ли подобный токен,
    /// если да то возвращаем картеж [(bool, Result<UserInfo, ResponseError>)]
    /// bool в случае да будет равен true а в Result будет находиться Ok(UserInfo)
    /// Если же мы получим ошибку от api.lo bool будет false а в Result будет Err(ResponseError)
    pub async fn request_is_exist_token(&self, token: &String) -> (bool, Result<UserInfo, ResponseError>) {
        let token_to_header_value = HeaderValue::from_str(token.as_str());

        if let Err(_) = token_to_header_value {
            return (false, Err((4234321, "Не удалось token конвертировать в строку".to_string())));
        }

        let get_user_info = handler_user_info(RpcRequest::get_userinfo_by_token(Some(&token_to_header_value.unwrap())).await);
        match get_user_info {
            Ok(user_info) => (true, Ok(user_info)),
            Err(error) => (false, Err(error))
        }
    }

    /// Проверяем существует ли в [TokensStorage] запись с подобным token-ом
    pub fn is_exist_token(&self, token: &String) -> bool {
        self.tokens.get(token.as_str()).is_some()
    }

    /// Проверяем есть ли пользователь в TokenStorage
    /// Если пользователь не был найден то делаем проверку, существует ли пользователь в api.lo
    /// если он существует мы его добавляем в TokenStorage
    /// если нет то мы выбрасываем ошибку
    pub async fn check_for_existence_of_user_and_add_it(&mut self, token: &String) -> Result<UserInfo, ResponseError> {
        let token_info = self.get_mut_token(token);

        let user_info = match token_info {
            Some(_) => {
                self.check_token_and_get_user_info(token.clone()).await
            }
            None => {
                let token_result = self.check_token_and_get_user_info(token.clone()).await;

                if let Err(error) = token_result {
                    return Err(error);
                }

                let user_info = token_result.unwrap();

                if !self.set_new_token(token.clone(), user_info.user_type.clone()) {
                    warn!("Токен {} уже существует в token_storage", token.clone());
                }

                Ok(user_info)
            }
        };

        return user_info;
    }

    /// Получаем мутабельную структуру [Token]
    pub fn get_mut_token(&mut self, token: &String) -> Option<&mut Token> {
        self.tokens.get_mut(token.as_str())
    }

    /// Вставляет в [TokensStorage] новый токен если такого нету
    pub fn set_new_token(&mut self, token: String, user_type: Option<String>) -> bool {
        match self.tokens.get(token.as_str()) {
            None => {
                let now = get_now_time_in_unix_sec_format();
                self.tokens.insert(token, Token::new(now + 61, user_type));
                true
            }
            Some(_) => false,
        }
    }

    /// Функция для очистки старых токенов
    pub fn clean_up(&mut self) {
        self.tokens.retain(|key, token| {
            if get_now_time_in_unix_sec_format() - token.create_at <= INTERVAL_CLEAN_UP_TOKENS_STORAGE {
                info!("Токен {} по прежнему актуален", key);
                true
            } else {
                return if token.is_admin {
                    info!("Админ токен {} был удален", key);
                    false
                } else {
                    info!("Токен {} был удален", key);
                    false
                }
            }
        });
    }
}