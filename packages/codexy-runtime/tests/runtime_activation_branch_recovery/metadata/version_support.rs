use std::{fs, path::Path};

/// Version transitions for fixture states that intentionally exercise stale inputs.
pub fn make_uv_lock_stale(repo: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let lock = repo.join("packages/getcodexy/uv.lock");
    let lock_text = fs::read_to_string(&lock)?;
    let current_version = lock_text
        .lines()
        .find_map(|line| line.trim().strip_prefix("version = \"")?.strip_suffix('"'))
        .ok_or("getcodexy lock version")?;
    let stale_version = next_patch_version(current_version)?;
    let stale_lock = lock_text.replacen(
        &format!("version = \"{current_version}\""),
        &format!("version = \"{stale_version}\""),
        1,
    );
    fs::write(lock, stale_lock)?;
    Ok(())
}

pub fn next_patch_version(version: &str) -> Result<String, Box<dyn std::error::Error>> {
    let (major, remainder) = version.split_once('.').ok_or("major version")?;
    let (minor, patch) = remainder.split_once('.').ok_or("minor version")?;
    let patch = patch.parse::<u64>()?.checked_add(1).ok_or("patch overflow")?;
    Ok(format!("{major}.{minor}.{patch}"))
}
