use super::{capture_markers::*, fallback_routes::*, lane_scope_tokens::*};

pub(crate) fn preceding_defect_scope_lines<'a>(
    lines: &[&'a str],
    start: usize,
    index: usize,
    lane: Option<&str>,
) -> Vec<&'a str> {
    let mut skip_handoff_metadata_block = false;
    lines[start..index]
        .iter()
        .filter_map(|line| {
            if is_handoff_metadata_item_for_different_lane(line, lane) {
                skip_handoff_metadata_block = true;
                return None;
            }
            if skip_handoff_metadata_block && is_unlisted_or_list_handoff_metadata_item(line) {
                if is_handoff_metadata_item_explicitly_for_lane(line, lane) {
                    skip_handoff_metadata_block = false;
                    return Some(*line);
                }
                return None;
            }
            skip_handoff_metadata_block = false;
            Some(*line)
        })
        .collect()
}

pub(crate) fn is_handoff_list_metadata_item_for_lane(line: &str, lane: Option<&str>) -> bool {
    is_unlisted_handoff_metadata_item_for_lane(strip_list_prefix(line), lane)
}

pub(crate) fn is_unlisted_handoff_metadata_item_for_lane(line: &str, lane: Option<&str>) -> bool {
    let lower = line.to_ascii_lowercase();
    let Some(field) = strip_leading_lane_prefix_for_lane(&lower, lane) else {
        return false;
    };
    let Some(scope) = strip_leading_lane_prefix_for_lane(line, lane) else {
        return false;
    };
    is_handoff_metadata_field_line(field) && !names_later_lane_handoff(scope, lane)
}

pub(crate) fn is_exact_handler_error_metadata_item(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let line = strip_leading_lane_prefix_for_lane(&lower, None).unwrap_or(&lower);
    line.starts_with("exact missing-handler error:")
        && line.contains("no handler registered for tool:")
}

pub(crate) fn defect_lane_label(lines: &[&str], start: usize, index: usize) -> Option<String> {
    mentioned_lane_label(lines[index]).or_else(|| {
        lines[start..index]
            .iter()
            .rev()
            .find_map(|line| lane_header_label(line))
    })
}

pub(crate) fn mentioned_lane_label(line: &str) -> Option<String> {
    lane_label_prefix(line)
        .map(|(label, _)| label.to_ascii_lowercase())
        .or_else(|| mentioned_lane_label_in_phrase(line))
}

fn mentioned_lane_label_in_phrase(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    [
        "for lane ",
        "in lane ",
        "assigned to lane ",
        "targeting lane ",
    ]
    .into_iter()
    .find_map(|marker| {
        let start = lower.find(marker)?;
        if is_negated_lane_marker(&lower, start) {
            return None;
        }
        let label = line[start + marker.len()..]
            .split(|ch: char| ch.is_whitespace() || ch == ':' || ch == '-' || ch == '.')
            .next()
            .unwrap_or_default()
            .trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
        (is_lane_label_token(label) || is_lowercase_lane_label_token(label))
            .then(|| label.to_ascii_lowercase())
    })
}

pub(crate) fn current_defect_clause_scope_for_lane<'a>(
    line: &'a str,
    lane: Option<&str>,
) -> &'a str {
    let mentioned_lane = if lane.is_none() {
        mentioned_lane_label(line)
    } else {
        None
    };
    let lane = lane.or(mentioned_lane.as_deref());
    trim_at_other_lane_handoff_clause(current_defect_clause_scope(line), lane)
}

fn is_unlisted_or_list_handoff_metadata_item(line: &str) -> bool {
    is_handoff_list_metadata_item_for_lane(line, None)
        || is_unlisted_handoff_metadata_item_for_lane(line, None)
}

