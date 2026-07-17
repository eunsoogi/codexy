use super::policy::{finish_block, has_active_content, leading_ascii_spaces, policy_line};

pub(super) fn delivery(section: &str, instruction_starts: &[&str]) -> Vec<(&'static str, String)> {
    const DIRECTIONS: [&str; 2] = [
        "Parent-to-generic-child delivery MUST pass",
        "child-to-root delivery MUST pass",
    ];
    let mut blocks = Vec::new();
    let mut block = None;
    for line in section.lines() {
        let indentation = leading_ascii_spaces(line);
        let trimmed = line.trim_start_matches(' ').trim_end();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            finish_block(&mut blocks, &mut block);
        } else if has_active_content(line, indentation)
            && policy_line(trimmed).is_some_and(|instruction| {
                DIRECTIONS
                    .iter()
                    .any(|direction| instruction.starts_with(direction))
                    || instruction_starts
                        .iter()
                        .any(|start| instruction.starts_with(start))
                        && DIRECTIONS
                            .iter()
                            .any(|direction| instruction.contains(direction))
            })
        {
            finish_block(&mut blocks, &mut block);
            block = policy_line(trimmed).map(str::to_owned);
        } else if let Some(current) = &mut block {
            if has_active_content(line, indentation) && indentation > 0 {
                current.push(' ');
                current.push_str(trimmed);
            } else {
                finish_block(&mut blocks, &mut block);
            }
        }
    }
    finish_block(&mut blocks, &mut block);
    blocks
        .into_iter()
        .flat_map(|block| split(&block, &DIRECTIONS))
        .collect()
}

pub(super) fn split(block: &str, directions: &[&'static str]) -> Vec<(&'static str, String)> {
    let mut starts = directions
        .iter()
        .flat_map(|direction| {
            block
                .match_indices(direction)
                .map(move |(start, _)| (start, *direction))
        })
        .collect::<Vec<_>>();
    starts.sort_by_key(|(start, _)| *start);
    starts
        .iter()
        .enumerate()
        .map(|(index, (start, direction))| {
            let end = starts.get(index + 1).map_or(block.len(), |(next, _)| *next);
            (*direction, block[start + direction.len()..end].to_owned())
        })
        .collect()
}
