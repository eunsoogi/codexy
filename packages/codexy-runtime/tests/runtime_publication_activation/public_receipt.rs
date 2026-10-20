use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::support::FixtureCommand;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

const STAGING_COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ACTIVATION_COMMIT: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

pub(super) fn write(
    root: &Path,
    archive: &Path,
) -> std::io::Result<PathBuf> {
    // Bind both source commits and the staging run to the exact public archive bytes.
    let receipt = root.join("public-release/runtime-release-receipt.json");
    fs::create_dir_all(receipt.parent().expect("public receipt parent"))?;
    let document = json!({
        "schema": "codexy-runtime-release-receipt/v1",
        "source": {
            "activationCommit": ACTIVATION_COMMIT,
            "stagingSourceCommit": STAGING_COMMIT,
        },
        "release": {"tag": "v1.3.0"},
        "staging": {"runId": 42, "runAttempt": 1},
        "provenance": {"runId": 42, "runAttempt": 1},
        "artifact": {
            "sha256": format!("{:x}", Sha256::digest(fs::read(archive)?)),
        },
    });
    fs::write(
        &receipt,
        serde_json::to_vec(&document).map_err(std::io::Error::other)?,
    )?;
    Ok(receipt)
}

pub(super) fn set_runtime_classes(root: &Path, candidate: &Value) -> std::io::Result<()> {
    let path = root.join("public-release/runtime-release-receipt.json");
    let mut receipt: Value = serde_json::from_slice(&fs::read(&path)?).map_err(std::io::Error::other)?;
    let source = candidate["candidate"]["source"].clone();
    let classes = candidate["candidate"]["classes"]
        .as_object()
        .ok_or_else(|| std::io::Error::other("staged candidate classes"))?;
    let handoff = classes
        .get("coreHandoff")
        .ok_or_else(|| std::io::Error::other("staged candidate coreHandoff"))?;
    let mut runtime_classes = serde_json::Map::new();
    runtime_classes.insert(
        "coreHandoff".to_owned(),
        json!({
            "sha256": class_digest(handoff)?,
            "manifestSha256": handoff["manifest"]["sha256"].clone(),
            "source": source.clone(),
        }),
    );
    if let Some(watcher) = classes.get("coreWatcherMcp") {
        runtime_classes.insert(
            "coreWatcherMcp".to_owned(),
            json!({
                "sha256": class_digest(watcher)?,
                "source": source,
            }),
        );
    }
    receipt["runtimeClasses"] = Value::Object(runtime_classes);
    fs::write(&path, serde_json::to_vec(&receipt).map_err(std::io::Error::other)?)
}

