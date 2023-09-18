pub mod creator_of_chunks {
    use std::fs::File;
    use actix_web::web::Json;
    use csv::Reader;
    use tracing::{error, warn};
    use crate::error::errors_utils::err_utils::get_first_error_message_and_code;
    use crate::helper::from_string_record_to_vec;
    use crate::helper::generate_xlsx::create_workbook;
    use crate::indexing_report_struct::IndexingReport;
    use crate::r#trait::automated_report_response::Response;
    use crate::r#trait::filter_report::{Filter, ReportItemType, ReportType};
    use crate::r#type::types::{ChunksInReport, ResponseError};

    const CHUNK_SIZE: usize = 256;

    pub fn build_chunks_for_share(
        rdr: &mut Reader<File>,
        organization_provider_id: &str,
        filter: &Filter,
        collect_indexing: &IndexingReport,
        report_type: &ReportType
    ) -> Result<ChunksInReport, ResponseError> {
        let mut record_index: u128 = 0;
        let mut record_for_share: Vec<String> = Vec::new();

        let mut chunks: ChunksInReport = Vec::new();
        let mut chunk = Vec::with_capacity(CHUNK_SIZE);

        let header = rdr
            .headers()
            .unwrap()
            .iter()
            .collect::<Vec<&str>>();

        for _ in 0..header.len() {
            record_for_share.push("".to_string());
        }

        for result in rdr.records() {
            if let Ok(record) = result {
                record_index += 1;
                // Собираем карту индексов, и скипаем итерацию
                if record_index == 1 {
                    continue;
                }

                let last_index = record.len();
                // Проверяем подходит ли нам строка по фильтрам
                let filter_validation = filter.filter_validation(&record, &collect_indexing, organization_provider_id, report_type);

                if let Err(error) = filter_validation {
                    return Err(error)
                }

                if !filter_validation.unwrap() {
                    continue;
                }

                for (i, field) in record.iter().enumerate() {
                    // Если это последний элемент в страке
                    if i == last_index - 1 {
                        record_for_share[i] = field.into();

                        // ============================================================
                        // Собираем chunks
                        chunk.push(record_for_share.clone());

                        // Если chunk достиг нужного размера,
                        // То мы его пушим в "chunks"
                        // И очищаем "chunk" для того что-бы заполнить его заново
                        if chunk.len() == CHUNK_SIZE {
                            chunks.push(std::mem::take(&mut chunk));
                            chunk = Vec::with_capacity(CHUNK_SIZE);
                        }
                        // Очищаем каждое поле в "record_for_share"
                        for j in 0..record_for_share.len() {
                            record_for_share[j].clear();
                        }
                    } else {
                        record_for_share[i] = field.into();
                    }
                }
            }
        }

        is_empty_chunk(&mut chunks, &mut chunk);

        Ok(chunks)
    }

    pub fn create_chunks_by_types(
        filters: &mut Vec<Filter>,
        organization_provider_id: String,
        generation_type: ReportType
    ) -> Result<Vec<(ReportItemType, ChunksInReport, &Filter, IndexingReport)>, Vec<(i32, String)>> {
        let mut errors = Vec::new();
        let mut chunks_by_type_file: Vec<(ReportItemType, ChunksInReport, &Filter, IndexingReport)> = Vec::new();

        for filter in filters.iter_mut() {
            if let Err(error) = filter.set_type_report_that_generated() {
                errors.push(error);
                break;
            }

            let rdr_chunks_result = create_workbook(filter.get_path_to_file().unwrap_or("".to_string()).as_ref());
            if let Err(ref error) = rdr_chunks_result {
                errors.push(error.clone());
                return Err(errors);
            }

            let mut rdr_chunks = rdr_chunks_result.unwrap();

            let mut index_collection = IndexingReport::new();

            match filter.get_type_report_that_generated() {
                None => {}
                Some(rp) => {
                    match rp {
                        ReportItemType::Remittance => {
                            let type_of_report_we_depend_opt = filter.type_of_report_we_depend.clone();
                            let type_of_report_we_depend = type_of_report_we_depend_opt.unwrap_or("".to_string());

                            match type_of_report_we_depend.as_str() {
                                "c2card" | "c2cCOMANYNAME" => {
                                    index_collection.find_index_by_name(
                                        from_string_record_to_vec(rdr_chunks.headers().unwrap()),
                                        ReportItemType::Remittance
                                    );

                                    // Проверка отсутвующих полей
                                    if let Err(error) = index_collection.check_which_fields_not_found(ReportItemType::Remittance) {
                                        errors.push(error);
                                        warn!("Remittance не будут добавленны в отчет");
                                        continue;
                                    }

                                    let chunks = build_chunks_for_share(
                                        &mut rdr_chunks, organization_provider_id.as_str(),
                                        filter, &index_collection,
                                        &generation_type
                                    );

                                    if let Ok(chunk) = chunks {
                                        chunks_by_type_file.push((ReportItemType::Remittance, chunk, filter, index_collection));
                                    } else {
                                        if let Err(error) = chunks {
                                            errors.push(error);
                                        }
                                    }
                                }
                                _ => {
                                    errors.push((
                                        2321331,
                                        format!("Тип файла {} не может быть обработан", type_of_report_we_depend)
                                    ));
                                }
                            }
                        }
                        ReportItemType::Payments => {
                            let type_of_report_we_depend_opt = filter.type_of_report_we_depend.clone();
                            let type_of_report_we_depend = type_of_report_we_depend_opt.unwrap_or("".to_string());

                            match type_of_report_we_depend.as_str() {
                                "pay" | "pay_f" => {
                                    index_collection.find_index_by_name(
                                        from_string_record_to_vec(rdr_chunks.headers().unwrap()),
                                        ReportItemType::Payments
                                    );

                                    // Проверка отсутвующих полей
                                    if let Err(error) = index_collection.check_which_fields_not_found(ReportItemType::Payments) {
                                        errors.push(error);
                                        warn!("Payments не будут добавленны в отчет");
                                        continue;
                                    }

                                    let chunks = build_chunks_for_share(
                                        &mut rdr_chunks, organization_provider_id.as_str(),
                                        filter, &index_collection,
                                        &generation_type
                                    );

                                    if let Ok(chunk) = chunks {
                                        chunks_by_type_file.push((ReportItemType::Payments, chunk, filter, index_collection));
                                    } else {
                                        if let Err(error) = chunks {
                                            errors.push(error);
                                        }
                                    }
                                }
                                _ => {
                                    errors.push((
                                        2321331,
                                        format!("Тип файла {} не может быть обработан", type_of_report_we_depend)
                                    ));
                                }
                            }
                        }
                        _ => {
                            errors.push((
                                2321331,
                                "Переданы данные с неизвестным типом транзакций, передайте пожалуйста платежи или переводы".to_string()
                            ));
                        }
                    }
                }
            }
        }

        return if !errors.is_empty() { Err(errors) } else { Ok(chunks_by_type_file) }
    }

    pub fn chunk_processing<'a>(
        chunks_result: Result<Vec<(ReportItemType, ChunksInReport, &'a Filter, IndexingReport)>, Vec<(i32, String)>>,
        filters: &mut Vec<Filter>,
        user_id: &str
    ) -> Result<Vec<(ReportItemType, ChunksInReport, &'a Filter, IndexingReport)>, Json<Response>> {
        return match chunks_result {
            Ok(result) => Ok(result),
            Err(errors) => {
                for error in errors.iter() {
                    let file_id = filters.iter().map(|filter| filter.id).collect::<Vec<u32>>();
                    error!("user_id: {}\nfile_id: {:?}\nerror: {:?}", user_id, file_id, error);
                }

                Err(Json(Response::new(
                    None,
                    Some(get_first_error_message_and_code(&errors)),
                    None
                )))
            }
        };
    }

    pub fn is_empty_chunk(chunks: &mut ChunksInReport, chunk: &mut Vec<Vec<String>>) {
        if !chunk.is_empty() {
            chunks.push(std::mem::take(chunk));
        }
    }
}