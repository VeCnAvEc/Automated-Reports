use actix_web::web::Data;
use tokio::sync::RwLock;
use crate::server::tokens_storage::TokensStorage;
use crate::share::Share;

/// Массив со строками из csv файла
pub type RecordStrings = Vec<String>;
/// Информация о файле по которому будет генерироваться отчет.
/// С базы данных.
/// 1. id Файла
/// 2. Путь до файла
/// 3. Сегмент
/// 4. Статус Файла
/// 5. Отчет за период "От"
/// 6. Отчет за период "До"
/// 7. User_id
pub type InformationAboutFileMicroApiDBResult = Vec<Result<(usize, String, isize, isize, String, String, isize), ResponseError>>;
/// Информация о файле по которому будет генерироваться отчет.
/// С базы данных.
/// 1. id Файла
/// 2. Путь до файла
/// 3. Тип файла
/// 4. Статус Файла
/// 5. Отчет за период "От"
/// 6. Отчет за период "До"
/// 7. User_id
pub type InformationAboutFileMicroApiDB = Vec<(usize, String, isize, isize, String, String, isize)>;
/// Тип ошибки для ответа пользователю
pub type ResponseError = (i32, String);
/// Дата from, to отчетов по которым геерируется отчет
pub type ReportsDateRange = Vec<(String, String)>;
/// Сборник чанков
pub type ChunksInReport = Vec<Vec<Vec<String>>>;
/// Хранилище отчетов для формулирования отчетов Share
pub type ReportsStorage = Data<RwLock<Share>>;
/// Token Storage
pub type TokensStorageT = Data<RwLock<TokensStorage>>;
