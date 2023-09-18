use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime};

use csv::StringRecord;
use dotenv_codegen::dotenv;
use actix_web::http::header::{HeaderMap, HeaderValue};
use actix_web::web::Data;
use crate::api_server::api_requests::RpcRequest;
use crate::api_server::response_handlers::resp_provider::handlers_provider::handler_provider_recent_deposits;
use crate::api_server::response_handlers::resp_user::handlers_user::AccountReplenishment;
use crate::args::Settings;
use crate::r#type::types::ResponseError;

pub mod create_file;
pub mod date;
pub mod file_struct;
pub mod generate_xlsx;
pub mod handler_info_about_file_by_id;
pub mod xlsx_help_fun;
pub mod build_tasks;
pub mod working_with_xlsx_list;
pub mod chunks;
pub mod user_info;
pub mod report_type;

#[allow(dead_code)]
enum PaymentSystem {
    UZS(String),
    USD(String),
    EUR(String),
    RUB(String),
    NULL
}

#[allow(dead_code)]
impl PaymentSystem {
    pub fn new(payment_platform_name: String) -> Self {
        match payment_platform_name.to_lowercase().as_str() {
            "humo" | "uzcard" => PaymentSystem::UZS("Национальная валюта".to_string()),
            "pulz" => PaymentSystem::UZS("Кошелек UZS".to_string()),
            "Наличный" => PaymentSystem::UZS("Наличный платеж".to_string()),
            "ecomm kapital24 usd" => PaymentSystem::USD("$".to_string()),
            "ecomm kapital24 eur" => PaymentSystem::EUR("€".to_string()),
            "mir pay" => PaymentSystem::RUB("₽".to_string()),
            _ => PaymentSystem::NULL
        }
    }
}

pub fn type_report_that_generated(tp: i8) -> String {
    match tp {
        1 => "pay",
        4 => "pay_f",
        2 => "c2card",
        5 => "c2cCOMANYNAME",
        6 => "c2cplum",
        7 => "c2ckapitalbank",
        8 => "c2cpayme",
        3 => "terminal",
        9 => "c2cuzcard",
        _ => "null",
    }
    .to_lowercase()
}

pub fn from_string_record_to_vec<'a>(record: &'a StringRecord) -> Vec<&'a str> {
    record.iter().map(|field| {
        field
    }).collect::<Vec<&'a str>>()
}

pub fn get_mutex_result_or_error<T>(mutex: &Arc<Mutex<T>>) -> Result<MutexGuard<T>, String> {
    mutex.lock().map_or_else(|error| {
        Err(format!("Возникла прпоблема при блокирования мютекса: {:?}", error))
    }, |mutex_guard| {
        Ok(mutex_guard)
    })
}

pub struct GuardWrapper<'a, T> {
    pub guard: MutexGuard<'a, T>,
}

pub fn compare_user_id(user_id: &Option<String>, id_of_file_owner: Vec<&isize>) -> bool {
    let error_user_id = "-1".to_string();
    let user_id = match user_id {
        Some(int) => int,
        None => &error_user_id
    };
    let int_id = user_id.parse::<isize>().unwrap_or(-1);

    if int_id == -1 {
        return false;
    }

    let all_equal = id_of_file_owner.iter().all(|&x| x == &int_id);
    all_equal
}

pub fn is_exist_file(keys: &Vec<String>, user_id: isize, settings: Data<Settings>) -> Option<String> {
    let report_dir = if settings.get_prod() {
        dotenv!("PROD_REPORTS_DIR")
    } else {
        dotenv!("REPORTS_DIR")
    };

    for key in keys {
        let full_path = format!("{}/reports/{}/{key}", report_dir, user_id);
        let path = Path::new(&full_path);

        return if path.exists() {
            Some(full_path)
        } else {
            None
        }
    }

    return None
}

pub fn get_token_from_header(headers: &HeaderMap) -> Result<&HeaderValue, ResponseError> {
    return match headers.get("token") {
        Some(token) => Ok(token),
        None => Err((54346941, "Не удалось получить токен".to_string()))
    };
}

pub  fn get_now_time_in_unix_sec_format() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}

pub fn build_payment_filter_name(payments_systems: &Vec<Vec<String>>) -> String {
    let mut p_s = Vec::new();
    let mut p_build = "".to_string();

    for system in payments_systems {
        let mut f_p_s = system.clone();
        f_p_s.sort();
        p_s.push(system);
    }

for sort_systems in p_s {
        p_build.push_str(sort_systems.join("").as_str());
    }

    p_build
}

pub async fn get_refill(provider_id: String, token: String) -> Result<Vec<AccountReplenishment>, ResponseError> {
    let Provider_info_res = RpcRequest::get_provider_info(provider_id, token.clone()).await;

    if let Err(error) = Provider_info_res {
        return Err(error);
    }

    handler_provider_recent_deposits(&Provider_info_res.unwrap())
}