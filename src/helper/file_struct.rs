use crate::args::Settings;
use actix_web::web::Data;
use dotenv_codegen::dotenv;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct FilePath {
    pub path: String,
}

impl FilePath {
    pub fn get_path(path_to_file: &String, settings: Data<Settings>, user_id: String) -> String {
        let reports_dir = if settings.get_prod() {
            dotenv!("PROD_REPORTS_DIR")
        } else {
            dotenv!("REPORTS_DIR")
        };
        format!("{}/reports/{}/{}", reports_dir, user_id, path_to_file)
    }
}
