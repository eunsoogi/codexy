use std::{fs, path::Path};

use anyhow::{Context as _, Result, bail};
use serde_json::{Map, Value};
use sha2::{Digest as _, Sha256};

use crate::{
    paths::display_relative,
    validation::{
        load_json,
        runtime_release_schema::{
            digest, exact, exact_keys, integer, object, object_field, string,
        },
    },
};

const PLATFORMS: [&str; 3] = ["darwin-arm64", "linux-x86_64", "windows-x86_64"];

pub(super) fn check_source(
    source: &Map<String, Value>,
    state: &str,
    repository: &str,
    legacy_commit: &str,
    path: &Path,
) -> Result<()> {
    let fields =
        if matches!(state, "candidate-proven" | "source-selected") && source.contains_key("tree") {
            &["repository", "commit", "tree"][..]
        } else {
            &["repository", "commit"][..]
        };
    exact_keys(source, fields, path)?;
    exact(
        string(source, "repository", path)?,
        repository,
        "source.repository",
        path,
    )?;
    let commit = string(source, "commit", path)?;
    lower_hex(commit, 40, "source.commit", path)?;
    if state == "legacy-public" {
        exact(commit, legacy_commit, "source.commit", path)
    } else if source.contains_key("tree") {
        lower_hex(string(source, "tree", path)?, 40, "source.tree", path)
    } else {
        Ok(())
    }
}

pub(super) fn check_source_surface(plugin_root: &Path, supported: &[String]) -> Result<()> {
    let public = ["darwin-arm64".to_owned(), "linux-x86_64".to_owned()];
    if supported == public.as_slice() {
        return Ok(());
    }
    let wrapper_path = plugin_root.join("mcp/codexy-mcp-devtools");
    let wrapper = fs::read_to_string(&wrapper_path)?;
    let actual = wrapper
        .lines()
        .find_map(|line| line.strip_prefix("bundled_platforms=\"")?.strip_suffix('"'))
        .unwrap_or_default()
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if actual != supported {
        bail!(
            "{} bundled platforms must match supportedPlatforms: expected {:?}, got {:?}",
            display_relative(&wrapper_path),
            supported,
            actual
        );
    }
    bail!(
        "{} source marketplace must retain the darwin/linux public-bootstrap platforms",
        display_relative(&plugin_root.join(".github/workflows/plugin-runtime-binaries.yml"))
    )
}

pub(crate) fn check(
    classes: &Map<String, Value>,
    source: &Map<String, Value>,
    devtools_platforms: &Map<String, Value>,
    path: &Path,
) -> Result<()> {
    exact_keys(classes, &["devtoolsMcp", "coreHandoff"], path)?;
    let devtools = object_field(classes, "devtoolsMcp", path)?;
    exact_keys(devtools, &["platforms"], path)?;
    if devtools.get("platforms") != Some(&Value::Object(devtools_platforms.clone())) {
        bail!(
            "{} devtoolsMcp class must bind release platforms",
            display_relative(path)
        );
    }
    let core = object_field(classes, "coreHandoff", path)?;
    exact_keys(core, &["manifest", "platforms"], path)?;
    let manifest = object_field(core, "manifest", path)?;
    exact_keys(manifest, &["path", "sha256"], path)?;
    exact(
        string(manifest, "path", path)?,
        "handoff-runtime.json",
        "core manifest path",
        path,
    )?;
    digest(
        string(manifest, "sha256", path)?,
        "core manifest digest",
        path,
    )?;
    let platforms = object_field(core, "platforms", path)?;
    exact_keys(platforms, &PLATFORMS, path)?;
    for platform in PLATFORMS {
        let bridge = object_field(platforms, platform, path)?;
        exact_keys(bridge, &["path", "sha256", "kind"], path)?;
        let extension = if platform == "windows-x86_64" {
            "exe"
        } else {
            "bin"
        };
        exact(
            string(bridge, "path", path)?,
            &format!("runtime/codexy-handoff-validate-{platform}.{extension}"),
            "core bridge path",
            path,
        )?;
        digest(string(bridge, "sha256", path)?, "core bridge digest", path)?;
        let kind = match platform {
            "darwin-arm64" => "mach-o",
            "linux-x86_64" => "elf",
            "windows-x86_64" => "pe",
            _ => bail!("unsupported core bridge platform: {platform}"),
        };
        exact(
            string(bridge, "kind", path)?,
            kind,
            "core bridge kind",
            path,
        )?;
    }
    lower_hex(string(source, "commit", path)?, 40, "source.commit", path)?;
    lower_hex(string(source, "tree", path)?, 40, "source.tree", path)
}

pub(super) fn check_manifest(
    plugin_root: &Path,
    release: &Map<String, Value>,
    release_path: &Path,
) -> Result<()> {
    let classes = object_field(release, "classes", release_path)?;
    let core = object_field(classes, "coreHandoff", release_path)?;
    let identity = object_field(core, "manifest", release_path)?;
    let path = plugin_root.join(string(identity, "path", release_path)?);
    let metadata = fs::symlink_metadata(&path)
        .with_context(|| format!("{} core manifest is missing", display_relative(&path)))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        bail!(
            "{} core manifest must be a regular file",
            display_relative(&path)
        );
    }
    let bytes = fs::read(&path)?;
    exact(
        &format!("{:x}", Sha256::digest(&bytes)),
        string(identity, "sha256", release_path)?,
        "core manifest digest",
        &path,
    )?;
    let document = load_json(&path)?;
    let manifest = object(&document, "core manifest", &path)?;
    exact_keys(
        manifest,
        &["schema", "version", "source", "platforms"],
        &path,
    )?;
    exact(
        string(manifest, "schema", &path)?,
        "codexy.handoff-runtime.v1",
        "schema",
        &path,
    )?;
    integer(manifest, "version", &path, 1)?;
    let source = object_field(manifest, "source", &path)?;
    exact_keys(source, &["commit", "tree"], &path)?;
    let release_source = object_field(release, "source", release_path)?;
    for field in ["commit", "tree"] {
        exact(
            string(source, field, &path)?,
            string(release_source, field, release_path)?,
            field,
            &path,
        )?;
    }
    if manifest.get("platforms") != core.get("platforms") {
        bail!(
            "{} core manifest identity differs from release",
            display_relative(&path)
        );
    }
    Ok(())
}

pub(super) fn lower_hex(value: &str, length: usize, field: &str, path: &Path) -> Result<()> {
    if value.len() != length
        || value
            .bytes()
            .any(|byte| !byte.is_ascii_hexdigit() || byte.is_ascii_uppercase())
    {
        bail!(
            "{} {field} must be lowercase {length}-character hexadecimal",
            display_relative(path)
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::lower_hex;
    use std::path::Path;

    #[test]
    fn tree_and_digest_require_lowercase_hex() {
        for (length, valid) in [(40, "a".repeat(40)), (64, "b".repeat(64))] {
            assert!(lower_hex(&valid, length, "identity", Path::new("manifest")).is_ok());
            for invalid in ["A".repeat(length), "g".repeat(length)] {
                assert!(lower_hex(&invalid, length, "identity", Path::new("manifest")).is_err());
            }
        }
    }
}
