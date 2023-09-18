use std::collections::HashMap;
use std::fs::File;
use std::sync::Arc;

use tokio::sync::RwLock as TokioRwLock;

use actix_web::web::Data;
use chrono::{DateTime, Local};

use rust_xlsxwriter::{
    ColNum, Format,
    FormatAlign, FormatBorder, RowNum,
    Workbook, Worksheet, XlsxColor
};

use crate::r#trait::filter_report::{
    Filter, ReportType,
    Status, ReportItemType
};
use crate::share::{Report, ReportItem};

use tokio::task;
use tokio::task::JoinHandle;

use csv::{Reader as ReaderCsv, ReaderBuilder};
use rust_xlsxwriter::FormatAlign::{Center, Right};

use tracing::error;

use crate::api_server::response_handlers::resp_user::handlers_user::AccountReplenishment;

use crate::args::Settings;

use crate::error::errors_utils::err_utils::{get_last_error_message_and_code, is_check_on_errors_message_and_code};

use crate::handlers::generate_report::GenerateFile;

use crate::helper::xlsx_help_fun::{write_number_of_amount_per_day, write_number_of_commission_per_day, write_number_of_transactions_per_day, write_vendor_info, write_vendor_name};
use crate::helper::create_file::create_fs::{
    create_dir, create_file,
};
use crate::helper::date::{build_date_ymd, get_date_for_general_taxi_compony_list};
use crate::helper::get_refill;
use crate::helper::report_type::agent::agent_report::agent_report;
use crate::helper::report_type::merchant::merchant::merchant_report;
use crate::helper::report_type::taxi_company::taxi_company::taxi_company_report;
use crate::helper::user_info::user::UserInfo;

use crate::indexing_report_struct::IndexingReport;

use crate::r#type::types::{ResponseError};

// Месяц и его номер
pub const MOUNTS_NUMBER: [(i32, &str); 12] = [
    (1, "Январь"),
    (2, "Февраль"),
    (3, "Март"),
    (4, "Апрель"),
    (5, "Май"),
    (6, "Июнь"),
    (7, "Июль"),
    (8, "Август"),
    (9, "Сентябрь"),
    (10, "Октябрь"),
    (11, "Ноябрь"),
    (12, "Декабрь"),
];

/// [create list in report] Создает лист "Пополнение счета" и заполняет его данными
pub async fn create_refill(
    worksheet_refill: &mut Worksheet,
    date_mask: String,
    item_report: &mut ReportItem,
    refill: &Vec<AccountReplenishment>
) {
    let from_to = date_mask.split("#").collect::<Vec<&str>>();
    let mut y_m = from_to[0].split("-").collect::<Vec<&str>>();
    y_m.remove(y_m.len() - 1);

    let mut refill_amount: f64 = 0.0;

    let filter_refill = refill
        .into_iter()
        .filter(|demo| {
            let demo_date = demo
                .date
                .as_ref()
                .unwrap()
                .split("-")
                .collect::<Vec<&str>>();

            demo_date[0] == y_m[0] && demo_date[1] == y_m[1]
        })
        .collect::<Vec<&AccountReplenishment>>();

    let format = Format::new()
        .set_bold()
        .set_border_top(FormatBorder::Medium)
        .set_border_bottom(FormatBorder::Medium)
        .set_border_left(FormatBorder::Medium)
        .set_border_right(FormatBorder::Medium)
        .set_border_color(XlsxColor::Automatic)
        .set_font_color(XlsxColor::Black)
        .set_align(Center)
        .set_align(FormatAlign::VerticalCenter);

    let mut row: RowNum = 2;
    let mut col: ColNum = 2;

    let header: [&str; 5] = ["ID", "Пользователь", "Сумма", "Комментарий", "Дата"];

    for i in 0..filter_refill.len() {
        worksheet_refill.set_column_width(i as ColNum, 25).unwrap();
    }

    header.into_iter().for_each(|title| {
        worksheet_refill
            .write_string_with_format(row, col, title, &format)
            .unwrap();
        col += 1;
    });

    filter_refill.iter().for_each(|element| {
        if let Some(amount) = element.amount.as_ref() {
            if let Ok(am) = amount.parse::<f64>() {
                refill_amount += am;
            }
        }

        row += 1;
        col = 2;
        // Вставляем id
        worksheet_refill
            .write_string_with_format(
                row,
                col,
                element.id.as_ref().unwrap_or(&"None".to_string()),
                &format,
            )
            .unwrap();
        worksheet_refill.set_column_width(col, 10).unwrap();
        col += 1;

        let personal_information = format!(
            "{}({} {})",
            element.username.as_ref().unwrap(),
            element.first_name.as_ref().unwrap(),
            element.last_name.as_ref().unwrap()
        );
        worksheet_refill
            .write_string_with_format(row, col, &personal_information.clone(), &format)
            .unwrap();
        worksheet_refill.set_column_width(col, 30).unwrap();
        col += 1;

        worksheet_refill
            .write_number_with_format(
                row,
                col,
                element
                    .amount
                    .as_ref()
                    .unwrap_or(&"0.0".to_string())
                    .parse::<f64>()
                    .unwrap_or(0.0),
                &format,
            )
            .unwrap();
        worksheet_refill.set_column_width(col, 16).unwrap();
        col += 1;

        worksheet_refill
            .write_string_with_format(
                row,
                col,
                element.comment.as_ref().unwrap_or(&"".to_string()),
                &format,
            )
            .unwrap();
        worksheet_refill.set_column_width(col, 30).unwrap();
        col += 1;

        worksheet_refill
            .write_string_with_format(
                row,
                col,
                element.date.as_ref().unwrap_or(&"None".to_string()),
                &format,
            )
            .unwrap();
        worksheet_refill.set_column_width(col, 25).unwrap();
        col = 2;
    });

    row += 1;
    col += 2;
    worksheet_refill
        .write_number_with_format(row, col, refill_amount, &format)
        .unwrap();

    item_report.set_refill_amount(refill_amount);
}

