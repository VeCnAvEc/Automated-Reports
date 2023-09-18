use crate::download_report_chunks::DownloadReportChunks;

pub trait IDownloadReportChunks {
    fn new() -> DownloadReportChunks;

    fn increment(&mut self);
}
