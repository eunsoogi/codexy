use super::child_lane_thread_tool_handler_capture::has_absent_defect_capture;
use super::child_lane_thread_tool_handler_lane_mentions::has_unnegated_different_lane_phrase;

pub(super) fn scope_start_until_blank(evidence: &str, line_start: usize) -> (usize, Option<usize>) {
    let mut previous_start = line_start;
    let mut cursor = line_start;
    while cursor > 0 {
        let previous_end = cursor - 1;
        let candidate_start = evidence[..previous_end]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        if evidence[candidate_start..previous_end].trim().is_empty() {
            return (previous_start, Some(candidate_start));
        }
        previous_start = candidate_start;
        cursor = candidate_start;
    }
    (previous_start, None)
}
pub(super) fn previous_nonempty_block_start(evidence: &str, block_end: usize) -> Option<usize> {
    let mut block_start = block_end;
    let mut cursor = block_end;
    while cursor > 0 {
        let previous_end = cursor - 1;
        let candidate_start = evidence[..previous_end]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        if evidence[candidate_start..previous_end].trim().is_empty() {
            break;
        }
        block_start = candidate_start;
        cursor = candidate_start;
    }
    (block_start != block_end).then_some(block_start)
}
pub(super) fn capture_end_before_unrelated_evidence(
    evidence: &str,
    capture_start: usize,
    handler_start: usize,
) -> usize {
    let mut cursor = line_end(evidence, handler_start);
    let mut scope_lane = lane_label_for_scope(evidence, capture_start, cursor)
        .or_else(|| lane_label_for_current_scope(evidence, handler_start, cursor));
    let mut saw_capture = is_capture_related(&evidence[capture_start..cursor]);
    let mut saw_handler_defect_capture =
        is_handler_defect_capture_line(&evidence[capture_start..cursor]);
    let mut pending_defect_capture = has_open_defect_capture(&evidence[capture_start..cursor]);
    let mut saw_unscoped_handoff_metadata = false;
    while cursor < evidence.len() {
        let line_start = cursor + 1;
        let line_end = line_end(evidence, line_start);
        let line = &evidence[line_start..line_end];
        let line_opens_defect_capture = has_open_defect_capture(line);
        let line_is_unscoped_lane_prefixed_defect_capture = scope_lane.is_none()
            && !saw_unscoped_handoff_metadata
            && is_handler_defect_capture_line(line);
        if is_different_lane_line(line, scope_lane.as_deref())
            && !line_is_unscoped_lane_prefixed_defect_capture
        {
            return line_start;
        }
        let line_matches_upcoming_defect_lane = scope_lane.is_none()
            && is_lane_scoped_defect_preface_metadata_line(line)
            && metadata_line_matches_upcoming_defect_lane(evidence, line_end, line);
        let line_is_unscoped_defect_without_different_lane = scope_lane.is_none()
            && line_opens_defect_capture
            && !has_unnegated_different_lane_phrase(line)
            && !has_different_lane_mention(line);
        let line_is_pending_unscoped_capture_without_different_lane = scope_lane.is_none()
            && pending_defect_capture
            && is_handler_capture_line(line)
            && !has_absent_defect_capture(line)
            && !has_unnegated_different_lane_phrase(line)
            && !has_different_lane_mention(line);
        let line_names_different_lane = line_mentions_different_lane(line, scope_lane.as_deref())
            && !line_is_unscoped_defect_without_different_lane
            && !line_is_pending_unscoped_capture_without_different_lane
            && !line_matches_upcoming_defect_lane;
        if line_names_different_lane {
            return line_start;
        }
        if scope_lane.is_none()
            && is_handler_defect_capture_line(line)
            && !has_different_lane_mention(line)
        {
            scope_lane = lane_mention_labels(line).into_iter().next();
        }
        let line_is_unrelated_metadata = is_unrelated_metadata_line(line)
            && (saw_handler_defect_capture
                || !is_excluded_lane_metadata_line(line)
                || line_names_different_lane);
        let line_has_handler_defect_capture = is_handler_defect_capture_line(line)
            || pending_defect_capture
                && is_handler_capture_line(line)
                && !has_absent_defect_capture(line);
        let line_extends_capture =
            if is_handoff_metadata_line(line) || line_matches_upcoming_defect_lane {
                !line_names_different_lane
            } else {
                is_capture_related(line)
                    && (!line_is_unrelated_metadata || is_handler_capture_line(line))
            };
        if line.trim().is_empty()
            || saw_capture && !line_extends_capture && line_is_unrelated_metadata
        {
            return line_start;
        }
        saw_capture |= line_extends_capture;
        saw_handler_defect_capture |= line_has_handler_defect_capture;
        pending_defect_capture |= line_opens_defect_capture;
        saw_unscoped_handoff_metadata |= is_handoff_metadata_line(line)
            && lane_mention_labels(line).is_empty()
            && !line_matches_upcoming_defect_lane;
        cursor = line_end;
    }
    evidence.len()
}
fn line_end(text: &str, line_start: usize) -> usize {
    text[line_start..]
        .find('\n')
        .map_or(text.len(), |index| line_start + index)
}
fn is_capture_related(line: &str) -> bool {
    [
        "dogfooding defect",
        "tool-exposure defect",
        "dogfooding/tool-exposure defect",
        "handler",
        "missing-handler",
        "no handler registered",
        "fallback route",
        "fallback-route",
        "fallback path",
        "fallback-path",
    ]
    .into_iter()
    .any(|marker| line.contains(marker))
}
fn is_unrelated_metadata_line(line: &str) -> bool {
    let Some((key, _)) = line_key_value(line) else {
        return false;
    };
    !is_capture_related(&key.to_ascii_lowercase())
}
fn is_excluded_lane_metadata_line(line: &str) -> bool {
    let Some((key, _)) = line_key_value(line) else {
        return false;
    };
    let key = key.trim_start().to_ascii_lowercase();
    [
        "lane owner",
        "lane owners",
        "lane ownership",
        "lane metadata",
    ]
    .iter()
    .any(|prefix| key.starts_with(prefix))
}
fn line_mentions_different_lane(line: &str, current_lane: Option<&str>) -> bool {
    if has_unnegated_different_lane_phrase(line) {
        return true;
    }
    lane_mention_labels(line)
        .into_iter()
        .any(|lane| current_lane.is_none_or(|current_lane| lane != current_lane))
}

