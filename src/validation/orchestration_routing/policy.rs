use super::super::markdown::{Fence, fence_marker};
use super::assignments;

pub(super) fn section_for_heading(skill: &str, heading: &str) -> Option<String> {
    sections_for_heading(skill, heading).into_iter().next()
}

pub(super) fn sections_for_heading(skill: &str, heading: &str) -> Vec<String> {
    let mut sections = Vec::new();
    let (mut section, mut fence) = (None::<String>, None::<Fence>);
    let mut in_comment = false;
    for line in skill.lines() {
        if line.starts_with("    ") || line.starts_with('\t') {
            continue;
        }
        let mut trimmed = line.trim_start_matches(' ');
        if let Some(marker) = fence {
            if has_active_content(line, leading_ascii_spaces(line)) && marker.closes(trimmed) {
                fence = None;
            }
            continue;
        }
        let active_line = strip_comments(line, &mut in_comment);
        trimmed = active_line.trim_start_matches(' ');
        if trimmed.is_empty() {
            if let Some(section) = &mut section {
                section.push('\n');
            }
            continue;
        }
        if !has_active_content(&active_line, leading_ascii_spaces(&active_line)) {
            continue;
        }
        if let Some(marker) = fence_marker(trimmed) {
            fence = Some(marker);
            continue;
        }
        if trimmed == heading {
            if let Some(section) = section.take() {
                sections.push(section);
            }
            section = Some(String::new());
            continue;
        }
        if section.is_some() && trimmed.starts_with("## ") {
            sections.push(section.take().expect("active section"));
            continue;
        }
        if let Some(section) = &mut section {
            section.push_str(&active_line);
            section.push('\n');
        }
    }
    sections.extend(section);
    sections
}

pub(super) fn policy_bullets(section: &str) -> Vec<String> {
    let mut bullets = Vec::new();
    let mut continues = false;
    for line in section.lines() {
        let indentation = leading_ascii_spaces(line);
        let trimmed = line.trim_start_matches(' ').trim_end();
        if has_active_content(line, indentation) {
            if let Some(bullet) = trimmed.strip_prefix("- ") {
                bullets.push(bullet.to_owned());
                continues = true;
                continue;
            }
        }
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continues = false;
        } else if continues && has_active_content(line, indentation) && indentation > 0 {
            if let Some(bullet) = bullets.last_mut() {
                bullet.push(' ');
                bullet.push_str(trimmed);
            }
        } else {
            continues = false;
        }
    }
    bullets
}

pub(super) fn recipient_policy_instructions(section: &str, starts: &[&str]) -> Vec<String> {
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
            trimmed.starts_with("- ") || starts.iter().any(|start| instruction.starts_with(start))
        });
        if has_active_content(line, indentation) && instruction.is_some() {
            finish_block(&mut instructions, &mut current);
            current = instruction.map(str::to_owned);
        } else if has_active_content(line, indentation) && indentation > 0 {
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

pub(super) fn delivery_assignments(section: &str) -> Vec<(&'static str, String)> {
    const DIRECTIONS: [&str; 2] = [
        "Parent-to-generic-child delivery MUST pass",
        "child-to-root delivery MUST pass",
    ];
    let mut blocks = Vec::new();
    let mut block = None;
    for line in section.lines() {
        let indentation = leading_ascii_spaces(line);
        let trimmed = line.trim_start_matches(' ').trim_end();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            finish_block(&mut blocks, &mut block);
        } else if has_active_content(line, indentation)
            && policy_line(trimmed).is_some_and(|line| {
                DIRECTIONS
                    .iter()
                    .any(|direction| line.starts_with(direction))
            })
        {
            finish_block(&mut blocks, &mut block);
            block = policy_line(trimmed).map(str::to_owned);
        } else if let Some(current) = &mut block {
            if has_active_content(line, indentation) && indentation > 0 {
                current.push(' ');
                current.push_str(trimmed);
            } else {
                finish_block(&mut blocks, &mut block);
            }
        }
    }
    finish_block(&mut blocks, &mut block);
    blocks
        .into_iter()
        .flat_map(|block| assignments::split(&block, &DIRECTIONS))
        .collect()
}

pub(super) fn has_negated_delivery_assignment(section: &str, direction: &str) -> bool {
    let negated = direction.replacen(" MUST pass", " MUST NOT pass", 1);
    section.lines().any(|line| {
        has_active_content(line, leading_ascii_spaces(line))
            && policy_line(line.trim_start_matches(' ').trim_end())
                .is_some_and(|line| line.contains(&negated))
    })
}

pub(super) fn affirmative_field_values<'a>(assignment: &'a str, field: &str) -> Vec<&'a str> {
    let marker = format!("{field}: \"");
    assignment
        .match_indices(&marker)
        .filter(|(start, _)| assignment[..*start].ends_with('`'))
        .filter(|(start, _)| !inside_html_comment(assignment, *start))
        .filter_map(|(start, _)| {
            let before = &assignment[..start];
            let clause_start = before
                .rfind(';')
                .map_or(0, |index| index + 1)
                .max(sentence_start(before));
            let clause = &before[clause_start..];
            (!clause.contains("MUST NOT")).then(|| {
                let value = &assignment[start + marker.len()..];
                value.split_once('"').map_or(value, |(value, _)| value)
            })
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
    if let Some(block) = block.take() {
        blocks.push(block);
    }
}

fn policy_line(line: &str) -> Option<&str> {
    line.strip_prefix("- ")
        .or_else(|| {
            let digits = line.chars().take_while(char::is_ascii_digit).count();
            line.get(digits..)
                .and_then(|rest| rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")))
                .filter(|_| digits > 0)
        })
        .or(Some(line))
}
