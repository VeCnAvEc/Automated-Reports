use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tokio::time::interval;
use crate::r#type::types::{ReportsStorage, ResponseError};
use crate::share::{ArcMutexWrapper, Report};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShareHelper {
    pub reports: ReportsHelper,
    pub generated_now: u16,
    pub max_count_record_in_reports: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportsHelper {
    pub data: HashMap<String, ArcMutexWrapperHelper<Report>>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcMutexWrapperHelper<T> (T);

impl ShareHelper {
    pub async fn share_to_share_helper(share: ReportsStorage) -> Result<Self, ResponseError> {
        let mut interval = interval(Duration::from_millis(10));
        let mut error_result = None;

        let share_reader = share.read().await;
        let reports_reader = share_reader.reports.data.read().await.clone();

        let mut share_generated_now = 0;
        let mut share_max_count_record_in_reports = 0;

        let mut attempt_try_lock_generated_now = 0;
        // Пытаемся получить generated_now
        while let Err(error) = share_reader.generated_now.try_lock() {
            if attempt_try_lock_generated_now >= 50  {
                error_result = Some((2543271, format!("Не удалось получаить generated_now: {}", error)));
                break;
            }

            attempt_try_lock_generated_now += 1;
            interval.tick().await;
        }

        if let Some(error) = error_result {
            return Err(error);
        }

        share_generated_now = *share_reader.generated_now.lock().unwrap();

        let mut attempt_try_lock_max_count_record_in_reports = 0;
        // Пытаемся получить max_count_record_in_reports
        while let Err(error) = share_reader.max_count_record_in_reports.try_lock() {
            if attempt_try_lock_max_count_record_in_reports >= 50 {
                error_result = Some((2543271, format!("Не удалось получаить generated_now: {}", error)));
                break;
            }

            attempt_try_lock_max_count_record_in_reports += 1;
            interval.tick().await;
        }

        if let Some(error) = error_result {
            return Err(error);
        }

        share_max_count_record_in_reports = *share_reader.max_count_record_in_reports.lock().unwrap();

        let mut new_reports_data = HashMap::new();

        for (key, report_wrapper) in reports_reader.iter() {
            let arc_mutex_wrapper_helper = ArcMutexWrapperHelper::arc_mutex_wrapper_to_arc_mutex_wrapper_helper(
                report_wrapper.clone()
            ).await;
            new_reports_data.insert(key.clone(), arc_mutex_wrapper_helper);
        }

        let share_helper = ShareHelper {
            reports: ReportsHelper {
                data: new_reports_data,
            },
            generated_now: share_generated_now,
            max_count_record_in_reports: share_max_count_record_in_reports,
        };

        return Ok(share_helper);
    }
}

impl ArcMutexWrapperHelper<Report> {
    pub async fn arc_mutex_wrapper_to_arc_mutex_wrapper_helper(arc_mutex_wrapper: ArcMutexWrapper<Report>) -> Self {
        let report = arc_mutex_wrapper.0.read().await.clone();
        ArcMutexWrapperHelper(report)
    }
}