use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use crate::paths::display_relative;

#[path = "merge_authorization_routes.rs"]
mod merge_authorization_routes;

const AUTHORIZATION_REFERENCE: &str = "skills/git-workflow/references/merge-authorization.md";
const GLOBAL_SURFACES: &[&str] = &[
    AUTHORIZATION_REFERENCE,
    "skills/codex-orchestration/references/classification-and-control.md",
    "skills/proof-driven-completion/SKILL.md",
];

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    merge_authorization_routes::check(plugin_root, &mut errors);
    check_global_surfaces(plugin_root, &mut errors);
    check_profile_defaults(plugin_root, &mut errors);
    errors
}

fn check_global_surfaces(root: &Path, errors: &mut Vec<String>) {
    for relative in GLOBAL_SURFACES {
        let path = root.join(relative);
        match fs::read_to_string(&path) {
            Ok(text) if is_global_rule(&prose_blocks(&text).join(" ")) => {}
            Ok(_) => errors.push(format!(
                "{} must preserve the global merge-authorization prohibition",
                display_relative(&path)
            )),
            Err(error) => errors.push(format!(
                "{} could not be read: {error}",
                display_relative(&path)
            )),
        }
    }
}

fn is_global_rule(line: &str) -> bool {
    let line = line.to_ascii_lowercase();
    let generic_is_denied = line.contains("generic finish") && line.contains("non-authoritative");
    let authority_is_required = line.contains("authoritative merge authorization")
        || (line.contains("checked contract") && line.contains("sole merge authorization"))
        || (line.contains("global invariant") && line.contains("workflow profile"));
    generic_is_denied && authority_is_required
}

fn check_profile_defaults(root: &Path, errors: &mut Vec<String>) {
    let mut paths = BTreeSet::new();
    if collect_markdown(&root.join("skills"), &mut paths).is_err() {
        errors.push("skills could not be read for merge-authorization policy".into());
        return;
    }
    for path in paths {
        if path.ends_with(AUTHORIZATION_REFERENCE) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        if prose_blocks(&text)
            .iter()
            .flat_map(|block| clauses(block))
            .any(clause_grants_merge)
        {
            errors.push(format!(
                "{} must not let a workflow profile turn gates into merge permission",
                display_relative(&path)
            ));
        }
    }
}

fn clause_grants_merge(text: &str) -> bool {
    let line = text.to_ascii_lowercase();
    let grants = [
        "authorize merge",
        "merge consent",
        "merge permission",
        "permission to merge",
        "imply merge",
    ]
    .iter()
    .any(|term| line.contains(term));
    let denied = [
        "must not",
        "must never",
        "do not",
        "cannot",
        "not authorization",
    ]
    .iter()
    .any(|term| line.contains(term));
    let gate_grant = line.contains("passing gates")
        || line.contains("green gates")
        || line.contains("gates imply");
    line.contains("merge") && grants && gate_grant && !denied
}

fn clauses(block: &str) -> Vec<&str> {
    let mut clauses = Vec::new();
    for sentence in block.split(['.', ';', '!', '?']) {
        let mut remainder = sentence;
        while let Some((prefix, suffix)) = adversative_boundary(remainder) {
            clauses.push(prefix);
            remainder = suffix;
        }
        clauses.push(remainder);
    }
    clauses
}

fn adversative_boundary(text: &str) -> Option<(&str, &str)> {
    [", but ", ", however ", ", yet ", ", although "]
        .iter()
        .filter_map(|marker| text.find(marker).map(|index| (index, *marker)))
        .min_by_key(|(index, _)| *index)
        .map(|(index, marker)| (&text[..index], &text[index + marker.len()..]))
}

fn prose_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut paragraph = Vec::new();
    let mut fence = None;
    for raw in text.lines() {
        let line = raw.trim();
        if let Some(marker) = fence_marker(line) {
            fence = if fence == Some(marker) {
                None
            } else if fence.is_none() {
                Some(marker)
            } else {
                fence
            };
            continue;
        }
        if fence.is_some() {
            continue;
        }
        if line.is_empty() || line.starts_with('#') || list_item(line) {
            flush(&mut blocks, &mut paragraph);
            if list_item(line) {
                blocks.push(strip_list_item(line).to_owned());
            }
        } else {
            paragraph.push(line);
        }
    }
    flush(&mut blocks, &mut paragraph);
    blocks
}

fn flush(blocks: &mut Vec<String>, paragraph: &mut Vec<&str>) {
    if !paragraph.is_empty() {
        blocks.push(std::mem::take(paragraph).join(" "));
    }
}

fn fence_marker(line: &str) -> Option<char> {
    line.starts_with("```")
        .then_some('`')
        .or_else(|| line.starts_with("~~~").then_some('~'))
}

fn list_item(line: &str) -> bool {
    line.starts_with("- ")
        || line.starts_with("* ")
        || line.starts_with("+ ")
        || line
            .find(|character: char| !character.is_ascii_digit())
            .is_some_and(|index| {
                matches!(line.as_bytes().get(index), Some(b'.' | b')'))
                    && line.as_bytes().get(index + 1) == Some(&b' ')
            })
}

fn strip_list_item(line: &str) -> &str {
    if line
        .get(..2)
        .is_some_and(|prefix| matches!(prefix, "- " | "* " | "+ "))
    {
        &line[2..]
    } else {
        line.find(|character: char| !character.is_ascii_digit())
            .and_then(|index| line.get(index + 2..))
            .unwrap_or(line)
    }
}

pub(super) fn collect_markdown(root: &Path, paths: &mut BTreeSet<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_markdown(&path, paths)?;
        } else if path.extension().is_some_and(|extension| extension == "md") {
            paths.insert(path);
        }
    }
    Ok(())
}
