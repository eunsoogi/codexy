//! Recognize complete capsule layouts without treating a partial upgrade as absent.

use std::path::Path;

use anyhow::{Context as _, Result, bail};

const SCHEMA: &str = "packages/codexy-runtime/schemas/handoff-runtime.schema.json";
const CURRENT: [&str; 3] = [
    "plugins/codexy/skills/self-improvement/scripts/resumable-context-capsule.sh",
    "plugins/codexy/skills/self-improvement/scripts/resumable-context-capsule.cmd",
    "plugins/codexy/skills/self-improvement/scripts/resumable_context_capsule.py",
];
const PREVIOUS: [&str; 3] = [
    "plugins/codexy/skills/dreaming/scripts/resumable-context-capsule.sh",
    "plugins/codexy/skills/dreaming/scripts/resumable-context-capsule.cmd",
    "plugins/codexy/skills/dreaming/scripts/resumable_context_capsule.py",
];

pub(super) fn validate_core_aware_tree(root: &Path) -> Result<bool> {
    let schema = root.join(SCHEMA).is_file();
    let count = |paths: &[&str]| paths.iter().filter(|p| root.join(p).is_file()).count();
    let current = count(&CURRENT);
    let previous = count(&PREVIOUS);
    if schema
        && ((current == CURRENT.len() && previous == 0)
            || (previous == PREVIOUS.len() && current == 0))
    {
        return Ok(true);
    }
    if schema || current != 0 || previous != 0 {
        bail!("core-handoff source inventory is partial or mixed before activation");
    }
    // Only a proven tree predating both layouts can omit the core handoff class.
    let output = std::process::Command::new("git")
        .args(["rev-list", "-1", "HEAD", "--", SCHEMA])
        .args(CURRENT)
        .args(PREVIOUS)
        .current_dir(root)
        .output()
        .context("proving pre-core-handoff Git tree")?;
    if !output.status.success() || !output.stdout.iter().all(u8::is_ascii_whitespace) {
        bail!("core-handoff sources may be absent only on a proven older Git tree");
    }
    Ok(false)
}
