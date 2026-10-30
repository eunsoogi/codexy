pub(super) struct ActiveLine {
    pub(super) text: String,
    pub(super) raw_text: String,
}

/// Keeps current active evidence lines and preserves both normalized and original text for classification.
pub(super) fn active_lines(evidence: &str) -> Vec<ActiveLine> {
    // The private separator is never accepted from authored evidence.
    let sanitized = evidence.replace('\u{1f}', "\u{fffd}");
    let text = super::readiness_context::current_text(&sanitized);
    let mut lines = Vec::new();
    let fragments = text.lines().collect::<Vec<_>>();
    let mut index = 0;
    while index < fragments.len() {
        let fragment = fragments[index];
        if let Some((record, consumed)) = table_record(&fragments[index..]) {
            lines.push(ActiveLine {
                text: record.to_ascii_lowercase(),
                raw_text: record,
            });
            index += consumed;
            continue;
        }
        if let Some(text) = super::readiness_context::active_line(fragment) {
            lines.push(ActiveLine {
                text: text.to_ascii_lowercase(),
                raw_text: text.to_owned(),
            });
        }
        index += 1;
    }
    lines
}

/// Coalesces only explicitly named lifecycle tables; quoted history is removed before this step.
pub(super) fn table_record(lines: &[&str]) -> Option<(String, usize)> {
    use super::child_lane_gfm_classification_table::{
        classification_table_row, is_field_value_header, is_two_column_separator,
    };
    let title = lines.first()?.trim();
    let title = match title.to_ascii_lowercase().as_str() {
        "parent handoff:" => "Terminal parent handoff:",
        "completed phase handoff:" => "Idle wait handoff:",
        "wait report:" => "Nonterminal wait handoff:",
        _ => title,
    };
    if ![
        "terminal parent handoff:",
        "idle wait handoff:",
        "nonterminal wait handoff:",
        "blocked goal user-decision gate:",
        "blocked goal pre-mutation check:",
        "parent goal pre-delivery:",
        "parent goal post-result:",
        "parent direction event:",
        "parent route:",
        "source thread id:",
        "goal tool result:",
        "idle child state:",
        "terminal child transition:",
    ]
    .iter()
    .any(|name| title.eq_ignore_ascii_case(name))
    {
        return None;
    }
    let header_index = 1 + lines[1..]
        .iter()
        .take_while(|line| line.trim().is_empty())
        .count();
    let (header, value) = classification_table_row(lines.get(header_index)?.trim())?;
    if !is_field_value_header(header, value)
        || !is_two_column_separator(lines.get(header_index + 1)?.trim())
    {
        return None;
    }
    // A private delimiter preserves semicolons in authored cell text without requiring a second payload.
    let mut record = format!("{title} \u{1f}");
    let mut count = header_index + 2;
    while let Some(line) = lines.get(count).map(|line| line.trim()) {
        if !line.starts_with('|') {
            break;
        }
        let Some((key, value, row_lines)) = super::child_message_table::wrapped_row(lines, count)
        else {
            return Some((format!("{title} invalid table"), count + 1));
        };
        if count > header_index + 2 {
            record.push('\u{1f}');
        }
        if key.chars().chain(value.chars()).any(char::is_control) {
            return Some((format!("{title} invalid table"), count + row_lines));
        }
        record.push_str(canonical_key(title, &key));
        record.push('=');
        record.push_str(&super::child_message_table::cell_text(&value));
        count += row_lines;
    }
    Some((record, count))
}

/// Normalize unambiguous label alternatives while retaining the facts they identify.
fn canonical_key<'a>(title: &str, key: &'a str) -> &'a str {
    let key = key.trim();
    if key.eq_ignore_ascii_case("parent") {
        return "parent task";
    }
    if key.eq_ignore_ascii_case("child") {
        return "child task";
    }
    if key.eq_ignore_ascii_case("head sha") {
        return "head";
    }
    if key.eq_ignore_ascii_case("verification")
        && (title.eq_ignore_ascii_case("terminal parent handoff:")
            || title.eq_ignore_ascii_case("idle wait handoff:"))
    {
        return "last proof";
    }
    key
}
