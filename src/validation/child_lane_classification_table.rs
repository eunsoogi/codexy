use super::child_lane_classification_markdown::{
    is_in_non_rendering_block, is_indented_code_line, list_continuation_indent,
};
use super::child_lane_classification_owner::is_child_completion_owner;

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
        .filter(|index| parse_cells(lines[*index]).is_some_and(|cells| cells == HEADER))
        .filter(|index| is_separator(lines.get(index + 1).copied().unwrap_or("")))
        .filter(|index| table_can_start(raw_lines, *index))
        .filter(|index| !is_in_non_rendering_block(raw_lines, *index))
        .filter(|index| {
            !is_indented_code_line(raw_lines[*index]) || list_indent(raw_lines, *index).is_some()
        });
    let header_index = headers.next()?;
    if header_index >= setup_index || headers.next().is_some() {
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
    let list_indent = list_indent(raw_lines, header_index);
    if (header_index..=end).any(|index| {
        raw_lines.get(index).is_none_or(|line| {
            is_indented_code_line(line)
                && list_indent.is_none_or(|indent| leading_indent(line) < indent)
        })
    }) {
        return None;
    }
    if lines
        .get(end + 1)
        .and_then(|line| table_record_key(line))
        .is_some()
    {
        return None;
    }
    if ((end + 1)..lane_end).any(|index| table_header_at(raw_lines, lines, index)) {
        return None;
    }
    owner.filter(|value| is_child_completion_owner(value))?;
    Some(end)
}

fn table_header_at(raw_lines: &[&str], lines: &[&str], index: usize) -> bool {
    parse_cells(lines[index]).is_some_and(|cells| cells == HEADER)
        && is_separator(lines.get(index + 1).copied().unwrap_or(""))
        && !is_in_non_rendering_block(raw_lines, index)
}

fn table_can_start(raw_lines: &[&str], index: usize) -> bool {
    index == 0
        || list_indent(raw_lines, index).is_some()
        || list_block_boundary(raw_lines, index)
        || raw_lines.get(index - 1).is_some_and(|line| {
            let trimmed = line.trim();
            trimmed.is_empty()
                || starts_metadata_boundary(trimmed, "Lane ownership:")
                || starts_metadata_boundary(trimmed, "Owner decision:")
                || trimmed.starts_with('#')
                || trimmed.starts_with('>')
                || trimmed.starts_with('<')
                || trimmed.ends_with("-->")
                || trimmed.starts_with(['`', '~'])
                || is_indented_code_line(line)
        })
}

fn starts_metadata_boundary(line: &str, boundary: &str) -> bool {
    line.get(..boundary.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(boundary))
}

fn list_block_boundary(raw_lines: &[&str], index: usize) -> bool {
    raw_lines[..index]
        .iter()
        .rev()
        .find(|line| !line.trim().is_empty() && !line.starts_with(' '))
        .and_then(|line| list_continuation_indent(line))
        .is_some()
}

fn list_indent(raw_lines: &[&str], index: usize) -> Option<usize> {
    index
        .checked_sub(1)
        .and_then(|index| raw_lines.get(index))
        .and_then(|line| list_continuation_indent(line))
        .filter(|indent| {
            raw_lines
                .get(index)
                .is_some_and(|line| leading_indent(line) >= *indent)
        })
}

fn leading_indent(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}

pub(super) fn table_row(line: &str) -> Option<(&str, &str)> {
    let cells = parse_cells(line)?;
    Some((cells[0], cells[1]))
}

pub(super) fn is_table_header(line: &str) -> bool {
    parse_cells(line).is_some_and(|cells| cells == HEADER)
}

pub(super) fn is_table_line(line: &str) -> bool {
    is_table_header(line) || is_separator(line) || table_record_key(line).is_some_and(records_key)
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
    let mut separators = inner
        .match_indices('|')
        .map(|(index, _)| index)
        .filter(|index| !is_escaped(inner, *index));
    let separator = separators.next()?;
    separators
        .next()
        .is_none()
        .then_some([inner[..separator].trim(), inner[separator + 1..].trim()])
}

fn table_record_key(line: &str) -> Option<&str> {
    let inner = line.strip_prefix('|').unwrap_or(line);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    let separator = inner
        .match_indices('|')
        .map(|(index, _)| index)
        .find(|index| !is_escaped(inner, *index))?;
    Some(inner[..separator].trim())
}

fn is_escaped(text: &str, index: usize) -> bool {
    text.as_bytes()[..index]
        .iter()
        .rev()
        .take_while(|byte| **byte == b'\\')
        .count()
        % 2
        == 1
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
