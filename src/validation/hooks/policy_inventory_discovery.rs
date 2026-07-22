use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use regex::Regex;

use crate::paths::display_relative;

#[derive(Debug)]
pub(super) struct Discovered {
    pub(super) id: String,
    pub(super) digest: String,
    pub(super) source: String,
    pub(super) text: String,
}

pub(super) fn discover(plugin_root: &Path) -> Result<Vec<Discovered>> {
    let mut files = Vec::new();
    markdown_files(&plugin_root.join("skills"), &mut files)?;
    let pattern = Regex::new(r"\bMUST(?: NOT)?\b")?;
    let mut rules = Vec::new();
    for path in files {
        let relative = path
            .strip_prefix(plugin_root)?
            .to_string_lossy()
            .replace('\\', "/");
        let text = fs::read_to_string(&path)?;
        let mut ordinal = 0;
        let mut seen = BTreeMap::<String, usize>::new();
        for block in semantic_blocks(&text) {
            for _ in pattern.find_iter(&block.text) {
                ordinal += 1;
                let occurrence = seen
                    .entry(block.text.clone())
                    .and_modify(|value| *value += 1)
                    .or_insert(1);
                let digest = fnv(&format!("{relative}\0{}\0{occurrence}", block.text));
                rules.push(Discovered {
                    id: format!("norm-{digest}"),
                    digest,
                    source: format!("{relative}:{}:{ordinal}", block.line),
                    text: block.text.clone(),
                });
            }
        }
    }
    Ok(rules)
}

#[derive(Debug)]
struct MarkdownBlock {
    line: usize,
    text: String,
}

#[derive(Debug)]
enum BlockKind {
    List,
    Paragraph,
}

fn semantic_blocks(markdown: &str) -> Vec<MarkdownBlock> {
    let list = Regex::new(r"^(\s*)(?:[-+*]|[0-9]+[.)])\s+").expect("static list pattern");
    let mut blocks = Vec::new();
    let mut current: Option<(usize, BlockKind, Vec<String>)> = None;
    let mut fence: Option<(char, usize)> = None;
    let mut frontmatter = markdown
        .lines()
        .next()
        .is_some_and(|line| line.trim() == "---");
    for (index, line) in markdown.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();
        let marker = line.trim_start();
        if frontmatter {
            if line_number == 1 {
                continue;
            }
            if trimmed == "---" {
                frontmatter = false;
            } else if !trimmed.is_empty() {
                blocks.push(MarkdownBlock {
                    line: line_number,
                    text: normalize(trimmed),
                });
            }
            continue;
        }
        if let Some((character, length)) = fence {
            let closing = marker.chars().take_while(|item| *item == character).count();
            if closing >= length && marker[closing..].trim().is_empty() {
                fence = None;
            }
            continue;
        }
        let fence_character = marker.chars().next();
        let fence_length = fence_character.map_or(0, |character| {
            marker.chars().take_while(|item| *item == character).count()
        });
        if fence_length >= 3 && fence_character.is_some_and(|item| matches!(item, '`' | '~')) {
            flush(&mut current, &mut blocks);
            fence = fence_character.map(|character| (character, fence_length));
            continue;
        }
        if trimmed.is_empty() {
            flush(&mut current, &mut blocks);
            continue;
        }
        if marker.starts_with('#') && marker.trim_start_matches('#').starts_with(' ') {
            flush(&mut current, &mut blocks);
            blocks.push(MarkdownBlock {
                line: line_number,
                text: normalize(marker),
            });
            continue;
        }
        if list.is_match(line) {
            flush(&mut current, &mut blocks);
            current = Some((line_number, BlockKind::List, vec![marker.to_owned()]));
            continue;
        }
        match current.as_mut() {
            Some((_, BlockKind::List, lines)) => lines.push(trimmed.to_owned()),
            Some((_, BlockKind::Paragraph, lines)) => lines.push(trimmed.to_owned()),
            None => current = Some((line_number, BlockKind::Paragraph, vec![trimmed.to_owned()])),
        }
    }
    flush(&mut current, &mut blocks);
    blocks
}

fn flush(current: &mut Option<(usize, BlockKind, Vec<String>)>, blocks: &mut Vec<MarkdownBlock>) {
    if let Some((line, _, lines)) = current.take() {
        blocks.push(MarkdownBlock {
            line,
            text: normalize(&lines.join(" ")),
        });
    }
}

fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn fnv(value: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in value.as_bytes() {
        hash = (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn markdown_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in
        fs::read_dir(root).with_context(|| format!("reading {}", display_relative(root)))?
    {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            markdown_files(&entry.path(), files)?;
        } else if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "md")
        {
            files.push(entry.path());
        }
    }
    files.sort();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(markdown: &str) -> Result<Vec<Discovered>> {
        let fixture = tempfile::tempdir()?;
        let skills = fixture.path().join("skills/example");
        fs::create_dir_all(&skills)?;
        fs::write(skills.join("SKILL.md"), markdown)?;
        discover(fixture.path())
    }

    #[test]
    fn semantic_blocks_include_continuations_and_respect_markdown_boundaries() -> Result<()> {
        let found = rules(
            "# Heading\n\n## Agent MUST verify\n\n- MUST read every governing file from the\nfilesystem root through the target.\n- A sibling MUST remain separate.\n\nParagraph MUST preserve its complete\ncontinuation text.\n\n```text\n```still-example\nExample MUST NOT become policy.\n```\n~~~text\nSecond example MUST stay non-policy.\n~~~\n",
        )?;
        let texts = found
            .iter()
            .map(|rule| rule.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            texts,
            [
                "## Agent MUST verify",
                "- MUST read every governing file from the filesystem root through the target.",
                "- A sibling MUST remain separate.",
                "Paragraph MUST preserve its complete continuation text.",
            ]
        );
        assert_eq!(found[1].source, "skills/example/SKILL.md:5:2");
        Ok(())
    }

    #[test]
    fn cosmetic_reflow_is_stable_but_material_continuation_changes_digest() -> Result<()> {
        let original = rules("- MUST inspect the complete\n  filesystem scope.\n")?;
        let reflowed = rules("- MUST inspect the complete filesystem\n  scope.\n")?;
        let changed = rules("- MUST inspect the complete\n  repository scope.\n")?;
        assert_eq!(original[0].digest, reflowed[0].digest);
        assert_eq!(original[0].text, reflowed[0].text);
        assert_ne!(original[0].digest, changed[0].digest);
        Ok(())
    }

    #[test]
    fn blank_lines_and_nested_items_end_the_current_instruction() -> Result<()> {
        let found = rules(
            "1. MUST classify the lane:\n   - child behavior MUST stay bounded.\n   - sibling context is separate.\n\n   Detached continuation MUST form a paragraph.\n",
        )?;
        assert_eq!(found.len(), 3);
        assert_eq!(found[0].text, "1. MUST classify the lane:");
        assert_eq!(found[1].text, "- child behavior MUST stay bounded.");
        assert_eq!(
            found[2].text,
            "Detached continuation MUST form a paragraph."
        );
        Ok(())
    }
}
