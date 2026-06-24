pub(super) fn scope_start_until_blank(evidence: &str, line_start: usize) -> (usize, Option<usize>) {
    let mut previous_start = line_start;
    let mut cursor = line_start;
    while cursor > 0 {
        let previous_end = cursor - 1;
        let candidate_start = evidence[..previous_end]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        if evidence[candidate_start..previous_end].trim().is_empty() {
            return (previous_start, Some(candidate_start));
        }
        previous_start = candidate_start;
        cursor = candidate_start;
    }
    (previous_start, None)
}

pub(super) fn previous_nonempty_block_start(evidence: &str, block_end: usize) -> Option<usize> {
    let mut block_start = block_end;
    let mut cursor = block_end;
    while cursor > 0 {
        let previous_end = cursor - 1;
        let candidate_start = evidence[..previous_end]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        if evidence[candidate_start..previous_end].trim().is_empty() {
            break;
        }
        block_start = candidate_start;
        cursor = candidate_start;
    }
    (block_start != block_end).then_some(block_start)
}
