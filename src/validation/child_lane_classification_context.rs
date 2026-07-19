use super::child_lane_classification_owner::is_child_completion_owner;
use super::child_lane_classification_table::table_row;
use super::child_lane_ownership_phrases::metadata_key;

pub(super) fn context_metadata_key(line: &str) -> &str {
    let line = metadata_key(line);
    line.split_once(". ")
        .filter(|(prefix, _)| prefix.chars().all(|character| character.is_ascii_digit()))
        .map_or(line, |(_, value)| metadata_key(value))
}

pub(super) fn table_owner_value(line: &str) -> Option<&str> {
    table_row(line)
        .filter(|(key, _)| *key == "owner decision")
        .map(|(_, value)| value)
}

pub(super) fn is_child_classification_owner_line(line: &str) -> bool {
    table_owner_value(line).is_some_and(is_child_completion_owner)
}
