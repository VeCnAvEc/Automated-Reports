pub mod taxi_company {
    use std::sync::Arc;
    use rust_xlsxwriter::{Format, Workbook};
    use tokio::sync::RwLock as TokioRwLock;
    use tracing::error;
    use crate::api_server::response_handlers::resp_user::handlers_user::AccountReplenishment;
    use crate::helper::generate_xlsx::{create_general_payment_report, create_list_summary_by_day, create_list_summary_by_provider, create_refill};
    use crate::helper::report_type::constants::{SUMMARY_BY_DAY_NAME, SUMMARY_BY_PROVIDER_NAME, WORKSHEET_GENERATE_GENERAL_PAYMENT_REPORT_NAME, WORKSHEET_SUMMARY_BY_REFILL_NAME};
    use crate::helper::working_with_xlsx_list::sheet_creator::sheet_creator::create_worksheet;
    use crate::r#trait::filter_report::ReportItemType;
    use crate::r#type::types::ResponseError;
    use crate::share::{Report, Share};


    pub async fn taxi_company_report(
        workbook: &mut Workbook, report: Arc<TokioRwLock<Report>>,
        creators_first_name: &String, creators_last_name: &String,
        fee: Option<f64>, refill: &Vec<AccountReplenishment>,
        full_date_from_to: Vec<(usize, String, String)>,
    ) -> Result<(), ResponseError> {
        let header_format = Format::new().set_bold();
        let report_reader = report.read().await;

        /////////////////////////////////////////////  start  /////////////////////////////////////////////
        let report_type = report_reader.get_report_type().clone();
        let provider_name = report_reader.get_provider_name().clone();
        // Отформатированное время начала генерации отчета
        let formatted_date = report_reader.get_formatted_date();

        drop(report_reader);

        let mut report_writer = report.write().await;

        let item_report_by_needed_type = report_writer.get_report_item(&ReportItemType::Remittance).cloned();

        if let None = item_report_by_needed_type {
            error!("code: 4324234\nmessage: Не достаточно нуных данных для генерации отчета по TaxiCompany");
            return Err((4324234, "Не достаточно нуных данных для генерации отчета по TaxiCompany".to_string()));
        }

        let mut item_report = item_report_by_needed_type.unwrap();

        let worksheet_generate_general_payment_report = create_worksheet(
            workbook,
            WORKSHEET_GENERATE_GENERAL_PAYMENT_REPORT_NAME,
        );

        if let Err(error) = worksheet_generate_general_payment_report {
            error!("Не удалось создать лист: {:?}", error);
            return Err(error);
        }

        let item_report_by_needed_type = report_writer.get_remittance_and_payments();

        // Сделать запись в этом листе
        if let Err(error) = create_general_payment_report(
            worksheet_generate_general_payment_report.unwrap(),
            item_report_by_needed_type,
            &provider_name,
            &report_type,
            fee,
            creators_first_name,
            creators_last_name,
            full_date_from_to,
            formatted_date
        ) {
            return Err(error)
        };

        /////////////////////////////////////////////  end    /////////////////////////////////////////////


        /////////////////////////////////////////////  start  /////////////////////////////////////////////

        // Создаем лист (summary_by_provider)
        let worksheet_summary_by_provider = create_worksheet(
            workbook,
            SUMMARY_BY_PROVIDER_NAME,
        );

        if let Err(error) = worksheet_summary_by_provider {
            return Err(error);
        }

        // Сделать записит в этом листе
        create_list_summary_by_provider(
            worksheet_summary_by_provider.unwrap(),
            &mut item_report,
            &header_format,
            provider_name.clone(),
        );

        /////////////////////////////////////////////  end    /////////////////////////////////////////////


        /////////////////////////////////////////////  start  /////////////////////////////////////////////
        let summary_by_day_worksheet = create_worksheet(
            workbook,
            SUMMARY_BY_DAY_NAME,
        );

        if let Err(error) = summary_by_day_worksheet {
            return Err(error);
        }

        create_list_summary_by_day(
            summary_by_day_worksheet.unwrap(),
            &mut item_report,
            &header_format,
            Some(provider_name.clone()),
        );

        /////////////////////////////////////////////  end    /////////////////////////////////////////////

        /////////////////////////////////////////////  start  /////////////////////////////////////////////
        // Создаем лист (summary_by_Provider)
        let worksheet_summary_by_refill = create_worksheet(
            workbook,
            WORKSHEET_SUMMARY_BY_REFILL_NAME,
        );

        if let Err(error) = worksheet_summary_by_refill {
            return Err(error);
        }

        let item_report_by_needed_type = report_writer.get_report_item(&ReportItemType::Remittance);

        if let None = item_report_by_needed_type {
            return Err((4324240, "Не достаточно нуных данных для генерации отчета по TaxiCompony".to_string()));
        }

        let get_date_mask = report_writer.get_remittance_and_payments_date();

        // Сделать записит в этом листе
        if get_date_mask.is_some() {
            // Получаем дату от-до если данные обработанны успешно
            let (from, to) = get_date_mask.unwrap();

            // Получаем date_mask.
            let date_mask = Share::create_new_from_to(from.to_string(), to.to_string());

            create_refill(
                worksheet_summary_by_refill.unwrap(),
                date_mask.clone(),
                &mut item_report,
                &refill
            ).await;
        }
        /////////////////////////////////////////////  end    /////////////////////////////////////////////

        Ok(())
    }
}