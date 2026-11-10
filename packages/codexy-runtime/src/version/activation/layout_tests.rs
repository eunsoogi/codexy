//! Activation keeps the previous capsule layout usable across upgrades.

use std::fs;

use anyhow::Result;

use super::{activate, candidate_version, new_fixture};

#[test]
fn activation_accepts_complete_previous_capsule_source_layout() -> Result<()> {
    let fixture = new_fixture()?;
    let old = fixture.path("plugins/codexy/skills/dreaming");
    fs::create_dir_all(&old)?;
    fs::rename(
        fixture.path("plugins/codexy/skills/self-improvement/scripts"),
        old.join("scripts"),
    )?;
    // Upgrading a complete older source tree must retain its capsule class.
    assert_eq!(
        activate(&fixture.root, candidate_version(), &fixture.receipt)?,
        6
    );
    Ok(())
}
