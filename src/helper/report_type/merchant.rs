pub mod merchant {
    use std::sync::Arc;
    use rust_xlsxwriter::Workbook;
    use tokio::sync::RwLock as TokioRwLock;
    use crate::helper::generate_xlsx::create_general_payment_report;
    use crate::helper::report_type::constants::WORKSHEET_GENERATE_GENERAL_PAYMENT_REPORT_NAME;
    use crate::helper::working_with_xlsx_list::sheet_creator::sheet_creator::create_worksheet;
    use crate::r#type::types::ResponseError;
    use crate::share::Report;


    pub async fn merchant_report(
        workbook: &mut Workbook, report: Arc<TokioRwLock<Report>>,
        creators_first_name: &String, creators_last_name: &String,
        full_date_from_to: Vec<(usize, String, String)>
    ) -> Result<(), ResponseError>{

        let report_reader = report.read().await;

        let report_type = report_reader.get_report_type().clone();
        let provider_name = report_reader.get_provider_name();
        let formatted_date = report_reader.get_formatted_date();

        drop(report_reader);

        let worksheet_generate_general_payment_report = create_worksheet(
            workbook,
            WORKSHEET_GENERATE_GENERAL_PAYMENT_REPORT_NAME,
        );

        if let Err(error) = worksheet_generate_general_payment_report {
            return Err(error);
        }

        let item_report_by_needed_type = report.write().await.get_remittance_and_payments();

        if let None = item_report_by_needed_type.1 {
            return Err((4324243, "Не достаточно нуных данных для генерации отчета по Merchant".to_string()));
        }

        // Сделать запись в этом листе
        if let Err(error) = create_general_payment_report(
            worksheet_generate_general_payment_report.unwrap(),
            item_report_by_needed_type,
            &provider_name,
            &report_type,
            None,
            creators_first_name,
            creators_last_name,
            full_date_from_to,
            formatted_date
        ) {
            return Err(error);
        };

        Ok(())
    }
}