use super::child_lane_gfm_classification_table::classification_table_row;

/// Reads one bounded row, joining wrapped text without consuming a new row or control event.
pub(super) fn wrapped_row(lines: &[&str], start: usize) -> Option<(String, String, usize)> {
    let mut row = lines.get(start)?.trim().to_owned();
    if !row.starts_with('|') {
        return None;
    }
    let mut consumed = 1;
    while classification_table_row(&row).is_none() && consumed < 8 {
        let next = lines.get(start + consumed)?.trim();
        if next.is_empty() || next.starts_with('|') || next.contains(": ") {
            return None;
        }
        row.push(' ');
        row.push_str(next);
        consumed += 1;
    }
    let (key, value) = classification_table_row(&row)?;
    Some((key.to_owned(), value.to_owned(), consumed))
}

/// Interpret an entire inline code span or link by its visible text, then unescape pipes.
pub(super) fn cell_text(value: &str) -> String {
    let value = value.trim();
    let value = value
        .strip_prefix('`')
        .and_then(|v| v.strip_suffix('`'))
        .unwrap_or(value);
    let value = value
        .strip_prefix('[')
        .and_then(|v| v.split_once("]("))
        .and_then(|(label, target)| target.ends_with(')').then_some(label))
        .unwrap_or(value);
    value.replace("\\|", "|")
}
