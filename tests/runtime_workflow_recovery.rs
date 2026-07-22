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
fn candidate_publication_records_a_reproducible_success_binding()
-> Result<(), Box<dyn std::error::Error>> {
    let candidate = workflow("runtime-candidate.yml")?;
    let assembly = run(
        &candidate,
        "publish-candidate",
        "Assemble canonical candidate archive and receipt",
    )?;
    support::assert_structured_literals(
        assembly,
        "reproducible candidate archive",
        &["tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -C dist/candidate -czf dist/codexy-marketplace-plugin.tar.gz plugins/codexy"],
    );
    let publish = run(
        &candidate,
        "publish-candidate",
        "Create candidate tag and release once",
    )?;
    support::assert_structured_literals(
        publish,
        "candidate success binding",
        &["scripts/reconcile-runtime-candidate-assets"],
    );
    Ok(())
}

#[test]
fn activation_requires_a_successful_candidate_publication_binding()
-> Result<(), Box<dyn std::error::Error>> {
    let activation = workflow("runtime-activation.yml")?;
    let proof = run(
        &activation,
        "open-activation-pr",
        "Prove public bootstrap, release, run, and candidate bytes",
    )?;
    support::assert_structured_literals(
        proof,
        "activation candidate success binding",
        &["candidate-publication.json", "run.json", ".conclusion run.json)\" = \"success\""],
    );
    Ok(())
}

#[test]
fn activation_pr_creation_reuses_an_existing_verified_branch()
-> Result<(), Box<dyn std::error::Error>> {
    let activation = workflow("runtime-activation.yml")?;
    let branch = run(&activation, "open-activation-pr", "Prepare activation branch")?;
    support::assert_structured_literals(
        branch,
        "resumable activation pull request",
        &["git ls-remote --exit-code --heads origin \"$branch\"", "scripts/verify-runtime-activation-branch \"$branch\""],
    );
    let creation = run(&activation, "open-activation-pr", "Create activation pull request")?;
    support::assert_structured_literals(creation, "activation PR reuse", &["gh pr list --head \"$branch\" --state open", "activation branch differs from verified contract"]);
    Ok(())
}

#[test]
fn privileged_publication_requires_a_source_commit_reachable_from_main()
-> Result<(), Box<dyn std::error::Error>> {
    let candidate = workflow("runtime-candidate.yml")?;
    let candidate_guard = candidate["jobs"]["verify-source-commit"]["steps"]
        .as_sequence()
        .ok_or("candidate source guard steps")?;
    assert_eq!(candidate_guard[0]["with"]["ref"], "main");
    let guard = candidate_guard[1]["run"].as_str().ok_or("candidate source guard")?;
    support::assert_structured_literals(
        guard,
        "candidate protected-main source guard",
        &["git merge-base --is-ancestor \"$SOURCE_COMMIT\" origin/main"],
    );
    assert_eq!(candidate["jobs"]["build-runtime"]["needs"], "verify-source-commit");

    let bootstrap = workflow("bootstrap-package.yml")?;
    let bootstrap_steps = bootstrap["jobs"]["publish-bootstrap"]["steps"]
        .as_sequence()
        .ok_or("bootstrap steps")?;
    assert_eq!(bootstrap_steps[0]["with"]["ref"], "main");
    let bootstrap_guard = bootstrap_steps[1]["run"].as_str().ok_or("bootstrap source guard")?;
    support::assert_structured_literals(
        bootstrap_guard,
        "bootstrap protected-main source guard",
        &["git merge-base --is-ancestor \"$SOURCE_COMMIT\" origin/main"],
    );
    assert_eq!(bootstrap_steps[2]["with"]["ref"], "${{ inputs.source_commit }}");
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
