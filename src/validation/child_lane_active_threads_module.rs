#[path = "child_lane_active_threads.rs"]
mod check;
#[path = "child_lane_active_thread_capacity.rs"]
mod child_lane_active_thread_capacity;
#[path = "child_lane_active_thread_count.rs"]
mod child_lane_active_thread_count;
#[path = "child_lane_active_thread_count_negation.rs"]
mod child_lane_active_thread_count_negation;
#[path = "child_lane_active_thread_count_records.rs"]
mod child_lane_active_thread_count_records;
#[path = "child_lane_active_thread_count_segments.rs"]
mod child_lane_active_thread_count_segments;
#[path = "child_lane_active_thread_evidence.rs"]
mod child_lane_active_thread_evidence;
#[path = "child_lane_active_thread_freed_capacity.rs"]
mod child_lane_active_thread_freed_capacity;
#[path = "child_lane_active_thread_operations.rs"]
mod child_lane_active_thread_operations;
#[path = "child_lane_active_thread_owner_lookup_segments.rs"]
mod child_lane_active_thread_owner_lookup_segments;

pub(super) fn check(evidence: &str) -> Vec<String> {
    check::check(evidence)
}
