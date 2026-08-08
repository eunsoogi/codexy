use serde_json::Value;

use super::{PACKAGED_PROOF_PATHS, TestResult, proof_digest, proof_paths, sha256, support};

pub(super) fn assert_current(receipt: &Value, proof: &Value) -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let canonical_text = tempfile::tempdir()?;
    let manifest = canonical_text.path().join("plugin.json");
    support::materialize_lf_text_fixture(
        &root.join("plugins/codexy/.codex-plugin/plugin.json"),
        &manifest,
    )?;
    if proof["sourceManifestSha256"] != sha256(manifest)? {
        return Err("source manifest digest must match current LF-normalized bytes".into());
    }
    for list in [
        &receipt["installed"]["changedFiles"],
        &proof["sourceChangedFiles"],
        &proof["installedChangedFiles"],
    ] {
        if proof_paths(list)? != PACKAGED_PROOF_PATHS {
            return Err("proof paths must match the controlled packaged path set".into());
        }
    }
    for path in PACKAGED_PROOF_PATHS {
        let source = root.join("plugins/codexy").join(path);
        let materialized = canonical_text.path().join(path);
        support::materialize_lf_text_fixture(&source, &materialized)?;
        let digest = sha256(materialized)?;
        for list in [
            &receipt["installed"]["changedFiles"],
            &proof["sourceChangedFiles"],
            &proof["installedChangedFiles"],
        ] {
            if proof_digest(list, path)? != digest {
                return Err(format!("{path} digest must match current LF-normalized bytes").into());
            }
        }
    }
    Ok(())
}