// @33430
pub async fn generate_report_from_csv(
    data_by_generation: &mut GenerateFile,
    report: Arc<TokioRwLock<Report>>,
    settings: &Data<Settings>,
    token: String,
    filters_id: Vec<u128>,
    user_info: &UserInfo,
    full_date_from_to: Vec<(usize, String, String)>,
    hashes: Arc<TokioRwLock<Vec<String>>>,
    key: String,
) -> Result<String, ResponseError> {
    let mut workbook = Workbook::new();

    // id-шники файлов по которым будет идти генерация
    let mut filters_id = filters_id;
    // Сортируем от меньшего к большему
    filters_id.sort();
    // Фильтры полученные с запроса
    let filters = &mut data_by_generation.filters;
    filters.sort_by(|a, b| a.id.cmp(&b.id));

    let first_name = &UserInfo::get_pub_fields(&user_info.first_name);
    let last_name = &UserInfo::get_pub_fields(&user_info.last_name);

    match data_by_generation.report_type.unwrap_or(ReportType::Unknown) {
        ReportType::Agent => {
            let provider_id = report.read().await.get_organization_id();

            let refill_result = get_refill(provider_id, token.clone()).await;
            if let Err(error) = refill_result {
                return Err(error);
            }

            if let Err(error) = agent_report(
                &mut workbook, Arc::clone(&report),
                first_name.clone(), last_name.clone(),
                full_date_from_to.clone(), &refill_result.unwrap()
            ).await {
                return Err(error);
            };
        },
        ReportType::TaxiCompany => {
            let provider_id = report.read().await.get_organization_id();

            let refill_result = get_refill(provider_id, token.clone()).await;
            if let Err(error) = refill_result {
                return Err(error);
            }

            if let Err(error) = taxi_company_report(
                &mut workbook, Arc::clone(&report),
                first_name, last_name,
                data_by_generation.monthly_subscription_fee, &refill_result.unwrap(),
                full_date_from_to
            ).await {
                return Err(error);
            }
        },
        ReportType::Merchant => {
            if let Err(error) = merchant_report(
                &mut workbook, Arc::clone(&report),
                first_name, last_name,
                full_date_from_to
            ).await {
                return Err(error);
            }
        },
        ReportType::Unknown => return Err((1357836, "Передан не известный тип отчета: Unknown".to_string()))
    }

    let user_id_for_path_res = UserInfo::check_on_error(UserInfo::get_pub_fields(&user_info.id));

    if let Err(error) = user_id_for_path_res {
        return Err(error);
    }

    let user_id_for_path = user_id_for_path_res.unwrap();

    let mut errors = Vec::new();

    for hash in hashes.read().await.iter() {
        if hash == &key {
            error!("Невозможно начать генерацию переданного отчета: {hash}\nтак как данный отчет уже находится в режиме генерации.");
            return Err((4324324, "Невозможно сгенерировать отчет, так как данный отчет уже находится в режиме генерации".to_string()))
        }
    }

    let user_id = UserInfo::get_pub_fields(&user_info.id);
    let path_to_dir = match create_dir(Data::clone(&settings), &user_id) {
        Ok(path) => Ok(path),
        Err(_error) => Err((4324323, "Не удалось получить путь до папки с отчетами".to_string()))
    };
    if let Err(error) = path_to_dir {
        return Err(error);
    }

    for filter in filters.iter_mut() {
        if let Err(error) = filter.set_type_report_that_generated() {
            errors.push(error);
            break;
        }
    }

    if is_check_on_errors_message_and_code(&errors) {
        return Err(get_last_error_message_and_code(&errors));
    }

    let mut report_mutex = report.write().await;

    report_mutex.set_report_read_true();

    let xlsx = save_xlsx(
        key.as_str(),
        &mut workbook,
        user_id_for_path.clone(),
        settings,
    ).map_or_else(|error| Err(error), |path| Ok(path));

    return xlsx;
}

pub async fn create_task(
    report: Arc<TokioRwLock<Report>>,
    chunk_num: usize,
    chunk: Vec<Vec<String>>,
    number_of_chunks: usize,
    collect_indexing: &IndexingReport,
    type_item_report: &ReportItemType,
    report_type: &ReportType,
) -> JoinHandle<()> {
    let collect_indexing = collect_indexing.clone();
    let type_report_item = type_item_report.clone();
    let report_type = report_type.clone();

    let mut percent = 0.0;

    task::spawn( {
        async move {
            let mut report_guard = report.write().await;

            if percent < 100.0 {
                let result = report_guard.push_in_share_records_by_chunks(
                    chunk,
                    chunk_num,
                    number_of_chunks,
                    collect_indexing.clone(),
                    type_report_item,
                    report_type,
                );

                if let Err(error) = result {
                    error!("{:?}", error);
                    return;
                }
                percent = result.unwrap()
            }
        }
    })
}

// Создаем шапку для листа "Сводная по дням"
fn header_for_summary(
    worksheet_summary_by_day: &mut Worksheet,
    report: &mut ReportItem,
    header_format: &Format,
    Provider: Option<String>,
) {
    let mut row: RowNum = 0;
    let mut col: ColNum = 0;

    worksheet_summary_by_day
        .write_string_with_format(row, col, "mode", &header_format)
        .unwrap();
    col += 1;
    worksheet_summary_by_day
        .write_string_with_format(
            row,
            col,
            &report
                .filter
                .mode
                .as_ref()
                .unwrap_or(&"None".to_string())
                .as_str(),
            &header_format,
        )
        .unwrap();
    row += 1;
    col -= 1;

    worksheet_summary_by_day
        .write_string_with_format(row, col, "Provider", &header_format)
        .unwrap();
    col += 1;
    worksheet_summary_by_day
        .write_string_with_format(row, col, Provider.unwrap_or("".to_string()).as_ref(), &header_format)
        .unwrap();
    row += 1;
    col -= 1;

    worksheet_summary_by_day
        .write_string_with_format(row, col, "status", &header_format)
        .unwrap();
    col += 1;

    let status = match &report.filter.status {
        None => "None",
        Some(stat) => match stat {
            Status::Completed => "Завершена",
            Status::Mistake => "Ошибка",
            Status::Created => "Создана",
            Status::Cancel => "Отмена",
            Status::Null => "Null",
            Status::Unknown => "Unknown",
        },
    };

    worksheet_summary_by_day
        .write_string_with_format(row, col, status, &header_format)
        .unwrap();
}

pub fn create_list_summary_by_day(
    worksheet_summary_by_day: &mut Worksheet,
    report: &mut ReportItem,
    header_format: &Format,
    Provider: Option<String>
) {
    let mut row: RowNum = 0;
    let mut col: ColNum = 0;
    for i in 0..6 {
        worksheet_summary_by_day
            .set_column_width(i as ColNum, 30)
            .unwrap();
    }

    header_for_summary(worksheet_summary_by_day, report, header_format, Provider);

    row += 2;
    col += 1;

    let status = match report.filter.status.clone() {
        None => "None",
        Some(res) => match res {
            Status::Completed => "Завершена",
            Status::Mistake => "Ошибка",
            Status::Created => "Создана",
            Status::Cancel => "Отмена",
            Status::Null => "Null",
            Status::Unknown => "Unknown",
        },
    };

    worksheet_summary_by_day
        .write_string_with_format(row, col, status, &header_format)
        .unwrap();

    row += 2;
    col -= 1;

    worksheet_summary_by_day
        .write_string_with_format(row, col, "Названия строк", &header_format)
        .unwrap();

    col += 1;

    worksheet_summary_by_day
        .write_string_with_format(
            row,
            col,
            "Число элементов в столбце Сумма",
            &header_format,
        )
        .unwrap();

    col += 1;

    worksheet_summary_by_day
        .write_string_with_format(row, col, "Сумма по столбцу Сумма2", &header_format)
        .unwrap();

    col += 1;

    worksheet_summary_by_day
        .write_string_with_format(row, col, "Сумма по столбцу Комиссия", &header_format)
        .unwrap();

    col -= 3;
    row = 5;

    let mut days_in_report = report.days_in_report.iter().collect::<Vec<&String>>();

    days_in_report.sort();

    days_in_report.iter().for_each(|day| {
        worksheet_summary_by_day
            .write_string(row, col, day.as_str())
            .unwrap();
        row += 1;
    });

    row = 5;
    col += 1;

    report.days_len_transaction.sort();

    report
        .days_len_transaction
        .iter()
        .for_each(|(_, count_transactions)| {
            worksheet_summary_by_day
                .write_number(row, col, *count_transactions as f64)
                .unwrap();
            row += 1
        });

    row = 5;
    // col += 1;

    for amount in &report.days_amount {
        worksheet_summary_by_day
            .write_number(row, 2, format!("{:.2}", amount.1).parse::<f64>().unwrap_or(0.0))
            .unwrap();
        row += 1;
    }

    row = 5;

    let commission_by_day = report.commission_by_day.clone();

    for commission in &report.commission_by_day {
        worksheet_summary_by_day
            .write_number(row, 3, format!("{:.2}", commission.1).parse::<f64>().unwrap_or(0.0))
            .unwrap();
        row += 1;
    }

    worksheet_summary_by_day
        .write_string_with_format(
            report.days_in_report.len() as RowNum + 6,
            0,
            "Итог",
            header_format,
        )
        .unwrap();

    worksheet_summary_by_day
        .write_number_with_format(
            report.days_in_report.len() as RowNum + 6,
            1,
            report.len_transactions as f64,
            header_format,
        )
        .unwrap();
    worksheet_summary_by_day
        .write_number_with_format(
            report.days_in_report.len() as RowNum + 6,
            2,
            report.amount as f64,
            header_format,
        )
        .unwrap();
    worksheet_summary_by_day
        .write_number_with_format(
            report.days_in_report.len() as RowNum + 6,
            3,
            report.all_types_of_commissions.commission as f64,
            header_format,
        )
        .unwrap();
}

