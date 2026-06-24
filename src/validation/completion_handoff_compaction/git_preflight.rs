pub(super) fn has_git_graph_log_preflight(text: &str) -> bool {
    let lines: Vec<_> = text.lines().map(str::trim).collect();
    lines.iter().enumerate().any(|(index, line)| {
        is_git_preflight_line(line)
            && !is_unchecked_checklist_item(line)
            && has_positive_evidence(line)
            && {
                let block = git_preflight_evidence_block(&lines, index);
                has_all_commands(&block) && !has_negated_evidence(&block)
            }
    })
}

fn git_preflight_evidence_block(lines: &[&str], start: usize) -> String {
    let mut block = String::new();
    for (index, line) in lines.iter().enumerate().skip(start) {
        if index > start
            && starts_handoff_section(line)
            && !is_git_status_output_after_command(lines, index)
        {
            break;
        }
        if is_unchecked_checklist_item(line) {
            continue;
        }
        block.push_str(line);
        block.push('\n');
        if has_all_commands(&block) {
            break;
        }
    }
    block
}

fn starts_handoff_section(line: &str) -> bool {
    if line.trim_start().starts_with('#') {
        return true;
    }
    if starts_unrelated_list_section(line) {
        return true;
    }

    let line = metadata_line(line);
    let starts_known_section = [
        "codexy orchestration contract",
        "duplicate/no-active-work state",
        "parent/child ownership boundary",
        "stop condition",
        "authoritative stop condition",
        "next action",
    ]
    .iter()
    .any(|section| line.starts_with(section));

    starts_known_section || starts_unbulleted_section_label(line)
}

fn is_git_status_output_after_command(lines: &[&str], index: usize) -> bool {
    index > 0
        && lines[index - 1].contains("git status --short --branch")
        && is_git_status_short_branch_line(lines[index])
        && is_followed_by_status_or_command(lines, index)
}

fn is_git_status_short_branch_line(line: &str) -> bool {
    let Some(status) = line.strip_prefix("## ") else {
        return false;
    };
    let status = status.trim();
    if status.is_empty() {
        return false;
    }
    if status == "head (no branch)"
        || status.starts_with("no commits yet on ")
        || status.starts_with("initial commit on ")
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
    !branch.is_empty() && branch.chars().all(|character| !character.is_whitespace())
}

fn is_followed_by_status_or_command(lines: &[&str], index: usize) -> bool {
    lines
        .get(index + 1)
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
    matches!(
        byte,
        b'M' | b'A'
            | b'D'
            | b'R'
            | b'C'
            | b'U'
            | b'm'
            | b'a'
            | b'd'
            | b'r'
            | b'c'
            | b'u'
            | b'?'
            | b'!'
    )
}

fn starts_unrelated_list_section(line: &str) -> bool {
    let line = line.trim();
    if !line.starts_with(['-', '*']) {
        return false;
    }

    let line = metadata_line(line);
    if is_git_preflight_line(line) || starts_with_preflight_command(line) {
        return false;
    }
    line.contains(':') || is_plain_list_section_heading(line)
}

fn is_plain_list_section_heading(line: &str) -> bool {
    !line.is_empty()
        && line
            .chars()
            .all(|character| character.is_ascii_alphabetic() || character.is_ascii_whitespace())
}

fn starts_unbulleted_section_label(line: &str) -> bool {
    let line = line.trim();
    let Some((label, _)) = line.split_once(':') else {
        return false;
    };
    let label = label.trim();
    !label.is_empty()
        && !line.starts_with(['-', '*'])
        && !is_git_preflight_line(line)
        && !starts_with_preflight_command(line)
        && label
            .chars()
            .all(|character| character.is_ascii_alphabetic() || character.is_ascii_whitespace())
}

fn is_unchecked_checklist_item(line: &str) -> bool {
    line.trim()
        .trim_start_matches(['-', '*'])
        .trim_start()
        .starts_with("[ ]")
}

fn starts_with_preflight_command(line: &str) -> bool {
    [
        "pwd",
        "git status --short --branch",
        "git rev-parse head",
        "git rev-parse origin/main",
        "git log --graph",
    ]
    .iter()
    .any(|phrase| line.starts_with(phrase))
}

fn metadata_line(line: &str) -> &str {
    let line = line.trim().trim_start_matches(['-', '*']).trim_start();
    let line = line
        .strip_prefix("[x]")
        .or_else(|| line.strip_prefix("[X]"))
        .or_else(|| line.strip_prefix("[ ]"))
        .unwrap_or(line)
        .trim_start();
    line.trim_start_matches('#').trim_start()
}

fn has_all_commands(text: &str) -> bool {
    [
        "pwd",
        "git status --short --branch",
        "git rev-parse head",
        "git rev-parse origin/main",
        "git log --graph",
    ]
    .iter()
    .all(|phrase| text.contains(phrase))
}

fn is_git_preflight_line(line: &str) -> bool {
    line.contains("git graph/log preflight") || line.contains("git preflight")
}

fn has_positive_evidence(line: &str) -> bool {
    has_any(
        line,
        &["captured", "ran", "were run", "checked", "recorded"],
    )
}

fn has_negated_evidence(line: &str) -> bool {
    has_any(
        line,
        &[
            "did not run",
            "didn't run",
            "not run",
            "not captured",
            "not checked",
            "without running",
        ],
    )
}

fn has_any(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| text.contains(phrase))
}
