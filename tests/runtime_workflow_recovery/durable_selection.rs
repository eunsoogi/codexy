use std::{fs, path::Path};

use serde_yaml::Value;

use crate::support;

#[test]
fn selected_runtime_verification_uses_the_immutable_release_after_publication()
-> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(".github/workflows/plugin-runtime-binaries.yml");
    let workflow: Value = serde_yaml::from_str(&fs::read_to_string(path)?)?;
    let proof = workflow["jobs"]["verify-selected-package"]["steps"]
        .as_sequence()
        .and_then(|steps| {
            steps
                .iter()
                .find(|step| step["name"] == "Download and verify selected immutable bytes")
        })
        .and_then(|step| step["run"].as_str())
        .ok_or("selected immutable runtime proof")?;
    support::assert_structured_literals(
        proof,
        "durable selected runtime verification",
        &[
            "gh release view \"$RELEASE_TAG\"",
            "runtime-release-receipt.json",
            "public release receipt does not match activated staging identity",
        ],
    );
    let windows = workflow["jobs"]["verify-windows-selected-candidate"]["steps"]
        .as_sequence()
        .and_then(|steps| {
            steps
                .iter()
                .find(|step| step["name"] == "Verify immutable native Windows candidate bytes")
        })
        .and_then(|step| step["run"].as_str())
        .ok_or("Windows selected immutable runtime proof")?;
    support::assert_structured_literals(
        windows,
        "Windows public-release archive projection",
        &[
            "New-Item -ItemType Directory -Path dist -ErrorAction Stop",
            "if ($publicArchive)",
            "Copy-Item -LiteralPath $archive -Destination dist/codexy-marketplace-plugin.tar.gz",
        ],
    );
    Ok(())
}