pub(super) fn has_different_lane_mention(line: &str) -> bool {
    let lanes = lane_mention_labels(line);
    let Some(defect_lane) = lanes.first() else {
        return false;
    };
    lanes.iter().skip(1).any(|lane| lane != defect_lane)
}

fn metadata_line_matches_upcoming_defect_lane(
    evidence: &str,
    metadata_line_end: usize,
    line: &str,
) -> bool {
    let lanes = lane_mention_labels(line);
    let Some(metadata_lane) = lanes.first() else {
        return false;
    };
    if lanes.iter().skip(1).any(|lane| lane != metadata_lane) {
        return false;
    }

    let mut cursor = metadata_line_end;
    while cursor < evidence.len() {
        let next_start = cursor + 1;
        let next_end = line_end(evidence, next_start);
        let next_line = &evidence[next_start..next_end];
        if next_line.trim().is_empty() || is_different_lane_line(next_line, Some(metadata_lane)) {
            return false;
        }
        if is_handler_defect_capture_line(next_line) {
            let defect_lanes = lane_mention_labels(next_line);
            return defect_lanes.first() == Some(metadata_lane)
                && !defect_lanes
                    .iter()
                    .skip(1)
                    .any(|lane| lane != metadata_lane);
        }
        if has_open_defect_capture(next_line)
            && list_capture_matches_metadata_lane(evidence, next_end, metadata_lane)
        {
            return true;
        }
        if !is_lane_scoped_defect_preface_metadata_line(next_line) && !is_capture_related(next_line)
        {
            return false;
        }
        cursor = next_end;
    }
    false
}

fn list_capture_matches_metadata_lane(
    evidence: &str,
    defect_header_end: usize,
    metadata_lane: &str,
) -> bool {
    let header_start = evidence[..defect_header_end]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let header_lanes = lane_mention_labels(&evidence[header_start..defect_header_end]);
    let header_matches_metadata_lane = header_lanes.first().is_some_and(|lane| {
        lane == metadata_lane
            && !header_lanes
                .iter()
                .skip(1)
                .any(|lane| lane != metadata_lane)
    });
    let mut cursor = defect_header_end;
    while cursor < evidence.len() {
        let item_start = cursor + 1;
        let item_end = line_end(evidence, item_start);
        let item = &evidence[item_start..item_end];
        if item.trim().is_empty() || !is_list_item(item) {
            return false;
        }
        let item_lanes = lane_mention_labels(item);
        if item_lanes.iter().any(|lane| lane != metadata_lane) {
            return false;
        }
        if item_lanes.first().is_some_and(|lane| lane == metadata_lane)
            && is_handler_capture_line(item)
            && !has_absent_defect_capture(item)
        {
            return true;
        }
        if item_lanes.is_empty()
            && header_matches_metadata_lane
            && is_handler_capture_line(item)
            && !has_absent_defect_capture(item)
        {
            return true;
        }
        cursor = item_end;
    }
    false
}

