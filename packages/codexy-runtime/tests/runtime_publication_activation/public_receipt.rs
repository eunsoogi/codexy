use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

pub(super) fn write(
    root: &Path,
    archive: &Path,
) -> std::io::Result<PathBuf> {
    const STAGING_COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const ACTIVATION_COMMIT: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
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
