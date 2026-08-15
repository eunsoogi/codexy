use serde_yaml::{Mapping, Value};

use crate::support;

#[test]
fn validation_workflows_are_read_only_and_disable_checkout_credentials() -> Result<(), Box<dyn std::error::Error>> {
    for name in ["python-package.yml", "plugin-runtime-binaries.yml"] {
        let document = document(name)?;
        let permissions = mapping(&document["permissions"])?;
        if name == "plugin-runtime-binaries.yml" {
            assert_authenticated_read_only(permissions)?;
        } else {
            assert_exact(permissions, "contents", "read")?;
        }
        for job in document["jobs"].as_mapping().ok_or("jobs")?.values() {
            if let Some(permissions) = job.get("permissions") { assert_exact(mapping(permissions)?, "contents", "read")?; }
            for step in job["steps"].as_sequence().ok_or("steps")? { if step["uses"].as_str() == Some("actions/checkout@v7") { assert_eq!(step["with"]["persist-credentials"], Value::Bool(false)); } }
        }
    }
    Ok(())
}

#[test]
fn staging_activation_and_final_release_write_only_at_explicit_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    let staging = document("runtime-candidate.yml")?;
    let permissions = mapping(&staging["jobs"]["stage-runtime"]["permissions"])?;
    assert_eq!(permissions[Value::String("contents".into())], "read");
    assert_eq!(permissions[Value::String("id-token".into())], "write");
    assert_eq!(permissions[Value::String("attestations".into())], "write");
    assert!(!checkout_persists(&staging, "build-runtime")?);
    assert!(!checkout_persists(&staging, "stage-runtime")?);

    let activation = document("runtime-activation.yml")?;
    let permissions = mapping(&activation["permissions"])?;
    assert_eq!(permissions[Value::String("contents".into())], "write");
    assert_eq!(permissions[Value::String("pull-requests".into())], "write");
    assert!(checkout_persists(&activation, "open-activation-pr")?);

    let publisher = document("publish-version-release.yml")?;
    let permissions = mapping(&publisher["permissions"])?;
    assert_eq!(permissions[Value::String("contents".into())], "write");
    assert_eq!(permissions[Value::String("id-token".into())], "write");
    assert_eq!(permissions[Value::String("attestations".into())], "write");
    assert!(!checkout_persists(&publisher, "publish-v1-3-0")?);
    let publish = run(&publisher, "publish-v1-3-0", "Create and verify the only public version release")?;
    assert!(command(publish, &["gh", "release", "create", "v1.3.0"]));
    let public = mapping(&publisher["jobs"]["verify-v1-3-0"]["permissions"])?;
    assert_eq!(public.len(), 2);
    assert_eq!(public[Value::String("contents".into())], "read");
    assert_eq!(public[Value::String("attestations".into())], "read");
    assert!(!checkout_persists(&publisher, "verify-v1-3-0")?);
    let verify = run(
        &publisher,
        "verify-v1-3-0",
        "Smoke public release without a token",
    )?;
    support::assert_structured_literals(
        verify,
        "tokenless public release smoke",
        &["python -m venv public-bootstrap"],
    );
    let step = publisher["jobs"]["verify-v1-3-0"]["steps"]
        .as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["name"] == "Smoke public release without a token"))
        .ok_or("public release smoke step")?;
    assert_eq!(step["env"]["GH_TOKEN"], "");
    assert_eq!(step["env"]["GITHUB_TOKEN"], "");
    let attestation = publisher["jobs"]["verify-v1-3-0"]["steps"]
        .as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["name"] == "Verify public release attestations with a read-only token"))
        .ok_or("public release attestation step")?;
    assert_eq!(attestation["env"]["GH_TOKEN"], "${{ github.token }}");
    assert_eq!(attestation["env"]["GITHUB_TOKEN"], "");
    Ok(())
}

fn document(name: &str) -> Result<Value, Box<dyn std::error::Error>> { Ok(serde_yaml::from_str(&std::fs::read_to_string(codexy_runtime::paths::repository_root().join(".github/workflows").join(name))?)?) }
fn mapping(value: &Value) -> Result<&Mapping, Box<dyn std::error::Error>> { value.as_mapping().ok_or_else(|| "mapping".into()) }
fn assert_exact(mapping: &Mapping, name: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> { assert_eq!(mapping.len(), 1); assert_eq!(mapping[Value::String(name.into())], value); Ok(()) }
fn assert_authenticated_read_only(mapping: &Mapping) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(mapping.len(), 2);
    assert_eq!(mapping[Value::String("contents".into())], "read");
    assert_eq!(mapping[Value::String("actions".into())], "read");
    Ok(())
}
fn run<'a>(value: &'a Value, job: &str, name: &str) -> Result<&'a str, Box<dyn std::error::Error>> { value["jobs"][job]["steps"].as_sequence().and_then(|steps| steps.iter().find(|step| step["name"] == name)).and_then(|step| step["run"].as_str()).ok_or_else(|| "run".into()) }
fn command(run: &str, words: &[&str]) -> bool { run.lines().map(str::trim).any(|line| line.split_ascii_whitespace().collect::<Vec<_>>().windows(words.len()).any(|actual| actual == words)) }
fn checkout_persists(value: &Value, job: &str) -> Result<bool, Box<dyn std::error::Error>> { value["jobs"][job]["steps"].as_sequence().and_then(|steps| steps.iter().find(|step| step["uses"] == "actions/checkout@v7")).and_then(|step| step["with"]["persist-credentials"].as_bool()).ok_or_else(|| "checkout credentials".into()) }
