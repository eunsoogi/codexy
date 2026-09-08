use super::{Located, context};

pub(super) fn terminal_result(raw: &str) -> Result<Option<Located>, String> {
    let mut matches = Vec::new();
    let lines = context::operative_lines(raw);
    let terminal_control = context::section_membership(
        &lines,
        |_| true,
        |level, title| level == 2 && title.eq_ignore_ascii_case("terminal control"),
    );
    let terminal_handoff = context::section_membership(
        &lines,
        |_| true,
        |level, title| level == 2 && title.eq_ignore_ascii_case("terminal handoff"),
    );
    for (index, (start, _, line)) in lines.iter().enumerate() {
        let lower = line.trim().to_ascii_lowercase();
        let has_terminal_label = terminal_label(&lower, terminal_handoff[index]);
        let value = if let Some((result, offset)) =
            terminal_assignment(line, terminal_control[index])?
        {
            Some((result, *start + offset))
        } else if lower.starts_with("##") && lower.contains("blocking findings") {
            result_value(line).map(|(result, offset)| (result, *start + offset))
        } else if has_terminal_label {
            match line.split_once(':') {
                Some((prefix, value)) => result_value(value)
                    .map(|(result, offset)| (result, *start + prefix.len() + 1 + offset)),
                None => lines[index + 1..]
                    .iter()
                    .find(|(_, _, next)| !next.trim().is_empty())
                    .and_then(|(next_start, _, next)| {
                        result_value(next).map(|(result, offset)| (result, *next_start + offset))
                    }),
            }
        } else {
            None
        };
        if has_terminal_label && value.is_none() {
            return Err("markdown terminal result label is invalid".into());
        }
        if let Some((value, offset)) = value {
            let length = value.len();
            matches.push(Located {
                value,
                start: offset,
                end: offset + length,
            });
        }
    }
    unique(matches, "markdown terminal result")
}

fn terminal_assignment(
    line: &str,
    in_terminal_control: bool,
) -> Result<Option<(String, usize)>, String> {
    if !in_terminal_control {
        return Ok(None);
    }
    let leading = line.len() - line.trim_start().len();
    let mut content = &line[leading..];
    let mut content_start = leading;
    if let Some(rest) = content.strip_prefix("- ") {
        content = rest;
        content_start += 2;
    }
    if content.starts_with('`') && content.ends_with('`') && content.len() > 1 {
        content = &content[1..content.len() - 1];
        content_start += 1;
    }
    let Some(equal) = content.find('=') else {
        return Ok(None);
    };
    let label = content[..equal]
        .trim()
        .trim_matches('`')
        .trim()
        .to_ascii_lowercase()
        .replace(['-', '_'], " ");
    if !matches!(label.trim(), "terminal result" | "terminal verdict") {
        return Ok(None);
    }
    let (result, offset) =
        result_value(&content[equal + 1..]).ok_or("markdown terminal result label is invalid")?;
    Ok(Some((result, content_start + equal + 1 + offset)))
}

fn terminal_label(line: &str, in_terminal_handoff: bool) -> bool {
    let line = line.trim();
    let line = line.strip_prefix('-').map_or(line, str::trim_start);
    let line = line.trim_start_matches('#').trim_start();
    let label = line.split_once(':').map_or(line, |(label, _)| label);
    matches!(label.trim(), "terminal result" | "terminal verdict")
        || (in_terminal_handoff && label.trim() == "result")
}

fn unique(matches: Vec<Located>, label: &str) -> Result<Option<Located>, String> {
    let mut matches = matches.into_iter();
    let Some(first) = matches.next() else {
        return Ok(None);
    };
    if matches.any(|candidate| candidate.value != first.value) {
        return Err(format!("{label} is contradictory"));
    }
    Ok(Some(first))
}

pub(super) fn result_value(value: &str) -> Option<(String, usize)> {
    super::result::parse(value)
}
