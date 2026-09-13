use std::path::Path;

pub(super) fn assert_manifest_aware_overlay_isolation(
    first: &Path,
    second: &Path,
    relative: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let seed_path = codexy_runtime::paths::repository_root()
        .join("plugins/codexy")
        .join(relative);
    let seed = std::fs::read_to_string(&seed_path)?;
    let sibling = std::fs::read_to_string(second.join(relative))?;
    let mutation = "{\"mutated\":true}\n";

    std::fs::write(first.join(relative), mutation)?;

    assert_eq!(std::fs::read_to_string(first.join(relative))?, mutation);
    assert_eq!(sibling, seed);
    assert_eq!(std::fs::read_to_string(second.join(relative))?, sibling);
    assert_eq!(std::fs::read_to_string(seed_path)?, seed);
    Ok(())
}
