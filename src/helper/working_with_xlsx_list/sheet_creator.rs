
pub mod sheet_creator {
    use rust_xlsxwriter::XlsxError;
    use rust_xlsxwriter::{Workbook, Worksheet};
    use tracing::error;
    use crate::handlers::cryptography::cryptography::generate_rand_hash;
    use crate::r#type::types::ResponseError;

    pub fn create_worksheet<'a>(workbook: &'a mut Workbook, worksheet_name: &str) -> Result<&'a mut Worksheet, ResponseError> {
        let mut file_name = format!("{}", worksheet_name);
        workbook.worksheets().iter().for_each(|sheet_name| {
            if file_name == sheet_name.name() {
                let additional_prefix = &generate_rand_hash()[..3];
                file_name.push_str("_");
                file_name.push_str(additional_prefix)
            }
        });

        let worksheet: Result<&mut Worksheet, XlsxError> = match workbook
            .add_worksheet()
            .set_name(file_name.as_str()) {
                Ok(worksheet) => Ok(worksheet),
                Err(error) => {
                    match error {
                        XlsxError::SheetnameReused(err_text) => {
                            error!("[create_worksheet - SheetnameReused]: {}", err_text);
                            Err(XlsxError::SheetnameReused(err_text))
                        },
                        any_error => {
                            error!("[create_worksheet - any_error]: {}", format!("{any_error}"));
                            Err(any_error)
                        }
                    }
                }
            };

        worksheet.map_or_else(|_| Err((34524543, "Не удалось создать лист".to_string())), |sheet| Ok(sheet))
    }

    pub fn is_exist_sheet_name(workseets: &Vec<Worksheet>, sheet_name: &str, prefix: &str) -> bool {
        let mut is_exist = false;

        let sheet_name = format!("({}){}", prefix, sheet_name);
        workseets.iter().for_each(|sheet| {
           if sheet.name() == sheet_name {
                is_exist = true;
           }
        });
        is_exist
    }
}

