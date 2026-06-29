pub(super) fn is_git_status_output_after_command(lines: &[&str], index: usize) -> bool {
    index > 0
        && lines[index - 1].contains("git status --short --branch")
        && is_git_status_short_branch_line(lines[index])
        && is_followed_by_status_or_command(lines, index)
}

pub(super) fn is_git_log_graph_output_line(line: &str) -> bool {
    let Some(candidate) = line
        .trim_start_matches([' ', '*', '|', '/', '\\', '_'])
        .split_whitespace()
        .next()
    else {
        return false;
    };
    candidate.len() >= 4
        && candidate
            .chars()
            .all(|character| character.is_ascii_hexdigit())
}

fn is_git_status_short_branch_line(line: &str) -> bool {
    let Some(status) = line.strip_prefix("## ") else {
        return false;
    };
    let status = status.trim();
    if status.is_empty() {
        return false;
    }
    let status_lower = status.to_ascii_lowercase();
    if status_lower == "head (no branch)"
        || status_lower.starts_with("no commits yet on ")
        || status_lower.starts_with("initial commit on ")
    {
        return true;
    }

    let branch = status
        .split("...")
        .next()
        .unwrap_or(status)
        .split('[')
        .next()
        .unwrap_or(status)
        .trim();
    if is_likely_markdown_section_heading(status) || is_known_markdown_section_heading(status) {
        return false;
    }
    !branch.is_empty() && branch.chars().all(|character| !character.is_whitespace())
}

fn is_likely_markdown_section_heading(text: &str) -> bool {
    text.chars().any(char::is_whitespace)
        && text
            .chars()
            .all(|character| character.is_ascii_alphabetic() || character.is_ascii_whitespace())
}

fn is_known_markdown_section_heading(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    "acceptance artifacts blockers checks commands evidence findings handoff notes results review summary tests verification"
        .split_whitespace()
        .any(|heading| heading == text)
}

fn is_followed_by_status_or_command(lines: &[&str], index: usize) -> bool {
    lines
        .iter()
        .skip(index + 1)
        .find(|line| !line.trim().is_empty())
        .is_none_or(|line| line.starts_with("$ ") || is_porcelain_status_line(line))
}

fn is_porcelain_status_line(line: &str) -> bool {
    if line.len() < 3 {
        return false;
    }
    let bytes = line.as_bytes();
    if bytes.first().is_some_and(|byte| is_status_byte(*byte)) && matches!(bytes.get(1), Some(b' '))
    {
        return true;
    }
    (matches!(bytes.first(), Some(b' ')) || bytes.first().is_some_and(|byte| is_status_byte(*byte)))
        && (matches!(bytes.get(1), Some(b' '))
            || bytes.get(1).is_some_and(|byte| is_status_byte(*byte)))
        && matches!(bytes.get(2), Some(b' '))
}

fn is_status_byte(byte: u8) -> bool {
    b"MADRCUmadrcu?!".contains(&byte)
}
