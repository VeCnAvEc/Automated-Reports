pub mod agent_report {
    use std::sync::Arc;
    use tokio::sync::RwLock as TokioRwLock;
    use rust_xlsxwriter::{Format, Workbook};
    use crate::api_server::response_handlers::resp_user::handlers_user::AccountReplenishment;
    use crate::helper::generate_xlsx::{create_general_payment_report, create_list_summary_by_day, create_list_summary_by_provider, create_refill};
    use crate::helper::report_type::constants::{SUMMARY_BY_DAY_NAME, SUMMARY_BY_PROVIDER_NAME, WORKSHEET_GENERATE_GENERAL_PAYMENT_REPORT_NAME, WORKSHEET_SUMMARY_BY_REFILL_NAME};
    use crate::helper::working_with_xlsx_list::sheet_creator::sheet_creator::create_worksheet;
    use crate::r#trait::filter_report::ReportItemType;
    use crate::r#type::types::ResponseError;
    use crate::share::{Report, Share};

    pub async fn agent_report(
        workbook: &mut Workbook, report: Arc<TokioRwLock<Report>>,
        creators_first_name: String, creators_last_name: String,
        full_date_from_to: Vec<(usize, String, String)>, refill: &Vec<AccountReplenishment>
    ) -> Result<(), ResponseError> {
        /////////////////////////////////////////////  start  /////////////////////////////////////////////
        //                                   Общий отчет о платежах                                  //

        let worksheet_generate_general_payment_report = create_worksheet(
            workbook,
            WORKSHEET_GENERATE_GENERAL_PAYMENT_REPORT_NAME,
        );

        if let Err(error) = worksheet_generate_general_payment_report {
            return Err(error);
        }

        let item_report_by_needed_type = report.write().await.get_remittance_and_payments();

        let report_reader = report.read().await;

        let provider_name = report_reader.get_provider_name();
        let report_type = report_reader.get_report_type().clone();
        let formatted_date = report_reader.get_formatted_date();

        drop(report_reader);

        let mut report_writer = report.write().await;

        let get_date_mask = report_writer.get_remittance_and_payments_date();

        if let None = item_report_by_needed_type.1 {
            return Err((4324242, "Не достаточно нуных данных для генерации отчета по Agent".to_string()));
        }

        // Сделать запись в этом листе
        if let Err(error) = create_general_payment_report(
            worksheet_generate_general_payment_report.unwrap(),
            item_report_by_needed_type,
            &provider_name,
            &report_type,
            None,
            &creators_first_name,
            &creators_last_name,
            full_date_from_to,
            formatted_date
        ) {
            return Err(error);
        };
        /////////////////////////////////////////////  end    /////////////////////////////////////////////



        /////////////////////////////////////////////  start  /////////////////////////////////////////////
        //                              Создаем лист (summary_by_Provider)
        // Создаем лист (summary_by_Provider)
        let header_format = Format::new().set_bold();

        let worksheet_summary_by_provider = create_worksheet(
            workbook,
            SUMMARY_BY_PROVIDER_NAME,
        );

        if let Err(error) = worksheet_summary_by_provider {
            return Err(error);
        }

        let item_report_by_needed_type = report_writer.get_report_item(&ReportItemType::Payments);

        let item_report = item_report_by_needed_type.unwrap();
        // Сделать записит в этом листе
        create_list_summary_by_provider(
            worksheet_summary_by_provider.unwrap(),
            item_report,
            &header_format,
            provider_name.clone(),
        );

        /////////////////////////////////////////////  end    /////////////////////////////////////////////



        /////////////////////////////////////////////  start  /////////////////////////////////////////////
        //                          Сводная по дням
        let summary_by_day_worksheet = create_worksheet(
            workbook,
            SUMMARY_BY_DAY_NAME,
        );

        if let Err(error) = summary_by_day_worksheet {
            return Err(error);
        }

        create_list_summary_by_day(
            summary_by_day_worksheet.unwrap(),
            item_report,
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

        if get_date_mask.is_some() {
            // Получаем дату от-до если данные обработанны успешно
            let (from, to) = get_date_mask.unwrap();

            // Получаем date_mask.
            let date_mask = Share::create_new_from_to(from.to_string(), to.to_string());
            // Сделать записит в этом листе
            create_refill(
                worksheet_summary_by_refill.unwrap(),
                date_mask.clone(),
                item_report,
                &refill
            ).await;
        }

        /////////////////////////////////////////////  end    /////////////////////////////////////////////

        Ok(())
    }
}