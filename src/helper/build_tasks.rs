use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::sync::RwLock as TokioRwLock;

use crate::helper::generate_xlsx::create_task;

use crate::indexing_report_struct::IndexingReport;
use crate::r#trait::filter_report::{ReportItemType, ReportType};
use crate::r#type::types::ChunksInReport;
use crate::share::Report;

pub async fn processing_tasks(
    chunks: ChunksInReport,
    report: Arc<TokioRwLock<Report>>,
    report_item_type: &ReportItemType,
    collect_indexing: &IndexingReport,
    provider_name: &mut String,
    report_type: ReportType,
    number_of_chunks: usize,
) -> Vec<JoinHandle<()>> {
    let mut tasks = Vec::new();

    for (chunk_index, chunk) in chunks.into_iter().enumerate() {
        if chunk_index == 0 && provider_name.is_empty() {
            provider_name.push_str(chunk[0][collect_indexing.index_provider.unwrap()].as_str());
        }

        let task = create_task(
            Arc::clone(&report),
            chunk_index,
            chunk,
            number_of_chunks,
            collect_indexing,
            report_item_type,
            &report_type,
        ).await;

        tasks.push(task);
    }

    tasks
}
