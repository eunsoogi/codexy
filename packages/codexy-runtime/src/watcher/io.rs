use std::collections::BTreeMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result, bail};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use sha2::{Digest as _, Sha256};

pub(super) const MAX_STATE_BYTES: usize = 1_048_576;

pub(super) fn state_root() -> Result<PathBuf> {
    let root = env::var_os("CODEXY_WATCHER_STATE_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os(if cfg!(windows) {
                "LOCALAPPDATA"
            } else {
                "XDG_STATE_HOME"
            })
            .map(PathBuf::from)
        })
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .context("watcher state directory is unavailable")?;
    if !root.is_absolute() {
        bail!(
            "watcher state directory must be absolute: {}",
            root.display()
        );
    }
    Ok(root.join("codexy-watcher"))
}

pub(super) fn ensure_dir(path: &Path) -> Result<()> {
    if path.exists() {
        reject_link(path)?;
        if !path.is_dir() {
            bail!("watcher state path is not a directory: {}", path.display());
        }
    } else {
        fs::create_dir_all(path)
            .with_context(|| format!("creating watcher state directory {}", path.display()))?;
    }
    set_private_mode(path, 0o700)?;
    Ok(())
}

pub(super) fn reject_link(path: &Path) -> Result<()> {
    if fs::symlink_metadata(path)
        .with_context(|| format!("inspecting watcher state path {}", path.display()))?
        .file_type()
        .is_symlink()
    {
        bail!(
            "watcher state path must not be a symlink: {}",
            path.display()
        );
    }
    Ok(())
}

pub(super) fn safe_id(value: &str, label: &str) -> Result<()> {
    if value.is_empty() || value.len() > 128 || value.contains('/') || value.contains('\\') {
        bail!("watcher {label} is invalid");
    }
    if value.chars().any(char::is_control) {
        bail!("watcher {label} contains a control character");
    }
    Ok(())
}

pub(super) fn random_hex(bytes: usize) -> Result<String> {
    let mut value = vec![0_u8; bytes];
    getrandom::fill(&mut value).context("generating watcher capability")?;
    Ok(value.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub(super) fn token_hash(token: &str) -> String {
    hash_text(token)
}

pub(super) fn hash_text(value: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(value.as_bytes());
    format!("{:x}", digest.finalize())
}

pub(super) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            duration.as_millis().min(u128::from(u64::MAX)) as u64
        })
}

pub(super) fn read_json<T: DeserializeOwned>(path: &Path, label: &str) -> Result<T> {
    reject_link(path)?;
    let bytes = fs::read(path).with_context(|| format!("reading {label} {}", path.display()))?;
    if bytes.len() > MAX_STATE_BYTES {
        bail!("watcher {label} exceeds the state size limit");
    }
    serde_json::from_slice(&bytes).with_context(|| format!("parsing watcher {label}"))
}

pub(super) fn write_json(path: &Path, value: &Value) -> Result<()> {
    let bytes = serde_json::to_vec(value).context("serializing watcher state")?;
    atomic_write(path, &bytes)
}

pub(super) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if bytes.len() > MAX_STATE_BYTES {
        bail!("watcher state exceeds the size limit");
    }
    reject_link(path).or_else(|error| if path.exists() { Err(error) } else { Ok(()) })?;
    let parent = path.parent().context("watcher state file has no parent")?;
    ensure_dir(parent)?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("state");
    let temporary = parent.join(format!(".{name}.tmp-{}", random_hex(12)?));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .with_context(|| {
            format!(
                "creating watcher state temporary file {}",
                temporary.display()
            )
        })?;
    set_private_mode(&temporary, 0o600)?;
    file.write_all(bytes).context("writing watcher state")?;
    file.sync_all().context("syncing watcher state")?;
    drop(file);
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error).with_context(|| format!("committing watcher state {}", path.display()));
    }
    Ok(())
}

pub(super) fn canonical(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| (key.clone(), canonical(value)))
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect::<Map<String, Value>>(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(canonical).collect()),
        other => other.clone(),
    }
}

pub(super) fn canonical_text(value: &Value) -> Result<String> {
    serde_json::to_string(&canonical(value)).context("serializing canonical watcher value")
}

pub(super) fn set_private_mode(_path: &Path, _mode: u32) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(_path, fs::Permissions::from_mode(_mode))?;
    }
    Ok(())
}
