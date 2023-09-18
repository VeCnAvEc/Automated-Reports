use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

use lazy_static::lazy_static;

use actix_web::web::{Data, Json};
use actix_web::{HttpRequest, Responder};

use serde::Deserialize;

use tokio::sync::{ Mutex as TokioMutex, RwLock as TokioRwLock };

use crate::args::Settings;
use dotenv_codegen::dotenv;
use mysql_async::Conn;
use serde_json::Value;
use tracing::{error, info};
use crate::api_server::api_token::utils::format_utils::token_utils::{handle_token_error, token_format_to_string};

use crate::db::connect::{get_info_about_files_by_id, get_last_id_from_table_name};
use crate::error::errors_utils::err_utils::{chunk_is_empty, get_first_error_message_and_code, get_last_error_message_and_code};
use crate::helper::generate_xlsx::{generate_report_from_csv};
use crate::helper::handler_info_about_file_by_id::{handle_info_about_file, handle_last_id};
use crate::helper::{compare_user_id, is_exist_file, type_report_that_generated};
use crate::helper::chunks::chunk_manager::creator_of_chunks::create_chunks_by_types;
use crate::helper::user_info::user::UserInfo;
use crate::r#trait::automated_report_response::Response;
use crate::r#trait::filter_report::{Filter, ReportType, Status};
use crate::r#type::types::{InformationAboutFileMicroApiDB, InformationAboutFileMicroApiDBResult, ReportsDateRange, ReportsStorage, ResponseError};
use crate::server::tokens_storage::TokensStorage;
use crate::share::{ArcMutexWrapper, Report, Share};

// Временное хранилище для генерируемых хешей
lazy_static! {
    pub static ref GENERATED_HASHES: Arc<TokioRwLock<Vec<String>>> = Arc::new(TokioRwLock::new(Vec::new()));
}

pub async fn check_generated_hashes(share: ReportsStorage) {
    for key in share.read().await.reports.get_keys().await.iter() {
        GENERATED_HASHES.write().await.retain(|item| item != key);
    }
}

#[derive(Debug, Deserialize)]
enum TypeGenerateReport {
    Csv,
    Xlsx,
}

#[derive(Debug, Deserialize)]
pub struct GenerateFile {
    /// [Provider id] Provider id это уникальный id провайдера, по которому будет ввестись фильтрация
    #[serde(default, deserialize_with = "deserialize_string_or_integer")]
    pub provider_id: Option<String>,
    /// [Merchantid] Merchantid это уникальный id вендора, по которому будет ввестись фильтрация
    pub merchant_id: Option<String>,
    /// [Filters] Дополнительные филтры, в которых находится инфа по файлу, иногда будет 2 [FileDate]
    /// к примеру когда мы будем генерировать отчет для таксопарка. Для отчета таксопарка нужно 2 файла один переводы,
    /// другой платежи.
    pub filters: Vec<Filter>,
    /// [Reporty type] Есть 3 вида отчета, [Agent, TaxiCompony, Merchant] пользователь должен ввести один из трех типов
    pub report_type: Option<ReportType>,
    /// [Monthly subscription fee]
    #[serde(default, deserialize_with = "deserialize_float_or_integer")]
    pub monthly_subscription_fee: Option<f64>
}

pub fn deserialize_string_or_integer<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Value = serde::Deserialize::deserialize(deserializer)?;

    match value {
        Value::String(s) => Ok(Some(s)),
        Value::Number(n) if n.is_i64() => Ok(Some(n.to_string())),
        _ => Ok(None),
    }
}

fn deserialize_float_or_integer<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Value = serde::Deserialize::deserialize(deserializer)?;

    match value {
        Value::String(s) => Ok(Some(s.parse::<f64>().unwrap_or(1_000_000.0))),
        Value::Number(n) if n.is_i64() => Ok(Some(n.as_f64().unwrap_or(1_000_000.0))),
        _ => Ok(None), // Handle other cases as needed
    }
}

