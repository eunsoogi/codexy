use std::path::Path;

use anyhow::{Context as _, Result, bail};

use crate::paths::display_relative;

const FILES: &[(&str, &str)] = &[
    (
        "codexy-admission.sh",
        include_str!("../../../plugins/codexy/hooks/codexy-admission.sh"),
    ),
    (
        "codexy-admission.cmd",
        include_str!("../../../plugins/codexy/hooks/codexy-admission.cmd"),
    ),
    (
        "codexy-admission.py",
        include_str!("../../../plugins/codexy/hooks/codexy-admission.py"),
    ),
    (
        "codexy_policy/__init__.py",
        include_str!("../../../plugins/codexy/hooks/codexy_policy/__init__.py"),
    ),
    (
        "codexy_policy/admission.py",
        include_str!("../../../plugins/codexy/hooks/codexy_policy/admission.py"),
    ),
    (
        "codexy_policy/merge.py",
        include_str!("../../../plugins/codexy/hooks/codexy_policy/merge.py"),
    ),
    (
        "codexy_policy/repository.py",
        include_str!("../../../plugins/codexy/hooks/codexy_policy/repository.py"),
    ),
    (
        "codexy_policy/shell.py",
        include_str!("../../../plugins/codexy/hooks/codexy_policy/shell.py"),
    ),
    (
        "codexy_policy/titles.py",
        include_str!("../../../plugins/codexy/hooks/codexy_policy/titles.py"),
    ),
    (
        "codexy_policy/wrappers.py",
        include_str!("../../../plugins/codexy/hooks/codexy_policy/wrappers.py"),
    ),
];

pub(super) fn is_launcher(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("codexy-admission.sh" | "codexy-admission.cmd")
    )
}

pub(super) fn check(plugin_root: &Path) -> Result<()> {
    let hooks = plugin_root.join("hooks");
    for (relative, expected) in FILES {
        let path = hooks.join(relative);
        let metadata = std::fs::symlink_metadata(&path).with_context(|| {
            format!(
                "reading packaged admission artifact {}",
                display_relative(&path)
            )
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            bail!(
                "packaged admission artifact must be a regular non-symlink file: {}",
                display_relative(&path)
            );
        }
        let actual = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", display_relative(&path)))?;
        if actual != *expected {
            bail!(
                "packaged admission artifact bytes must match the validator-pinned source: {}",
                display_relative(&path)
            );
        }
    }
    Ok(())
}
