use crate::args::Settings;

use crate::r#type::types::{InformationAboutFileMicroApiDBResult, ResponseError};

use actix_web::web::Data;

use dotenv_codegen::dotenv;

use mysql_async::{Row, Value};
use mysql_async;

use tracing::error;

use mysql_async::Value as MysqlValue;

pub fn handle_info_about_file(
    row_from_db: Result<Vec<Row>, ResponseError>,
    errors: &mut Vec<(i32, String)>,
    settings: &Data<Settings>,
) -> InformationAboutFileMicroApiDBResult {
    let mut files_info: InformationAboutFileMicroApiDBResult = Vec::new();
    for result_db in row_from_db.iter() {
        for row in result_db.iter() {
            let segmet: Option<isize> = row.get(2);
            match segmet {
                Some(segmet_file) => {
                    match segmet_file {
                        -1 => {
                            error!("Сегмент файлаа -1, файла поврежден");
                            files_info
                                .push(Err((534653, "Сегмент файла -1, файла поврежден".to_string())));
                        }
                        0 => {
                            files_info.push(Err((
                                534654,
                                "Сегмент файла 0, файла подготавливается к генерации!".to_string(),
                            )));
                        }
                        1 => {
                            files_info.push(Err((
                                534655,
                                "Сегмент файла 1, файла по которому был сделан запрос на данный момент в процссе генерации!".to_string())
                            ));
                        }
                        2 => {
                            // Получаем тип файлаа
                            let file_type: isize = row.get(1).unwrap();
                            // Получаем путь до папки arhive
                            let mut report_dir = if settings.get_prod() {
                                dotenv!("PROD_PATH_REPORT").to_string()
                            } else {
                                dotenv!("PATH_REPORT").to_string()
                            };
                            // Получаем имя файлаа

                            let file_name_bytes: Option<MysqlValue> = match row.get(0) {
                                Some(MysqlValue::Bytes(bytes)) => {
                                    Some(Value::Bytes(bytes))
                                },
                                Some(MysqlValue::NULL) => {
                                    files_info.push(Err((534656, "Не удалось получить file name\nВозможно файла по которому вы генерируете отчет не является .csv".to_string())));
                                    None
                                }
                                _ => {
                                    files_info.push(Err((534657, "Не удалочь получить путь к файла".to_string())));
                                    None
                                }
                            };
                            if let None = file_name_bytes { break; }
                            let bytes_to_string = String::from_utf8(Vec::try_from(file_name_bytes.unwrap()).unwrap());
                            let file_name = match bytes_to_string {
                                Ok(result) => Some(result),
                                Err(error) => {
                                    files_info.push(Err((534656, format!("Не удалось байты конвертировать в строку: {}", error.to_string()))));
                                    break;
                                }
                            };

                            // Получаем file id
                            let file_id: usize = row.get(3).unwrap();

                            // Отчет за период `от`
                            let from: Result<String, (i32, String)> = match row.get(4) {
                                // year, month, day, hour, minutes, seconds, micro seconds
                                Some(date) => {
                                    match date {
                                        MysqlValue::Date(year, month, day, hour, minutes, seconds, _) => {
                                            Ok(format!(
                                                "{}-{}-{} {}-{}-{}",
                                                year, format_date_with_one_character(month),
                                                format_date_with_one_character(day),
                                                format_date_with_one_character(hour),
                                                format_date_with_one_character(minutes),
                                                format_date_with_one_character(seconds)
                                            ).to_string())
                                        },
                                        _ => Err((235434, "Не удалось получить дату `от`-`до`".to_string()))
                                    }
                                }
                                _ => {
                                    Err((235434, "Не удалось получить дату `от`-`до`".to_string()))
                                }
                            };

                            // Отчет за период `до`
                            let to: Result<String, (i32, String)> =  match row.get(5) {
                                Some(date) => {
                                    match date {
                                        // year, month, day, hour, minutes, seconds, micro seconds
                                        MysqlValue::Date(year, month, day, hour, minutes, seconds, _) => {
                                            Ok(format!(
                                                "{}-{}-{} {}-{}-{}",
                                                year, format_date_with_one_character(month),
                                                format_date_with_one_character(day),
                                                format_date_with_one_character(hour),
                                                format_date_with_one_character(minutes),
                                                format_date_with_one_character(seconds)
                                            ).to_string())
                                        },
                                        _ => Err((235434, "Не удалось получить дату `от`-`до`".to_string()))
                                    }
                                }
                                _ => Err((235434, "Не удалось получить дату `от`-`до`".to_string()))
                            };

                            let user_id: isize = row.get(6).unwrap_or(-1);

                            if user_id == -1 {
                                files_info.push(Err((534656, "Не удалось получить корректный id".to_string())))
                            }

                            // Собираем полный путь до файла.
                            report_dir.push_str(file_name.unwrap().as_str());
                            // Отпровляем в files для дальнейшей обработки
                            files_info.push(Ok((
                                file_id, report_dir, file_type, 2, from.map_or_else(
                                |error| {
                                    let error_message = error.1.clone();
                                    errors.push(error);
                                    error_message
                                },
                                |from| from
                                ) , to.map_or_else(|error| {
                                    let error_message = error.1.clone();
                                    errors.push(error);
                                    error_message
                                }, |to| to), user_id)
                            ));
                        }
                        _ => {
                            error!("Поучен не известный segmet.");
                            files_info
                                .push(Err((534656, "Поучен не известный segmet.".to_string())));
                        }
                    }
                }
                None => {
                    files_info.push(Err((
                        432424,
                        "Не удалось проучить Сегмент файла".to_string(),
                    )));
                }
            }
        }

        files_info.iter().for_each(|file| {
            if let Err(error) = file {
                errors.push(error.clone());
            }
        });
    }

    files_info
}

pub fn handle_last_id(id_result: Result<Vec<Row>, ResponseError>) -> Result<isize, ResponseError> {
    if let Err(error) = id_result {
        return Err(error);
    }

    let id_row = id_result.unwrap();

    let mut result: Option<Result<isize, ResponseError>> = None;

    for row in id_row {
        let id: Result<isize, ResponseError> = match row.get::<isize, _>(0) {
            Some(g_id) => Ok(g_id),
            None => Err((2332132, "Не удалось получить id для сравнения".to_string()))
        };

        result = Some(id);
    }

    return if result.is_some() {
        result.unwrap()
    } else {
        Err((324324, "Не удалось корректно обработать id".to_string()))
    }
}

fn format_date_with_one_character(character: u8) -> String {
    let mut new_character = String::from("");
    let binding = character.to_string();

    return if binding.len() == 1 {
        new_character.push_str("0");
        new_character.push_str(binding.as_str());

        new_character
    } else {
        character.to_string()
    }
}
