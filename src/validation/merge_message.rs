pub(super) fn check(expected_issue: u64, message: &str) -> Vec<String> {
    if has_unique_final_closing_reference(expected_issue, message) {
        Vec::new()
    } else {
        vec![format!(
            "merge commit message must contain exactly one closing reference, and the final closing line must be exactly: Fixes #{expected_issue}"
        )]
    }
}

fn has_unique_final_closing_reference(expected_issue: u64, message: &str) -> bool {
    let expected_line = format!("Fixes #{expected_issue}");
    let non_empty_lines = message
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    let closing_lines = non_empty_lines
        .iter()
        .filter(|line| is_closing_reference_line(line))
        .collect::<Vec<_>>();
    closing_lines.len() == 1 && non_empty_lines.last() == Some(&expected_line.as_str())
}

fn is_closing_reference_line(line: &str) -> bool {
    let Some(issue) = line.strip_prefix("Fixes #") else {
        return false;
    };
    !issue.is_empty() && issue.chars().all(|character| character.is_ascii_digit())
}
