use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use crate::paths::display_relative;

const MERGE_REFERENCE: &str = "skills/git-workflow/references/merge-and-main-sync.md";
const AUTHORIZATION_REFERENCE: &str = "skills/git-workflow/references/merge-authorization.md";
const GLOBAL_SURFACES: &[&str] = &[
    AUTHORIZATION_REFERENCE,
    "skills/codex-orchestration/references/classification-and-control.md",
    "skills/proof-driven-completion/SKILL.md",
];

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    check_merge_route(plugin_root, &mut errors);
    check_global_surfaces(plugin_root, &mut errors);
    check_profile_defaults(plugin_root, &mut errors);
    errors
}

fn check_merge_route(root: &Path, errors: &mut Vec<String>) {
    let path = root.join(MERGE_REFERENCE);
    let Ok(text) = fs::read_to_string(&path) else {
        errors.push(format!("{} could not be read", display_relative(&path)));
        return;
    };
    let lines = route_lines(&text);
    let validated = lines.iter().position(|line| {
        line.trim_start()
            .starts_with("if ! scripts/validate-plugin-config --check-merge-authorization")
    });
    let merged = lines.iter().position(|line| line.contains("gh pr merge"));
    if !matches!((validated, merged), (Some(before), Some(after)) if before < after) {
        errors.push(format!(
            "{} must validate authoritative merge authorization before mutation",
            display_relative(&path)
        ));
    }
}

fn check_global_surfaces(root: &Path, errors: &mut Vec<String>) {
    for relative in GLOBAL_SURFACES {
        let path = root.join(relative);
        match fs::read_to_string(&path) {
            Ok(text) if is_global_rule(&active_lines(&text).join(" ")) => {}
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
        if markdown_blocks(&text)
            .iter()
            .any(|fragment| permits_profile_default(fragment))
        {
            errors.push(format!(
                "{} must not let a workflow profile turn gates into merge permission",
                display_relative(&path)
            ));
        }
    }
}

fn permits_profile_default(text: &str) -> bool {
    text.split(['.', ';']).any(clause_grants_merge)
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

fn active_lines(text: &str) -> Vec<String> {
    markdown_blocks(text)
        .into_iter()
        .flat_map(|fragment| {
            fragment
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn markdown_blocks(text: &str) -> Vec<String> {
    let mut fragments = Vec::new();
    let mut paragraph = Vec::new();
    let mut fence = None;
    text.lines().for_each(|line| {
        let line = line.trim();
        if matches!(line.get(..3), Some("```") | Some("~~~")) {
            if fence == line.get(..3) {
                fence = None;
            } else if fence.is_none() {
                fence = line.get(..3);
            }
        } else if fence.is_none()
            && (line.is_empty() || line.starts_with('#') || numbered_rule(line))
        {
            if !paragraph.is_empty() {
                fragments.push(paragraph.join(" "));
                paragraph.clear();
            }
            if numbered_rule(line) {
                fragments.push(line.to_owned());
            }
        } else if fence.is_none() {
            paragraph.push(line.to_owned());
        }
    });
    if !paragraph.is_empty() {
        fragments.push(paragraph.join(" "));
    }
    fragments
}

fn numbered_rule(line: &str) -> bool {
    line.split_once('.').is_some_and(|(number, _)| {
        !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn route_lines(text: &str) -> Vec<String> {
    let mut fence = None;
    let mut shell = false;
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if matches!(line.get(..3), Some("```") | Some("~~~")) {
                if fence == line.get(..3) {
                    fence = None;
                    shell = false;
                } else if fence.is_none() {
                    fence = line.get(..3);
                    shell = line == "```bash" || line == "```sh";
                }
                return None;
            }
            ((fence.is_none() || shell)
                && !line.starts_with('#')
                && !line.starts_with("echo")
                && !line.starts_with("printf"))
            .then(|| line.to_owned())
        })
        .collect()
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
