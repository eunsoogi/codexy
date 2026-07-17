use super::super::markdown::{Fence, fence_marker};

pub(super) use super::assignments::affirmative_field_values;

pub(super) fn sections_for_heading(skill: &str, heading: &str) -> Vec<String> {
    let mut sections = Vec::new();
    let (mut section, mut fence) = (None::<String>, None::<Fence>);
    let section_level = heading_level(heading).expect("validated heading");
    let mut in_comment = false;
    let (mut list_context, mut nested_list_indent) = (false, None::<usize>);
    for line in skill.lines() {
        let mut trimmed = line.trim_start_matches(' ');
        if let Some(marker) = fence {
            if has_active_content(line, leading_ascii_spaces(line)) && marker.closes(trimmed) {
                fence = None;
            }
            continue;
        }
        let indentation = leading_ascii_spaces(line);
        let nested_marker = indentation >= 4
            && (list_context || nested_list_indent.is_some())
            && has_list_marker(trimmed);
        let nested_continuation = indentation >= 4
            && nested_list_indent.is_some_and(|list_indent| indentation >= list_indent);
        if line.starts_with('\t') || (indentation >= 4 && !nested_marker && !nested_continuation) {
            if in_comment && line.contains("-->") {
                in_comment = false;
            }
            (list_context, nested_list_indent) = (false, None);
            continue;
        }
        let active_line = strip_comments(line, &mut in_comment);
        let indentation = leading_ascii_spaces(&active_line);
        trimmed = active_line.trim_start_matches(' ');
        let logical_line = if nested_marker {
            nested_list_indent = policy_line(trimmed)
                .map(|instruction| indentation + trimmed.len() - instruction.len());
            trimmed
        } else if nested_continuation {
            &active_line[indentation - 2..]
        } else {
            active_line.as_str()
        };
        trimmed = logical_line.trim_start_matches(' ');
        let structural = trimmed.trim_end();
        if trimmed.is_empty() {
            if let Some(section) = &mut section {
                section.push('\n');
            }
            continue;
        }
        if !has_active_content(logical_line, leading_ascii_spaces(logical_line)) {
            continue;
        }
        if let Some(marker) = fence_marker(trimmed) {
            fence = Some(marker);
            continue;
        }
        if heading_matches(structural, heading) {
            if let Some(section) = section.take() {
                sections.push(section);
            }
            section = Some(String::new());
            (list_context, nested_list_indent) = (false, None);
            continue;
        }
        if section.is_some()
            && heading_level(structural).is_some_and(|level| level <= section_level)
        {
            sections.push(section.take().expect("active section"));
            (list_context, nested_list_indent) = (false, None);
            continue;
        }
        if let Some(section) = &mut section {
            section.push_str(logical_line);
            section.push('\n');
        }
        list_context = has_list_marker(trimmed);
        if indentation < 4 {
            nested_list_indent = None;
        }
    }
    sections.extend(section);
    sections
}

pub(super) fn policy_instructions(section: &str, starts: &[&str]) -> Vec<String> {
    let mut instructions = Vec::new();
    let mut current = None;
    for line in section.lines() {
        let indentation = leading_ascii_spaces(line);
        let trimmed = line.trim_start_matches(' ').trim_end();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            finish_block(&mut instructions, &mut current);
            continue;
        }
        let instruction = policy_line(trimmed).filter(|instruction| {
            has_list_marker(trimmed)
                || starts.iter().any(|start| {
                    normalized_instruction(instruction)
                        .to_ascii_lowercase()
                        .starts_with(&normalized_instruction(start).to_ascii_lowercase())
                })
        });
        if has_active_content(line, indentation) && instruction.is_some() {
            finish_block(&mut instructions, &mut current);
            current = instruction.map(str::to_owned);
        } else if has_active_content(line, indentation) {
            if let Some(current) = &mut current {
                current.push(' ');
                current.push_str(trimmed);
            }
        } else {
            finish_block(&mut instructions, &mut current);
        }
    }
    finish_block(&mut instructions, &mut current);
    instructions
}

fn heading_level(line: &str) -> Option<usize> {
    let level = line.bytes().take_while(|byte| *byte == b'#').count();
    (level > 0
        && line[level..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace))
    .then_some(level)
}

fn heading_without_closing_hashes(line: &str) -> &str {
    let without_hashes = line.trim_end_matches('#');
    let separator = without_hashes.chars().next_back();
    if without_hashes.len() < line.len() && separator.is_some_and(char::is_whitespace) {
        without_hashes.trim_end()
    } else {
        line
    }
}

fn heading_matches(line: &str, expected: &str) -> bool {
    let Some(level) = heading_level(line) else {
        return false;
    };
    heading_level(expected) == Some(level)
        && heading_without_closing_hashes(line)[level..].trim_start()
            == expected[level..].trim_start()
}

pub(super) fn normalized_instruction(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_comments(line: &str, in_comment: &mut bool) -> String {
    let mut active = String::new();
    let mut remainder = line;
    loop {
        if *in_comment {
            let Some((_, after)) = remainder.split_once("-->") else {
                return active;
            };
            *in_comment = false;
            remainder = after;
        } else if let Some((before, after)) = remainder.split_once("<!--") {
            active.push_str(before);
            *in_comment = true;
            remainder = after;
        } else {
            active.push_str(remainder);
            return active;
        }
    }
}

fn leading_ascii_spaces(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}

fn has_active_content(line: &str, indentation: usize) -> bool {
    indentation < 4
        && line[indentation..]
            .chars()
            .next()
            .is_some_and(|character| !character.is_whitespace())
}

fn finish_block(blocks: &mut Vec<String>, block: &mut Option<String>) {
    blocks.extend(block.take());
}

fn policy_line(line: &str) -> Option<&str> {
    list_item(line)
        .map(strip_task_marker)
        .or((!line.starts_with('#')).then_some(line))
}

fn has_list_marker(line: &str) -> bool {
    list_item(line).is_some()
}

fn list_item(line: &str) -> Option<&str> {
    ["-", "*", "+"]
        .iter()
        .find_map(|marker| line.strip_prefix(marker).and_then(list_separator))
        .or_else(|| {
            let digits = line.chars().take_while(char::is_ascii_digit).count();
            line.get(digits..)
                .and_then(|rest| rest.strip_prefix('.').or_else(|| rest.strip_prefix(')')))
                .and_then(list_separator)
                .filter(|_| digits > 0)
        })
}

fn list_separator(rest: &str) -> Option<&str> {
    rest.strip_prefix(' ').or_else(|| rest.strip_prefix('\t'))
}

fn strip_task_marker(line: &str) -> &str {
    let content = line.trim_start_matches([' ', '\t']);
    let Some(rest) = content.strip_prefix('[') else {
        return line;
    };
    let Some(marker) = rest.chars().next() else {
        return line;
    };
    let Some(after) = rest[marker.len_utf8()..].strip_prefix(']') else {
        return line;
    };
    (matches!(marker, 'x' | 'X') || marker.is_whitespace())
        .then(|| after.trim_start_matches([' ', '\t']))
        .unwrap_or(line)
}
