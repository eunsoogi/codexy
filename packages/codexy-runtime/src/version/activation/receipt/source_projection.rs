use anyhow::{Context as _, Result};
use serde_json::{Map, Value, json};

use super::fields::object_field;

const SOURCE_PLATFORMS: [&str; 2] = ["darwin-arm64", "linux-x86_64"];
const REPOSITORY: &str = "https://github.com/eunsoogi/codexy";

pub(super) fn build(
    candidate: &Map<String, Value>,
    artifact: &Map<String, Value>,
    provenance: &Map<String, Value>,
    release_tag: &str,
    core_aware: bool,
) -> anyhow::Result<Value> {
    let source = object_field(candidate, "source", "candidate")?;
    let compatibility = object_field(candidate, "compatibility", "candidate")?;
    let platforms = source_platforms(object_field(candidate, "platforms", "candidate")?)?;
    let artifact_sha = artifact
        .get("sha256")
        .cloned()
        .context("candidate artifact is missing sha256")?;
    let payload_sha = artifact
        .get("payloadManifestSha256")
        .cloned()
        .context("candidate artifact is missing payloadManifestSha256")?;
    let mut release = Map::from_iter([
        (
            "schema".to_owned(),
            Value::String("codexy-runtime-release/v1".to_owned()),
        ),
        (
            "state".to_owned(),
            Value::String("source-selected".to_owned()),
        ),
        ("source".to_owned(), Value::Object(source.clone())),
        (
            "artifact".to_owned(),
            json!({
                "tag": release_tag,
                "url": format!("{REPOSITORY}/releases/download/{release_tag}/codexy-runtime-package.tar.gz"),
                "sha256": artifact_sha,
                "payloadManifestSha256": payload_sha,
            }),
        ),
        ("provenance".to_owned(), Value::Object(provenance.clone())),
        (
            "compatibility".to_owned(),
            Value::Object(compatibility.clone()),
        ),
        ("platforms".to_owned(), Value::Object(platforms.clone())),
    ]);
    if core_aware {
        let classes = object_field(candidate, "classes", "candidate")?;
        release.insert(
            "classes".to_owned(),
            Value::Object(project_classes(classes, &platforms)?),
        );
    }
    Ok(Value::Object(release))
}

fn source_platforms(platforms: &Map<String, Value>) -> Result<Map<String, Value>> {
    let mut projected = Map::new();
    for platform in SOURCE_PLATFORMS {
        projected.insert(
            platform.to_owned(),
            object_field(platforms, platform, "candidate platforms")?
                .clone()
                .into(),
        );
    }
    Ok(projected)
}

fn project_classes(
    classes: &Map<String, Value>,
    source_platforms: &Map<String, Value>,
) -> Result<Map<String, Value>> {
    let mut projected = classes.clone();
    let devtools = object_field(classes, "devtoolsMcp", "candidate classes")?;
    let mut projected_devtools = devtools.clone();
    projected_devtools.insert(
        "platforms".to_owned(),
        Value::Object(source_platforms.clone()),
    );
    projected.insert("devtoolsMcp".to_owned(), Value::Object(projected_devtools));
    Ok(projected)
}
