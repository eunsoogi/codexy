use std::path::Path;

use anyhow::{Result, bail};
use serde_json::{Map, Value};

use crate::validation::runtime_release_schema::{digest, exact, exact_keys, object_field, string};

use super::PLATFORMS;

pub(super) fn check(watcher: &Map<String, Value>, path: &Path) -> Result<()> {
    exact_keys(watcher, &["platforms"], path)?;
    let platforms = object_field(watcher, "platforms", path)?;
    exact_keys(platforms, &PLATFORMS, path)?;
    for platform in PLATFORMS {
        let binary = object_field(platforms, platform, path)?;
        exact_keys(binary, &["path", "sha256", "kind"], path)?;
        let extension = if platform == "windows-x86_64" {
            "exe"
        } else {
            "bin"
        };
        exact(
            string(binary, "path", path)?,
            &format!("runtime/codexy-mcp-watcher-{platform}.{extension}"),
            "core watcher path",
            path,
        )?;
        digest(string(binary, "sha256", path)?, "core watcher digest", path)?;
        let kind = match platform {
            "darwin-arm64" => "mach-o",
            "linux-x86_64" => "elf",
            "windows-x86_64" => "pe",
            _ => bail!("unsupported core watcher platform: {platform}"),
        };
        exact(
            string(binary, "kind", path)?,
            kind,
            "core watcher kind",
            path,
        )?;
    }
    Ok(())
}
