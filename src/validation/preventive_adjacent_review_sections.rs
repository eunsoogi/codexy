pub(super) fn has_false_readiness_before_evidence(text: &str, start: usize) -> bool {
    ["readiness:", "pr readiness:", "pr ready:"]
        .iter()
        .any(|label| {
            text[..start].rfind(label).is_some_and(|index| {
                let value = text[index + label.len()..start]
                    .trim_start()
                    .trim_start_matches(['-', '*'])
                    .trim_start();
                ["not ", "false", "isn't ready", "aren't ready"]
                    .iter()
                    .any(|state| value.starts_with(state))
            })
        })
}

pub(super) fn has_readiness_not_applicable_state(text: &str) -> bool {
    ["pr readiness:", "readiness:", "pr ready:"]
        .iter()
        .any(|label| {
            text.find(label).is_some_and(|index| {
                [
                    "false",
                    "not ready",
                    "not currently ready",
                    "isn't ready",
                    "isn't currently ready",
                    "aren't ready",
                    "aren't currently ready",
                    "not applicable",
                    "isn't applicable",
                    "aren't applicable",
                    "not requested",
                    "isn't requested",
                    "aren't requested",
                ]
                .iter()
                .any(|state| text[index + label.len()..].trim_start().starts_with(state))
            })
        })
}

pub(super) fn preventive_adjacent_review_end(text: &str, start: usize) -> usize {
    let suffix = &text[start..];
    let section_blank = suffix
        .match_indices("\n\n")
        .map(|(index, _)| index)
        .find(|index| !is_preventive_adjacent_heading_blank(suffix, *index));
    [
        section_blank,
        suffix.find("\n#"),
        suffix.find("\nreview "),
        plain_handoff_section_boundary(suffix),
    ]
    .into_iter()
    .flatten()
    .min()
    .map_or(text.len(), |index| start + index)
}

fn plain_handoff_section_boundary(suffix: &str) -> Option<usize> {
    suffix.match_indices('\n').find_map(|(index, _)| {
        let line = suffix[index + 1..].trim_start();
        if line.starts_with("tests:") && is_preventive_adjacent_section_label(suffix, index) {
            return None;
        }
        [
            "verification:",
            "tests:",
            "codex feedback:",
            "review feedback:",
            "reviewer feedback:",
            "review thread:",
            "review comment:",
            "review comments:",
            "reviewer comments:",
            "review suggestion:",
            "review suggestions:",
            "not run:",
            "blockers:",
            "waiting:",
            "sentinel:",
        ]
        .iter()
        .any(|label| line.starts_with(label))
        .then_some(index)
    })
}

fn is_preventive_adjacent_section_label(suffix: &str, index: usize) -> bool {
    suffix[..index]
        .rsplit("\n\n")
        .take(2)
        .any(is_preventive_adjacent_section)
}

fn is_preventive_adjacent_section(section: &str) -> bool {
    let first_line = section
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");
    let heading = first_line
        .trim()
        .trim_matches(|ch: char| ch.is_ascii_whitespace() || matches!(ch, '#' | ':' | '-' | '.'));
    heading == "preventive adjacent review"
        || first_line
            .trim_start()
            .starts_with("preventive adjacent review:")
        || heading.starts_with("preventive adjacent review evidence")
        || heading.starts_with("preventive adjacent review no-change rationale")
        || heading.starts_with("preventive adjacent review no change rationale")
}

fn is_preventive_adjacent_heading_blank(suffix: &str, index: usize) -> bool {
    let heading = suffix[..index]
        .trim()
        .trim_matches(|ch: char| ch.is_ascii_whitespace() || matches!(ch, '#' | ':' | '-' | '.'));
    heading == "preventive adjacent review"
        || heading.starts_with("preventive adjacent review evidence")
        || heading.starts_with("preventive adjacent review no-change rationale")
        || heading.starts_with("preventive adjacent review no change rationale")
}
