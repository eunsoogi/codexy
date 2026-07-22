use std::{fs, path::Path};

use serde_yaml::Value;

use crate::support;

#[test]
fn activation_requires_clean_bootstrap_entrypoint_and_successful_candidate_run()
-> Result<(), Box<dyn std::error::Error>> {
    let activation = workflow("runtime-activation.yml")?;
    let proof = run(
        &activation,
        "open-activation-pr",
        "Prove public bootstrap, release, run, and candidate bytes",
    )?;
    support::assert_structured_literals(
        proof,
        "activation bootstrap and candidate workflow proof",
        &[
            "python -m venv public-bootstrap",
            "getcodexy==${BOOTSTRAP_VERSION}",
            "public-bootstrap/bin/codexy-mcp-runtime --help",
            "test \"$(jq -r .status run.json)\" = \"completed\"",
            "test \"$(jq -r .conclusion run.json)\" = \"success\"",
        ],
    );
    Ok(())
}

#[test]
fn candidate_publication_recovers_without_overwriting_assets()
-> Result<(), Box<dyn std::error::Error>> {
    let candidate = workflow("runtime-candidate.yml")?;
    let publish = run(
        &candidate,
        "publish-candidate",
        "Create candidate tag and release once",
    )?;
    support::assert_structured_literals(
        publish,
        "recoverable immutable candidate publication",
        &[
            "refs/tags/$CANDIDATE_TAG^{}",
            "test \"$remote_commit\" = \"$SOURCE_COMMIT\"",
            "gh release view \"$CANDIDATE_TAG\"",
            "scripts/reconcile-runtime-candidate-assets",
        ],
    );
    assert_eq!(publish.matches("--clobber").count(), 0, "immutable assets must not be overwritten");
    Ok(())
}

#[test]
fn candidate_builds_run_platform_local_lsp_and_codegraph_protocol_smokes()
-> Result<(), Box<dyn std::error::Error>> {
    let candidate = workflow("runtime-candidate.yml")?;
    let steps = candidate["jobs"]["build-runtime"]["steps"]
        .as_sequence()
        .ok_or("build-runtime steps")?;
    let smoke = named_step(steps, "Smoke platform-local MCP protocols")?;
    let package = step_index(steps, "Package declared platform binaries")?;
    assert!(smoke.0 < package, "protocol smoke must precede packaging");
    let script = smoke.1["run"].as_str().ok_or("smoke run")?;
    support::assert_structured_literals(
        script,
        "platform-local MCP protocol smokes",
        &[
            "codexy-mcp-lsp",
            "codexy-mcp-codegraph",
            "\"method\": \"initialize\"",
            "\"protocolVersion\": \"2024-11-05\"",
            "\"name\": \"lsp_status\"",
            "\"name\": \"codegraph_overview\"",
        ],
    );
    Ok(())
}

fn workflow(name: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows").join(name);
    Ok(serde_yaml::from_str(&fs::read_to_string(path)?)?)
}

fn run<'a>(
    value: &'a Value,
    job: &str,
    name: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    value["jobs"][job]["steps"]
        .as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["name"] == name))
        .and_then(|step| step["run"].as_str())
        .ok_or_else(|| format!("missing run step {name:?}").into())
}

fn named_step<'a>(
    steps: &'a [Value],
    name: &str,
) -> Result<(usize, &'a Value), Box<dyn std::error::Error>> {
    steps
        .iter()
        .enumerate()
        .find(|(_, step)| step["name"] == name)
        .ok_or_else(|| format!("missing step {name:?}").into())
}

fn step_index(steps: &[Value], name: &str) -> Result<usize, Box<dyn std::error::Error>> {
    named_step(steps, name).map(|(index, _)| index)
}
