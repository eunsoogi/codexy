pub(super) fn check(expected_issue: u64, message: &str) -> Vec<String> {
    let reference = format!("#{expected_issue}");
    if contains_issue_reference(message, &reference) {
        Vec::new()
    } else {
        vec![format!(
            "merge commit message must contain expected issue reference {reference}"
        )]
    }
}

fn contains_issue_reference(message: &str, reference: &str) -> bool {
    message
        .split(|character: char| {
            character.is_whitespace()
                || matches!(
                    character,
                    '(' | ')' | '[' | ']' | '{' | '}' | ',' | '.' | ';' | ':' | '!' | '?'
                )
        })
        .any(|token| token == reference)
}
