use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::support::{self, PluginFixture, TestResult};

const GROUPS: [(&str, &str, usize); 6] = [
    ("loc-exception-policy", "skills/git-workflow/SKILL.md", 11),
    ("parallel-isolation", ".codex-plugin/plugin.json", 10),
    ("agent-registration", "agents/catalog.toml", 8),
    (
        "external-gate-policy",
        "skills/codex-orchestration/SKILL.md",
        8,
    ),
    ("subagent-delegation", "agents/codexy-cartographer.toml", 7),
    (
        "library-parity",
        "skills/proof-driven-completion/SKILL.md",
        7,
    ),
];

pub(super) struct Measurement {
    pub(super) baseline_seconds: f64,
    pub(super) candidate_seconds: f64,
    pub(super) baseline_files: u64,
    pub(super) baseline_bytes: u64,
    pub(super) candidate_files: u64,
    pub(super) candidate_bytes: u64,
    pub(super) cases: usize,
}

pub(super) fn measure_resettable_private_sessions() -> TestResult<Measurement> {
    let mut measurement = Measurement {
        baseline_seconds: 0.0,
        candidate_seconds: 0.0,
        baseline_files: 0,
        baseline_bytes: 0,
        candidate_files: 0,
        candidate_bytes: 0,
        cases: 0,
    };
    for (group, relative, cases) in GROUPS {
        let relative = Path::new(relative);
        let baseline = Instant::now();
        let mut baseline_results = Vec::new();
        for _ in 0..cases {
            let fixture = private_fixture(relative)?;
            let (files, bytes) = tree_profile(fixture.root())?;
            measurement.baseline_files += files;
            measurement.baseline_bytes += bytes;
            baseline_results.push(exercise(&fixture, relative)?);
        }
        measurement.baseline_seconds += baseline.elapsed().as_secs_f64();

        let candidate = Instant::now();
        let session = private_fixture(relative)?;
        let (files, bytes) = tree_profile(session.root())?;
        measurement.candidate_files += files;
        measurement.candidate_bytes += bytes;
        let mut candidate_results = Vec::new();
        for _ in 0..cases {
            candidate_results.push(exercise(&session, relative)?);
        }
        measurement.candidate_seconds += candidate.elapsed().as_secs_f64();
        assert_eq!(baseline_results, candidate_results, "{group} parity");
        measurement.cases += cases;
    }
    Ok(measurement)
}

fn private_fixture(relative: &Path) -> TestResult<PluginFixture> {
    Ok(support::plugin_fixture_with_mutable_files(&[relative])?)
}

fn exercise(fixture: &PluginFixture, relative: &Path) -> TestResult<(bool, String)> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("plugins/codexy")
        .join(relative);
    let sibling = private_fixture(relative)?;
    let path = fixture.root().join(relative);
    let original = std::fs::read(&source)?;
    let mut permissions = std::fs::metadata(&path)?.permissions();
    permissions.set_readonly(false);
    std::fs::set_permissions(&path, permissions)?;
    std::fs::write(&path, b"stage11 private write\n")?;
    fixture.reset_file(relative)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&path)?;
    file.write_all(b"stage11 private truncate\n")?;
    drop(file);
    fixture.reset_file(relative)?;
    let moved = moved_path(&path)?;
    std::fs::rename(&path, &moved)?;
    std::fs::remove_file(&moved)?;
    fixture.reset_file(relative)?;
    assert_eq!(std::fs::read(&path)?, original, "fixture reset");
    assert_eq!(
        std::fs::read(sibling.root().join(relative))?,
        original,
        "sibling isolation"
    );
    assert_eq!(std::fs::read(source)?, original, "seed isolation");
    let output = support::validator(fixture.root(), "--check")?;
    let stderr = support::stderr(&output);
    assert!(
        output.status.success(),
        "fixture validator failed: {stderr}"
    );
    Ok((output.status.success(), stderr))
}

fn moved_path(path: &Path) -> TestResult<PathBuf> {
    let name = path
        .file_name()
        .ok_or("fixture file name")?
        .to_string_lossy();
    Ok(path.with_file_name(format!("{name}.stage11-moved")))
}

fn tree_profile(root: &Path) -> TestResult<(u64, u64)> {
    let mut profile = (0, 0);
    collect(root, &mut profile)?;
    Ok(profile)
}

fn collect(root: &Path, profile: &mut (u64, u64)) -> std::io::Result<()> {
    for entry in std::fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(&path, profile)?;
        } else {
            profile.0 += 1;
            profile.1 += std::fs::metadata(path)?.len();
        }
    }
    Ok(())
}
