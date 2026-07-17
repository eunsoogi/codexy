use super::policy::normalized_instruction;

const DIRECTIONS: [&str; 2] = [
    "parent-to-generic-child delivery must pass",
    "child-to-root delivery must pass",
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
        .flat_map(|instruction| {
            let instruction = normalized_instruction(instruction);
            let lower = instruction.to_ascii_lowercase();
            has_direction(&lower, instruction_starts, &DIRECTIONS)
                .then(|| split(&instruction, &lower, &DIRECTIONS))
                .into_iter()
                .flatten()
        })
        .collect()
}

pub(super) fn has_negated(
    instructions: &[String],
    instruction_starts: &[&str],
    direction: &str,
) -> bool {
    let negated = direction
        .to_ascii_lowercase()
        .replacen(" must pass", " must not pass", 1);
    instructions.iter().any(|instruction| {
        let instruction = normalized_instruction(instruction).to_ascii_lowercase();
        instruction.starts_with(&negated)
            || instruction_starts.iter().any(|start| {
                instruction.starts_with(&normalized_instruction(start).to_ascii_lowercase())
            }) && instruction.contains(&negated)
    })
}

fn has_direction(instruction: &str, instruction_starts: &[&str], directions: &[&str]) -> bool {
    directions
        .iter()
        .any(|direction| instruction.starts_with(direction))
        || instruction_starts.iter().any(|start| {
            instruction.starts_with(&normalized_instruction(start).to_ascii_lowercase())
        }) && directions
            .iter()
            .any(|direction| instruction.contains(direction))
}

fn split(block: &str, lower: &str, directions: &[&'static str]) -> Vec<(&'static str, String)> {
    let mut starts = directions
        .iter()
        .flat_map(|direction| {
            lower
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
