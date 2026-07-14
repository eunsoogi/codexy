use super::{
    candidate_scopes::defect_list_item_lane_label,
    lane_scope_filters::{mentioned_lane_label, preceding_defect_scope_lines},
    lane_scope_tokens::lane_label_prefix,
};

#[test]
fn detects_a_leading_lane_label_on_a_defect_header() {
    assert_eq!(
        mentioned_lane_label(
            "Lane B dogfooding/tool-exposure defect: recorded missing-handler evidence"
        )
        .as_deref(),
        Some("b")
    );
}

#[test]
fn detects_a_leading_lane_label_on_a_bulleted_capture() {
    assert_eq!(
        defect_list_item_lane_label("- Lane A: recorded missing-handler evidence").as_deref(),
        Some("a")
    );
}

#[test]
fn excludes_prefixed_metadata_for_another_lane_from_a_preceding_scope() {
    let lines = [
        "Lane B Fallback route: no fallback route was available",
        "Lane B Tracking issue: #205",
        "Lane A dogfooding/tool-exposure defect: recorded missing-handler evidence",
    ];

    assert!(preceding_defect_scope_lines(&lines, 0, 2, Some("a")).is_empty());
}

#[test]
fn keeps_lane_ownership_metadata_out_of_lane_prefix_normalization() {
    assert!(lane_label_prefix("Lane ownership: parent-owned").is_none());
}
