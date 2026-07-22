pub(super) fn has_parent_supplied_child_metadata_before(
    lines: &[&str],
    classification_start: usize,
) -> bool {
    let metadata_end = classification_start
        .checked_sub(1)
        .filter(|index| lines.get(*index) == Some(&"task classification:"))
        .unwrap_or(classification_start);
    let Some((source, ownership)) = metadata_end
        .checked_sub(2)
        .and_then(|start| lines.get(start..metadata_end))
        .and_then(|metadata| metadata.first().zip(metadata.get(1)))
    else {
        return false;
    };

    *source == "ownership metadata source: parent-supplied"
        && *ownership == "lane ownership: child-owned"
}
