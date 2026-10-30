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

/// Apply the same readiness rules to table facts and prose, without retaining competing raw rows.
pub(super) fn normalized(text: &str) -> String {
    let current = super::readiness_context::current_text(text);
    let lines = current.lines().map(str::trim).collect::<Vec<_>>();
    let mut result = String::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        result.push_str(line);
        result.push('\n');
        if line.eq_ignore_ascii_case("child handoff:") {
            let mut cursor = index + 1;
            while lines.get(cursor).is_some_and(|line| line.is_empty()) {
                cursor += 1;
            }
            if lines
                .get(cursor)
                .and_then(|line| classification_table_row(line))
                .is_some_and(|(key, value)| is_field_value_header(key, value))
                && lines
                    .get(cursor + 1)
                    .is_some_and(|line| is_two_column_separator(line))
            {
                cursor += 2;
                while let Some((key, value, consumed)) =
                    super::child_message_table::wrapped_row(&lines, cursor)
                {
                    result.push_str(&format!(
                        "{key}: {}\n",
                        super::child_message_table::cell_text(&value)
                    ));
                    cursor += consumed;
                }
                index = cursor;
                continue;
            }
        }
        index += 1;
    }
    result
}
