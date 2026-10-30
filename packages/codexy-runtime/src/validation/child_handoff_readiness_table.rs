use super::child_lane_gfm_classification_table::{
    classification_table_row, is_field_value_header, is_two_column_separator,
};

/// Extracts only current, named child-handoff tables, preserving each table's separate context.
pub(super) fn rows(text: &str) -> Vec<Vec<(String, String)>> {
    let current = super::readiness_context::current_text(text);
    let lines = current.lines().map(str::trim).collect::<Vec<_>>();
    let mut tables = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if !line.eq_ignore_ascii_case("child handoff:") {
            continue;
        }
        let mut cursor = index + 1;
        while lines.get(cursor).is_some_and(|line| line.is_empty()) {
            cursor += 1;
        }
        if !lines
            .get(cursor)
            .and_then(|line| classification_table_row(line))
            .is_some_and(|(key, value)| is_field_value_header(key, value))
            || !lines
                .get(cursor + 1)
                .is_some_and(|line| is_two_column_separator(line))
        {
            continue;
        }
        cursor += 2;
        let mut table = Vec::new();
        while let Some((key, value, consumed)) =
            super::child_message_table::wrapped_row(&lines, cursor)
        {
            table.push((key, super::child_message_table::cell_text(&value)));
            cursor += consumed;
        }
        tables.push(table);
    }
    tables
}

/// Reuses the existing negative-label rules for facts carried in table cells.
pub(super) fn label_text(text: &str) -> String {
    rows(text)
        .into_iter()
        .flatten()
        .map(|(key, value)| format!("{key}: {value}\n"))
        .collect()
}
