pub(super) struct OwnerLookupSegment<'a> {
    pub(super) text: &'a str,
    pub(super) position: usize,
}

pub(super) fn owner_lookup_segments(line: &str) -> Vec<OwnerLookupSegment<'_>> {
    line.split(';')
        .flat_map(|segment| segment.split(". "))
        .flat_map(|segment| segment.split(", but "))
        .flat_map(|segment| segment.split(" but "))
        .flat_map(split_owner_lookup_comma_clauses)
        .flat_map(split_owner_lookup_and_clauses)
        .map(|segment| OwnerLookupSegment {
            text: segment.trim(),
            position: segment_offset(line, segment),
        })
        .collect()
}

fn segment_offset(line: &str, segment: &str) -> usize {
    segment.as_ptr() as usize - line.as_ptr() as usize
}

fn split_owner_lookup_comma_clauses(segment: &str) -> Vec<&str> {
    split_owner_lookup_clauses(segment, ", ")
}

fn split_owner_lookup_and_clauses(segment: &str) -> Vec<&str> {
    split_owner_lookup_clauses(segment, " and ")
}

fn split_owner_lookup_clauses<'a>(segment: &'a str, separator: &str) -> Vec<&'a str> {
    let lower = segment.to_ascii_lowercase();
    let mut clauses = Vec::new();
    let mut start = 0;
    let mut cursor = 0;
    while let Some(relative) = lower[cursor..].find(separator) {
        let marker_start = cursor + relative;
        let next_start = marker_start + separator.len();
        if starts_owner_lookup_clause(lower[next_start..].trim_start()) {
            clauses.push(&segment[start..marker_start]);
            start = next_start;
        }
        cursor = next_start;
    }
    clauses.push(&segment[start..]);
    clauses
}

fn starts_owner_lookup_clause(clause: &str) -> bool {
    [
        "existing owner thread",
        "existing issue owner thread",
        "existing pr owner thread",
        "existing issue/pr owner thread",
        "existing issue or pr owner thread",
        "no existing owner thread",
        "no existing issue owner thread",
        "no existing pr owner thread",
        "no existing issue/pr owner thread",
        "no existing issue or pr owner thread",
        "found no existing owner thread",
        "found no existing issue owner thread",
        "found no existing pr owner thread",
        "found no existing issue/pr owner thread",
        "found no existing issue or pr owner thread",
        "owner thread not found",
    ]
    .into_iter()
    .any(|prefix| clause.starts_with(prefix))
}
