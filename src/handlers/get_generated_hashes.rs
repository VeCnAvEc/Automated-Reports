use actix_web::web::{Data, Json};

use crate::handlers::generate_report::GENERATED_HASHES;

pub async fn get_generated_hashes(generated_hashes: Data<GENERATED_HASHES>) -> Json<Vec<String>> {
    let generated_hashes = generated_hashes.read().await.clone();
    Json(generated_hashes)
}