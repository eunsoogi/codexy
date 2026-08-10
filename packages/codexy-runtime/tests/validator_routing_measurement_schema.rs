use std::{fs, path::Path};

use serde_json::{Value, json};

use crate::support::{self, TestResult};

const SCHEMA: &str = "skills/orchestration/references/routing-evaluation-results.schema.json";

#[test]
fn canonical_measurement_schema_rejects_every_closed_contract_mutation() -> TestResult {
    let fixture = support::plugin_fixture_with_mutable_files(&[Path::new(SCHEMA)])?;
    let path = fixture.root().join(SCHEMA);
    for (name, mutate) in [
        ("top level closure", mutation(|schema| schema["additionalProperties"] = json!(true))),
        ("corpus property", mutation(|schema| { schema["properties"].as_object_mut().unwrap().remove("corpus_id"); })),
        ("required corpus identity", mutation(|schema| { schema["required"] = json!(["schema", "selected_effort", "results"]); })),
        ("schema constant", mutation(|schema| schema["properties"]["schema"] = json!({"const":"other"}))),
        ("effort enum", mutation(|schema| schema["properties"]["selected_effort"] = json!({"enum":["high"]}))),
        ("observation closure", mutation(|schema| schema["properties"]["results"]["items"]["additionalProperties"] = json!(true))),
        ("integer tokens", mutation(|schema| schema["properties"]["results"]["items"]["properties"]["tokens"] = json!({"type":"number"}))),
        ("token minimum", mutation(|schema| schema["properties"]["results"]["items"]["properties"]["tokens"]["minimum"] = json!(-1))),
        ("cost shape", mutation(|schema| schema["properties"]["results"]["items"]["properties"]["observed_cost_usd"] = json!({"type":"integer","minimum":0}))),
    ] {
        let mut schema: Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
        mutate(&mut schema);
        fs::write(&path, serde_json::to_vec(&schema)?)?;
        let output = support::validator(fixture.root(), "--check")?;
        assert!(!output.status.success(), "{name} unexpectedly passed");
        assert!(String::from_utf8_lossy(&output.stderr).contains("closed routing-measurement JSON schema"));
        fixture.reset_file(Path::new(SCHEMA))?;
    }
    Ok(())
}

type Mutation = Box<dyn Fn(&mut Value)>;

fn mutation(action: impl Fn(&mut Value) + 'static) -> Mutation {
    Box::new(action)
}
