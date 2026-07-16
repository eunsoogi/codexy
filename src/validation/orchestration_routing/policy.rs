use super::super::markdown::{Fence, fence_marker};

pub(super) fn section_for_heading(skill: &str, heading: &str) -> Option<String> {
    sections_for_heading(skill, heading).into_iter().next()
}

pub(super) fn sections_for_heading(skill: &str, heading: &str) -> Vec<String> {
    let mut sections = Vec::new();
    let mut section = None;
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
        if trimmed.is_empty() {
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
            section.push_str(section_line);
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
