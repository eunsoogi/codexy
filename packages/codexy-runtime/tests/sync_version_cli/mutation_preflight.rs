use std::{fs, process::Command};

use super::{
    archive_repository, shared_repository_archive,
    isolation::version_surface_contents,
    strict_manifest::select_version_advance,
};
use super::isolation::{fixture_version, next_patch_version};

const COMPONENT_MANIFEST: &str = "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json";

#[test]
fn markerless_version_mutation_rejects_strict_component_manifest_inputs_without_writes()
-> Result<(), Box<dyn std::error::Error>> {
    let (version, version_needle) = {
        let text = fs::read_to_string(
            codexy_runtime::paths::repository_root().join(COMPONENT_MANIFEST),
        )?;
        let manifest: serde_json::Value = serde_json::from_str(&text)?;
        let version = manifest["components"]
            .as_array()
            .and_then(|components| components.first())
            .and_then(|component| component["version"].as_str())
            .ok_or("component manifest version")?;
        (version.to_owned(), format!("\"version\": \"{version}\""))
    };
    let leading_zero = format!("\"version\": \"0{version}\"");
    let malformed = format!(
        "\"version\": \"{}\"",
        version.rsplit_once('.').map_or(version.as_str(), |(prefix, _)| prefix)
    );
    let prerelease = format!("\"version\": \"{version}-beta\"");
    for (label, needle, replacement) in [
        (
            "top-level duplicate",
            "\"schema\": \"getcodexy.component-manifest.v1\",",
            "\"schema\": \"getcodexy.component-manifest.v1\", \"schema\": \"getcodexy.component-manifest.v1\",",
        ),
        (
            "nested duplicate",
            "\"pluginId\": \"codexy@codexy\",",
            "\"pluginId\": \"codexy@codexy\", \"pluginId\": \"codexy@codexy\",",
        ),
        (
            "leading-zero semver",
            version_needle.as_str(),
            leading_zero.as_str(),
        ),
        (
            "malformed semver",
            version_needle.as_str(),
            malformed.as_str(),
        ),
        (
            "prerelease semver",
            version_needle.as_str(),
            prerelease.as_str(),
        ),
        (
            "dependency-invalid compatible combination",
            "\"components\": [\"core\", \"github\"],",
            "\"components\": [\"github\"],",
        ),
    ] {
        let temp = tempfile::tempdir()?;
        let repo = archive_repository(shared_repository_archive()?, &temp, label)?;
        let selected_version = fixture_version(&repo)?;
        let target = next_patch_version(&selected_version)?;
        select_version_advance(&repo, &target)?;
        let manifest = repo.join(COMPONENT_MANIFEST);
        let text = fs::read_to_string(&manifest)?;
        let corrupted = if label == "dependency-invalid compatible combination" {
            let mut manifest: serde_json::Value = serde_json::from_str(&text)?;
            let combination = manifest["compatibleCombinations"]
                .as_array_mut()
                .and_then(|combinations| {
                    combinations
                        .iter_mut()
                        .find(|combination| combination["components"] == serde_json::json!(["core", "github"]))
                })
                .ok_or("compatible combination fixture")?;
            combination["components"] = serde_json::json!(["github"]);
            format!("{}\n", serde_json::to_string_pretty(&manifest)?)
        } else {
            text.replacen(needle, replacement, 1)
        };
        assert_ne!(corrupted, text, "{label} fixture did not change");
        fs::write(&manifest, corrupted)?;
        let before = version_surface_contents(&repo)?;

        let output = sync_version(&repo, &target)?;
        assert!(
            !output.status.success(),
            "{label} unexpectedly completed mutation\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            version_surface_contents(&repo)?,
            before,
            "{label} changed a managed version surface before rejection"
        );
    }
    Ok(())
}

#[test]
fn markerless_version_mutation_rejects_late_cargo_rewriter_inputs_without_writes()
-> Result<(), Box<dyn std::error::Error>> {
    for (line_ending_label, line_ending) in [("LF", "\n"), ("CRLF", "\r\n")] {
        for (label, relative) in [
            (
                "Cargo.toml alternate version spacing",
                "packages/codexy-runtime/Cargo.toml",
            ),
            (
                "Cargo.lock package field ordering",
                "packages/codexy-runtime/Cargo.lock",
            ),
        ] {
            let label = format!("{label} ({line_ending_label})");
            let temp = tempfile::tempdir()?;
            let repo = archive_repository(shared_repository_archive()?, &temp, &label)?;
            let selected_version = fixture_version(&repo)?;
            let target = next_patch_version(&selected_version)?;
            let (needle, replacement) = if relative.ends_with("Cargo.toml") {
                (
                    format!("version = \"{selected_version}\""),
                    format!("version=\"{selected_version}\""),
                )
            } else {
                (
                    format!("name = \"codexy-runtime\"\nversion = \"{selected_version}\""),
                    format!("version = \"{selected_version}\"\nname = \"codexy-runtime\""),
                )
            };
            select_version_advance(&repo, &target)?;
            let path = repo.join(relative);
            let text = with_line_endings(&fs::read_to_string(&path)?, line_ending);
            let corrupted = text.replacen(
                &needle.replace('\n', line_ending),
                &replacement.replace('\n', line_ending),
                1,
            );
            assert_ne!(corrupted, text, "{label} fixture did not change");
            fs::write(&path, corrupted)?;
            let before = version_surface_contents(&repo)?;

            let output = sync_version(&repo, &target)?;
            assert!(
                !output.status.success(),
                "{label} unexpectedly completed mutation\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                version_surface_contents(&repo)?,
                before,
                "{label} changed a managed version surface before rejection"
            );
        }
    }
    Ok(())
}

fn sync_version(root: &std::path::Path, version: &str) -> Result<std::process::Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--version", version])
        .env("CODEXY_REPO_ROOT", root)
        .current_dir(root)
        .output()
}

fn with_line_endings(text: &str, line_ending: &str) -> String {
    text.replace("\r\n", "\n").replace('\n', line_ending)
}