fn is_handoff_metadata_item_for_different_lane(line: &str, lane: Option<&str>) -> bool {
    let Some(lane) = lane else {
        return false;
    };
    let line = if is_list_item(line) {
        strip_list_prefix(line)
    } else {
        line
    };
    is_handoff_metadata_field_line(
        &strip_leading_lane_prefix_for_lane(line, None)
            .unwrap_or(line)
            .to_ascii_lowercase(),
    ) && mentions_different_lane(line, lane)
}

fn is_handoff_metadata_item_explicitly_for_lane(line: &str, lane: Option<&str>) -> bool {
    let Some(lane) = lane else {
        return false;
    };
    let line = if is_list_item(line) {
        strip_list_prefix(line)
    } else {
        line
    };
    is_handoff_metadata_field_line(
        &strip_leading_lane_prefix_for_lane(line, None)
            .unwrap_or(line)
            .to_ascii_lowercase(),
    ) && mentioned_lane_label(line).as_deref() == Some(lane)
}

fn is_handoff_metadata_field_line(line: &str) -> bool {
    let line = strip_leading_lane_prefix_for_lane(line, None).unwrap_or(line);
    is_fallback_metadata_field(line)
        || [
            "separate dogfood issue",
            "separate dogfooding issue",
            "separate tracking issue",
            "tracking issue",
            "tracked in issue",
            "tracked by issue",
            "follow-up issue",
        ]
        .into_iter()
        .any(|field| line.starts_with(field))
}

fn names_later_lane_handoff(line: &str, lane: Option<&str>) -> bool {
    lane.is_some_and(|lane| mentions_different_lane(line, lane))
}

fn lane_header_label(line: &str) -> Option<String> {
    let line = line.trim_start();
    let marker_end = line.bytes().take_while(|byte| *byte == b'#').count();
    let line = if marker_end > 0 && line[marker_end..].starts_with(' ') {
        line[marker_end..].trim_start()
    } else {
        line
    };
    let line = line.to_ascii_lowercase();
    let rest = line.strip_prefix("lane ")?;
    let label = rest.trim_end_matches([':', '.', '-']).trim();
    (!label.is_empty()
        && !is_excluded_lane_metadata_label(label)
        && label.bytes().all(|byte| byte.is_ascii_alphanumeric()))
    .then(|| label.to_string())
}

fn is_excluded_lane_metadata_label(label: &str) -> bool {
    matches!(
        label,
        "owner" | "owners" | "ownership" | "metadata" | "type"
    )
}

fn current_defect_clause_scope(line: &str) -> &str {
    let Some(defect_start) = line.find("defect") else {
        return line;
    };
    let search_start = defect_start + "defect".len();
    let lower = line.to_ascii_lowercase();
    "dogfooding defect|tool-exposure defect|dogfooding/tool-exposure defect"
        .split('|')
        .filter_map(|marker| {
            let index = search_start + lower[search_start..].find(marker)?;
            let prefix = lower[..index].trim_end();
            let suffix = lower[index + marker.len()..].trim_start();
            (is_defect_label_boundary(prefix) && suffix.starts_with(':')).then_some(index)
        })
        .min()
        .map_or(line, |next| &line[..next])
}

fn trim_at_other_lane_handoff_clause<'a>(line: &'a str, lane: Option<&str>) -> &'a str {
    let mut start = 0;
    while let Some((separator, length)) = next_handoff_clause_separator(line, start) {
        let clause = line[separator + length..].trim_start();
        if is_handoff_metadata_field_line(&clause.to_ascii_lowercase())
            && !is_unlisted_handoff_metadata_item_for_lane(clause, lane)
        {
            return line[..separator].trim_end();
        }
        start = separator + length;
    }
    line
}

fn next_handoff_clause_separator(line: &str, start: usize) -> Option<(usize, usize)> {
    [
        line[start..].find(';').map(|offset| (start + offset, 1)),
        line[start..].find(". ").map(|offset| (start + offset, 2)),
    ]
    .into_iter()
    .flatten()
    .min_by_key(|(index, _)| *index)
}
