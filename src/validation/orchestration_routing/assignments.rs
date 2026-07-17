const DIRECTIONS: [&str; 2] = [
    "Parent-to-generic-child delivery MUST pass",
    "child-to-root delivery MUST pass",
];

pub(super) const INSTRUCTION_STARTS: &[&str] = &[
    "Parent-to-generic-child delivery MUST",
    "child-to-root delivery MUST",
];

pub(super) fn delivery(
    instructions: &[String],
    instruction_starts: &[&str],
) -> Vec<(&'static str, String)> {
    instructions
        .iter()
        .filter(|instruction| has_direction(instruction, instruction_starts, &DIRECTIONS))
        .flat_map(|instruction| split(instruction, &DIRECTIONS))
        .collect()
}

pub(super) fn has_negated(
    instructions: &[String],
    instruction_starts: &[&str],
    direction: &str,
) -> bool {
    let negated = direction.replacen(" MUST pass", " MUST NOT pass", 1);
    instructions.iter().any(|instruction| {
        instruction.starts_with(&negated)
            || instruction_starts
                .iter()
                .any(|start| instruction.starts_with(start))
                && instruction.contains(&negated)
    })
}

fn has_direction(instruction: &str, instruction_starts: &[&str], directions: &[&str]) -> bool {
    directions
        .iter()
        .any(|direction| instruction.starts_with(direction))
        || instruction_starts
            .iter()
            .any(|start| instruction.starts_with(start))
            && directions
                .iter()
                .any(|direction| instruction.contains(direction))
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
