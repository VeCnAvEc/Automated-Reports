pub mod create_fs {
    use crate::args::Settings;
    use crate::r#trait::filter_report::{ReportType, Status};
    use crate::r#type::types::ReportsDateRange;
    use actix_web::web::Data;
    use dotenv_codegen::dotenv;
    use std::path::Path;
    use tracing::info;
    use crate::handlers::cryptography::cryptography::generate_hash_key_for_report;
    use crate::helper::build_payment_filter_name;

    pub fn create_dir(settings: Data<Settings>, user_id: &String) -> Result<String, std::io::Error> {
        let report_dir = if settings.get_prod() {
            dotenv!("PROD_REPORTS_DIR")
        } else {
            dotenv!("REPORTS_DIR")
        };
        let path_string = format!("{}/reports/{}", report_dir, user_id);
        let path = Path::new(&path_string);

        if path.is_dir() {
            Ok(format!("{}", path.display()))
        } else {
            let dir_to_reports = match std::fs::create_dir_all(format!("{}", path.display())) {
                Ok(_) => Ok(format!("{}", path.display())),
                Err(err) => {
                    info!("{}/reports", path.display());
                    Err(err)
                }
            };
            dir_to_reports
        }
    }

    pub fn create_file(path_to_dir: String, file_name: &str) -> String {
        format!("{}/{}.xlsx", path_to_dir, file_name)
    }

    pub fn create_file_name(
        report_type: &ReportType,
        organization_provider_id: &str,
        from_to: &ReportsDateRange,
        status: &Vec<Status>,
        modes: &Vec<String>,
        id: String,
        payments_system: &Vec<Vec<String>>
    ) -> String {
        let status_string_build = status.iter().map(|stat| stat.to_string()).collect::<Vec<String>>().join(" ");
        let mode_build = modes.join(" ");
        let payments_system_build = build_payment_filter_name(payments_system);

        let get_hash_name = generate_hash_key_for_report(
            report_type, organization_provider_id,
            from_to, id,
            status_string_build, mode_build,
            payments_system_build
        );

        let hash_to_string = get_hash_name.0.iter()
            .map(|byte| format!("{:02x}", byte)).collect::<Vec<String>>().concat();

        hash_to_string[..16].to_string().clone()
    }
}