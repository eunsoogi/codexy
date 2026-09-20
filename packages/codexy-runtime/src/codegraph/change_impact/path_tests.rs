#[path = "fixtures.rs"]
mod fixtures;

use anyhow::Result;

use super::{ImpactOptions, UnknownReason, analyze_with_options};
use fixtures::repository;

#[test]
fn duplicate_snapshot_paths_do_not_trigger_path_limit() -> Result<()> {
    let fixture = repository(&[("dep.py", "VALUE = 1\n"), ("consumer.py", "import dep\n")])?;
    let base = fixture.head()?;
    fixture.write("dep.py", "VALUE = 2\n")?;
    let head = fixture.commit("change dependency")?;

    let result = analyze_with_options(
        fixture.path(),
        &fixture.comparison(&base, &head)?,
        ImpactOptions {
            max_files: 10,
            max_paths: 2,
        },
    )?;

    assert_eq!(result.connecting_paths.len(), 2);
    assert!(!result.limits.paths_truncated);
    assert!(!result.partial);
    assert!(
        !result
            .limits
            .unknown
            .iter()
            .any(|area| area.reason == UnknownReason::PathLimit)
    );
    Ok(())
}