pub fn create_list_summary_by_provider(
    worksheet_summary_by_day: &mut Worksheet,
    report: &mut ReportItem,
    header_format: &Format,
    provider: String,
) {
    let mut row: RowNum = 0;
    let mut col: ColNum = 0;
    for i in 0..6 {
        worksheet_summary_by_day
            .set_column_width(i as ColNum, 30)
            .unwrap();
    }

    header_for_summary(
        worksheet_summary_by_day,
        report,
        header_format,
        Some(provider),
    );

    row += 4;
    col += 0;

    worksheet_summary_by_day
        .write_string_with_format(row, col, "Названия строк", header_format)
        .unwrap();
    col += 1;

    worksheet_summary_by_day
        .write_string_with_format(row, col, "Число элементов в столбце Сумма", header_format)
        .unwrap();
    col += 1;

    worksheet_summary_by_day
        .write_string_with_format(row, col, "Сумма по столбцу Сумма2", header_format)
        .unwrap();
    col += 1;

    worksheet_summary_by_day
        .write_string_with_format(row, col, "Сумма по столбцу Комиссия", header_format)
        .unwrap();
    col = 0;
    row += 2;

    write_vendor_name(
        report.summary_by_Provider.as_ref(),
        worksheet_summary_by_day,
        row,
        col,
    );

    col += 1;
    row = 1;
    row += 5;

    let all_transaction = write_number_of_transactions_per_day(
        report.summary_by_Provider.as_ref(),
        worksheet_summary_by_day,
        row,
        col,
    );
    col += 1;
    row = 1;
    row += 5;

    let all_amount = format!("{:.2}", write_number_of_amount_per_day(
        report.summary_by_Provider.as_ref(),
        worksheet_summary_by_day,
        row,
        col,
    ));
    col += 1;
    row = 1;
    row += 5;

    write_number_of_commission_per_day(
        report.summary_by_Provider.as_ref(),
        worksheet_summary_by_day,
        row,
        col,
    );

    row = report.summary_by_Provider.len() as RowNum + 6;

    row += 1;
    col = 0;
    worksheet_summary_by_day
        .write_string_with_format(row, col, "Общий итог", header_format)
        .unwrap();

    col += 1;
    worksheet_summary_by_day
        .write_number_with_format(row, col, all_transaction, header_format)
        .unwrap();

    col += 1;
    worksheet_summary_by_day
        .write_number_with_format(row, col, all_amount.parse::<f64>().unwrap(), header_format)
        .unwrap();

    col += 1;
    worksheet_summary_by_day
        .write_number_with_format(
            row, col,
            format!("{:.2}", report.commission)
                .parse::<f64>().unwrap_or(0.0), header_format
        )
        .unwrap();
}

