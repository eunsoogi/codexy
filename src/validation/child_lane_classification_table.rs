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
    raw_lines: &[&str],
    lines: &[&str],
    lane_start: usize,
    setup_index: usize,
    lane_end: usize,
) -> Option<usize> {
    let mut headers = (lane_start..lane_end)
        .filter(|index| parse_cells(lines[*index]).is_some_and(|cells| cells == HEADER));
    let header_index = headers.next()?;
    if header_index >= setup_index
        || headers.next().is_some()
        || is_inside_fenced_code_block(raw_lines, header_index)
        || is_inside_html_comment(raw_lines, header_index)
        || !is_separator(lines.get(header_index + 1).copied()?)
    {
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
    if (header_index..=end).any(|index| {
        raw_lines
            .get(index)
            .is_none_or(|line| is_indented_code_line(line))
    }) {
        return None;
    }
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

fn is_indented_code_line(line: &str) -> bool {
    line.starts_with('\t')
        || line
            .as_bytes()
            .iter()
            .take_while(|byte| **byte == b' ')
            .count()
            >= 4
}

fn is_inside_fenced_code_block(lines: &[&str], index: usize) -> bool {
    let mut open = None;
    for line in lines.iter().take(index) {
        let Some(candidate) = fence_candidate(line) else {
            continue;
        };
        match open {
            Some((marker, length)) if closes_fence(candidate, marker, length) => open = None,
            None => open = opens_fence(candidate),
            _ => {}
        }
    }
    open.is_some()
}

fn is_inside_html_comment(lines: &[&str], index: usize) -> bool {
    let mut open = false;
    for line in lines.iter().take(index) {
        let mut remaining = *line;
        loop {
            if open {
                let Some(end) = remaining.find("-->") else {
                    break;
                };
                open = false;
                remaining = &remaining[end + 3..];
            } else {
                let Some(start) = remaining.find("<!--") else {
                    break;
                };
                open = true;
                remaining = &remaining[start + 4..];
            }
        }
    }
    open
}

fn fence_candidate(line: &str) -> Option<&str> {
    let spaces = line
        .as_bytes()
        .iter()
        .take_while(|byte| **byte == b' ')
        .count();
    (spaces <= 3 && !line.starts_with('\t')).then(|| &line[spaces..])
}

fn opens_fence(line: &str) -> Option<(u8, usize)> {
    let marker = *line.as_bytes().first()?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }
    let length = line
        .as_bytes()
        .iter()
        .take_while(|byte| **byte == marker)
        .count();
    (length >= 3 && (marker != b'`' || !line[length..].contains('`'))).then_some((marker, length))
}

fn closes_fence(line: &str, marker: u8, minimum: usize) -> bool {
    let length = line
        .as_bytes()
        .iter()
        .take_while(|byte| **byte == marker)
        .count();
    length >= minimum && line[length..].trim().is_empty()
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
        cells.iter().all(|cell| {
            let hyphens = cell.strip_prefix(':').unwrap_or(cell);
            let hyphens = hyphens.strip_suffix(':').unwrap_or(hyphens);
            hyphens.len() >= 3 && hyphens.chars().all(|ch| ch == '-')
        })
    })
}

fn is_child_completion_owner(value: &str) -> bool {
    is_current_thread_owner(value)
        || (!is_parent_owned_value(value) && is_child_delegation_owner_decision(value))
}

fn is_current_thread_owner(value: &str) -> bool {
    value.starts_with("current-thread-owned")
        && (value.contains("implementation lane")
            || value.contains("child implementation")
            || value.contains("구현"))
        && !value.contains("not current-thread-owned")
        && !value.contains("not implementation")
        && !value.contains("no implementation")
        && !value.contains("without implementation")
        && !value.contains("구현을 소유하지")
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
        assert_eq!(
            complete_child_classification_index(&lines, &lines, 1, 11, lines.len()),
            Some(10)
        );
    }
}
