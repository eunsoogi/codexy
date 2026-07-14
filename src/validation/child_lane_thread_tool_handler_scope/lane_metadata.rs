use super::super::child_lane_thread_tool_handler_lane_mentions::has_unnegated_different_lane_phrase;
use super::super::child_lane_thread_tool_handler_scope_labels::strip_list_prefix;
use super::ownership_boundaries::{previous_nonempty_block_start, scope_start_until_blank};

pub(crate) fn lane_label_for_scope(evidence: &str, start: usize, end: usize) -> Option<String> {
    evidence[start..end].lines().filter_map(lane_label).last()
}

pub(crate) fn lane_label_for_current_scope(
    evidence: &str,
    line_start: usize,
    end: usize,
) -> Option<String> {
    let (block_start, blank_start) = scope_start_until_blank(evidence, line_start);
    lane_label_for_scope(evidence, line_start, end)
        .or_else(|| lane_label_for_scope(evidence, block_start, end))
        .or_else(|| {
            let blank_start = blank_start?;
            let previous_start = previous_nonempty_block_start(evidence, blank_start)?;
            lane_label_for_scope(evidence, previous_start, blank_start)
        })
}

pub(crate) fn is_different_lane_line(line: &str, current_lane: Option<&str>) -> bool {
    let Some(next_lane) = lane_label(line) else {
        return false;
    };
    current_lane.is_none_or(|current_lane| next_lane != current_lane)
}

pub(crate) fn handoff_metadata_lane(
    evidence: &str,
    line_start: usize,
    line: &str,
) -> Option<String> {
    lane_label(line).or_else(|| lane_label_for_scope(evidence, 0, line_start))
}

pub(crate) fn lane_label(line: &str) -> Option<String> {
    let trimmed = strip_markdown_heading_prefix(strip_list_prefix(line));
    let rest = trimmed
        .strip_prefix("Lane ")
        .or_else(|| trimmed.strip_prefix("lane "))?;
    let label = rest
        .split(|ch: char| ch.is_whitespace() || ch == ':' || ch == '-' || ch == '.')
        .next()
        .unwrap_or_default()
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
    normalized_lane_label(label)
}

pub(crate) fn lane_mention_labels(line: &str) -> Vec<String> {
    let mut labels = Vec::new();
    let lane_expression = line
        .replace("and/or", " and ")
        .replace("and-or", " and ")
        .replace(',', " , ")
        .replace('/', " / ");
    let tokens = lane_expression
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != ',' && ch != '/')
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    for (index, token) in tokens.iter().enumerate() {
        let previous = index.checked_sub(1).map_or("", |previous| tokens[previous]);
        if (token.eq_ignore_ascii_case("lane") || token.eq_ignore_ascii_case("lanes"))
            && !previous.eq_ignore_ascii_case("same")
            && !is_negated_explicit_lane_mention(&tokens, index)
        {
            let plural_lane_marker = token.eq_ignore_ascii_case("lanes");
            let context = explicit_lane_mention_context(&tokens, index).unwrap_or(previous);
            let mut label_index = index + 1;
            while let Some(label) = tokens.get(label_index).copied() {
                if label_index != index + 1
                    && !is_unambiguous_conjunction_lane_label(label, plural_lane_marker)
                {
                    break;
                }
                let Some(lane_label) = normalized_lane_mention_label(label, context) else {
                    break;
                };
                labels.push(lane_label);
                let mut next_label_index = label_index + 1;
                let Some(connector) = tokens.get(next_label_index) else {
                    break;
                };
                if !is_lane_conjunction(connector) {
                    break;
                }
                while tokens
                    .get(next_label_index)
                    .is_some_and(|token| is_lane_conjunction(token))
                {
                    next_label_index += 1;
                }
                label_index = next_label_index;
            }
        }
    }
    labels
}

pub(crate) fn line_mentions_different_lane(line: &str, current_lane: Option<&str>) -> bool {
    if has_unnegated_different_lane_phrase(line) {
        return true;
    }
    lane_mention_labels(line)
        .into_iter()
        .any(|lane| current_lane.is_none_or(|current_lane| lane != current_lane))
}

