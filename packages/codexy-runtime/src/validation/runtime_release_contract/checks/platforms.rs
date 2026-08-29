use std::path::Path;

use anyhow::{Result, bail};
use serde_json::{Map, Value};

use crate::{
    paths::display_relative,
    validation::runtime_release_schema::{digest, exact, exact_keys, object_field, string},
};

use super::{LEGACY_PLATFORMS, SERVERS};

pub(super) fn check_platforms(
    value: &Map<String, Value>,
    supported: &[String],
    state: &str,
    path: &Path,
) -> Result<()> {
    let legacy = LEGACY_PLATFORMS
        .iter()
        .map(|item| (*item).to_owned())
        .collect::<Vec<_>>();
    let expected = if matches!(state, "legacy-public" | "source-selected") {
        legacy.clone()
    } else {
        supported.to_vec()
    };
    if state == "legacy-public" && supported != legacy.as_slice() {
        bail!(
            "{} legacy-public state must retain the selected two-platform baseline",
            display_relative(path)
        );
    }
    if value.keys().cloned().collect::<Vec<_>>() != expected {
        bail!(
            "{} platforms must exactly be {:?}",
            display_relative(path),
            expected
        );
    }
    for platform in &expected {
        let inventory = object_field(value, platform, path)?;
        exact_keys(inventory, &SERVERS, path)?;
        for server in SERVERS {
            let binary = object_field(inventory, server, path)?;
            let fields = if state == "legacy-public" {
                &["sha256"][..]
            } else {
                &["path", "sha256"][..]
            };
            exact_keys(binary, fields, path)?;
            digest(string(binary, "sha256", path)?, "platform digest", path)?;
            if state != "legacy-public" {
                let extension = if platform == "windows-x86_64" {
                    "exe"
                } else {
                    "bin"
                };
                exact(
                    string(binary, "path", path)?,
                    &format!("runtime/codexy-mcp-{server}-{platform}.{extension}"),
                    "candidate runtime path",
                    path,
                )?;
            }
        }
    }
    Ok(())
}
