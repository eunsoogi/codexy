use super::policy::normalized_instruction;

pub(super) const ROUTES: [(&str, &str, &str, &str, &str); 2] = [
    (
        "Captured #433 parent-to-generic-child evidence",
        "gpt-5.6-terra",
        "gpt-5.6-sol",
        "child-433",
        "parent-to-generic-child",
    ),
    (
        "Reverse child-to-root evidence",
        "gpt-5.6-sol",
        "gpt-5.6-terra",
        "root-433",
        "child-to-root",
    ),
];

pub(super) fn invalid(
    bullets: &[String],
    instruction_starts: &[&str],
    marker: &str,
    recipient: &str,
    sender: &str,
    thread: &str,
) -> bool {
    let evidence = bullets
        .iter()
        .filter(|bullet| is_active_instruction(bullet, instruction_starts))
        .flat_map(|bullet| records(bullet, marker))
        .collect::<Vec<_>>();
    evidence.is_empty()
        || evidence
            .into_iter()
            .any(|bullet| !valid(bullet, marker, recipient, sender, thread))
}

fn is_active_instruction(instruction: &str, starts: &[&str]) -> bool {
    let instruction = normalized_instruction(instruction).to_ascii_lowercase();
    starts
        .iter()
        .any(|start| instruction.starts_with(&normalized_instruction(start).to_ascii_lowercase()))
}

fn records<'a>(instruction: &'a str, marker: &str) -> Vec<&'a str> {
    instruction
        .match_indices(marker)
        .map(|(start, _)| {
            let after_marker = start + marker.len();
            let end = ROUTES
                .iter()
                .filter_map(|(next, ..)| {
                    instruction[after_marker..]
                        .find(next)
                        .map(|index| after_marker + index)
                })
                .min()
                .unwrap_or(instruction.len());
            instruction[start..end].trim_end()
        })
        .collect()
}

fn valid(bullet: &str, marker: &str, recipient: &str, sender: &str, thread: &str) -> bool {
    let Some(metadata) = bullet
        .strip_prefix(marker)
        .and_then(|rest| rest.strip_prefix(':'))
    else {
        return false;
    };
    let Some((metadata, call)) = metadata.split_once("send_message_to_thread({") else {
        return false;
    };
    let Some((arguments, suffix)) = call.split_once("})") else {
        return false;
    };
    suffix.trim() == "."
        && fields(metadata, ';', '=').as_deref()
            == Some(&[
                ("configured_ui_model", recipient),
                ("actual_turn_context_model", sender),
                ("per_message_model", recipient),
            ])
        && fields(arguments, ',', ':').as_deref()
            == Some(&[
                ("threadId", thread),
                ("model", recipient),
                ("thinking", "high"),
            ])
}

fn fields(text: &str, separator: char, assignment: char) -> Option<Vec<(&str, &str)>> {
    text.split(separator)
        .filter(|part| !part.trim().is_empty())
        .map(|part| {
            let (name, value) = part.trim().split_once(assignment)?;
            Some((
                name.trim(),
                value.trim().strip_prefix('"')?.strip_suffix('"')?,
            ))
        })
        .collect()
}