// @423432
pub fn create_general_payment_report(
    worksheet_general_payment_report: &mut Worksheet,
    items_report: (Option<ReportItem>, Option<ReportItem>),
    Provider: &String,
    report_type: &ReportType,
    fee: Option<f64>,
    creators_first_name: &String,
    creators_last_name: &String,
    date: Vec<(usize, String, String)>,
    time_of_report_generation: DateTime<Local>
) -> Result<(), ResponseError> {
    let mut row: RowNum = 3;
    let mut col: ColNum = 2;

    let item_report_pay = items_report.1;
    let item_report_c2card = items_report.0;

    worksheet_general_payment_report
        .set_column_width(col, 40)
        .unwrap();
    worksheet_general_payment_report
        .set_row_height(row, 22)
        .unwrap();

    match report_type {
        ReportType::Agent => {
           if let Some(ref item_report_pay) = item_report_pay {
                // Дата по которой генерируется отчет
                let min_mount_pay: u8 = item_report_pay.get_min_mount_from_filed_day_in_report();
                // Отформатированный вид даты когда собрался отчет
                let formatted_date_in_d_m_y = time_of_report_generation.format("%d.%m.%y").to_string();
                let new_formatted = formatted_date_in_d_m_y.split(".").collect::<Vec<&str>>();

                // Header color
                let header_color = Format::new().set_background_color(XlsxColor::RGB(0x5789bb));
                // Color bg
                let background_color = Format::new().set_background_color(XlsxColor::White);

                // Перекрашиваем background нашего листа
                for i in 0..15 {
                    worksheet_general_payment_report
                        .set_column_format(i, &background_color)
                        .unwrap();
                }

               create_title_for_general_report_sheet_agent(worksheet_general_payment_report);

                for z in 2..8 {
                    for i in 6..10 {
                        // Собираем header для (Общий)Отчет о Платежах
                        create_header_for_general_report_sheet_agent(
                            worksheet_general_payment_report,
                            Provider,
                            i,
                            z,
                            &new_formatted,
                            min_mount_pay,
                            &header_color,
                            creators_first_name,
                            creators_last_name
                        );

                        let row_and_col = create_body_for_general_report_sheet_agent(
                            worksheet_general_payment_report,
                            &item_report_pay,
                            row,
                            col,
                        );

                        row = row_and_col.0;
                        col = row_and_col.1;
                    }
                }
            } else {
                error!("{:?}", (2423432, "Отчет по которому идет генерация пуст или не содержит подходящих данных".to_string()));
                return Err((2423432, "Отчет по которому идет генерация пуст не содержит подходящих данных".to_string()));
            }

            let format_description = Format::new()
                    .set_font_color(XlsxColor::White)
                    .set_bold()
                    .set_background_color(XlsxColor::RGB(0x5b9bd5))
                    .set_align(FormatAlign::Left);

            row += 3;
            col -= 6;
            if let Some(ref item_report_c2card) = item_report_c2card {
                for _ in row..row + 1 {
                    for _ in col..col + 8 {
                        create_descriptions_header(
                            row, col,
                            worksheet_general_payment_report.set_row_height(row, 29).unwrap(),
                            &format_description, report_type
                        );
                        col += 1;
                    }
                    col -= 7;
                    row += 1;
                }

                let edit_col_and_row = create_body_for_general_report_sheet(
                    worksheet_general_payment_report,
                    item_report_c2card,
                    row,
                    col,
                    ReportItemType::Remittance,
                    report_type,
                    0.0,
                );

                col = edit_col_and_row.0 - 1;
                row = edit_col_and_row.1;
            }

            create_footer_for_general_sheet_agent(worksheet_general_payment_report, Provider, row, col);
            Ok(())
        },
        ReportType::TaxiCompany => {
            let mut tc_col: ColNum = 2;
            let mut tc_row: RowNum = 2;

            if let Some(ref item_report) = item_report_pay {
                let formatted_date_in_d_m_y = time_of_report_generation.format("%d.%m.%y").to_string();

                let first_row = tc_row;

                let full_date = get_date_for_general_taxi_compony_list(date.clone(), &item_report);

                for _ in tc_row..tc_row + 5 {
                    for _ in tc_col..tc_col + 6 {
                        create_header_for_general_report_sheet_taxi_compony_and_merchant(
                            first_row,
                            tc_row,
                            tc_col,
                            worksheet_general_payment_report.set_row_height(tc_row, 29).unwrap(),
                            &formatted_date_in_d_m_y,
                            &item_report.filter,
                            Some("Пополнение Яндекс баланса".to_string()),
                                creators_first_name,
                            creators_last_name,
                            full_date.clone()
                        );
                        tc_col += 1;
                    }
                    tc_col -= 6;
                    tc_row += 1;
                }

                let format_description = Format::new()
                    .set_font_color(XlsxColor::White)
                    .set_bold()
                    .set_background_color(XlsxColor::RGB(0x5b9bd5))
                    .set_align(FormatAlign::Left);

                // Создаем descriptions под каждое отдельное поле
                for _ in tc_row..tc_row + 1 {
                    for _ in tc_col..tc_col + 6 {
                        create_descriptions_header(
                            tc_row, tc_col,
                            worksheet_general_payment_report.set_row_height(tc_row, 29).unwrap(),
                            &format_description, report_type
                        );
                        tc_col += 1;
                    }
                    tc_col -= 6;
                    tc_row += 1;
                }

                let edit_col_and_row = create_body_for_general_report_sheet(
                    worksheet_general_payment_report,
                    &item_report_pay.clone().unwrap(),
                    tc_row,
                    tc_col,
                    ReportItemType::Payments,
                    report_type,
                    0.0,
                );

                tc_col = edit_col_and_row.0;
                tc_row = edit_col_and_row.1;
            }

            if let Some(ref item_report) = item_report_c2card {
                let formatted_date_in_d_m_y = time_of_report_generation.format("%d.%m.%y").to_string();

                let fee = fee.unwrap_or(1_000_000.0);

                let first_row = tc_row;

                let format_description = Format::new()
                    .set_font_color(XlsxColor::White)
                    .set_bold()
                    .set_background_color(XlsxColor::RGB(0x5b9bd5))
                    .set_align(FormatAlign::Left);

                let full_date = get_date_for_general_taxi_compony_list(date.clone(), &item_report);

                // Создаем шапку для отчета
                for _ in tc_row..tc_row + 5 {
                    for _ in tc_col..tc_col + 6 {
                        create_header_for_general_report_sheet_taxi_compony_and_merchant(
                            first_row,
                            tc_row,
                            tc_col,
                            worksheet_general_payment_report,
                            &formatted_date_in_d_m_y,
                            &item_report.filter,
                            Some("Пополнение карты".to_string()),
                            creators_first_name,
                            creators_last_name,
                            full_date.clone()
                        );
                        tc_col += 1;
                    }
                    tc_col -= 6;
                    tc_row += 1;
                }

                // Создаем descriptions под каждое отдельное поле
                for _ in tc_row..tc_row + 1 {
                    for _ in tc_col..tc_col + 6 {
                        create_descriptions_header(
                            tc_row, tc_col,
                            worksheet_general_payment_report.set_row_height(tc_row, 29).unwrap(),
                            &format_description, &report_type
                        );
                        tc_col += 1;
                    }
                    tc_col -= 6;
                    tc_row += 1;
                }

                col -= 6;

                let edit_col_and_row = create_body_for_general_report_sheet(
                    worksheet_general_payment_report,
                    &item_report_c2card.unwrap(),
                    tc_row,
                    tc_col,
                    ReportItemType::Remittance,
                    report_type,
                    fee,
                );

                tc_col = edit_col_and_row.0;
                tc_row = edit_col_and_row.1;
            }
            Ok(())
        },
        ReportType::Merchant => {
            let mut tc_col: ColNum = 2;
            let mut tc_row: RowNum = 2;

            let format_bold = Format::new()
                .set_bold()
                .set_border_bottom(FormatBorder::Double)
                .set_border_color(XlsxColor::RGB(0x5789bb));

            let mut total_result_of_payments_system = None;

            if let Some(ref item_report) = item_report_pay {
                let formatted_date_in_d_m_y = time_of_report_generation.format("%d.%m.%y").to_string();

                let first_row = tc_row;

                let full_date = get_date_for_general_taxi_compony_list(date.clone(), &item_report);

                for _ in tc_row..tc_row + 5 {
                    for _ in tc_col..tc_col + 6 {
                        create_header_for_general_report_sheet_taxi_compony_and_merchant(
                            first_row,
                            tc_row,
                            tc_col,
                            worksheet_general_payment_report.set_row_height(tc_row, 29).unwrap(),
                            &formatted_date_in_d_m_y,
                            &item_report.filter,
                            None,
                                creators_first_name,
                            creators_last_name,
                            full_date.clone()
                        );
                        tc_col += 1;
                    }
                    tc_col -= 6;
                    tc_row += 1;
                }

                let format_description = Format::new()
                    .set_font_color(XlsxColor::White)
                    .set_bold()
                    .set_background_color(XlsxColor::RGB(0x5b9bd5))
                    .set_align(FormatAlign::Left);

                // Создаем descriptions под каждое отдельное поле
                for _ in tc_row..tc_row + 1 {
                    for _ in tc_col..tc_col + 6 {
                        create_descriptions_header(
                            tc_row, tc_col,
                            worksheet_general_payment_report.set_row_height(tc_row, 29).unwrap(),
                            &format_description, &report_type
                        );
                        tc_col += 1;
                    }
                    tc_col -= 6;
                    tc_row += 1;
                }

                let result_creation_of_body = create_body_for_general_report_sheet_merchant(
                    worksheet_general_payment_report, &item_report_pay.clone().unwrap(),
                    tc_row, tc_col,
                    &format_bold
                );

                tc_row = result_creation_of_body.1.0;
                tc_col = result_creation_of_body.1.1;
                total_result_of_payments_system = Some(result_creation_of_body.0);
            }

            let result_of_payments_system = match total_result_of_payments_system {
                Some(result) => Ok(result),
                None => Err((3412431, "Не удалось создать итог отчета".to_string()))
            };

            if let Err(error) = result_of_payments_system {
                return Err(error);
            }

            let payments_system_info = result_of_payments_system.unwrap();

            worksheet_general_payment_report.write_string_with_format(tc_row, tc_col, "Итого", &format_bold).unwrap();
            tc_row += 1;

            for payment_system_info in payments_system_info {
                tc_col += 1;
                worksheet_general_payment_report.write_string_with_format(tc_row, tc_col, payment_system_info.0.as_str(), &format_bold).unwrap();
                tc_col += 1;
                worksheet_general_payment_report.write_number_with_format(tc_row, tc_col, payment_system_info.1.0 as f64, &format_bold).unwrap();
                tc_col += 1;
                worksheet_general_payment_report.write_number_with_format(tc_row, tc_col, payment_system_info.1.1 as f64, &format_bold).unwrap();
                tc_col += 1;
                worksheet_general_payment_report.write_string_with_format(tc_row, tc_col, "", &format_bold).unwrap();
                tc_col += 1;
                worksheet_general_payment_report.write_number_with_format(tc_row, tc_col, payment_system_info.1.2 as f64, &format_bold).unwrap();
                tc_col -= 5;
                tc_row += 1;
            }

            Ok(())
        },
        ReportType::Unknown => return Err((2423434, "Пока не реализовано".to_string())),
    }
}

