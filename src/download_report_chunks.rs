use crate::r#trait::chunks_trait::IDownloadReportChunks;

#[derive(Debug, Clone)]
pub struct DownloadReportChunks {
    pub(crate) chunk_num: usize,
}

impl IDownloadReportChunks for DownloadReportChunks {
    fn new() -> DownloadReportChunks {
        DownloadReportChunks { chunk_num: 0 }
    }

    fn increment(&mut self) {
        self.chunk_num += 1;
    }
}
