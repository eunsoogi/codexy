//! Candidate cleanup removes stale repository-only skill bundles.

use std::fs;
use super::{CandidateFixture, FIRST_DECLARATION};

#[test]
fn candidate_assembly_removes_stale_repository_only_skills() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = CandidateFixture::new(FIRST_DECLARATION)?;
    let stale = ["plugin-marketplace-prep", "release-engineering"]
        .map(|skill| fixture.root().join(format!("dist/candidate/plugins/codexy-devtools/skills/{skill}/SKILL.md")));
    for path in &stale {
        fs::create_dir_all(path.parent().ok_or("stale skill parent missing")?)?;
        fs::write(path, "stale packaged skill\n")?;
    }

    let output = fixture.assemble();

    assert!(output.status.success(), "candidate assembly failed: {}", String::from_utf8_lossy(&output.stderr));
    for path in stale {
        assert!(!path.exists(), "candidate archive payload retained {path:?}");
    }
    Ok(())
}
