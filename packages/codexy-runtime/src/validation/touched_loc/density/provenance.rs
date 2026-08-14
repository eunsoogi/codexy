use std::path::Path;

use serde_json::Value;

use super::{Disposition, portable_path};

const MANIFEST: &str = include_str!("provenance_manifest.json");

pub(super) fn classify(path: &Path, text: &str) -> Option<Disposition> {
    let document = serde_json::from_str::<Value>(MANIFEST).ok()?;
    let source = document.get("sources")?.as_array()?.iter().find(|source| {
        source.get("path").and_then(Value::as_str) == Some(portable_path(path).as_str())
            && matches_source(source, text)
    })?;
    match source.get("classification").and_then(Value::as_str) {
        Some("exact-fixture") => Some(Disposition::ExactFixture),
        Some("generated") => Some(Disposition::Generated),
        _ => None,
    }
}

fn matches_source(source: &Value, text: &str) -> bool {
    let Ok(document) = serde_json::from_str::<Value>(text) else {
        return false;
    };
    source
        .get("schema")
        .and_then(Value::as_str)
        .is_some_and(|schema| document.get("schema").and_then(Value::as_str) == Some(schema))
        || source
            .get("marker")
            .and_then(Value::as_str)
            .is_some_and(|marker| {
                document.get("description").and_then(Value::as_str) == Some(marker)
            })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_accepts_windows_style_paths() {
        let path =
            Path::new(r"packages\getcodexy\tests\fixtures\component-installation-cases.json");
        let text = r#"{"schema":"getcodexy.component-installation-cases.v1"}"#;
        assert!(matches!(
            classify(path, text),
            Some(Disposition::ExactFixture)
        ));
    }
}
