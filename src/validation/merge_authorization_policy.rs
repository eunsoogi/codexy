use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use crate::paths::display_relative;

const AUTHORIZATION_REFERENCE: &str = "skills/git-workflow/references/merge-authorization.md";
const GLOBAL_SURFACES: &[&str] = &[
    AUTHORIZATION_REFERENCE,
    "skills/codex-orchestration/references/classification-and-control.md",
    "skills/proof-driven-completion/SKILL.md",
];

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    check_merge_routes(plugin_root, &mut errors);
    check_global_surfaces(plugin_root, &mut errors);
    check_profile_defaults(plugin_root, &mut errors);
    errors
}

fn check_merge_routes(root: &Path, errors: &mut Vec<String>) {
    let mut paths = BTreeSet::new();
    if collect_markdown(&root.join("skills"), &mut paths).is_err() {
        errors.push("skills could not be read for merge-authorization policy".into());
        return;
    }
    for path in paths {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for block in command_blocks(&text) {
            let mut validated = false;
            for line in block {
                let line = line.trim_start();
                validated |=
                    line.starts_with("if ! plugins/codexy/hooks/codexy-merge-admission-check.sh");
                if merge_command(line) && !validated {
                    errors.push(format!(
                        "{} must validate authoritative merge authorization before mutation",
                        display_relative(&path)
                    ));
                    break;
                }
            }
        }
    }
}

fn merge_command(line: &str) -> bool {
    line.starts_with("gh pr merge") || line.starts_with("if ! gh pr merge")
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
    ["generic", "authorization", "merge"]
        .iter()
        .all(|term| line.contains(term))
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

fn clauses(block: &str) -> impl Iterator<Item = &str> {
    block.split(['.', ';', '!', '?'])
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

fn command_blocks(text: &str) -> Vec<Vec<String>> {
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    let mut fence = None;
    for raw in text.lines() {
        let line = raw.trim();
        if let Some(marker) = fence_marker(line) {
            if fence == Some(marker) {
                blocks.push(std::mem::take(&mut current));
                fence = None;
            } else if fence.is_none() {
                fence = Some(marker);
            }
        } else if fence.is_some()
            && !line.starts_with('#')
            && !line.starts_with("echo")
            && !line.starts_with("printf")
        {
            current.push(line.to_owned());
        }
    }
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

fn collect_markdown(root: &Path, paths: &mut BTreeSet<PathBuf>) -> std::io::Result<()> {
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
