use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::Value as Json;
use serde_yaml::Value as Yaml;

const RECEIPT_SCHEMA: &str = "codexy.runtime-candidate-receipt.v1";

#[test]
fn publication_workflows_are_independent_and_immutable() -> Result<(), Box<dyn std::error::Error>> {
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
    assert!(
        candidate.1.contains("git rev-parse")
            && candidate.1.contains("SOURCE_COMMIT")
            && candidate.1.contains("sha256")
            && candidate.1.contains("provenance")
            && candidate.1.contains("curl --fail")
            && candidate.1.contains("runtime-candidate.json"),
        "candidate publication lacks immutable public proof"
    );
    assert!(
        !candidate.1.contains("--clobber") && !candidate.1.contains("gh release edit"),
        "candidate publication must never clobber immutable assets"
    );
    Ok(())
}

fn workflow(name: &str) -> Result<Workflow, Box<dyn std::error::Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows").join(name);
    let text = fs::read_to_string(&path)?;
    Ok((path, text.clone(), serde_yaml::from_str(&text)?))
}

#[test]
fn invalid_activation_is_byte_identical() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let gate = root.join("scripts/activate-runtime-contract");
    assert!(
        gate.is_file(),
        "missing activation gate entrypoint: {}",
        gate.display()
    );
    let before = activation_bytes(root)?;
    let temp = tempfile::tempdir()?;
    let receipt = temp.path().join("invalid-receipt.json");
    fs::write(&receipt, r#"{"schema":"invalid"}"#)?;
    let output = Command::new(&gate)
        .args([
            "--repo-root",
            root.to_str().ok_or("non-UTF-8 repository root")?,
            "--bootstrap-version",
            "1.2.2",
            "--candidate-receipt",
            receipt.to_str().ok_or("non-UTF-8 receipt path")?,
        ])
        .output()?;
    assert!(
        !output.status.success(),
        "activation accepted an invalid candidate receipt"
    );
    assert_eq!(
        activation_bytes(root)?,
        before,
        "failed activation mutated a public pointer"
    );
    Ok(())
}

#[test]
fn ordinary_version_sync_preserves_runtime_pointers() -> Result<(), Box<dyn std::error::Error>> {
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
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--version", "9.9.9"])
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
fn runtime_contract_requires_a_public_windows_receipt() -> Result<(), Box<dyn std::error::Error>> {
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
        let receipt = candidate_receipt(&root.join("plugins/codexy"))?;
        assert_eq!(receipt["schema"], RECEIPT_SCHEMA);
        let windows = receipt["platforms"]["windows-x86_64"]
            .as_object()
            .ok_or("Windows lacks candidate proof")?;
        for proof in ["lsp", "codegraph", "nativeProtocolProof"] {
            assert!(
                !windows[proof].is_null(),
                "Windows candidate receipt lacks {proof}"
            );
        }
    }
    Ok(())
}

type Workflow = (PathBuf, String, Yaml);

fn has_dispatch(document: &Yaml) -> bool {
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

fn activation_bytes(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, Box<dyn std::error::Error>> {
    let mut bytes = BTreeMap::new();
    for relative in [
        "plugins/codexy/.codex-plugin/plugin.json",
        "plugins/codexy/runtime-release.json",
        "plugins/codexy/mcp/codexy-mcp-lsp",
        "plugins/codexy/mcp/codexy-mcp-codegraph",
    ] {
        let path = root.join(&relative);
        bytes.insert(PathBuf::from(relative), fs::read(path)?);
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

fn candidate_receipt(plugin_root: &Path) -> Result<Json, Box<dyn std::error::Error>> {
    for entry in fs::read_dir(plugin_root)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) == Some("json") {
            let receipt: Json = serde_json::from_str(&fs::read_to_string(&path)?)?;
            if receipt["schema"] == RECEIPT_SCHEMA {
                return Ok(receipt);
            }
        }
    }
    Err("Windows advertised without packaged public candidate receipt".into())
}
