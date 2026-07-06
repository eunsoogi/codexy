pub(super) fn owner_lookup_segments(line: &str) -> Vec<String> {
    line.split(';')
        .flat_map(|segment| segment.split(". "))
        .flat_map(split_owner_lookup_and_clauses)
        .map(|segment| segment.trim().to_owned())
        .collect()
}

fn split_owner_lookup_and_clauses(segment: &str) -> Vec<&str> {
    let lower = segment.to_ascii_lowercase();
    let mut clauses = Vec::new();
    let mut start = 0;
    let mut cursor = 0;
    while let Some(relative) = lower[cursor..].find(" and ") {
        let marker_start = cursor + relative;
        let next_start = marker_start + " and ".len();
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
