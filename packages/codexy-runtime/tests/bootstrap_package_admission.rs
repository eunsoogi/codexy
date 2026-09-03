use std::fs;

use serde_yaml::Value;

#[test]
fn bootstrap_first_pypi_publication_is_explicitly_fail_closed()
-> Result<(), Box<dyn std::error::Error>> {
    let bootstrap = workflow()?;
    let job = bootstrap["jobs"]["publish-bootstrap"]
        .as_mapping()
        .ok_or("bootstrap job")?;
    let steps = job["steps"].as_sequence().ok_or("bootstrap steps")?;
    assert_eq!(steps.len(), 1);
    let guard = steps[0]["run"].as_str().ok_or("bootstrap guard")?;
    assert!(guard.contains("bootstrap-first PyPI publication is retired"));
    assert!(guard.contains("final publisher"));
    assert!(guard.contains("exit 1"));
    assert!(
        !steps.iter().any(|step| step["uses"]
            .as_str()
            .is_some_and(|uses| uses.starts_with("pypa/"))),
        "retired bootstrap workflow must not retain a PyPI publisher"
    );
    assert!(job.get("environment").is_none());
    assert!(job.get("permissions").is_none());
    Ok(())
}

fn workflow() -> Result<Value, Box<dyn std::error::Error>> {
    let path = codexy_runtime::paths::repository_root()
        .join(".github/workflows/bootstrap-package.yml");
    Ok(serde_yaml::from_str(&fs::read_to_string(path)?)?)
}