pub(crate) fn has_different_lane_mention(line: &str) -> bool {
    let lanes = lane_mention_labels(line);
    let Some(defect_lane) = lanes.first() else {
        return false;
    };
    lanes.iter().skip(1).any(|lane| lane != defect_lane)
}

fn normalized_lane_label(label: &str) -> Option<String> {
    (is_lane_label_token(label) && !is_excluded_lane_label(label))
        .then(|| format!("lane {}", label.to_ascii_lowercase()))
}

fn normalized_lane_mention_label(label: &str, previous: &str) -> Option<String> {
    let explicit_lowercase_context =
        previous.eq_ignore_ascii_case("for") || previous.eq_ignore_ascii_case("in");
    (is_lane_label_token(label)
        && !is_excluded_lane_label(label)
        && (explicit_lowercase_context || !is_lowercase_lane_label_token(label)))
    .then(|| format!("lane {}", label.to_ascii_lowercase()))
}

fn is_lane_label_token(label: &str) -> bool {
    !label.is_empty()
        && (label.bytes().all(|byte| byte.is_ascii_digit())
            || label.len() == 1 && label.bytes().all(|byte| byte.is_ascii_alphabetic())
            || label
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_uppercase())
            || is_lowercase_lane_label_token(label))
}

fn is_lowercase_lane_label_token(label: &str) -> bool {
    label
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && !matches!(
            label,
            "context" | "handoff" | "metadata" | "review" | "setup" | "thread" | "workflow"
        )
}

fn is_excluded_lane_label(label: &str) -> bool {
    ["owner", "owners", "ownership", "metadata", "type"]
        .contains(&label.to_ascii_lowercase().as_str())
}

fn strip_markdown_heading_prefix(line: &str) -> &str {
    let trimmed = line.trim_start();
    let marker_end = trimmed.bytes().take_while(|byte| *byte == b'#').count();
    if marker_end > 0 && trimmed[marker_end..].starts_with(' ') {
        trimmed[marker_end..].trim_start()
    } else {
        line
    }
}

fn is_unambiguous_conjunction_lane_label(label: &str, plural_lane_marker: bool) -> bool {
    !label.eq_ignore_ascii_case("i")
        && (label.bytes().all(|byte| byte.is_ascii_digit())
            || label.len() == 1 && label.bytes().all(|byte| byte.is_ascii_alphabetic())
            || label
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_uppercase())
            || plural_lane_marker && is_lowercase_lane_label_token(label))
}

fn is_lane_conjunction(token: &str) -> bool {
    matches!(
        token.to_ascii_lowercase().as_str(),
        "and" | "or" | "and/or" | "and-or" | "," | "/"
    )
}

fn explicit_lane_mention_context<'a>(tokens: &'a [&str], lane_index: usize) -> Option<&'a str> {
    let previous = tokens.get(lane_index.checked_sub(1)?)?;
    if previous.eq_ignore_ascii_case("targeting") {
        return Some("in");
    }
    if previous.eq_ignore_ascii_case("to")
        && lane_index >= 2
        && tokens[lane_index - 2].eq_ignore_ascii_case("assigned")
    {
        return Some("in");
    }
    None
}

fn is_negated_explicit_lane_mention(tokens: &[&str], lane_index: usize) -> bool {
    if lane_index == 0 {
        return false;
    }
    let previous = tokens[lane_index - 1].to_ascii_lowercase();
    let before_previous = lane_index
        .checked_sub(2)
        .map(|index| tokens[index].to_ascii_lowercase());
    let before_before_previous = lane_index
        .checked_sub(3)
        .map(|index| tokens[index].to_ascii_lowercase());
    is_lane_mention_negation(&previous)
        || matches!(previous.as_str(), "for" | "in")
            && before_previous
                .as_deref()
                .is_some_and(is_lane_mention_negation)
        || previous == "to"
            && before_previous.as_deref() == Some("assigned")
            && before_before_previous
                .as_deref()
                .is_some_and(is_lane_mention_negation)
        || previous == "targeting"
            && before_previous
                .as_deref()
                .is_some_and(is_lane_mention_negation)
}

fn is_lane_mention_negation(token: &str) -> bool {
    matches!(token, "not" | "never" | "without")
}

#[cfg(test)]
#[path = "lane_metadata_tests.rs"]
mod tests;
