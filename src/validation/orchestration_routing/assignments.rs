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

pub(super) fn affirmative_field_values<'a>(assignment: &'a str, field: &str) -> Vec<&'a str> {
    assignment
        .match_indices(field)
        .filter(|(start, _)| {
            assignment[..*start]
                .chars()
                .next_back()
                .is_none_or(|character| {
                    !character.is_ascii_alphanumeric() && !matches!(character, '_' | '-' | '.')
                })
        })
        .filter_map(|(start, _)| {
            let value = assignment[start + field.len()..]
                .trim_start_matches(char::is_whitespace)
                .strip_prefix(':')?
                .trim_start_matches(char::is_whitespace)
                .strip_prefix('"')?;
            Some((start, value))
        })
        .filter(|(start, _)| !inside_html_comment(assignment, *start))
        .filter_map(|(start, value)| {
            let before = &assignment[..start];
            let clause_start = before
                .rfind(';')
                .map_or(0, |index| index + 1)
                .max(sentence_start(before));
            (!before[clause_start..].contains("MUST NOT"))
                .then(|| value.split_once('"').map_or(value, |(value, _)| value))
        })
        .collect()
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

fn inside_html_comment(text: &str, index: usize) -> bool {
    text[..index]
        .rfind("<!--")
        .is_some_and(|open| text[..index].rfind("-->").is_none_or(|close| close < open))
}

fn sentence_start(text: &str) -> usize {
    text.match_indices(". ").fold(0, |last, (index, _)| {
        let abbreviation = text[..index].ends_with("e.g") || text[..index].ends_with("i.e");
        if abbreviation {
            last
        } else {
            text[index + 2..]
                .chars()
                .next()
                .filter(|character| character.is_uppercase())
                .map_or(last, |_| index + 2)
        }
    })
}