impl GenerateFile {
    pub fn filters_validation_for_uniqueness<T, F>(&self, field: F, field_name: &str) -> Result<(), Response>
    where
        T: Debug + Eq + Hash,
        F: Fn(&Filter) -> &T,
    {
        let mut set = HashSet::new();

        for filter in &self.filters {
            let value = field(filter);
            if !set.insert(value) {
                return Err(Response::new::<String>(
                    Some((6453453, format!("У вас не может быть нескольких фильтров с одним и тем же {}: {:?}.", field_name, value))),
                    None,
                    None
                ));
            }
        }

        Ok(())
    }

    pub fn set_default_monthly_subscription_fee(&mut self) {
        if let None = self.monthly_subscription_fee {
            self.monthly_subscription_fee = Some(1_000_000.0)
        }
    }

    pub async fn beyond_last_id(conn: Data<Arc<TokioMutex<Conn>>>, ids: &Vec<u128>) -> Result<(), ResponseError> {
        let last_id = get_last_id_from_table_name(conn).await;
        let handle_id = handle_last_id(last_id);

        if let Err(error) = handle_id {
            return Err(error);
        }

        let mut errors: Vec<ResponseError> = Vec::new();

        let handle_id = handle_id.unwrap();

        ids.iter().for_each(|id| {
            if handle_id < *id as isize {
                errors.push((6546534, format!("Id: {} превышает максимальный id", id)));
            }
        });

        return if !errors.is_empty() {
            Err(get_last_error_message_and_code(&errors))
        } else {
            Ok(())
        }
    }

    pub fn trim<T>(field: &mut Option<T>) -> String
        where T: ToString + std::borrow::Borrow<str>
    {
        match field  {
            None => "".to_string(),
            Some(str) => str.borrow().trim().to_string(),
        }
    }

    /// Получаем все статусы, моды, платежные системы в переданных фильрах
    pub fn get_all_s_m_p(&self) -> (Vec<Status>, Vec<String>, Vec<Vec<String>>) {
        // Для статусов
        let mut s = Vec::new();
        // Для модов
        let mut m = Vec::new();
        // Для платженых систем
        let mut p = Vec::new();

        for filter in self.filters.iter() {
            s.push(filter.status.clone().unwrap_or(Status::Unknown));
            m.push(filter.mode.clone().unwrap_or("".to_string()));
            let mut f_p_s = filter.payments_system.clone().unwrap_or(Vec::new());
            f_p_s.sort();
            p.push(f_p_s);
        }

        (s, m, p)
    }

    pub fn check_merchant_id_by_report_type(&self) -> Result<(), ResponseError>{
        fn is_none_merchant_id(merchant_id: &Option<String>) -> bool {
            if merchant_id.is_none() {
                true
            } else {
                false
            }
        }

        // Если тип отчета Merchant то все то поле merchant_id может быть быстым а может и не быть пустым
        // Ну а если report_type не Merchnt и поле merchant_id не пустое, то в таком случае мы возвращаем ошибку
        match self.report_type {
            None => Err((7357542, "report type не был передан в запрос".to_string())),
            Some(rp_type) => {
                let error = Err((7357543, "merchant_id может быть передан только для отчета Merchant".to_string()));
                match rp_type {
                    ReportType::Merchant => {
                        if !is_none_merchant_id(&self.merchant_id) {Ok(())} else {Ok(())}
                    }
                    _ => {
                        if is_none_merchant_id(&self.merchant_id) {Ok(())} else {return error}
                    }
                }
            }
        }

    }

