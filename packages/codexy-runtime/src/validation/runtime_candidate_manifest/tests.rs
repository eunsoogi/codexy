use std::path::Path;

use serde_json::{Map, Value, json};

use super::positive_staging_identity;

#[test]
fn staging_identity_fields_accept_only_positive_i64_values() {
    for field in ["stagingRunId", "stagingRunAttempt"] {
        assert!(
            check_missing(field).is_err(),
            "{field} was allowed to be absent"
        );
        for value in [
            json!(true),
            json!(0),
            json!(-1),
            json!(1.5),
            json!("1"),
            json!(null),
            json!(u64::MAX),
        ] {
            assert!(
                check(field, value).is_err(),
                "{field} accepted a non-positive-integer value"
            );
        }
        for value in [json!(1), json!(i64::MAX)] {
            assert!(
                check(field, value).is_ok(),
                "{field} rejected a positive i64"
            );
        }
    }
}

fn check_missing(field: &str) -> anyhow::Result<()> {
    positive_staging_identity(&Map::new(), field, Path::new("runtime-candidate.json"))
}

fn check(field: &str, value: Value) -> anyhow::Result<()> {
    let mut artifact = Map::new();
    artifact.insert(field.to_owned(), value);
    positive_staging_identity(&artifact, field, Path::new("runtime-candidate.json"))
}
