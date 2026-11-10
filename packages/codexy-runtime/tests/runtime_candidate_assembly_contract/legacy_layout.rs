//! Candidate assembly preserves the preceding capsule source layout.

use std::fs;
use super::{CandidateFixture, FIRST_DECLARATION};

#[test]
fn candidate_assembly_accepts_complete_previous_capsule_layout() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = CandidateFixture::new(FIRST_DECLARATION)?;
    fixture.enable_core_runtime()?;
    let old = fixture.root().join("plugins/codexy/skills/dreaming");
    fs::create_dir_all(&old)?;
    fs::rename(fixture.root().join("plugins/codexy/skills/self-improvement/scripts"), old.join("scripts"))?;
    let output = fixture.assemble();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(fixture.root().join("dist/candidate/plugins/codexy-devtools/handoff-runtime.json").is_file());
    fs::remove_file(old.join("scripts/resumable-context-capsule.cmd"))?;
    assert!(!fixture.assemble().status.success(), "partial old layout must fail");
    Ok(())
}