    pub fn check_provider_id_by_report_type(&self) -> Result<(), ResponseError>{
        fn is_none_provider_id(provider_id: &Option<String>) -> bool {
            if provider_id.is_none() {
                true
            } else {
                false
            }
        }

        // Если тип отчета Merchant то все то поле merchant_id может быть быстым а может и не быть пустым
        // Ну а если report_type не Merchnt и поле merchant_id не пустое, то в таком случае мы возвращаем ошибку
        match self.report_type {
            None => Err((7357542, "report type не был передан в запрос".to_string())),
            Some(rp_type) => {
                let error = Err((8357543, "provider_id может быть передан только для отчета Agent/TaxiCompany".to_string()));
                match rp_type {
                    ReportType::Agent | ReportType::TaxiCompany => {
                        if !is_none_provider_id(&self.provider_id) {Ok(())} else {Ok(())}
                    }
                    _ => {
                        if is_none_provider_id(&self.provider_id) {Ok(())} else {return error}
                    }
                }
            }
        }

    }
}
/// [Генерация Отчетов с Фильтрами] [Post Request] Получить и сгенерировать отчет по фильтрам [impl Filter]
pub async fn generate_report(
    req: HttpRequest,
    mut reqeust_generate: Json<GenerateFile>,
    token_storage: Data<TokioRwLock<TokensStorage>>,
    share: Data<TokioRwLock<Share>>,
    conn_db: Data<Arc<TokioMutex<Conn>>>,
    settings: Data<Settings>,
) -> impl Responder {
    if let Err(error) = reqeust_generate.check_merchant_id_by_report_type() {
        return Json(Response::new::<String>(
            Some(error),
            None,
            None
        ))
    }

    if let Err(error) = reqeust_generate.check_provider_id_by_report_type() {
        return Json(Response::new::<String>(
            Some(error),
            None,
            None
        ))
    }

    let organization_provider_id = match reqeust_generate.report_type {
        None => "".to_string(),
        Some(rp_type) => {
            match rp_type {
                ReportType::Agent => {
                    reqeust_generate.provider_id = Some(GenerateFile::trim(&mut reqeust_generate.provider_id));
                    GenerateFile::trim(&mut reqeust_generate.provider_id)
                },
                ReportType::TaxiCompany => {
                    reqeust_generate.provider_id = Some(GenerateFile::trim(&mut reqeust_generate.provider_id));
                    GenerateFile::trim(&mut reqeust_generate.provider_id)
                },
                ReportType::Merchant => {
                    reqeust_generate.merchant_id = Some(GenerateFile::trim(&mut reqeust_generate.merchant_id));
                    GenerateFile::trim(&mut reqeust_generate.merchant_id)
                },
                ReportType::Unknown => "".to_string()
            }
        }
    };
    // Приводим значение Provider в порядок

    // Устанавливаем нижний регистр для фильтров платежных систем
    for filter in reqeust_generate.filters.iter_mut() {
        filter.set_to_lowercase_payments_system_field();
    }

    let token_res = token_format_to_string(req.headers());

    if let Some(error) = handle_token_error(&token_res) {
        return error;
    };

    let token = token_res.unwrap();

    let mut user_info_opt: Option<UserInfo> = None;

    if !token_storage.read().await.is_exist_token(&token) {
        match token_storage.write().await.check_for_existence_of_user_and_add_it(&token).await {
            Ok(usr_info) => user_info_opt = Some(usr_info),
            Err(error) => {
                return Json(Response::new::<String>(
                    Some(error),
                    None,
                    None
                ));
            }
        }
    } else {
        match token_storage.read().await.request_is_exist_token(&token.clone()).await.1 {
            Ok(result) => user_info_opt = Some(result),
            Err(error) => {
                return Json(Response::new::<String>(
                    Some(error),
                    None,
                    None
                ))
            }
        }
    }

    if let None = user_info_opt {
        return Json(Response::new::<String>(
            Some((1213352, "Не удалось получить user info".to_string())),
            None,
            None
        ))
    }

    let user_info = user_info_opt.unwrap();
    let user_id = UserInfo::check_on_error(UserInfo::get_pub_fields(&user_info.id));

    if let Err((code, message)) = &user_id {
       error!("code: {} message: {}", code, message);
       info!("Генерация отчета запущена, для пользователя: -1");
    } else {
        let id = user_id.clone().unwrap();
        let keys = share.read().await.reports.get_keys().await;

        // Возвращаем путь до файла если он уже сгенерирован.
        if let Some(path) = is_exist_file( &keys, id.parse::<isize>().unwrap_or(-1), Data::clone(&settings) ) {
            return Json(Response::new(
                None,
                Some(path),
                Some("path")
            ))
        }

        info!("Генерация отчета запущена, для пользователя: {}", id);
    }

    // Проверяем уникальность каждого переданного id
    if let Err(error) = reqeust_generate.filters_validation_for_uniqueness(|filter| &filter.id, "id") {
        let error_message = match error.error.get("message") {
            Some(message) => serde_json::from_value::<String>(message.clone())
                .map_err(|error| format!("Произашла не известная ошибка в функции filters_validation_for_uniqueness\n-Error: {}", error.to_string())).unwrap(),
            None => "Произашла не известная ошибка в функции `filters_validation_for_uniqueness`".to_string()
        };

        error!("{}", error_message);

        return Json(error);
    }

    let mut all_filters_id = reqeust_generate
        .filters
        .iter()
        .map(|file_info| file_info.id as u128)
        .collect::<Vec<u128>>();

    // Делаем проверку переданных id, если вдруг переданные id превышают последний id по номеру в db, то мы возвращаем ошибку
    let check_last_id = GenerateFile::beyond_last_id(Data::clone(&conn_db), &all_filters_id).await;
    if let Err(error) = check_last_id {
        return Json(Response::new::<String>(
            Some(error),
            None,
            None
        ));
    }

    let mut errors: Vec<ResponseError> = Vec::new();

    // Даем запрос в базу данных на нужные данные по котором мы будем генерировать отчет
    let info_about_files_by_id = get_info_about_files_by_id(all_filters_id.clone(), Data::clone(&conn_db))
        .await
        .map_or_else(
            |error| Err(error),
            |info_about_csv_file| Ok(info_about_csv_file),
        );

    // Return json error
    if let Err(error) = info_about_files_by_id {
        error!("code: {} message: {}", error.0, error.1);
        return Json(Response::new::<String>(Some(error), None, None));
    }

    // 1. index_file, 2. path_to_file, 3. file_type, 4. file_status, 5. from, 6. to, 7. user_id
    // Функция handle_info_about_file принимает &mut errors если в процессе обработки файловой информации возникнет ошибка
    // в errors передастся ошибка
    let files_info: InformationAboutFileMicroApiDBResult = handle_info_about_file(info_about_files_by_id, &mut errors, &settings);

    // Если errors не пустой то возвращаем последнию ошибку в errors
    if !errors.is_empty() {
        let last_error = get_last_error_message_and_code(&errors);

        error!("code: {} message: {}", last_error.0, last_error.1);

        return Json(Response::new::<String>(
            Some(
                last_error,
            ),
            None,
            None
        ));
    }

    // Данные о файлах по которым мы будем генерировать отчет
    let files_info = files_info.iter()
        .map(|file_data| file_data.clone().unwrap()).collect::<InformationAboutFileMicroApiDB>();

    // id, from, to информация о дате по которой был сформирован отчет
    let full_date_from_to = files_info.iter()
        .map(|info_file| (info_file.0, info_file.4.clone(), info_file.5.clone())).collect::<Vec<(usize, String, String)>>();

    // Получаем user_id каждого запрошенного файла
    let ids_of_files_owners = files_info.iter().map(|(_, _, _, _, _, _, user_id )| user_id).collect::<Vec<&isize>>();

    // Проверяем, можно ли генерировать пользователю который запросил отчет, генерировать файл по запрошенным id файлов.
    // true значит можно
    // false значит файл принадлежит не текущему пользователю
    if compare_user_id(&user_info.id, ids_of_files_owners) {
        let rp_tp = files_info
            .iter()
            .map(|file_info| {
                let (id_file, _, file_type, _, _, _, _) = &file_info;
                (id_file, type_report_that_generated(*file_type as i8))
            })
        .collect::<Vec<(&usize, String)>>();

        // В этих циклах мы записываем path до файла в фильтре.
        for file in files_info.iter() {
            for filter in reqeust_generate.filters.iter_mut() {
                let fl = file;
                if fl.0 == filter.id as usize {
                    filter.set_path_to_file(fl.1.clone());
                }
            }
        }

        let report_type = match reqeust_generate.report_type {
            None => ReportType::Unknown,
            Some(report_item) => report_item
        };

        rp_tp.iter().for_each(|(id, file_type)| {
            reqeust_generate.filters.iter_mut().for_each(|filter| {
                if filter.id == **id as u32 {
                    filter.set_type_of_report_we_depend(file_type.clone());
                }
            })
        });

        let mut path_to_files = Vec::new();
        let mut from_to: ReportsDateRange = Vec::new();

        for file in files_info.iter() {
            path_to_files.push(file.clone().1);
            from_to.push((file.clone().4, file.clone().5));
        }

        all_filters_id.sort();
        let build_id_for_name = all_filters_id.iter().map(|id| id.to_string()).collect::<Vec<String>>().join("");

        let key = share.read().await.reports.initial_key(
            &report_type,
            organization_provider_id.as_str(),
            &from_to,
            reqeust_generate.get_all_s_m_p(),
            build_id_for_name
        );

        let (is_exist_file, file_path) = share.read().await.is_exist_file_report(&key, user_id.clone().unwrap_or("".to_string()).as_ref());

        // Если файл уже существует то мы возвращаем к нему путь
        if is_exist_file {
            return Json(Response::new(
                None,
                Some(file_path),
                Some("path")
            ));
        } else {
            let is_exist_report = share.read().await.is_exist_report(&key).await;

            if share.read().await.get_number_simultaneous_generations() >= dotenv!("MAX_NUM_OF_SIMULTANEOUS_GENERATIONS_CSV_IN_XLSX").parse::<u16>().unwrap_or(1000) {
                let (code, message): ResponseError = (4325437, "Лимит одновременных генераций был превышен".to_string());
                error!("code: {} message: {}", code, message);
                return Json(Response::new::<String>(
                    Some((code, message)),
                    None,
                    None
                ))
            }

            share.write().await.add_generation();

            // Проверяем есть ли report в share
            if is_exist_report {
                let report_opt = share.read().await.reports.get_report(key.as_str()).await;

                let report_rw_lock = match report_opt {
                    Some(report) => Ok(report),
                    None => {
                        share.write().await.take_away_generation();
                        Err((8564791, "Не удалось получить отчет из share".to_string()))
                    }
                };

                if let Err(error) = report_rw_lock {
                    error!("{}. {}", error.0, error.1);
                    return Json(Response::new::<String>(
                        Some(error),
                        None,
                        None
                    ));
                }

                let report = report_rw_lock.unwrap();
                report.write().await.set_provider_id(organization_provider_id);

                match generate_report_from_csv(
                    &mut reqeust_generate,
                    Arc::clone(&report),
                    &settings,
                    token.clone(),
                    all_filters_id,
                    &user_info,
                    full_date_from_to,
                    Arc::clone(&GENERATED_HASHES),
                    key.clone(),
                ).await {
                    Ok(path) => {
                        share.write().await.take_away_generation();
                        check_generated_hashes(Data::clone(&share)).await;

                        if let Err(_) = &user_id {
                            info!("Генерация отчета окончена для пользователя: -1");
                        } else {
                            let id = user_id.unwrap();
                            info!("Генерация отчета окончена для пользователя: {}", id);
                        }

                        Json(Response::new(
                            None,
                            Some(path),
                            Some("path")
                        ))
                    }
                    Err(error) => {
                        share.write().await.take_away_generation();
                        Json(Response::new::<String>(
                            Some(error),
                            None,
                            None
                        ))
                    }
                }
            } else {
                let report = Arc::new(TokioRwLock::new(Report::new(report_type, organization_provider_id.clone())));
                let mut Provider_name = String::from("");

                let report_type = reqeust_generate.report_type.unwrap_or(ReportType::Unknown);

                let link_filters_request = &mut reqeust_generate.filters;

                let chunks_create_res = create_chunks_by_types(
                    link_filters_request,
                    organization_provider_id.clone(),
                    report_type
                );

                let chunks = match chunks_create_res {
                    Ok(result) => result,
                    Err(errors) => {
                        for error in errors.iter() {
                            let file_id = reqeust_generate.filters.iter().map(|filter| filter.id).collect::<Vec<u32>>();
                            error!("user_id: {}\nfile_id: {:?}\nerror: {:?}", user_id.clone().unwrap_or("-1".to_string()), file_id, error);
                        }

                        share.write().await.take_away_generation();
                        return Json(Response::new::<String>(
                            Some(get_first_error_message_and_code(&errors)),
                            None,
                            None
                        ));
                    }
                };

                for (_, chunks, filter, index_collection) in chunks {
                    // Проверяем есть ли пустые чанки
                    let error_checking_result = match chunk_is_empty(&chunks, filter.id) {
                        Err(error) => {
                            Err(Json(Response::new(
                                None,
                                Some(error),
                                None
                            )))
                        }
                        Ok(_) => Ok(())
                    };

                    if let Err(error) = error_checking_result {
                        share.write().await.take_away_generation();
                        return error;
                    }

                    let working_with_report = Share::processing_chunks(
                        Arc::clone(&report),
                        chunks,
                        &mut Provider_name,
                        filter,
                        &index_collection,
                        &report_type
                    ).await;

                    if let Err(error) = working_with_report {
                        share.write().await.take_away_generation();

                        return Json(Response::new::<String>(
                            Some(error),
                            None,
                            None
                        ));
                    }
                }

                match generate_report_from_csv(
                    &mut reqeust_generate,
                    Arc::clone(&report),
                    &settings,
                    token,
                    all_filters_id,
                    &user_info,
                    full_date_from_to,
                    Arc::clone(&GENERATED_HASHES),
                    key.clone(),
                ).await {
                    Ok(path) => {
                        share.write().await.take_away_generation();

                        check_generated_hashes(Data::clone(&share)).await;

                        if let Err(_) = &user_id {
                            info!("Генерация отчета окончена для пользователя: -1");
                        } else {
                            let id = user_id.unwrap();
                            info!("Генерация отчета окончена для пользователя: {}", id);
                        }
                        let share_writer = share.write().await;

                        share_writer.reports.insert_new_report(key.clone(), ArcMutexWrapper::new_arc_mutex_wrapper(report)).await;
                        // share_writer.take_away_generation();
                        Json(Response::new(
                            None,
                            Some(path),
                            Some("path")
                        ))
                    },
                    Err(error) => {
                        share.write().await.take_away_generation();

                        for key in share.read().await.reports.get_keys().await.iter() {
                            GENERATED_HASHES.write().await.retain(|item| item != key)
                        }

                        let id = match user_id.clone() {
                            Ok(id) => id,
                            Err(_) => "-1".to_string()
                        };

                        info!("Не удалось сгенерировать файл для пользователя: {}", id);
                        error!("Message: {}, code: {}", error.1, error.0);

                        // share.write().await.take_away_generation();
                        Json(Response::new::<String>(
                            Some(error),
                            None,
                            None
                        ))
                    }
                }
            }
        }
    } else {
        let error = (4324323, "По переданным id не возможно сгенерировать файл".to_string());
        error!("code: {} message: {}", error.0, error.1);
        return Json(Response::new::<String>(
            Some(error),
            None,
            None
        ))
    }
}