use std::collections::BTreeMap;

use super::support::{contract, SURFACE_SIDECARS};
use crate::support::TestResult;

#[test]
fn product_boundary_contract_loads_responsibility_sidecars() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let contract = contract(root)?;
    let records = contract["surfaceRecords"]
        .as_array()
        .ok_or("composed surfaceRecords must be an array")?;
    let mut counts = BTreeMap::new();
    for record in records {
        *counts.entry(record["target"].as_str().ok_or("missing record target")?)
            .or_insert(0) += 1;
    }
    assert_eq!(
        counts,
        BTreeMap::from([
            ("codexy", 9),
            ("codexy-devtools", 11),
            ("codexy-github", 3),
            ("repository-only", 7),
        ])
    );
    for (path, target) in SURFACE_SIDECARS {
        let sidecar: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
            root.join(path),
        )?)?;
        assert_eq!(sidecar["target"].as_str(), Some(target));
        assert!(!sidecar["surfaceRecords"].as_array().unwrap().is_empty());
    }
    Ok(())
}
