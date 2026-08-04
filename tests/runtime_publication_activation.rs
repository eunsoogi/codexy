use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::Value as Json;
use serde_yaml::Value as Yaml;

use crate::support;

#[path = "runtime_publication_activation/activation_immutability.rs"]
mod activation_immutability;
#[path = "runtime_publication_activation/artifact_download.rs"]
mod artifact_download;
#[path = "runtime_publication_activation/final_archive.rs"]
mod final_archive;
#[path = "runtime_publication_activation/final_archive_fixture.rs"]
mod final_archive_fixture;
#[path = "runtime_publication_activation/final_archive_lifecycle.rs"]
mod final_archive_lifecycle;
#[path = "runtime_publication_activation/shell_fixtures.rs"]
mod shell_fixtures;
#[path = "runtime_publication_activation/staging.rs"]
mod staging;
#[path = "runtime_publication_activation/staging_zip_fixture.rs"]
mod staging_zip_fixture;

const CANDIDATE_SCHEMA: &str = "codexy-runtime-candidate/v1";

#[test]
fn publication_workflows_are_independent_and_staging_bound() -> Result<(), Box<dyn std::error::Error>> {
    let bootstrap = workflow("bootstrap-package.yml")?;
    let candidate = workflow("runtime-candidate.yml")?;

    assert_ne!(
        bootstrap.0, candidate.0,
        "bootstrap and candidate publication must be separate"
    );
    assert!(
        has_dispatch(&bootstrap.2),
        "bootstrap publication needs workflow_dispatch"
    );
    assert!(
        has_dispatch(&candidate.2),
        "candidate publication needs workflow_dispatch"
    );
    support::assert_structured_literals(
        &candidate.1,
        "immutable runtime staging publication",
        &[
            "git rev-parse",
            "SOURCE_COMMIT",
            "sha256",
            "provenance",
            "actions/upload-artifact",
            "runtime-candidate.json",
        ],
    );
    assert_eq!(
        candidate.1.matches("--clobber").count() + candidate.1.matches("gh release create").count(),
        0,
        "runtime staging must never mutate a public release",
    );
    Ok(())
}

pub(super) fn workflow(name: &str) -> Result<Workflow, Box<dyn std::error::Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows").join(name);
    let text = fs::read_to_string(&path)?;
    Ok((path, text.clone(), serde_yaml::from_str(&text)?))
}

#[test]
fn already_selected_version_sync_preserves_runtime_pointers()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let repo = archive_repository(&temp)?;
    let preserved = [
        (
            "plugins/codexy/runtime-release.json",
            "{\"runtime\":\"immutable\"}\n",
        ),
        (
            "plugins/codexy/mcp/codexy-mcp-lsp",
            "#!/bin/sh\necho pinned\n",
        ),
        (
            "plugins/codexy/mcp/codexy-mcp-codegraph",
            "#!/bin/sh\necho pinned\n",
        ),
    ];
    for (relative, contents) in preserved {
        let path = repo.join(relative);
        fs::create_dir_all(path.parent().ok_or("fixture parent")?)?;
        fs::write(path, contents)?;
    }
    let before = preserved
        .iter()
        .map(|(relative, _)| Ok((relative.to_string(), fs::read(repo.join(relative))?)))
        .collect::<Result<BTreeMap<_, _>, std::io::Error>>()?;
    let mut before = before;
    let bootstrap = "packages/getcodexy/pyproject.toml";
    before.insert(bootstrap.into(), fs::read(repo.join(bootstrap))?);
    let manifest: Json = serde_json::from_slice(&fs::read(
        repo.join("plugins/codexy/.codex-plugin/plugin.json"),
    )?)?;
    let selected = manifest["version"].as_str().ok_or("selected version")?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--version", selected])
        .env("CODEXY_REPO_ROOT", &repo)
        .output()?;
    assert!(
        output.status.success(),
        "version sync fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    for (relative, expected) in before {
        assert_eq!(
            fs::read(repo.join(&relative))?,
            expected,
            "ordinary version sync changed {relative}"
        );
    }
    Ok(())
}

#[test]
fn runtime_contract_requires_authenticated_windows_staging_identity()
-> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let contract: Json = serde_json::from_str(&fs::read_to_string(
        root.join("plugins/codexy/runtime-release.json"),
    )?)?;
    let artifact = contract["artifact"]
        .as_object()
        .ok_or("runtime-release artifact must be an object")?;
    for field in ["sha256", "payloadManifestSha256"] {
        assert!(
            !artifact[field].is_null(),
            "runtime-release contract lacks {field}"
        );
    }
    let platforms = contract["platforms"]
        .as_object()
        .ok_or("runtime-release platforms must be an object")?;
    if platforms.contains_key("windows-x86_64") {
        let candidate: Json = serde_json::from_str(&fs::read_to_string(
            root.join("plugins/codexy/runtime-candidate.json"),
        )?)?;
        assert_eq!(candidate["schema"], CANDIDATE_SCHEMA);
        assert!(candidate["artifact"]["stagingRunId"].as_u64().is_some_and(|value| value > 0));
        assert!(candidate["artifact"]["stagingRunAttempt"].as_u64().is_some_and(|value| value > 0));
        let windows = candidate["platforms"]["windows-x86_64"]
            .as_object()
            .ok_or("Windows lacks authenticated staging proof")?;
        for server in ["lsp", "codegraph"] {
            assert!(windows[server]["path"].as_str().is_some());
            assert!(windows[server]["sha256"].as_str().is_some());
        }
    }
    Ok(())
}

pub(super) type Workflow = (PathBuf, String, Yaml);

pub(super) fn has_dispatch(document: &Yaml) -> bool {
    let root = match document.as_mapping() {
        Some(value) => value,
        None => return false,
    };
    root.iter().any(|(key, value)| {
        (key.as_str() == Some("on") || *key == Yaml::Bool(true))
            && value.as_mapping().is_some_and(|triggers| {
                triggers.contains_key(Yaml::String("workflow_dispatch".into()))
            })
    })
}

pub(super) fn activation_bytes(
    root: &Path,
) -> Result<BTreeMap<PathBuf, Option<Vec<u8>>>, Box<dyn std::error::Error>> {
    let mut bytes = BTreeMap::new();
    for relative in [
        "plugins/codexy/.codex-plugin/plugin.json",
        "plugins/codexy/runtime-release.json",
        "plugins/codexy/runtime-candidate.json",
        "plugins/codexy/mcp/codexy-mcp-lsp",
        "plugins/codexy/mcp/codexy-mcp-codegraph",
    ] {
        let path = root.join(&relative);
        bytes.insert(PathBuf::from(relative), fs::read(path).ok());
    }
    Ok(bytes)
}

fn archive_repository(temp: &tempfile::TempDir) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let archive = temp.path().join("repo.tar");
    let repo = temp.path().join("repo");
    assert!(
        Command::new("git")
            .args(["archive", "--format=tar", "HEAD", "-o"])
            .arg(&archive)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()?
            .success()
    );
    fs::create_dir(&repo)?;
    assert!(
        Command::new("tar")
            .args(["-xf"])
            .arg(&archive)
            .arg("-C")
            .arg(&repo)
            .status()?
            .success()
    );
    Ok(repo)
}
