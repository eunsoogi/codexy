use std::fs;

use anyhow::{Context as _, Result, bail};

use super::{mutation::Update, repo_path, require_semver};

pub(super) const VERSION: &str = "1.4.0";
pub(super) const CANDIDATE_VERSION: &str = "1.5.0";

const PATH: &str = "packages/codexy-runtime/src/version/bootstrap.rs";

pub(super) fn selected_version() -> Result<String> {
    let text = fs::read_to_string(repo_path(PATH)?)?;
    one_version(&text, "VERSION")
}

pub(super) fn candidate_version() -> Result<String> {
    let text = fs::read_to_string(repo_path(PATH)?)?;
    one_version(&text, "CANDIDATE_VERSION")
}

pub(super) fn prepare_candidate_version(version: &str) -> Result<Update> {
    require_semver(version)?;
    let path = repo_path(PATH)?;
    let text = fs::read_to_string(&path)?;
    let declaration = declaration(&text, "CANDIDATE_VERSION")?;
    let mut updated = text;
    updated.replace_range(declaration.start..declaration.end, version);
    Ok(Update::bytes(path, updated.into_bytes()))
}

fn one_version(text: &str, name: &str) -> Result<String> {
    Ok(declaration(text, name)?.version)
}

struct Declaration {
    version: String,
    start: usize,
    end: usize,
}

fn declaration(text: &str, name: &str) -> Result<Declaration> {
    let prefix = format!("pub(super) const {name}: &str = \"");
    let marker_count = text.matches(&prefix).count();
    let mut declarations = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let without_newline = line.strip_suffix('\n').unwrap_or(line);
        let body = without_newline
            .strip_suffix('\r')
            .unwrap_or(without_newline);
        if let Some(value) = body.strip_prefix(&prefix) {
            let version = value
                .strip_suffix("\";")
                .context("bootstrap version declaration is malformed")?;
            require_semver(version)?;
            let start = offset + prefix.len();
            declarations.push(Declaration {
                version: version.to_owned(),
                start,
                end: start + version.len(),
            });
        }
        offset += line.len();
    }
    if marker_count != 1 || declarations.len() != 1 {
        bail!("{name} must contain exactly one semantic version")
    }
    declarations
        .pop()
        .context("bootstrap version declaration was not found")
}