fn lane_mention_labels(line: &str) -> Vec<String> {
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

fn is_unambiguous_conjunction_lane_label(label: &str, plural_lane_marker: bool) -> bool {
    label.bytes().all(|byte| byte.is_ascii_digit())
        || label.len() == 1
            && label.bytes().all(|byte| byte.is_ascii_alphabetic())
            && !label.eq_ignore_ascii_case("i")
        || plural_lane_marker && is_lowercase_lane_label_token(label)
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

    matches!(previous.as_str(), "for" | "in")
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
pub(super) fn is_handoff_metadata_line(line: &str) -> bool {
    let Some((key, _)) = line_key_value(line) else {
        return false;
    };
    matches!(
        key.to_ascii_lowercase().trim(),
        "fallback route"
            | "fallback-route"
            | "fallback route used"
            | "fallback path"
            | "fallback-path"
            | "no fallback route"
            | "no fallback-route"
            | "no fallback path"
            | "no fallback-path"
            | "tracking issue"
            | "tracked in issue"
            | "tracked by issue"
            | "separate tracking issue"
            | "separate dogfood issue"
            | "separate dogfooding issue"
            | "follow-up issue"
    )
}

fn is_lane_scoped_defect_preface_metadata_line(line: &str) -> bool {
    if is_handoff_metadata_line(line) {
        return true;
    }
    let Some((key, _)) = line_key_value(line) else {
        return false;
    };
    matches!(
        key.to_ascii_lowercase().trim(),
        "pending worktree"
            | "pending worktree id"
            | "pending worktree ids"
            | "child thread"
            | "child thread id"
            | "child thread ids"
            | "thread"
            | "thread id"
            | "thread ids"
    )
}

pub(super) fn preceding_handoff_metadata_start(evidence: &str, line_start: usize) -> usize {
    let mut capture_start = line_start;
    let mut cursor = line_start;
    let current_line = &evidence[line_start..line_end(evidence, line_start)];
    let current_lane = lane_label(current_line)
        .or_else(|| lane_label_for_current_scope(evidence, line_start, line_start));
    while cursor > 0 {
        let previous_end = cursor - 1;
        let previous_start = evidence[..previous_end]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let previous_line = &evidence[previous_start..previous_end];
        if is_different_lane_line(previous_line, current_lane.as_deref()) {
            break;
        }
        if is_handoff_metadata_line(previous_line)
            && handoff_metadata_lane(evidence, previous_start, previous_line)
                .is_some_and(|lane| Some(lane.as_str()) != current_lane.as_deref())
        {
            break;
        }
        if !is_handoff_metadata_line(previous_line) || has_absent_defect_capture(previous_line) {
            break;
        }
        capture_start = previous_start;
        cursor = previous_start;
    }
    if capture_start != line_start && previous_line_is_defect_label(evidence, capture_start) {
        return line_start;
    }
    capture_start
}

fn previous_line_is_defect_label(evidence: &str, line_start: usize) -> bool {
    let mut cursor = line_start;
    while cursor > 0 {
        let previous_end = cursor - 1;
        let previous_start = evidence[..previous_end]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let previous_line = &evidence[previous_start..previous_end];
        if is_exact_handler_error_metadata_line(previous_line) {
            cursor = previous_start;
            continue;
        }
        return !has_absent_defect_capture(previous_line)
            && [
                "dogfooding defect",
                "tool-exposure defect",
                "dogfooding/tool-exposure defect",
            ]
            .into_iter()
            .any(|marker| previous_line.contains(marker));
    }
    false
}

fn is_exact_handler_error_metadata_line(line: &str) -> bool {
    let Some((key, value)) = line_key_value(line) else {
        return false;
    };
    key.trim()
        .eq_ignore_ascii_case("exact missing-handler error")
        && value
            .to_ascii_lowercase()
            .contains("no handler registered for tool:")
}
pub(super) fn following_handoff_metadata_has(
    evidence: &str,
    line_start: usize,
    predicate: impl Fn(&str) -> bool,
) -> bool {
    let mut cursor = line_end(evidence, line_start);
    let current_lane = lane_label_for_current_scope(evidence, line_start, cursor);
    while cursor < evidence.len() {
        let next_start = cursor + 1;
        let next_end = line_end(evidence, next_start);
        let line = &evidence[next_start..next_end];
        if line.trim().is_empty() {
            return false;
        }
        if is_different_lane_line(line, current_lane.as_deref()) {
            return false;
        }
        if predicate(line) {
            return true;
        }
        if !is_handoff_metadata_line(line) {
            return false;
        }
        cursor = next_end;
    }
    false
}

pub(super) fn is_list_item(line: &str) -> bool {
    strip_list_prefix(line) != line.trim_start()
}
fn line_key_value(line: &str) -> Option<(&str, &str)> {
    let trimmed = strip_list_prefix(line);
    let (key, value) = trimmed.split_once(':')?;
    let key = strip_lane_label_prefix(key);
    if key.trim().is_empty() {
        return value.trim_start().split_once(':');
    }
    Some((key, value))
}
fn strip_list_prefix(line: &str) -> &str {
    let line = line.trim_start();
    if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
        return rest.trim_start();
    }
    let Some((marker, rest)) = line.split_once(['.', ')']) else {
        return line;
    };
    if !marker.is_empty() && marker.bytes().all(|byte| byte.is_ascii_digit()) {
        rest.trim_start()
    } else {
        line
    }
}
fn strip_lane_label_prefix(key: &str) -> &str {
    let Some(rest) = key
        .trim_start()
        .strip_prefix("Lane ")
        .or_else(|| key.trim_start().strip_prefix("lane "))
    else {
        return key;
    };
    let label_end = rest
        .find(|ch: char| ch.is_whitespace() || ch == '-' || ch == '.')
        .unwrap_or(rest.len());
    let label = rest[..label_end].trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
    if label.is_empty() {
        return key;
    }
    if is_excluded_lane_label(label) {
        return key;
    }
    rest[label_end..].trim_start_matches(|ch: char| ch.is_whitespace() || ch == '-' || ch == '.')
}

fn lane_label_for_scope(evidence: &str, start: usize, end: usize) -> Option<String> {
    evidence[start..end].lines().filter_map(lane_label).last()
}

fn lane_label_for_current_scope(evidence: &str, line_start: usize, end: usize) -> Option<String> {
    let (block_start, blank_start) = scope_start_until_blank(evidence, line_start);
    lane_label_for_scope(evidence, line_start, end)
        .or_else(|| lane_label_for_scope(evidence, block_start, end))
        .or_else(|| {
            let blank_start = blank_start?;
            let previous_start = previous_nonempty_block_start(evidence, blank_start)?;
            lane_label_for_scope(evidence, previous_start, blank_start)
        })
}

fn is_different_lane_line(line: &str, current_lane: Option<&str>) -> bool {
    let Some(next_lane) = lane_label(line) else {
        return false;
    };
    current_lane.is_none_or(|current_lane| next_lane != current_lane)
}

fn handoff_metadata_lane(evidence: &str, line_start: usize, line: &str) -> Option<String> {
    lane_label(line).or_else(|| lane_label_for_scope(evidence, 0, line_start))
}

fn lane_label(line: &str) -> Option<String> {
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

fn is_affirmative_capture_line(line: &str) -> bool {
    "captured|classified|recorded|reported|routed|tracked"
        .split('|')
        .any(|marker| line.contains(marker))
}

fn is_handler_capture_line(line: &str) -> bool {
    is_affirmative_capture_line(line)
        && ["handler", "missing-handler", "no handler registered"]
            .into_iter()
            .any(|marker| line.contains(marker))
}

fn is_handler_defect_capture_line(line: &str) -> bool {
    is_handler_capture_line(line) && has_defect_label(line) && !has_absent_defect_capture(line)
}

fn has_open_defect_capture(line: &str) -> bool {
    has_defect_label(line) && !has_absent_defect_capture(line)
}

fn has_defect_label(line: &str) -> bool {
    line.contains("dogfooding defect")
        || line.contains("tool-exposure defect")
        || line.contains("dogfooding/tool-exposure defect")
}
