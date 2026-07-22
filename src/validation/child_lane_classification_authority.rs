pub(super) fn has_authoritative_ownership_metadata_before(
    lines: &[&str],
    classification_start: usize,
) -> bool {
    if lines.get(classification_start) != Some(&"task classification:") {
        return false;
    }
    let Some((source, ownership)) = classification_start
        .checked_sub(2)
        .and_then(|start| lines.get(start..classification_start))
        .and_then(|metadata| metadata.first().zip(metadata.get(1)))
    else {
        return false;
    };

    matches!(
        (*source, *ownership),
        (
            "ownership metadata source: parent-supplied",
            "lane ownership: child-owned"
        ) | (
            "ownership metadata source: current-thread-classified",
            "lane ownership: current-thread-owned"
        )
    )
}
