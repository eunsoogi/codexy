use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use serde_json::{Value, json};

use super::{Update, receipt::PLATFORMS};

pub(super) fn platform_updates(root: &Path) -> Result<Vec<Update>> {
    Ok(vec![
        set_platforms(root.join("plugins/codexy-devtools/.codex-plugin/plugin.json"), &[])?,
        set_platforms(
            root.join(".agents/plugins/marketplace.json"),
            &["plugins", "0"],
        )?,
    ])
}

fn set_platforms(path: PathBuf, object_path: &[&str]) -> Result<Update> {
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("reading activation metadata: {}", path.display()))?;
    let mut document: Value = serde_json::from_str(&text)
        .with_context(|| format!("invalid activation metadata: {}", path.display()))?;
    let mut current = &mut document;
    for segment in object_path {
        current = if *segment == "0" {
            current
                .as_array_mut()
                .and_then(|items| items.get_mut(0))
                .with_context(|| {
                    format!("activation metadata lacks {segment} in {}", path.display())
                })?
        } else {
            current
                .as_object_mut()
                .and_then(|object| object.get_mut(*segment))
                .with_context(|| {
                    format!("activation metadata lacks {segment} in {}", path.display())
                })?
        };
    }
    let object = current
        .as_object_mut()
        .with_context(|| format!("activation metadata must be an object: {}", path.display()))?;
    let field = if object.contains_key("supportedPlatforms") {
        "supportedPlatforms"
    } else {
        "platforms"
    };
    let supported = object.get_mut(field).with_context(|| {
        format!(
            "activation metadata lacks platform declaration: {}",
            path.display()
        )
    })?;
    if !supported.is_array() {
        bail!(
            "activation platform declaration must be an array: {}",
            path.display()
        );
    }
    *supported = json!(PLATFORMS);
    Ok(Update {
        path,
        bytes: format!("{}\n", serde_json::to_string_pretty(&document)?).into_bytes(),
    })
}
