use super::{has_different_lane_mention, lane_mention_labels};

#[test]
fn detects_singular_multi_letter_lane_lists() {
    for line in [
        "for Lane Alpha and Beta",
        "for Lane Alpha or Beta",
        "for Lane Alpha and/or Beta",
        "for Lane Alpha and-or Beta",
        "for Lane Alpha, Beta",
        "for Lane Alpha/Beta",
    ] {
        assert!(
            has_different_lane_mention(line),
            "expected {line} to be multi-lane"
        );
    }
}

#[test]
fn ignores_first_person_conjunction_prose() {
    for line in [
        "for Lane A and I recorded the handoff",
        "for Lane A or I can provide the evidence",
        "for Lane A and/or I can follow up",
        "for Lane A and-or I can follow up",
        "for Lane Alpha and we recorded the handoff",
    ] {
        assert!(
            !has_different_lane_mention(line),
            "expected {line} to remain same-lane prose"
        );
    }
}

#[test]
fn ignores_shorthand_negated_lane_mentions() {
    assert!(lane_mention_labels("Fallback route: not Lane B").is_empty());
}