pub(super) fn run_release_receipt_fixture(
    checkout: &Path,
    runtime: &Path,
    bundle: &Path,
    staging_receipt: &Path,
    staging_run: &Path,
    receipt: &Path,
    release_tag: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    // Keep interpreter selection and argument paths portable for the Python producer.
    let output = FixtureCommand::new(checkout.join("scripts/create_release_train_receipt.py"))
        .arg_path(runtime).arg_path(bundle).arg_path(runtime).arg_path(staging_receipt).arg_path(staging_run).arg_path(receipt)
        .current_dir(checkout)
        .env("RELEASE_TAG", release_tag)
        .env("ACTIVATION_COMMIT", ACTIVATION_COMMIT)
        .env("STAGING_SOURCE_COMMIT", STAGING_COMMIT)
        .env("STAGING_RUN_ID", "42")
        .output()?;
    if !output.status.success() {
        return Err(format!("release receipt producer failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(serde_json::from_slice(&fs::read(receipt)?)?)
}

fn class_digest(value: &Value) -> std::io::Result<String> {
    let bytes = serde_json::to_vec(&sorted_json(value)).map_err(std::io::Error::other)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn sorted_json(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut keys: Vec<_> = object.keys().collect();
            keys.sort();
            let mut sorted = serde_json::Map::new();
            for key in keys {
                sorted.insert(key.clone(), sorted_json(&object[key]));
            }
            Value::Object(sorted)
        }
        Value::Array(values) => Value::Array(values.iter().map(sorted_json).collect()),
        _ => value.clone(),
    }
}

fn staged_candidate(include_watcher: bool) -> Value {
    let mut classes = json!({
        "coreHandoff": {
            "manifest": {"sha256": "c".repeat(64)},
            "platforms": {"linux-x86_64": {"path": "runtime/handoff.bin", "sha256": "d".repeat(64)}},
        },
        "devtoolsMcp": {"platforms": {}},
    });
    if include_watcher {
        classes["coreWatcherMcp"] = json!({
            "platforms": {"linux-x86_64": {"path": "runtime/watcher.bin", "sha256": "e".repeat(64)}},
        });
    }
    json!({
        "candidate": {
            "source": {"commit": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "tree": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"},
            "classes": classes,
        }
    })
}

fn public_receipt_check_script() -> Result<String, Box<dyn std::error::Error>> {
    // Trace the workflow's exact predicate with captured output for platform-specific failures.
    let (_, _, workflow) = super::workflow("verify-version-release.yml")?;
    let steps = workflow["jobs"]["verify-public-release"]["steps"]
        .as_sequence()
        .ok_or("public release verification steps")?;
    let step = steps
        .iter()
        .find(|step| {
            step["name"].as_str()
                == Some("Download and verify reconciled public release without a token")
        })
        .ok_or("public release download step")?;
    let lines: Vec<_> = step["run"].as_str().ok_or("public release run script")?.lines().collect();
    let start = lines
        .iter()
        .position(|line| {
            line.contains("watcher_class_sha256=")
                || (line.contains("jq -e") && line.contains(".runtimeClasses"))
        })
        .ok_or("runtime class receipt check start")?;
    let end = lines
        .iter()
        .enumerate()
        .skip(start)
        .find(|(_, line)| line.contains("public-receipt.json") && line.contains(">/dev/null"))
        .map(|(index, _)| index)
        .ok_or("runtime class receipt check end")?;
    Ok(format!("set -x\n{}", lines[start..=end].join("\n").replace(" >/dev/null", "")))
}

fn run_public_receipt_check(
    selected_has_watcher: bool,
    receipt_has_watcher: bool,
    mutation: Option<&str>,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path();
    fs::create_dir_all(root.join(".agents/plugins"))?;
    let selected = staged_candidate(selected_has_watcher);
    fs::write(
        root.join(".agents/plugins/runtime-activation.json"),
        serde_json::to_vec(&selected)?,
    )?;
    let archive = root.join("public.tar.gz");
    fs::write(&archive, b"public archive fixture")?;
    write(root, &archive)?;
    set_runtime_classes(root, &staged_candidate(receipt_has_watcher))?;
    if let Some(mutation) = mutation {
        let receipt_path = root.join("public-release/runtime-release-receipt.json");
        let mut receipt: Value = serde_json::from_slice(&fs::read(&receipt_path)?)?;
        match mutation {
            "source.commit" => receipt["runtimeClasses"]["coreWatcherMcp"]["source"]["commit"] = json!("f".repeat(40)),
            "source.tree" => receipt["runtimeClasses"]["coreWatcherMcp"]["source"]["tree"] = json!("f".repeat(40)),
            "sha256" => receipt["runtimeClasses"]["coreWatcherMcp"]["sha256"] = json!("f".repeat(64)),
            _ => return Err("unknown public receipt mutation".into()),
        }
        fs::write(receipt_path, serde_json::to_vec(&receipt)?)?;
    }
    fs::copy(
        root.join("public-release/runtime-release-receipt.json"),
        root.join("public-receipt.json"),
    )?;
    Ok(Command::new("bash")
        .args(["-e", "-o", "pipefail", "-c"])
        .arg(public_receipt_check_script()?)
        .current_dir(root)
        .env("STAGING_SOURCE_COMMIT", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        .output()?)
}

#[test]
fn public_receipt_verifier_matches_the_optional_watcher_class() -> Result<(), Box<dyn std::error::Error>> {
    let legacy = run_public_receipt_check(true, true, None)?;
    assert!(legacy.status.success(), "legacy Watcher receipt rejected: stdout={}, stderr={}", String::from_utf8_lossy(&legacy.stdout), String::from_utf8_lossy(&legacy.stderr));
    let no_watcher = run_public_receipt_check(false, false, None)?;
    assert!(no_watcher.status.success(), "no-Watcher receipt rejected: {}", String::from_utf8_lossy(&no_watcher.stderr));
    for (selected, receipt, mutation, check) in [
        (true, false, None, "coreWatcherMcp class presence"),
        (false, true, None, "coreWatcherMcp class presence"),
        (true, true, Some("source.commit"), "coreWatcherMcp receipt source commit"),
        (true, true, Some("source.tree"), "coreWatcherMcp receipt source tree"),
        (true, true, Some("sha256"), "coreWatcherMcp fingerprint"),
    ] {
        let rejected = run_public_receipt_check(selected, receipt, mutation)?;
        assert!(!rejected.status.success(), "invalid public receipt accepted: {mutation:?}");
        let stderr = String::from_utf8_lossy(&rejected.stderr);
        assert!(stderr.contains(check), "wrong receipt failure: expected {check}, stderr={stderr}");
    }
    Ok(())
}

pub(super) fn set_tag_for_root(root: &Path, release_tag: &str) -> std::io::Result<()> {
    set_tag(
        &root.join("public-release/runtime-release-receipt.json"),
        release_tag,
    )
}

pub(super) fn set_tag(path: &Path, release_tag: &str) -> std::io::Result<()> {
    let mut receipt: Value =
        serde_json::from_slice(&fs::read(path)?).map_err(std::io::Error::other)?;
    receipt["release"]["tag"] = Value::String(release_tag.to_owned());
    fs::write(
        path,
        serde_json::to_vec(&receipt).map_err(std::io::Error::other)?,
    )
}
