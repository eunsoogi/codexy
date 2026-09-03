use std::fs;

use serde_yaml::Value;

use crate::support;

#[cfg(unix)]
#[path = "release_lineage/projection_cases.rs"]
mod projection_cases;
#[cfg(unix)]
use projection_cases::assert_projection_cases;

#[test]
fn final_release_admits_explicit_lineage_before_publication() -> Result<(), Box<dyn std::error::Error>> {
    let path = codexy_runtime::paths::repository_root().join(".github/workflows/publish-version-release.yml");
    let publisher: Value = serde_yaml::from_str(&fs::read_to_string(path)?)?;
    let verifier_path = codexy_runtime::paths::repository_root().join(".github/workflows/verify-version-release.yml");
    let verifier: Value = serde_yaml::from_str(&fs::read_to_string(verifier_path)?)?;
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
    let public = verifier["jobs"]["verify-public-release"]["steps"]
        .as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["name"] == "Download and verify reconciled public release without a token"))
        .and_then(|step| step["run"].as_str())
        .ok_or("public release verification")?;
    support::assert_structured_literals(
        public,
        "public verifier current source projection",
        &[
            "git fetch --no-tags origin +refs/heads/main:refs/remotes/origin/main",
            "git show \"$GITHUB_SHA:scripts/project-release-verifiers.sh\" > \"$RUNNER_TEMP/project-release-verifiers\"",
            "\"$RUNNER_TEMP/project-release-verifiers\" \"$ACTIVATION_COMMIT\"",
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
            "git rev-parse \"$GITHUB_SHA:scripts/finalize-verified-release\"",
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
        "git ls-remote --refs origin \"${1:-$tag_ref}\"",
    ] {
        assert!(release.find(required).ok_or(required)? < create);
    }
    assert!(release.matches("verify_tag_if_present \"$remote_tag\"").count() >= 3);
    let upload = release.find("upload_release_asset").ok_or("asset upload")?;
    assert!(create < upload);
    let patch = release.find("gh api --method PATCH").ok_or("draft retarget")?;
    let reread = patch + release[patch..]
        .find("release_state \"$1\" > release-state.json")
        .ok_or("retarget readback")?;
    assert!(patch < reread && reread < upload);
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
            "retarget_existing_draft",
            "releases?per_page=100",
            "gh api --method PATCH",
            "-f \"tag_name=$RELEASE_TAG\"",
            "-f \"target_commitish=$ACTIVATION_COMMIT\"",
        ],
    );
    #[cfg(unix)]
    assert_projection_cases(&projection)?;
    Ok(())
}
