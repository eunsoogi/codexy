use serde_yaml::{Mapping, Value};

#[test]
fn validation_workflows_are_read_only_and_disable_checkout_credentials() -> Result<(), Box<dyn std::error::Error>> {
    for name in ["python-package.yml", "plugin-runtime-binaries.yml"] {
        let document = document(name)?;
        assert_exact(mapping(&document["permissions"])? , "contents", "read")?;
        for job in document["jobs"].as_mapping().ok_or("jobs")?.values() {
            if let Some(permissions) = job.get("permissions") { assert_exact(mapping(permissions)?, "contents", "read")?; }
            for step in job["steps"].as_sequence().ok_or("steps")? { if step["uses"].as_str() == Some("actions/checkout@v4") { assert_eq!(step["with"]["persist-credentials"], Value::Bool(false)); } }
        }
    }
    Ok(())
}

#[test]
fn candidate_and_activation_write_only_at_explicit_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    let candidate = document("runtime-candidate.yml")?;
    let permissions = mapping(&candidate["jobs"]["publish-candidate"]["permissions"])?;
    assert_eq!(permissions[Value::String("contents".into())], "write");
    assert_eq!(permissions[Value::String("id-token".into())], "write");
    let publish = run(&candidate, "publish-candidate", "Create candidate tag and release once")?;
    assert!(command(publish, &["gh", "release", "create"]));
    assert!(!command(publish, &["gh", "release", "edit"]));
    let activation = document("runtime-activation.yml")?;
    let permissions = mapping(&activation["permissions"])?;
    assert_eq!(permissions[Value::String("contents".into())], "write");
    assert_eq!(permissions[Value::String("pull-requests".into())], "write");
    Ok(())
}

fn document(name: &str) -> Result<Value, Box<dyn std::error::Error>> { Ok(serde_yaml::from_str(&std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows").join(name))?)?) }
fn mapping(value: &Value) -> Result<&Mapping, Box<dyn std::error::Error>> { value.as_mapping().ok_or_else(|| "mapping".into()) }
fn assert_exact(mapping: &Mapping, name: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> { assert_eq!(mapping.len(), 1); assert_eq!(mapping[Value::String(name.into())], value); Ok(()) }
fn run<'a>(value: &'a Value, job: &str, name: &str) -> Result<&'a str, Box<dyn std::error::Error>> { value["jobs"][job]["steps"].as_sequence().and_then(|steps| steps.iter().find(|step| step["name"] == name)).and_then(|step| step["run"].as_str()).ok_or_else(|| "run".into()) }
fn command(run: &str, words: &[&str]) -> bool { run.lines().map(str::trim).any(|line| line.split_ascii_whitespace().collect::<Vec<_>>().windows(words.len()).any(|actual| actual == words)) }
