use super::*;

#[test]
fn publication_phases_are_separate_and_explicitly_gated() -> Result<(), Box<dyn std::error::Error>>
{
    let bootstrap = document("bootstrap-package.yml")?;
    let staging = document("runtime-candidate.yml")?;
    let activation = document("runtime-activation.yml")?;
    let publisher = document("publish-version-release.yml")?;
    for workflow in [&bootstrap, &staging, &activation, &publisher] {
        assert_dispatch_only(workflow)?;
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
    for line in [
        "attempt=0",
        "test \"$attempt\" -lt 12 || exit 1",
        "for package_type in (\"bdist_wheel\", \"sdist\"):",
        "printf '%s  %s\\n' \"$digest\" \"public-${package_type}\" | sha256sum -c -",
    ] {
        assert!(lines(bootstrap_proof).any(|actual| actual == line));
    }
    let staging_assembly = run(
        &staging,
        "stage-runtime",
        "Assemble canonical staged archive and receipt",
    )?;
    assert_eq!(staging_assembly, "scripts/assemble-runtime-candidate");
    let staging_assembly = script("assemble-runtime-candidate")?;
    assert!(lines(&staging_assembly).any(|line| line == "rsync -a --exclude runtime --exclude runtime-release.json --exclude runtime-candidate.json plugins/codexy-devtools/ \"$root/\""));
    let copied = lines(&staging_assembly)
        .position(|line| line == "cp -R staged-runtime \"$root/runtime\"")
        .ok_or("staging copy")?;
    let executable = lines(&staging_assembly)
        .position(|line| line == "chmod 755 \"$root/runtime/codexy-mcp-${server}-${platform}.bin\"")
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
    let activation_proof = run(
        &activation,
        "open-activation-pr",
        "Prove public bootstrap and authenticated staging identity",
    )?;
    assert!(
        lines(activation_proof)
            .any(|line| line == "scripts/download-runtime-staging-artifact staging")
    );
    assert!(command_present(
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
    support::assert_structured_literals(
        activation_pr,
        "activation pull request metadata",
        &[
            "--title \"feat(runtime): activate v${BOOTSTRAP_VERSION}\"",
            "Fixes #502",
        ],
    );
    let release = run(
        &publisher,
        "publish-v1-3-0",
        "Create and verify the only public version release",
    )?;
    assert!(command_present(
        release,
        &["gh", "release", "create", "v1.3.0"]
    ));
    Ok(())
}
