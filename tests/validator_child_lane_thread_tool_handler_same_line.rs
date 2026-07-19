use std::process::Output;

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    crate::support::validator_child_lane_ownership_file(&evidence_path)
}

#[path = "validator_child_lane_thread_tool_handler_same_line/cross_lane_rejection.rs"]
mod cross_lane_rejection;
#[path = "validator_child_lane_thread_tool_handler_same_line/defect_capture_rejection.rs"]
mod defect_capture_rejection;
#[path = "validator_child_lane_thread_tool_handler_same_line/metadata_boundaries.rs"]
mod metadata_boundaries;
#[path = "validator_child_lane_thread_tool_handler_same_line/same_lane_captures.rs"]
mod same_lane_captures;
