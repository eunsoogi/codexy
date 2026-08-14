use std::path::Path;

use serde_json::Value;

use super::Disposition;

pub(super) fn disposition(path: &Path, text: &str, base: Disposition) -> Disposition {
    if base != Disposition::Maintained {
        return base;
    }
    if manifested(path, text) {
        Disposition::ExactFixture
    } else {
        Disposition::Maintained
    }
}

fn manifested(path: &Path, text: &str) -> bool {
    let Ok(manifest) = serde_json::from_str::<Value>(include_str!("exact_fixture_manifest.json"))
    else {
        return false;
    };
    manifest.get("schema").and_then(Value::as_str) == Some("codexy.exact-fixture-manifest.v1")
        && manifest
            .get("fixtures")
            .and_then(Value::as_array)
            .is_some_and(|fixtures| {
                fixtures
                    .iter()
                    .any(|fixture| matches_fixture(fixture, path, text))
            })
}

fn matches_fixture(fixture: &Value, path: &Path, text: &str) -> bool {
    if fixture.get("path").and_then(Value::as_str) != path.to_str() {
        return false;
    }
    match fixture.get("marker").and_then(Value::as_str) {
        Some(marker) => {
            text.lines().next() == Some(&format!("// codexy-exact-fixture-file: {marker}"))
        }
        None => serde_json::from_str::<Value>(text).is_ok(),
    }
}
