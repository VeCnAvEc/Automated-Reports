use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};

use actix_web::web::Data;
use actix_web::{http::header, web, Error, HttpResponse, HttpRequest, ResponseError};
use actix_web::body::BoxBody;
use actix_web::http::StatusCode;

use dotenv_codegen::dotenv;
use pin_project::pin_project;

use crate::args::Settings;
use crate::download_report_chunks::DownloadReportChunks;
use crate::error::error_response::{map_io_error, CustomError};
use crate::r#trait::chunks_trait::IDownloadReportChunks;
use sodiumoxide::crypto::secretstream::{self, Pull, Push, Stream, Tag};
use tokio::io::ReadBuf;
use tokio::{fs::File, io::AsyncRead};
use tokio_stream::Stream as TKStream;
use tracing::info;
use crate::api_server::api_requests::RpcRequest;
use crate::api_server::response_handlers::resp_user::handlers_user::handler_user_info;

use crate::helper::file_struct::FilePath;
use crate::helper::get_token_from_header;
use crate::helper::user_info::user::UserInfo;

#[pin_project]
pub struct Streamer {
    crypto_stream: Stream<Pull>,
    #[pin]
    file: File,
    stream_push: Stream<Push>,
    chunk_size: usize,
    current_chunk: DownloadReportChunks,
}

const CHUNK_SIZE: usize = 512;

impl TKStream for Streamer {
    type Item = Result<web::Bytes, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        let mut buf: [u8; CHUNK_SIZE] = [0; CHUNK_SIZE];
        let mut buffer = ReadBuf::new(&mut buf);

        if this.crypto_stream.is_not_finalized() {
            match this.file.poll_read(cx, &mut buffer) {
                Poll::Ready(res) => match res {
                    Ok(_) => {
                        let mut fd: Vec<u8> = Vec::new();
                        let mut _value: Result<(Vec<u8>, Tag), ()> = Ok((fd, Tag::Message));

                        if this.chunk_size.clone() == this.current_chunk.chunk_num {
                            fd = this.stream_push.push(&buf, None, Tag::Final).unwrap();
                            _value = this.crypto_stream.pull(&fd, None);
                            this.current_chunk.chunk_num = 0;
                        } else {
                            this.current_chunk.increment();
                            fd = this.stream_push.push(&buf, None, Tag::Message).unwrap();
                            _value = this.crypto_stream.pull(&fd, None);
                        }

                        match _value {
                            Ok((decrypted, _tag)) => Poll::Ready(Some(Ok(decrypted.into()))),
                            Err(_) => Poll::Ready(Some(Err(
                                actix_web::error::ErrorInternalServerError("Incorrect"),
                            ))),
                        }
                    }
                    Err(err) => {
                        Poll::Ready(Some(Err(actix_web::error::ErrorInternalServerError(err))))
                    }
                },
                Poll::Pending => Poll::Pending,
            }
        } else {
            Poll::Ready(None)
        }
    }
}

impl Streamer {
    pub async fn download_report(
        request: HttpRequest,
        path_to_file: web::Path<FilePath>,
        settings: Data<Settings>,
    ) -> Result<HttpResponse, CustomError> {
        let token = get_token_from_header(request.headers()).map_or_else(|_error| {
            return Err(CustomError::Unauthorized.error_response());
        }, |token| Ok(token));

        if let Err(error) = token {
            return Ok(error);
        }

        let user_id_result = match handler_user_info(RpcRequest::get_userinfo_by_token(Some(token.unwrap())).await) {
            Ok(user) => Ok(UserInfo::get_pub_fields(&user.id)),
            Err(error) => Err(error)
        };

        if let Err(error) = user_id_result {
            return Ok(HttpResponse::with_body(StatusCode::NOT_FOUND, BoxBody::new(error.1)))
        }

        // Path to file
        let report_dir = if settings.get_prod() {
            dotenv!("PROD_REPORTS_DIR")
        } else {
            dotenv!("REPORTS_DIR")
        };

        let file_path = format!("{}/reports{}/{}", report_dir, user_id_result.unwrap_or("-1".to_string()), path_to_file.path.clone());
        info!("Путь до файла: {}", file_path);
        let file_size = match std::fs::metadata(&file_path) {
            Ok(file) => Ok(file.len()),
            Err(error) => Err(map_io_error(error)),
        };

        if let Err(error_msg) = file_size {
            return Err(error_msg);
        }

        let num_chunks = (file_size.unwrap() as usize + CHUNK_SIZE - 1) / CHUNK_SIZE;
        // Generate key
        let secret_key = secretstream::gen_key();
        let (stream, header) = Stream::init_push(&secret_key).unwrap();

        // [Streamer] This is necessary for saving files in parts, this object is used for streaming data.
        // [Streamer] Эта структура необхадима для отправки файла по чанкам.
        let streamer = Streamer {
            crypto_stream: Stream::init_pull(&header, &secret_key).unwrap(),
            file: File::open(&Path::new(&file_path)).await.unwrap(),
            stream_push: stream,
            chunk_size: num_chunks,
            current_chunk: DownloadReportChunks { chunk_num: 0 },
        };

        Ok(HttpResponse::Ok()
            .content_type("application/vnd.openxmlformats-office document.spreadsheetml.sheet")
            .append_header((
                header::ACCESS_CONTROL_ALLOW_ORIGIN,
                "*"
            ))
            .append_header((
                header::CONTENT_DISPOSITION,
                format!("attachment; filename={}", path_to_file.path).as_str(),
            ))
            .streaming(streamer))
    }
}
