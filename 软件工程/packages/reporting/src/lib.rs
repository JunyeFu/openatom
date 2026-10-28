//! Actual frozen-run comparison; current edits never reinterpret historical Runs.
mod comparison;
mod cloud_data_export;
mod evaluation;
mod evaluation_output;
mod evaluation_pdf;
mod evaluation_reading;
mod evaluation_rows;
mod execution_evaluation;
mod execution_output;
mod exact_cost;
mod metrics;
pub use comparison::compare_runs;
pub use cloud_data_export::{CloudDataExportCounts, CloudDataExportInput, write_cloud_data_export};
pub use evaluation::build_evaluation;
pub use execution_evaluation::{ExecutionReportInput, ExecutionReportInputSession, PredictionInput, build_execution_report};
pub use execution_output::render_execution_report;
pub use evaluation_output::{
    EvaluationFile, EvaluationOutputFormat, EvaluationResources, render_evaluation,
};
