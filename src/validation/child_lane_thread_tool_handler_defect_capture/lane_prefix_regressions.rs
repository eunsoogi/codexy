use super::{
    candidate_scopes::defect_list_item_lane_label,
    lane_scope_filters::{mentioned_lane_label, preceding_defect_scope_lines},
    lane_scope_tokens::{lane_label_prefix, mentions_different_lane},
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

#[test]
fn detects_plural_handoff_markers_that_name_another_lane() {
    for marker in [
        "for lanes",
        "in lanes",
        "assigned to lanes",
        "targeting lanes",
    ] {
        assert!(
            mentions_different_lane(
                &format!("Fallback route: unavailable {marker} A and B"),
                "a"
            ),
            "expected {marker} to retain cross-lane scope"
        );
    }
}

#[test]
fn detects_supported_plural_lane_connectors() {
    for lanes in ["A or B", "A, B", "A/B"] {
        assert!(mentions_different_lane(
            &format!("Tracking issue: #205 for lanes {lanes}"),
            "a"
        ));
    }
}

#[test]
fn keeps_same_lane_and_negated_plural_markers_in_scope() {
    assert!(!mentions_different_lane(
        "Fallback route: unavailable for lanes A and A",
        "a"
    ));
    assert!(!mentions_different_lane(
        "Fallback route: not for lanes A and B",
        "a"
    ));
    assert!(!mentions_different_lane(
        "Fallback route: unavailable for lanes A and workflow",
        "a"
    ));
}

#[test]
fn detects_later_plural_marker_after_a_negated_occurrence() {
    for marker in [
        "for lanes",
        "in lanes",
        "assigned to lanes",
        "targeting lanes",
    ] {
        assert!(
            mentions_different_lane(
                &format!("not {marker} A and B; fallback route recorded {marker} A and C"),
                "a"
            ),
            "expected the later {marker} occurrence to retain cross-lane scope"
        );
    }
}
