use std::fs;

#[cfg(unix)]
use std::{path::Path, process::Command};

use serde_yaml::Value;

use crate::support;

#[test]
fn final_release_admits_explicit_lineage_before_publication() -> Result<(), Box<dyn std::error::Error>> {
    let path = codexy_runtime::paths::repository_root().join(".github/workflows/publish-version-release.yml");
    let publisher: Value = serde_yaml::from_str(&fs::read_to_string(path)?)?;
    let source = publisher["jobs"]["publish-release"]["steps"]
        .as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["name"] == "Verify selected protected-main source"))
        .and_then(|step| step["run"].as_str())
        .ok_or("protected main source verification")?;
    support::assert_structured_literals(source, "protected main commit admission", &[
        "for commit in \"$STAGING_SOURCE_COMMIT\" \"$ACTIVATION_COMMIT\"; do",
        "case \"$commit\" in *[!0-9a-f]*|'') exit 1 ;; esac",
        "test \"${#commit}\" -eq 40",
        "git merge-base --is-ancestor \"$ACTIVATION_COMMIT\" origin/main",
        "git show \"$GITHUB_SHA:scripts/project-release-verifiers.sh\" > \"$RUNNER_TEMP/project-release-verifiers\" && chmod 755 \"$RUNNER_TEMP/project-release-verifiers\" && \"$RUNNER_TEMP/project-release-verifiers\" \"$ACTIVATION_COMMIT\"",
    ]);
    support::assert_structured_absent_literals(
        source,
        "protected main source must not equate the dispatch SHA to activation",
        &["test \"$GITHUB_SHA\" = \"$ACTIVATION_COMMIT\""],
    );
    let step = publisher["jobs"]["publish-release"]["steps"]
        .as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["name"] == "Create and verify the only public version release"))
        .ok_or("final release step")?;
    for (name, input) in [
        ("STAGING_SOURCE_COMMIT", "staging_source_commit"),
        ("ACTIVATION_COMMIT", "activation_commit"),
        ("STAGING_RUN_ID", "staging_run_id"),
    ] {
        assert_eq!(step["env"][name], format!("${{{{ inputs.{input} }}}}"));
    }
    let public = publisher["jobs"]["verify-public-release"]["steps"]
        .as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["name"] == "Download and verify reconciled public release without a token"))
        .and_then(|step| step["run"].as_str())
        .ok_or("public release verification")?;
    support::assert_structured_literals(
        public,
        "public verifier current source projection",
        &[
            "git fetch --no-tags origin +refs/heads/main:refs/remotes/origin/main && git show \"$GITHUB_SHA:scripts/project-release-verifiers.sh\" > \"$RUNNER_TEMP/project-release-verifiers\" && chmod 755 \"$RUNNER_TEMP/project-release-verifiers\" && \"$RUNNER_TEMP/project-release-verifiers\" \"$ACTIVATION_COMMIT\"",
        ],
    );
    support::assert_structured_absent_literals(
        public,
        "public verifier must not equate the dispatch SHA to activation",
        &["test \"$GITHUB_SHA\" = \"$ACTIVATION_COMMIT\""],
    );
    let projection = std::fs::read_to_string(
        codexy_runtime::paths::repository_root().join("scripts/project-release-verifiers.sh"),
    )?;
    support::assert_structured_literals(
        &projection,
        "controlled verifier source projection",
        &[
            "test \"$GITHUB_SHA\" = \"$(git rev-parse origin/main)\"",
            "git checkout --detach \"$activation_commit\"",
            "git diff --name-only \"$activation_commit\" \"$GITHUB_SHA\" -- scripts | sort",
            "if test -n \"$actual_paths\"; then",
            "while IFS= read -r path; do",
            "scripts/project-release-verifiers.sh)",
            "scripts/reconcile-release-attestations | scripts/verify-release-attestation-set)",
            "git checkout \"$GITHUB_SHA\" -- \"$path\"",
            "git hash-object \"$verifier\"",
        ],
    );
    support::assert_structured_absent_literals(
        &projection,
        "controlled verifier source projection must remain version-relative",
        &["v1.4.0", "7b96e8ac24251aa7ea99e0323eb2b458c8ea6855", "899146ea3587eed1bfc5a0d7e44f49acd0061257"],
    );
    let release = std::fs::read_to_string(codexy_runtime::paths::repository_root().join("scripts/publish-verified-release"))?;
    assert_eq!(step["env"]["GH_TOKEN"], "${{ github.token }}");
    let create = release.find("release_create_response=\"$(gh api --method POST").ok_or("version release")?;
    for required in [
        "test \"$(jq -r .source.stagingSourceCommit dist/runtime-release-receipt.json)\" = \"$STAGING_SOURCE_COMMIT\"",
        "git ls-remote --refs origin \"$tag_ref\"",
    ] {
        assert!(release.find(required).ok_or(required)? < create);
    }
    let tag_readback = release.find("remote_tag_oid=").ok_or("tag readback")?;
    let upload = release.find("upload_release_asset").ok_or("asset upload")?;
    assert!(create < tag_readback && tag_readback < upload);
    assert!(!release.lines().any(|line| {
        line.split_ascii_whitespace().collect::<Vec<_>>().windows(2).any(|words| words == ["git", "push"])
    }));
    support::assert_structured_absent_literals(
        &release,
        "draft release must not use a standalone reference",
        &["repos/$GITHUB_REPOSITORY/git/refs", "tag_create_diagnostic", "-F draft=false"],
    );
    support::assert_structured_literals(
        &release,
        "exact-tag release creation",
        &[
            "release_create_response=\"$(gh api --method POST",
            "gh api --method POST --include",
            "repos/$GITHUB_REPOSITORY/releases\" -f \"tag_name=$RELEASE_TAG\"",
            "-f \"target_commitish=$ACTIVATION_COMMIT\" -f \"name=$RELEASE_TAG\"",
            "-f \"body=$changelog_notes\" -F draft=true -F prerelease=false",
            "release_create_diagnostic",
        ],
    );
    #[cfg(unix)]
    assert_projection_cases(&projection)?;
    Ok(())
}