// Создать для листа Общий отчет тело под agent
fn create_body_for_general_report_sheet_agent(
    worksheet_general_payment_report: &mut Worksheet,
    report: &ReportItem,
    mut row: RowNum,
    mut col: ColNum,
) -> (RowNum, ColNum) {
    // Стили, для описания колонок
    let title_name = Format::new()
        .set_bold()
        .set_align(Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_background_color(XlsxColor::RGB(0xdce6f2));

    row += 1;

    // Называние сталбцев для (Общий)Отчет о Платежах
    worksheet_general_payment_report
        .write_string_with_format(11, 2, "Поставщик", &title_name)
        .unwrap();
    worksheet_general_payment_report
        .set_column_width(2, 500)
        .unwrap();
    worksheet_general_payment_report
        .set_row_height(11, 40)
        .unwrap();
    worksheet_general_payment_report
        .write_string_with_format(11, 3, "кол-во", &title_name)
        .unwrap();
    worksheet_general_payment_report
        .set_column_width(2, 27)
        .unwrap();
    worksheet_general_payment_report
        .set_row_height(11, 40)
        .unwrap();
    worksheet_general_payment_report
        .write_string_with_format(11, 4, "Сумма", &title_name)
        .unwrap();
    worksheet_general_payment_report
        .set_column_width(4, 27)
        .unwrap();
    worksheet_general_payment_report
        .set_row_height(11, 40)
        .unwrap();
    worksheet_general_payment_report
        .write_string_with_format(11, 5, "Сумма комиссии\nс Поставщика", &title_name)
        .unwrap();
    worksheet_general_payment_report
        .set_column_width(5, 27)
        .unwrap();
    worksheet_general_payment_report
        .set_row_height(11, 40)
        .unwrap();
    worksheet_general_payment_report
        .write_string_with_format(11, 6, "Вознаграждение\nБанка", &title_name)
        .unwrap();
    worksheet_general_payment_report
        .set_column_width(6, 27)
        .unwrap();
    worksheet_general_payment_report
        .set_row_height(11, 40)
        .unwrap();
    worksheet_general_payment_report
        .write_string_with_format(11, 7, "Вознаграждение\nCOMANYNAME", &title_name)
        .unwrap();
    worksheet_general_payment_report
        .set_column_width(7, 27)
        .unwrap();
    worksheet_general_payment_report
        .set_row_height(11, 40)
        .unwrap();

    let footer_format = Format::new()
        .set_bold()
        .set_background_color(XlsxColor::RGB(0xdce6f2));

    row = 12;
    col = 2;

    let (mut row, mut col) = write_vendor_info(
        &report.summary_by_Provider, worksheet_general_payment_report,
        row, col
    );

    row = report.summary_by_Provider.len() as RowNum + 12;
    col = 2;

    worksheet_general_payment_report
        .write_string_with_format(row, col, "Общий итог", &footer_format)
        .unwrap();

    col += 1;

    worksheet_general_payment_report
        .write_number_with_format(row, col, report.len_transactions as f64, &footer_format)
        .unwrap();

    col += 1;

    worksheet_general_payment_report
        .write_number_with_format(row, col, report.amount, &footer_format)
        .unwrap();

    col += 1;

    worksheet_general_payment_report
        .write_number_with_format(
            row,
            col,
            report.all_types_of_commissions.commission,
            &footer_format,
        )
        .unwrap();

    col += 1;

    let format_commission_bank = format!("{:.2}", report.all_types_of_commissions.commission_bank).parse::<f64>().unwrap();

    worksheet_general_payment_report
        .write_number_with_format(row, col, format_commission_bank, &footer_format)
        .unwrap();

    col += 1;

    let format_commissions_pay_sys = format!("{:.2}", report.all_types_of_commissions.commission_pay_sys).parse::<f64>().unwrap();

    worksheet_general_payment_report
        .write_number_with_format(
            row,
            col,
            format_commissions_pay_sys,
            &footer_format,
        )
        .unwrap();

    row = report.summary_by_Provider.len() as RowNum + 13;
    col = 7;

    (row, col)
}

fn create_footer_for_general_sheet_agent(
    worksheet_general_payment_report: &mut Worksheet,
    Provider: &str,
    mut row: RowNum,
    mut col: ColNum
) {
    let sender_and_receiver = [
        "Платежная Организация",
        "Payment System Platorm LLC",
        "Платежный агент",
    ];
    let director_name = "Яхтанигов А.М __________________ ";

    let bold_format_sar = Format::new().set_bold();

    col += 1;
    worksheet_general_payment_report
        .write_string_with_format(row, col, sender_and_receiver[0], &bold_format_sar)
        .unwrap();
    row += 1;

    worksheet_general_payment_report
        .write_string_with_format(row, col, sender_and_receiver[1], &bold_format_sar)
        .unwrap();
    row += 2;

    worksheet_general_payment_report
        .write_string_with_format(row, col, director_name, &bold_format_sar)
        .unwrap();
    row -= 3;
    col += 5;

    worksheet_general_payment_report
        .write_string_with_format(row, col, sender_and_receiver[2], &bold_format_sar)
        .unwrap();
    row += 1;

    worksheet_general_payment_report
        .write_string_with_format(row, col, Provider, &bold_format_sar)
        .unwrap();

    row += 2;
    worksheet_general_payment_report
        .write_string(row, col, " __________________")
        .unwrap();
}

fn create_title_for_general_report_sheet_agent(
    worksheet_general: &mut Worksheet,
) {
    // Размер шрифта и толщина шрифта для заголовка
    let title_format_1 = Format::new().set_bold().set_font_size(18);
    // Размер шрифта и толщина шрифта для футера
    let title_format_2 = Format::new().set_bold().set_font_size(13);

    worksheet_general
        .write_string_with_format(4, 2, "Payment System Platorm LLC", &title_format_1)
        .unwrap().set_row_height(4, 25).unwrap();
    worksheet_general
        .write_string_with_format(
            5,
            2,
            "ОБЩЕСТВО С ОГРАНИЧЕННОЙ ОТВЕТСТВЕННОСТЬЮ",
            &title_format_2,
        )
        .unwrap();
}


// Создать для листа Общий отчет шапку под агентов
// Собираем header для (Общий)Отчет о Платежах
fn create_header_for_general_report_sheet_agent(
    worksheet_general: &mut Worksheet,
    Provider: &str,
    i: RowNum,
    z: ColNum,
    new_formatted: &Vec<&str>,
    min_mount: u8,
    header_color: &Format,
    creators_first_name: &String,
    creators_last_name: &String
) {
    // Разер шрифта, толщина шрифта, цвет фона для шапки
    let header_format = Format::new()
        .set_font_color(XlsxColor::White)
        .set_bold()
        .set_font_size(13)
        .set_background_color(XlsxColor::RGB(0x5789bb))
        .set_align(FormatAlign::VerticalCenter);

    worksheet_general
        .write_string_with_format(i, z, "", &header_color)
        .unwrap();
    if i == 7 && z == 2 {
        worksheet_general
            .write_string_with_format(i, z, "Агент", &header_format)
            .unwrap();
    }

    if i == 8 && z == 2 {
        worksheet_general
            .write_string_with_format(i, z, Provider, &header_format)
            .unwrap();
    }

    if i == 9 && z == 2 {
        worksheet_general
            .write_string_with_format(i, z, "Номер договора: ___________", &header_format)
            .unwrap();
    }

    if i == 6 && z == 4 {
        worksheet_general
            .write_string_with_format(
                i,
                z,
                "Отчет о ПЛАТЕЖАХ",
                &header_format.clone().set_align(FormatAlign::Center),
            )
            .unwrap();
    }

    if i == 7 && z == 4 {
        worksheet_general
            .write_string_with_format(
                i,
                z,
                &*format!(
                    "за {} 20{} года",
                    {
                        let mut mnt = "".to_string();
                        for mount in MOUNTS_NUMBER {
                            if mount.0 == min_mount as i32 {
                                mnt.push_str(mount.1);
                                break;
                            }
                        }
                        mnt
                    },
                    new_formatted[2]
                ),
                &header_format
                    .clone()
                    .set_align(Center)
                    .set_align(FormatAlign::VerticalCenter),
            )
            .unwrap();
        worksheet_general.set_column_width(4, 25).unwrap();
        worksheet_general.set_row_height(7, 22).unwrap();
    }

    if i == 9 && z == 4 {
        worksheet_general
            .write_string_with_format(
                i,
                z,
                "Ответственный:",
                &header_format
                    .clone()
                    .set_align(Center)
                    .set_align(FormatAlign::VerticalCenter),
            )
            .unwrap();
    }

    if i == 8 && z == 6 {
        worksheet_general
            .write_string_with_format(
                i,
                z,
                "Сформировано:",
                &header_format
                    .clone()
                    .set_align(Center)
                    .set_align(FormatAlign::VerticalCenter),
            )
            .unwrap();
    }

    // Собираем дату того когда был сформирован отчет
    // ==============================================================================================

    // Тут хранится результат даты
    let date = build_date_ymd(new_formatted);
    // ==============================================================================================

    // Число когда сформирован отчет.
    // Пример: 27 апрель 2023г
    if i == 8 && z == 7 {
        worksheet_general
            .write_string_with_format(
                i,
                z,
                date.as_str(),
                &header_format
                    .clone()
                    .set_align(Center)
                    .set_align(FormatAlign::VerticalCenter),
            )
            .unwrap();
    }

    if i == 9 && z == 6 {
        worksheet_general.merge_range(
            i,
            z,
            i,
            z + 1,
            format!("{} {}", creators_last_name, creators_first_name).as_str(),
            &header_format
                .clone()
                .set_font_size(14)
                .set_align(Center)
                .set_align(FormatAlign::VerticalCenter)
        ).unwrap();
        // worksheet_general
        //     .write_string_with_format(
        //         i,
        //         z,
        //         format!("{} {}", creators_last_name, creators_first_name).as_str(),
        //         &header_format
        //             .clone()
        //             .set_font_size(14)
        //             .set_align(Center)
        //             .set_align(FormatAlign::VerticalCenter),
        //     )
        //     .unwrap();
        worksheet_general.set_column_width(z, 30).unwrap();
        worksheet_general.set_row_height(i, 22).unwrap();
    }
}

// Нужно добавить From and To дату
fn create_header_for_general_report_sheet_taxi_compony_and_merchant(
    first_row: RowNum,
    row: RowNum,
    col: ColNum,
    worksheet: &mut Worksheet,
    date: &str,
    filter: &Filter,
    description: Option<String>,
    first_name: &String,
    last_name: &String,
    full_date: (String, String)
) {
    let description = description.unwrap_or_default();
    let default_style = Format::new()
        .set_font_size(12)
        .set_background_color(XlsxColor::RGB(0xdeebf7));
    worksheet.set_column_width(col, 26).unwrap();
    worksheet.set_row_height(row, 15).unwrap();

    if col == 2 && row == first_row {
        worksheet
            .write_string_with_format(
                row,
                col,
                description.as_str(),
                &default_style.clone().set_align(Right),
            )
            .unwrap();
        return;
    }

    if col == 4 && row == first_row {
        worksheet
            .write_string_with_format(
                row,
                col,
                "Фин отчет по услуге:",
                &default_style.clone().set_align(Right),
            )
            .unwrap();
        return;
    }

    if col == 5 && row == first_row {
        worksheet
            .write_string_with_format(
                row,
                col,
                "Комплекс Платежных услуг",
                &default_style.clone().set_align(FormatAlign::Center),
            )
            .unwrap();
        return;
    }

    if col == 4 && row == first_row + 1 {
        worksheet
            .write_string_with_format(
                row,
                col,
                "Дата составления:",
                &default_style.clone().set_align(FormatAlign::Right),
            )
            .unwrap();
        return;
    }

    if col == 5 && row == first_row + 1 {
        worksheet
            .write_string_with_format(
                row,
                col,
                date,
                &default_style.clone().set_align(FormatAlign::Center),
            )
            .unwrap();
        return;
    }

    if col == 2 && row == first_row + 2 {
        worksheet
            .write_string_with_format(row, col, "Статус", &default_style)
            .unwrap();
        return;
    }

    if col == 3 && row == first_row + 2 {
        worksheet
            .write_string_with_format(row, col, get_status(filter).unwrap_or("None"), &default_style)
            .unwrap();
        return;
    }

    if col == 6 && row == first_row + 2 {
        worksheet
            .write_string_with_format(
                row,
                col,
                "Период:",
                &default_style.clone().set_align(FormatAlign::Center),
            )
            .unwrap();
        return;
    }

    if col == 2 && row == first_row + 3 {
        worksheet
            .write_string_with_format(row, col, "Режим", &default_style)
            .unwrap();
        return;
    }

    if col == 3 && row == first_row + 3 {
        worksheet
            .write_string_with_format(
                row,
                col,
                filter.mode.clone().unwrap_or("None".to_string()).as_ref(),
                &default_style,
            )
            .unwrap();
        return;
    }

    if col == 6 && row == first_row + 3 {
        // например: с 00:00 1 Января 2023 по 23:59:59 31 Января 2023 года
        worksheet
            .write_string_with_format(row, col, format!("{} {}", full_date.0, full_date.1).as_str(), &default_style)
            .unwrap();
        return;
    }

    if col == 2 && row == first_row + 4 {
        worksheet
            .write_string_with_format(row, col, "Договор № ", &default_style)
            .unwrap();
        return;
    }

    if col == 3 && row == first_row + 4 {
        // Пока не имю нужных данных
        worksheet
            .write_string_with_format(row, col, "тут будет номер договора ", &default_style)
            .unwrap();
        return;
    }

    if col == 5 && row == first_row + 4 || col == 10 {
        worksheet
            .write_string_with_format(row, col, "Ответственный", &default_style)
            .unwrap();
        return;
    }

    if col == 6 && row == first_row + 4 {
        worksheet
            .write_string_with_format(
                row, col,
                format!("{} {}", first_name, last_name).as_str(),
                &default_style
            )
            .unwrap();
        return;
    }

    worksheet
        .write_string_with_format(
            row,
            col,
            "",
            &Format::new().set_background_color(XlsxColor::RGB(0xdeebf7)),
        )
        .unwrap();
}

fn create_descriptions_header(
    row: RowNum,
    col: ColNum,
    worksheet: &mut Worksheet,
    format: &Format,
    report_type: &ReportType
) {
    let descriptions = match report_type {
        ReportType::Agent => {
            ("Услуги COMANYNAME C2C", "VENDOR", "Количество", "Сумма без комиссий", "Комиссия", "Вознаграждение\nCOMANYNAME", "Вознаграждение\nАгента")
        }
        ReportType::TaxiCompany => {
            ("Услуга COMANYNAME PAM", "VENDOR", "Количество", "Сумма без комиссий", "Ставка\nкомиссий COMANYNAME", "Вознаграждение\nCOMANYNAME", "")
        }
        ReportType::Merchant => {
            ("Услуга COMANYNAME PAM", "Merchant", "кол-во", "Сумма без комиссий", "Ставка комиссий COMANYNAME", "Вознаграждение COMANYNAME", "")
        }
        ReportType::Unknown => {
            ("", "", "", "", "", "", "")
        }
    };

    if col == 2 {
        worksheet.set_column_width(col, 25).unwrap();
        worksheet.write_string_with_format(row, col, descriptions.0, format).unwrap();
    }
    if col == 3 {
        worksheet.set_column_width(col, 25).unwrap();
        worksheet.write_string_with_format(row, col, descriptions.1, format).unwrap();
    }
    if col == 4 {
        worksheet.set_column_width(col, 25).unwrap();
        worksheet.write_string_with_format(row, col, descriptions.2, format).unwrap();
    }
    if col == 5 {
        worksheet.set_column_width(col, 25).unwrap();
        worksheet.write_string_with_format(row, col, descriptions.3, format).unwrap();
    }
    if col == 6 {
        worksheet.set_column_width(col, 25).unwrap();
        worksheet.write_string_with_format(row, col, descriptions.4, format).unwrap();
    }
    if col == 7 {
        worksheet.set_column_width(col, 25).unwrap();
        worksheet.write_string_with_format(row, col, descriptions.5, format).unwrap();
    }
    if report_type == &ReportType::Agent && col == 8 {
        worksheet.set_column_width(col, 25).unwrap();
        worksheet.write_string_with_format(row, col, descriptions.6, format).unwrap();
    }
}

fn create_body_for_general_report_sheet(
    worksheet: &mut Worksheet,
    report: &ReportItem,
    mut row: RowNum,
    mut col: ColNum,
    item_type: ReportItemType,
    report_type: &ReportType,
    fee: f64
) -> (ColNum, RowNum) {
    let format_bold = Format::new()
        .set_bold()
        .set_border_bottom(FormatBorder::Double)
        .set_border_color(XlsxColor::RGB(0x5789bb));

    let process_content = match item_type {
        ReportItemType::Remittance => "Пополнение HUMO UZCARD",
        ReportItemType::Payments => "Пополнение Яндекс баланса",
        _ => "Ошибка"
    };

    worksheet
        .write_string_with_format(row, col, process_content, &format_bold)
        .unwrap();

    col += 1;
    let format_bold = Format::new()
        .set_align(Right)
        .set_bold()
        .set_border_bottom(FormatBorder::Double)
        .set_border_color(XlsxColor::RGB(0x5789bb));

    let format_small = Format::new()
        .set_align(Right)
        .set_border_bottom(FormatBorder::Double)
        .set_border_color(XlsxColor::RGB(0x5789bb));

    let mut total_transactions = 0.0;
    let mut total_amount = 0.0;
    let mut total_commission = 0.0;
    let mut total_COMANYNAME_award = fee;
    let mut total_remuneration_of_agents = 0.0;
    // Берем всех вендоров и записываем все данные по ним

    match report_type {
        ReportType::Agent => {
            for Provider_info in report.general_report_on_remittance_agent.iter() {
                // Merchantname
                worksheet
                    .write_string_with_format(row, col, Provider_info.0.as_ref(), &format_small)
                    .unwrap();
                col += 1;
                // amount transaction
                total_transactions += Provider_info.1 as f64;
                worksheet
                    .write_number_with_format(row, col, Provider_info.1 as f64, &format_small)
                    .unwrap();
                col += 1;
                // Сумма без комиссий
                total_amount += Provider_info.2;
                worksheet
                    .write_number_with_format(row, col, Provider_info.2, &format_small)
                    .unwrap();
                col += 1;
                // Коммиссия
                total_commission += Provider_info.3;
                worksheet
                    .write_number_with_format(row, col, Provider_info.3, &format_small)
                    .unwrap();
                col += 1;
                // Вознаграждение COMANYNAME
                total_COMANYNAME_award += Provider_info.4;
                worksheet
                    .write_number_with_format(row, col, Provider_info.4, &format_small)
                    .unwrap();
                col += 1;
                total_remuneration_of_agents += Provider_info.5;
                worksheet
                    .write_number_with_format(row, col, Provider_info.5, &format_small)
                    .unwrap();
                col -= 6;
                row += 1;
            }

            worksheet.write_string_with_format(row, col, "Общий итог", &format_bold)
                .unwrap();
            col += 1;
            worksheet.write_string_with_format(row, col, "", &format_bold)
                .unwrap();
            col += 1;
        }

        ReportType::TaxiCompany => {
            for vendor_info in report.general_report_on_payments_taxi_company.iter() {
                // Merchantname
                worksheet
                    .write_string_with_format(row, col, vendor_info.0.as_ref(), &format_small)
                    .unwrap();
                col += 1;
                // amount transaction
                total_transactions += vendor_info.1 as f64;
                worksheet
                    .write_number_with_format(row, col, vendor_info.1 as f64, &format_bold)
                    .unwrap();
                col += 1;
                // Сумма без комиссий
                total_amount += vendor_info.2;
                worksheet
                    .write_number_with_format(row, col, vendor_info.2, &format_bold)
                    .unwrap();
                col += 1;
                // Процентная ставка
                worksheet
                    .write_string_with_format(row, col, "", &format_small)
                    .unwrap();
                col += 1;
                // Вознаграждение COMANYNAME
                total_COMANYNAME_award += vendor_info.3;
                worksheet
                    .write_number_with_format(row, col, vendor_info.3, &format_bold)
                    .unwrap();
                col -= 4;
                row += 1;
            }

            if fee != 0.0 {
                match item_type {
                    ReportItemType::Remittance => {
                        col -= 1;
                        worksheet.merge_range(row, col, row, col + 1, "", &format_bold).unwrap();

                        worksheet.set_row_height(row, 40).unwrap().write_string_with_format(
                            row, col,
                            "Ежемесячная абонентская плата за обработку запросов\n телеграмм Бота \"YaPro2Card - COMANYNAME\"",
                            &format_bold.clone().set_align(Center)
                        ).unwrap();
                        col += 5;

                        worksheet.write_number_with_format(
                            row, col,
                            fee,
                            &format_bold
                        ).unwrap();
                        col -= 5;
                        row += 1;
                    },
                    // Тут не чего не должно происходить
                    _ => {}
                }
            } else {
                col -= 1;
            }

            worksheet.merge_range(row, col, row, col + 1, "", &format_bold).unwrap();
            worksheet.write_string_with_format(row, col, "ИТОГО Вознаграждение COMANYNAME", &format_bold)
                .unwrap()
                .set_column_width(col, 30)
                .unwrap();

            col += 2;
        }
        ReportType::Merchant => {}
        ReportType::Unknown => {}
    };

    worksheet
        .write_number_with_format(row, col, total_transactions, &format_bold)
        .unwrap();
    col += 1;
    worksheet
        .write_number_with_format(row, col, total_amount, &format_bold)
        .unwrap();
    if report_type == &ReportType::Agent {
        col += 1;
        worksheet
            .write_number_with_format(row, col, total_commission, &format_bold)
            .unwrap();
    }
    col += 1;
    if report_type == &ReportType::TaxiCompany {
        worksheet
            .write_string_with_format(row, col, "", &format_bold)
            .unwrap();
        col += 1;
    }
    worksheet
        .write_number_with_format(row, col, total_COMANYNAME_award, &format_bold)
        .unwrap();
    if report_type == &ReportType::Agent {
        col += 1;
        worksheet
            .write_number_with_format(row, col, total_remuneration_of_agents, &format_bold)
            .unwrap();
    }

    col = 2;
    row += 3;
    return (col, row);
}


/// Возвращает картеж с HashMap который в себе содержет имена платежных систем
/// и номер с колонкой на которой закончилась запись
pub fn create_body_for_general_report_sheet_merchant(
    worksheet: &mut Worksheet,
    report: &ReportItem,
    mut row: RowNum,
    mut col: ColNum,
    format_bold: &Format
) -> (HashMap<String, (u64, f64, f64)>, (RowNum, ColNum)) {
    let mut payment_system: HashMap<String, Vec<(String, u128, f64, f64)>> = HashMap::new();

    let _ = &report.general_report_on_payments_merchant.iter().for_each(|merchant_and_payments_system| {
        match payment_system.get_mut(&merchant_and_payments_system.1) {
            Some(p_system) => {
                p_system.push((
                    merchant_and_payments_system.0.clone(), merchant_and_payments_system.2,
                    merchant_and_payments_system.3, merchant_and_payments_system.4
                ));
            },
            None => {
                payment_system.insert(merchant_and_payments_system.1.clone(), vec![
                    (
                        merchant_and_payments_system.0.clone(), merchant_and_payments_system.2,
                        merchant_and_payments_system.3, merchant_and_payments_system.4
                    )
                ]);
            },
        }
    });

    let mut total_result_of_payments_system: HashMap<String, (u64, f64, f64)> = HashMap::new();

    for (platform_name, platform_info) in payment_system.iter() {
        let mut total_transactions: f64 = 0.0;
        let mut total_without_commissions: f64 = 0.0;
        let mut total_COMANYNAME_awards: f64 = 0.0;

        worksheet
            .write_string_with_format(row, col, platform_name.as_str(), format_bold)
            .unwrap();
        col += 1;
        for info in platform_info {
            worksheet
                .write_string_with_format(row, col, info.0.as_str(), format_bold).unwrap();
            col += 1;
            worksheet
                .write_number_with_format(row, col, info.1 as f64, format_bold).unwrap();
            total_transactions += info.1 as f64;
            col += 1;
            worksheet
                .write_number_with_format(row, col, info.2, format_bold).unwrap();
            total_without_commissions += info.2;
            col += 1;
            worksheet
                .write_string_with_format(row, col, "", format_bold).unwrap();
            col += 1;
            worksheet
                .write_number_with_format(row, col, info.3, format_bold).unwrap();
            total_COMANYNAME_awards += info.3;
            col -= 4;
            row += 1;
        }
        row += 1;
        col -= 1;

        match total_result_of_payments_system.get_mut(platform_name.as_str()) {
            Some(payment_system_info) => {
                payment_system_info.0 += total_transactions as u64;
                payment_system_info.1 += total_without_commissions;
                payment_system_info.2 += total_COMANYNAME_awards;
            }
            None => {
                total_result_of_payments_system.insert(
                    platform_name.to_string(),
                    (total_transactions as u64, total_without_commissions, total_COMANYNAME_awards)
                );
            }
        }
    }

    (total_result_of_payments_system, (row, col))
}

pub fn get_status(filter: &Filter) -> Option<&str> {
    return match filter.status.as_ref() {
        None => None,
        Some(stat) => match stat {
            Status::Completed => Some("Завершена"),
            Status::Mistake => Some("Ошибка"),
            Status::Created => Some("Создана"),
            Status::Cancel => Some("Отмена"),
            Status::Null => Some("Null"),
            Status::Unknown => Some("Unknown"),
        },
    };
}

pub fn create_workbook(path_to_file: &str) -> Result<ReaderCsv<File>, ResponseError> {
    match ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_path(path_to_file.to_string()) {
        Err(error) => {
            return Err((5435445, format!("Не удалось получить доступ к файлу: {}", error)))
        }
        Ok(result) => Ok(result)
    }
}

fn save_xlsx(
    key: &str,
    workbook: &mut Workbook,
    user_id: String,
    settings: &Data<Settings>
) -> Result<String, (i32, String)> {
    return match create_dir(Data::clone(&settings), &user_id) {
        Ok(path_to_dir) => {
            let create_file = create_file(path_to_dir, key);
            match workbook.save(create_file.clone()) {
                Ok(_) => Ok(create_file),
                Err(err) => {
                    error!("Xlsx error: {:?}", err);
                    Err((
                        3234253,
                        format!("Failed to create xlsx file. {}", err),
                    ))
                }
            }
        }
        Err(error) => {
            eprint!("Create error - failed to create file 1: {}", error);
            Err((
                2354536,
                "Nothing was found according to your request".to_string(),
            ))
        }
    };
}