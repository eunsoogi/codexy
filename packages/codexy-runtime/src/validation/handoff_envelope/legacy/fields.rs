use super::*;
use crate::validation::child_lane_gfm_classification_table::{
    classification_table_row, is_field_value_header, is_two_column_separator,
};

/// Reads a section's human table or its historical fields without relaxing duplicate checks.
pub(super) fn fields(
    input: &str,
    wanted: &[&str],
    delimiter: char,
) -> Result<BTreeMap<String, String>> {
    let mut result = BTreeMap::new();
    let lines = input.lines().collect::<Vec<_>>();
    let mut in_table = false;
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if delimiter == ':'
            && index + 1 < lines.len()
            && classification_table_row(line.trim())
                .is_some_and(|(key, value)| is_field_value_header(key, value))
            && is_two_column_separator(lines[index + 1].trim())
        {
            in_table = true;
            index += 2;
            continue;
        }
        if in_table {
            if line.trim().starts_with('|') {
                let (key, value, consumed) =
                    crate::validation::child_message_table::wrapped_row(&lines, index)
                        .ok_or_else(|| anyhow::anyhow!("malformed handoff table row"))?;
                insert(
                    &mut result,
                    wanted,
                    &key,
                    &crate::validation::child_message_table::cell_text(&value),
                )?;
                index += consumed;
                continue;
            }
            in_table = false;
        }
        let separator = if delimiter == ':' {
            '\n'
        } else if input.contains('\u{1f}') {
            '\u{1f}'
        } else {
            ';'
        };
        for part in line.split(separator) {
            let part = part.trim().strip_prefix("- ").unwrap_or(part.trim());
            if let Some((key, value)) = part.split_once(delimiter) {
                insert(&mut result, wanted, key.trim(), value.trim())?;
            }
        }
        index += 1;
    }
    Ok(result)
}

fn insert(
    result: &mut BTreeMap<String, String>,
    wanted: &[&str],
    key: &str,
    value: &str,
) -> Result<()> {
    if let Some(canonical) = wanted
        .iter()
        .find(|candidate| candidate.eq_ignore_ascii_case(key))
    {
        ensure!(!value.is_empty(), "legacy field {key} is empty");
        ensure!(
            result
                .insert((*canonical).to_owned(), value.to_owned())
                .is_none(),
            "legacy field {key} is repeated"
        );
    }
    Ok(())
}
