use super::super::markdown::{Fence, fence_marker};

pub(super) fn section_for_heading(skill: &str, heading: &str) -> Option<String> {
    sections_for_heading(skill, heading).into_iter().next()
}

pub(super) fn sections_for_heading(skill: &str, heading: &str) -> Vec<String> {
    let mut sections = Vec::new();
    let mut section: Option<String> = None;
    let mut fence: Option<Fence> = None;
    let mut in_comment = false;
    for line in skill.lines() {
        let mut section_line = line;
        if line.starts_with("    ") || line.starts_with('\t') {
            continue;
        }
        let mut trimmed = line.trim_start();
        if let Some(marker) = fence {
            if marker.closes(trimmed) {
                fence = None;
            }
            continue;
        }
        if in_comment {
            let Some((_, after)) = trimmed.split_once("-->") else {
                continue;
            };
            in_comment = false;
            trimmed = after.trim_start();
            section_line = trimmed;
        }
        if let Some(comment) = trimmed.strip_prefix("<!--") {
            let Some((_, after)) = comment.split_once("-->") else {
                in_comment = true;
                continue;
            };
            trimmed = after.trim_start();
            section_line = trimmed;
        }
        let active_line = strip_inline_comments(section_line);
        trimmed = active_line.trim_start();
        if trimmed.is_empty() {
            if let Some(section) = &mut section {
                section.push('\n');
            }
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
    if let Some(section) = section {
        sections.push(section);
    }
    sections
}

pub(super) fn policy_bullets(section: &str) -> Vec<String> {
    let mut bullets = Vec::new();
    let mut continues = false;
    for line in section.lines() {
        let trimmed = line.trim();
        if let Some(bullet) = trimmed.strip_prefix("- ") {
            bullets.push(bullet.to_owned());
            continues = true;
        } else if trimmed.starts_with('#') || trimmed.is_empty() {
            continues = false;
        } else if continues && (line.starts_with(' ') || line.starts_with('\t')) {
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

pub(super) fn recipient_policy_instructions(section: &str) -> Vec<String> {
    let mut instructions = policy_bullets(section);
    instructions.extend(section.lines().filter_map(|line| {
        let trimmed = line.trim();
        (!line.starts_with(' ') && !line.starts_with('\t') && !trimmed.starts_with("- "))
            .then(|| policy_line(trimmed))
            .flatten()
            .filter(|instruction| !instruction.is_empty() && !instruction.starts_with('#'))
            .map(str::to_owned)
    }));
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
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            finish_block(&mut blocks, &mut block);
        } else if policy_line(trimmed).is_some_and(|line| {
            DIRECTIONS
                .iter()
                .any(|direction| line.starts_with(direction))
        }) {
            finish_block(&mut blocks, &mut block);
            block = policy_line(trimmed).map(str::to_owned);
        } else if let Some(current) = &mut block {
            if line.starts_with(' ') || line.starts_with('\t') {
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
        .flat_map(|block| split_assignments(&block, &DIRECTIONS))
        .collect()
}

pub(super) fn has_negated_delivery_assignment(section: &str, direction: &str) -> bool {
    let negated = direction.replacen(" MUST pass", " MUST NOT pass", 1);
    section
        .lines()
        .any(|line| policy_line(line.trim()).is_some_and(|line| line.starts_with(&negated)))
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

fn strip_inline_comments(line: &str) -> String {
    let mut active = String::new();
    let mut remainder = line;
    while let Some((before, commented)) = remainder.split_once("<!--") {
        active.push_str(before);
        let Some((_, after)) = commented.split_once("-->") else {
            return active;
        };
        remainder = after;
    }
    active.push_str(remainder);
    active
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
                .and_then(|rest| rest.strip_prefix(". "))
                .filter(|_| digits > 0)
        })
        .or(Some(line))
}

fn split_assignments(block: &str, directions: &[&'static str]) -> Vec<(&'static str, String)> {
    let mut starts = directions
        .iter()
        .flat_map(|direction| {
            block
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
