pub(super) fn check(evidence: &str) -> Vec<String> {
    let normalized = evidence.to_ascii_lowercase();
    let mut errors = super::child_lane_classification_control::check(&normalized);
    errors.extend(super::child_lane_classification_setup::check(&normalized));
    errors
}
