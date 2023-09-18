use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use crate::helper::generate_xlsx::MOUNTS_NUMBER;
use crate::share::ReportItem;

/// EN generation of the month by day
/// RU генерация месяца по дням
pub fn days_in_month(year: i32, month: u32, day: u32) -> Option<Vec<NaiveDate>> {
    if month > 12 {
        return None;
    }

    let mut days_in_month: Vec<NaiveDate> = Vec::new();

    let first_day = NaiveDate::from_ymd_opt(year, month, day);
    for day in first_day.unwrap().iter_days() {
        if let Ok(mnt) = day.to_string().split("-").collect::<Vec<&str>>()[1].parse::<u32>() {
            if month == 12 && mnt == 1 {
                break;
            }
            if mnt == month + 1 {
                break;
            }
        }

        days_in_month.push(day);
    }

    Some(days_in_month)
}

/// EN Get date from - to
/// RU Получить дату от - до
pub fn get_date_from_to(chunks: &Vec<Vec<Vec<String>>>, date_index: usize) -> (String, String) {
    // EN getting first date from report
    // RU Получаем первую дату из переданного отчета
    let first_date = chunks[0][0][date_index].clone();
    // EN getting last date from report
    // RU Получаем последнию дату из переданного отчета
    let last_date =
        chunks[chunks.len() - 1][chunks[chunks.len() - 1].len() - 1][date_index].clone();
    // От - До (Время для datemask)
    let from = first_date.split(" ").collect::<Vec<&str>>()[0].to_string();
    let to = last_date.split(" ").collect::<Vec<&str>>()[0].to_string();

    (from, to)
}

/// EN Format Excel date in regular date
/// RU Отформотировать Excel дату в обычную
pub fn excel_date_to_naive_datetime(excel_date: f64) -> String {
    let epoch_start = NaiveDate::from_ymd_opt(1900, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let days = excel_date.floor() as i32;
    let seconds = ((excel_date - days as f64) * 86400.0) as u32;
    let dt = epoch_start
        + chrono::Duration::days(days as i64)
        + chrono::Duration::seconds(seconds as i64);

    let utc_datetime: DateTime<Utc> = Utc.from_utc_datetime(&dt);
    let formatted_utc_datetime = utc_datetime.format("%Y-%m-%d %H:%M:%S").to_string();
    formatted_utc_datetime
}

pub fn build_date_ymd(formatted: &Vec<&str>) -> String {
    let mut date = String::new();

    for mount in MOUNTS_NUMBER {
        if mount.0 == formatted[1].parse::<i32>().unwrap() {
            date.push_str(formatted[0]);
            date.push_str(" ");
            date.push_str(mount.1);
            date.push_str(" ");
            date.push_str("20");
            date.push_str(formatted[2]);
            date.push_str("г");
        }
    }
    date
}

pub fn get_date_for_general_taxi_compony_list(date: Vec<(usize, String, String)>, item_report: &ReportItem) -> (String, String) {
    let mut full_date: Option<(String, String)> = None;

    date.iter().for_each(|file_info| {
        if file_info.0 == item_report.filter.id as usize {
            full_date = Some((file_info.1.clone(), file_info.2.clone()));
        }
    });

    let full_date = match full_date {
        Some(date) => date,
        None => ("".to_string(), "".to_string())
    };

    full_date
}