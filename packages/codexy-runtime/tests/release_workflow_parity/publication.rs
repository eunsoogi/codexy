use std::fs;

use super::{
    assert_expected_triggers, document, lines, run, run_clean_preflight, script, step_index,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn publication_phases_are_separate_and_explicitly_gated() -> TestResult {
    let bootstrap = document("bootstrap-package.yml")?;
    let staging = document("runtime-candidate.yml")?;
    let activation = document("runtime-activation.yml")?;
    let publisher = document("publish-version-release.yml")?;
    for (workflow, has_pull_request) in [
        (&bootstrap, false),
        (&staging, true),
        (&activation, false),
        (&publisher, false),
    ] {
        assert_expected_triggers(workflow, has_pull_request)?;
    }
    assert_eq!(
        bootstrap["jobs"]["publish-bootstrap"]["permissions"]["id-token"],
        "write"
    );
    let bootstrap_proof = run(
        &bootstrap,
        "publish-bootstrap",
        "Prove public wheel and source distribution availability",
    )?;
    for fragment in [
        "attempt=",
        "test \"$attempt\"",
        "package_type",
        "sha256sum -c -",
    ] {
        assert!(
            bootstrap_proof.contains(fragment),
            "missing bootstrap proof: {fragment}"
        );
    }
    let bootstrap_clean_index = step_index(
        &bootstrap,
        "publish-bootstrap",
        "Prove clean public bootstrap install",
    )?;
    assert!(
        step_index(
            &bootstrap,
            "publish-bootstrap",
            "Prove public wheel and source distribution availability"
        )? < bootstrap_clean_index
    );
    let bootstrap_clean = run(
        &bootstrap,
        "publish-bootstrap",
        "Prove clean public bootstrap install",
    )?;
    for required in [
        "simple_index_attempt=0",
        "https://pypi.org/simple/getcodexy/",
        "BOOTSTRAP_VERSION=\"$BOOTSTRAP_VERSION\" python3 - simple-index.html <<'PY'",
        "version_prefix = f\"getcodexy-{version}\"",
        "simple_index_attempt",
        "refusing exact-version install",
        "python -m venv public-bootstrap",
        "public-bootstrap/bin/python -m pip install --index-url https://pypi.org/simple \"getcodexy==${BOOTSTRAP_VERSION}\"",
        "public-bootstrap/bin/codexy-mcp-runtime --help",
    ] {
        assert!(
            bootstrap_clean.contains(required),
            "missing clean-install propagation contract: {required}"
        );
    }
    assert!(!bootstrap_clean.contains("pip install --retries"));
    assert!(
        bootstrap_clean
            .find("python -m venv public-bootstrap")
            .unwrap()
            < bootstrap_clean.find("pip install --index-url").unwrap()
    );
    assert!(
        bootstrap_clean.find("pip install --index-url").unwrap()
            < bootstrap_clean.find("codexy-mcp-runtime --help").unwrap()
    );
    let staging_assembly = run(
        &staging,
        "stage-runtime",
        "Assemble canonical staged archive and receipt",
    )?;
    assert!(staging_assembly.contains("scripts/assemble-runtime-candidate"));
    let staging_assembly = script("assemble-runtime-candidate")?;
    assert!(
        staging_assembly.contains("rsync -a") && staging_assembly.contains("--exclude runtime")
    );
    let copied = lines(&staging_assembly)
        .position(|line| {
            line.contains("cp") && line.contains("staged-runtime") && line.contains("$root/runtime")
        })
        .ok_or("staging copy")?;
    let executable = lines(&staging_assembly)
        .position(|line| {
            line.contains("chmod")
                && line.contains("$root/runtime/codexy-mcp-")
                && line.contains("${server}-${platform}")
        })
        .ok_or("staging mode")?;
    assert!(copied < executable);
    let proof = step_index(
        &activation,
        "open-activation-pr",
        "Prove public bootstrap and authenticated staging identity",
    )?;
    let apply = step_index(
        &activation,
        "open-activation-pr",
        "Apply verified activation and version-selection contract",
    )?;
    let pr = step_index(
        &activation,
        "open-activation-pr",
        "Create exactly one activation pull request",
    )?;
    assert!(proof < apply && apply < pr);
    assert!(
        run(
            &activation,
            "open-activation-pr",
            "Apply verified activation and version-selection contract"
        )?
        .contains("scripts/sync-plugin-version.sh --version \"$BOOTSTRAP_VERSION\"")
    );
    let activation_proof = run(
        &activation,
        "open-activation-pr",
        "Prove public bootstrap and authenticated staging identity",
    )?;
    assert!(activation_proof.contains("scripts/download-runtime-staging-artifact staging"));
    assert!(super::command_present(
        activation_proof,
        &["gh", "attestation", "verify"]
    ));
    let activation_pr = run(
        &activation,
        "open-activation-pr",
        "Create exactly one activation pull request",
    )?;
    assert!(lines(activation_pr).any(|line| {
        line.starts_with("git add ")
            && line
                .split_ascii_whitespace()
                .any(|word| word == "plugins/codexy-devtools")
    }));
    assert!(lines(activation_pr).any(|line| {
        line.starts_with("git add ")
            && line
                .split_ascii_whitespace()
                .any(|word| word == ".agents/plugins")
    }));
    assert!(lines(activation_pr).any(|line| {
        line.starts_with("git add ")
            && line.split_ascii_whitespace().any(|word| {
                word == "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json"
            })
    }));
    assert!(lines(activation_pr).any(|line| {
        line.starts_with("git add ")
            && line
                .split_ascii_whitespace()
                .any(|word| word == "packages/getcodexy/uv.lock")
    }));
    crate::support::assert_structured_literals(
        activation_pr,
        "activation pull request metadata",
        &[
            "--title \"feat(runtime): activate v${BOOTSTRAP_VERSION}\"",
            "version ${BOOTSTRAP_VERSION}",
        ],
    );
    crate::support::assert_structured_absent_literals(
        activation_pr,
        "activation pull request metadata",
        &["Fixes #"],
    );
    let release = run(
        &publisher,
        "publish-release",
        "Create and verify the only public version release",
    )?;
    assert!(release.contains("scripts/publish-verified-release"));
    Ok(())
}
#[test]
fn clean_bootstrap_preflight_exercises_visibility_and_failure_boundaries() -> TestResult {
    let bootstrap = document("bootstrap-package.yml")?;
    let clean = run(
        &bootstrap,
        "publish-bootstrap",
        "Prove clean public bootstrap install",
    )?;
    let package: toml::Value = toml::from_str(&fs::read_to_string(
        codexy_runtime::paths::repository_root().join("packages/getcodexy/pyproject.toml"),
    )?)?;
    let version = package["project"]["version"]
        .as_str()
        .ok_or("package version")?;
    let exact_index = format!(
        "<a href=\"https://files.pythonhosted.org/getcodexy-{version}-py3-none-any.whl\">getcodexy-{version}-py3-none-any.whl</a>\\n<a href=\"https://files.pythonhosted.org/getcodexy-{version}.tar.gz\">getcodexy-{version}.tar.gz</a>"
    );
    let adjacent_index = format!(
        "<a href=\"https://files.pythonhosted.org/getcodexy-{version}.post1-py3-none-any.whl\">getcodexy-{version}.post1-py3-none-any.whl</a>"
    );
    let stale_root = tempfile::tempdir()?;
    let stale_curl = format!(
        "count=0; test -f simple-index-attempts && count=$(cat simple-index-attempts); count=$((count + 1)); printf '%s\\n' \"$count\" > simple-index-attempts; if test \"$count\" -eq 1; then return 7; fi; printf '%s\\n' '{adjacent_index}' > simple-index.html"
    );
    let stale = run_clean_preflight(clean, stale_root.path(), version, &stale_curl)?;
    let attempts = fs::read_to_string(stale_root.path().join("simple-index-attempts"))?;
    assert!(attempts.trim().parse::<u32>()? > 0);
    assert_eq!(stale.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&stale.stderr).contains("after 12 bounded checks"));
    let positive_root = tempfile::tempdir()?;
    let positive_curl = format!("printf '%s\\n' '{exact_index}' > simple-index.html");
    let positive = run_clean_preflight(clean, positive_root.path(), version, &positive_curl)?;
    assert!(positive.status.success());
    let transport_root = tempfile::tempdir()?;
    let transport_curl = format!(
        "count=0; test -f simple-index-attempts && count=$(cat simple-index-attempts); count=$((count + 1)); printf '%s\\n' \"$count\" > simple-index-attempts; if test \"$count\" -lt 2; then return 7; fi; printf '%s\\n' '{exact_index}' > simple-index.html"
    );
    let transport = run_clean_preflight(clean, transport_root.path(), version, &transport_curl)?;
    let retries = fs::read_to_string(transport_root.path().join("simple-index-attempts"))?;
    assert!(
        transport.status.success()
            && retries.trim().parse::<u32>()? > 1
            && String::from_utf8_lossy(&transport.stdout).contains("exposes getcodexy==")
    );
    Ok(())
}
