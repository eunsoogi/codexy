pub(super) fn invalid(
    bullets: &[String],
    marker: &str,
    recipient: &str,
    sender: &str,
    thread: &str,
) -> bool {
    let evidence = bullets
        .iter()
        .filter(|bullet| bullet.starts_with(marker))
        .collect::<Vec<_>>();
    evidence.is_empty()
        || evidence
            .into_iter()
            .any(|bullet| !valid(bullet, marker, recipient, sender, thread))
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