#[cfg(unix)]
fn assert_projection_cases(projection: &str) -> Result<(), Box<dyn std::error::Error>> {
    for (name, kind, expected_success) in [
        ("no-delta", "no-delta", true),
        ("allowed-verifier-delta", "verifier-delta", true),
        ("allowed-reconciliation-delta", "reconciliation-delta", true),
        ("forbidden-scripts-delta", "forbidden-delta", false),
    ] {
        run_projection_case(projection, name, kind, expected_success)?;
    }
    Ok(())
}

#[cfg(unix)]
fn run_projection_case(
    projection: &str,
    name: &str,
    kind: &str,
    expected_success: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path();
    run_git(root, &["init", "--quiet", "--initial-branch=main"])?;
    run_git(root, &["config", "user.email", "codexy-test@example.invalid"])?;
    run_git(root, &["config", "user.name", "codexy-test"])?;

    let scripts = root.join("scripts");
    fs::create_dir(&scripts)?;
    write_executable(&scripts.join("project-release-verifiers.sh"), projection)?;
    write_executable(&scripts.join("reconcile-release-attestations"), "activation-reconcile\n")?;
    write_executable(&scripts.join("verify-release-attestation-set"), "activation-set\n")?;
    run_git(root, &["add", "scripts"])?;
    run_git(root, &["commit", "--quiet", "-m", "activation"])?;
    let activation = run_git(root, &["rev-parse", "HEAD"])?.trim().to_owned();

    match kind {
        "no-delta" => {
            run_git(root, &["commit", "--quiet", "--allow-empty", "-m", "main"])?;
        }
        "verifier-delta" => {
            fs::write(
                scripts.join("verify-release-attestation-set"),
                "activation-set\nchanged-set\n",
            )?;
            run_git(root, &["add", "scripts/verify-release-attestation-set"])?;
            run_git(root, &["commit", "--quiet", "-m", "main"])?;
        }
        "reconciliation-delta" => {
            fs::write(
                scripts.join("reconcile-release-attestations"),
                "main-reconcile\n",
            )?;
            run_git(root, &["add", "scripts/reconcile-release-attestations"])?;
            run_git(root, &["commit", "--quiet", "-m", "main"])?;
        }
        "forbidden-delta" => {
            fs::write(scripts.join("unrelated-script"), "forbidden\n")?;
            run_git(root, &["add", "scripts/unrelated-script"])?;
            run_git(root, &["commit", "--quiet", "-m", "main"])?;
        }
        other => return Err(format!("unknown projection fixture: {other}").into()),
    }
    let current = run_git(root, &["rev-parse", "HEAD"])?.trim().to_owned();
    run_git(root, &["update-ref", "refs/remotes/origin/main", &current])?;

    let output = Command::new(scripts.join("project-release-verifiers.sh"))
        .current_dir(root)
        .env("GITHUB_SHA", &current)
        .env("GITHUB_REF", "refs/heads/main")
        .arg(&activation)
        .output()?;
    assert_eq!(
        output.status.success(),
        expected_success,
        "{name} projection case had unexpected status: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    if expected_success {
        assert_eq!(run_git(root, &["rev-parse", "HEAD"])?.trim(), activation);
        let verifier_set = fs::read_to_string(scripts.join("verify-release-attestation-set"))?;
        let reconciliation = fs::read_to_string(scripts.join("reconcile-release-attestations"))?;
        if kind == "verifier-delta" {
            assert_eq!(verifier_set, "activation-set\nchanged-set\n");
        } else {
            assert_eq!(verifier_set, "activation-set\n");
        }
        if kind == "reconciliation-delta" {
            assert_eq!(reconciliation, "main-reconcile\n");
        } else {
            assert_eq!(reconciliation, "activation-reconcile\n");
        }
    }
    Ok(())
}

#[cfg(unix)]
fn run_git(cwd: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git").current_dir(cwd).args(args).output()?;
    if !output.status.success() {
        return Err(format!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

#[cfg(unix)]
fn write_executable(path: &Path, contents: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    fs::write(path, contents)?;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}
