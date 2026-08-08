use super::{Block, canonical};

pub(super) fn blocks(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut headings: Vec<(usize, String)> = Vec::new();
    let mut lines = Vec::new();
    let mut fence = None;
    for line in text.lines() {
        if let Some(marker) = fence_marker(line) {
            if fence == Some(marker) {
                fence = None;
            } else if fence.is_none() {
                push_block(&mut blocks, &headings, &lines);
                lines.clear();
                fence = Some(marker);
            }
            continue;
        }
        if fence.is_some() {
            continue;
        }
        if let Some((level, heading)) = atx_heading(line) {
            push_block(&mut blocks, &headings, &lines);
            while headings.last().is_some_and(|(prior, _)| *prior >= level) {
                headings.pop();
            }
            headings.push((level, canonical(heading)));
            lines.clear();
        } else {
            lines.push(line);
        }
    }
    push_block(&mut blocks, &headings, &lines);
    blocks
}

fn fence_marker(line: &str) -> Option<char> {
    let trimmed = line.trim_start();
    (trimmed.starts_with("```") || trimmed.starts_with("~~~"))
        .then(|| trimmed.chars().next().expect("fence marker"))
}

fn atx_heading(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    let level = trimmed
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if level == 0 || level > 6 || !trimmed[level..].starts_with(char::is_whitespace) {
        return None;
    }
    Some((level, trimmed[level..].trim().trim_end_matches('#').trim()))
}

fn push_block(blocks: &mut Vec<Block>, headings: &[(usize, String)], lines: &[&str]) {
    let text = lines.join(" ");
    if !text.trim().is_empty() {
        blocks.push(Block {
            headings: headings
                .iter()
                .map(|(_, heading)| heading.clone())
                .collect(),
            text,
        });
    }
}
