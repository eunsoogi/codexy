use super::child_lane_owner_decision::{is_child_delegation_owner_decision, is_parent_owned_value};

const HEADER: [&str; 2] = ["task classification", "decision"];
const FIELDS: [&str; 8] = [
    "lane type",
    "secondary surfaces",
    "owner decision",
    "atomic scope",
    "required skills",
    "required tools/evidence",
    "first allowed action",
    "stop/blocker",
];

pub(super) fn complete_child_classification_index(
    lines: &[&str],
    lane_start: usize,
    setup_index: usize,
) -> Option<usize> {
    let mut headers = (lane_start..setup_index)
        .filter(|index| parse_cells(lines[*index]).is_some_and(|cells| cells == HEADER));
    let header_index = headers.next()?;
    if headers.next().is_some() || !is_separator(lines.get(header_index + 1).copied()?) {
        return None;
    }
    let mut owner = None;
    for (offset, expected) in FIELDS.iter().enumerate() {
        let index = header_index + offset + 2;
        let (field, value) = table_row(lines.get(index).copied()?)?;
        if field != *expected || value.is_empty() {
            return None;
        }
        if field == "owner decision" {
            owner = Some(value);
        }
    }
    let end = header_index + FIELDS.len() + 1;
    if lines
        .get(end + 1)
        .and_then(|line| table_row(line))
        .is_some()
    {
        return None;
    }
    owner.filter(|value| is_child_completion_owner(value))?;
    Some(end)
}

pub(super) fn table_row(line: &str) -> Option<(&str, &str)> {
    let cells = parse_cells(line)?;
    Some((cells[0], cells[1]))
}

pub(super) fn is_table_header(line: &str) -> bool {
    parse_cells(line).is_some_and(|cells| cells == HEADER)
}

pub(super) fn is_table_line(line: &str) -> bool {
    is_table_header(line)
        || is_separator(line)
        || table_row(line).is_some_and(|(key, _)| records_key(key))
}

pub(super) fn records_key(key: &str) -> bool {
    FIELDS.iter().any(|field| field.eq_ignore_ascii_case(key))
        || matches!(
            key,
            "required tools" | "required evidence" | "stop blocker" | "blocker"
        )
}

fn parse_cells(line: &str) -> Option<[&str; 2]> {
    let inner = line.strip_prefix('|')?.strip_suffix('|')?;
    let mut cells = inner.split('|').map(str::trim);
    let result = [cells.next()?, cells.next()?];
    cells.next().is_none().then_some(result)
}

fn is_separator(line: &str) -> bool {
    parse_cells(line).is_some_and(|cells| {
        cells
            .iter()
            .all(|cell| cell.trim_matches(':').chars().all(|ch| ch == '-') && cell.len() >= 3)
    })
}

fn is_child_completion_owner(value: &str) -> bool {
    is_current_thread_owner(value)
        || (!is_parent_owned_value(value) && is_child_delegation_owner_decision(value))
}

fn is_current_thread_owner(value: &str) -> bool {
    value.starts_with("current-thread-owned")
        && (value.contains("implementation") || value.contains("구현"))
        && !value.contains("not current-thread-owned")
}

#[cfg(test)]
mod tests {
    use super::complete_child_classification_index;

    #[test]
    fn parses_canonical_table() {
        let evidence = r#"Lane ownership: child-owned
| Task classification | Decision |
| --- | --- |
| Lane type | implementation |
| Secondary surfaces | workflow, validators |
| Owner decision | current-thread-owned implementation lane for #461 |
| Atomic scope | issue-sized |
| Required skills | task-classification, codex-orchestration |
| Required tools/evidence | goal, plan, codegraph, LSP, Sentinel |
| First allowed action | create branch after classification |
| Stop/blocker | None |
Child branch codexy/461-table was created after classification."#;
        let normalized = evidence.to_ascii_lowercase();
        let lines = normalized.lines().collect::<Vec<_>>();
        assert_eq!(complete_child_classification_index(&lines, 1, 11), Some(10));
    }
}
