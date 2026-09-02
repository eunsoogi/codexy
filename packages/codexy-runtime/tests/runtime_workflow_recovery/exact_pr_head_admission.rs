use std::{env, fs, process::Command};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use tempfile::tempdir;
#[path = "../runtime_candidate_assembly_contract/fixture.rs"]
#[allow(dead_code)]
mod candidate_fixture;
use super::{script, workflow};
use crate::support;
use candidate_fixture::CandidateFixture;
const REPOSITORY: &str = "eunsoogi/codexy";

#[test]
fn exact_pr_mode_contract() -> Result<(), Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(
        codexy_runtime::paths::repository_root().join(".github/workflows/runtime-candidate.yml"),
    )?;
    support::assert_structured_literals(
        &raw,
        "exact PR candidate workflow contract",
        &[
            "exact_pr_number:",
            "description: \"Optional exact same-repository PR number\"",
            "type: string",
            "default: \"\"",
            "scripts/verify-runtime-candidate-source.sh",
            "github.workflow_sha",
            "git fetch --no-tags origin \"$SOURCE_COMMIT\"",
            "runtime-pr-head-${{ github.run_id }}-${{ github.run_attempt }}",
            "runtime-staging-${{ github.run_id }}-${{ github.run_attempt }}",
        ],
    );
    assert_eq!(
        raw.matches("scripts/verify-runtime-candidate-source").count(),
        2,
        "admission must run before build and again before staging"
    );
    let candidate = workflow("runtime-candidate.yml")?;
    let build = &candidate["jobs"]["build-runtime"];
    assert_eq!(build["permissions"]["contents"], "read");
    assert!(build["permissions"]["pull-requests"].is_null());
    let build_text = serde_yaml::to_string(build)?;
    support::assert_structured_absent_literals(
        &build_text,
        "untrusted build permissions",
        &["id-token: write", "attestations: write", "contents: write", "pull-requests: write", "secrets:"],
    );
    let stage = &candidate["jobs"]["stage-runtime"];
    assert_eq!(stage["environment"], "runtime-candidate-staging");
    assert_eq!(stage["permissions"]["contents"], "read");
    assert_eq!(stage["permissions"]["pull-requests"], "read");
    assert_eq!(stage["permissions"]["id-token"], "write");
    assert_eq!(stage["permissions"]["attestations"], "write");
    let stage_text = serde_yaml::to_string(stage)?;
    support::assert_structured_literals(
        &stage_text,
        "trusted exact PR staging boundary",
        &[
            "ref: ${{ github.workflow_sha }}",
            "persist-credentials: false",
            "Recheck exact PR admission before staging",
            "dist/runtime-staging-receipt.json",
        ],
    );
    let assembly = script("assemble-runtime-candidate")?;
    support::assert_structured_literals(
        &assembly,
        "trusted exact PR archive assembly",
        &[
            "git ls-tree",
            "root_mode",
            "120000",
            "160000",
            "git archive",
            "plugins/codexy-devtools",
            "find \"$root\" -type l",
        ],
    );
    let downloader = script("download-runtime-staging-artifact")?;
    support::assert_structured_absent_literals(
        &downloader,
        "activation cannot select PR-head artifacts",
        &["runtime-pr-head"],
    );
    Ok(())
}
#[cfg(unix)]
#[test]
fn exact_pr_admission_accepts_only_the_current_same_repository_head()
-> Result<(), Box<dyn std::error::Error>> {
    let source = "a".repeat(40);
    let valid = pr_json(7, REPOSITORY, REPOSITORY, "main", "open", None, &source);
    let accepted = run_helper(&source, "7", &valid)?;
    assert!(accepted.status.success(), "valid admission: {}", stderr(&accepted));
    let cases = [
        ("fork", source.clone(), "7", pr_json(7, "attacker/codexy", REPOSITORY, "main", "open", None, &source)),
        ("wrong PR", source.clone(), "7", pr_json(8, REPOSITORY, REPOSITORY, "main", "open", None, &source)),
        ("wrong base repository", source.clone(), "7", pr_json(7, REPOSITORY, "other/codexy", "main", "open", None, &source)),
        ("wrong base ref", source.clone(), "7", pr_json(7, REPOSITORY, REPOSITORY, "release", "open", None, &source)),
        ("closed", source.clone(), "7", pr_json(7, REPOSITORY, REPOSITORY, "main", "closed", None, &source)),
        ("merged", source.clone(), "7", pr_json(7, REPOSITORY, REPOSITORY, "main", "open", Some("2026-08-27T00:00:00Z"), &source)),
        ("force-pushed head mismatch", source.clone(), "7", pr_json(7, REPOSITORY, REPOSITORY, "main", "open", None, &"b".repeat(40))),
        ("malformed source SHA", "not-a-full-sha".into(), "7", valid.clone()),
        ("zero PR", source.clone(), "0", valid.clone()),
        ("leading-zero PR", source.clone(), "07", valid.clone()),
        ("non-numeric PR", source.clone(), "7x", valid.clone()),
        ("malformed API JSON", source.clone(), "7", "{not-json".into()),
    ];
    for (name, input, pr_number, response) in cases {
        let output = run_helper(&input, pr_number, &response)?;
        assert!(!output.status.success(), "admission accepted {name}: {}", stderr(&output));
    }
    Ok(())
}
#[cfg(unix)]
#[test]
fn exact_pr_archive_rejects_symlinks_and_source_runtime_material() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = CandidateFixture::new("bundled_platforms=\"darwin-arm64 linux-x86_64\"\n")?;
    let root = fixture.root();
    let protected = fixture.assemble();
    assert!(protected.status.success(), "protected assembly failed: {}", stderr(&protected));
    let protected_receipt = read_receipt(root)?;
    assert_eq!(protected_receipt["schema"], "codexy-runtime-candidate-receipt/v1");
    assert_keys(&protected_receipt, "artifact,candidate,provenance,schema");
    let plugin = root.join("plugins/codexy-devtools");
    fs::create_dir_all(plugin.join("runtime"))?;
    fs::write(plugin.join("runtime/codexy-mcp-lsp-darwin-arm64.bin"), b"poison")?;
    for name in ["runtime-release.json", "runtime-candidate.json", "handoff-runtime.json"] {
        fs::write(plugin.join(name), b"poison")?;
    }
    let tar = root.join("test-bin/tar");
    fs::write(&tar, "#!/bin/sh\nset -eu\ncase \"$*\" in *--sort=name*) while test \"$#\" -gt 0; do if test \"$1\" = -czf; then : >\"$2\"; exit 0; fi; shift; done;; *) exec /usr/bin/tar \"$@\";; esac\n")?;
    support::make_executable(&tar)?;
    let path = format!("{}:{}", root.join("test-bin").display(), env::var("PATH")?);
    let directory = root.to_str().ok_or("non-utf8 fixture path")?;
    let git = |args: &[&str]| -> Result<(), Box<dyn std::error::Error>> {
        let status = Command::new("git").args(["-C", directory]).args(args).status()?;
        assert!(status.success(), "git command failed: {args:?}");
        Ok(())
    };
    git(&["add", "."])?;
    git(&["-c", "user.email=test@example.invalid", "-c", "user.name=Exact PR fixture", "commit", "-qm", "poison fixture"])?;
    let source_at = || -> Result<String, Box<dyn std::error::Error>> {
        Ok(String::from_utf8(Command::new("git").args(["-C", directory, "rev-parse", "HEAD"]).output()?.stdout)?.trim().to_owned())
    };
    let source = source_at()?;
    git(&["-c", "user.email=test@example.invalid", "-c", "user.name=Exact PR fixture", "commit", "--allow-empty", "-qm", "trusted workflow"])?;
    let workflow = source_at()?;
    let run = |source: &str, workflow: &str, pr: &str| {
        Command::new("sh").arg(root.join("scripts/assemble-runtime-candidate")).current_dir(root)
            .env("SOURCE_COMMIT", source).env("WORKFLOW_SHA", workflow).env("EXACT_PR_NUMBER", pr)
            .env("TARGET_VERSION", CandidateFixture::TARGET_VERSION)
            .env("ARTIFACT_NAME", if pr.is_empty() { "runtime-staging-1-1" } else { "runtime-pr-head-1-1" })
            .env("STAGING_RUN_ID", "1").env("STAGING_RUN_ATTEMPT", "1")
            .env("GITHUB_SERVER_URL", "https://github.invalid").env("GITHUB_REPOSITORY", REPOSITORY).env("PATH", &path).output()
    };
    let output = run(&source, &workflow, "7")?;
    assert!(output.status.success(), "poisoned runtime assembly failed: {}", stderr(&output));
    assert_eq!(fs::read(root.join("dist/candidate/plugins/codexy-devtools/runtime/codexy-mcp-lsp-darwin-arm64.bin"))?, fs::read(root.join("staged-runtime/codexy-mcp-lsp-darwin-arm64.bin"))?);
    assert!(!root.join("dist/candidate/plugins/codexy-devtools/runtime-release.json").exists());
    assert!(!root.join("dist/candidate/plugins/codexy-devtools/handoff-runtime.json").exists());
    let exact_receipt = read_receipt(root)?;
    assert_exact_receipt(&exact_receipt, root, &source, &workflow)?;
    fs::remove_file(plugin.join("runtime-candidate.json"))?;
    std::os::unix::fs::symlink("../../scripts/inspect-release-archive-contract.py", plugin.join("runtime-candidate.json"))?;
    git(&["add", "."])?;
    git(&["-c", "user.email=test@example.invalid", "-c", "user.name=Exact PR fixture", "commit", "-qm", "symlink fixture"])?;
    let before = fs::read(root.join("scripts/inspect-release-archive-contract.py"))?;
    let output = run(&source_at()?, &source_at()?, "7")?;
    assert!(!output.status.success(), "symlinked exact source was accepted");
    assert_eq!(fs::read(root.join("scripts/inspect-release-archive-contract.py"))?, before, "trusted validator was modified");
    Ok(())
}
#[test]
fn exact_pr_receipt_and_all_allowed_paths_stay_within_the_hard_limit()
-> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    for path in [
        ".github/workflows/runtime-candidate.yml",
        "scripts/verify-runtime-candidate-source.sh",
        "scripts/assemble-runtime-candidate",
        "packages/codexy-runtime/tests/runtime_workflow_recovery.rs",
        "packages/codexy-runtime/tests/runtime_workflow_recovery/exact_pr_head_admission.rs",
    ] {
        assert!(fs::read_to_string(root.join(path))?.lines().count() <= 250, "{path} exceeds hard limit");
    }
    Ok(())
}
fn read_receipt(root: &std::path::Path) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_slice(&fs::read(root.join("dist/runtime-staging-receipt.json"))?)?)
}
fn digest(path: &std::path::Path) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}
fn assert_keys(value: &Value, expected: &str) {
    let keys = value.as_object().expect("receipt object").keys().map(String::as_str).collect::<Vec<_>>();
    assert_eq!(keys.join(","), expected);
}
fn assert_exact_receipt(receipt: &Value, root: &std::path::Path, source: &str, workflow: &str) -> Result<(), Box<dyn std::error::Error>> {
    assert_keys(receipt, "admission,artifact,candidate,provenance,schema");
    assert_keys(&receipt["admission"], "baseRef,baseRepository,headRepository,mergedAt,pullRequestNumber,sourceSha,state,workflowSha");
    assert_keys(&receipt["provenance"], "artifactName,mode,repositoryId,runAttempt,runId,sourceCommit,workflowPath,workflowRunUrl,workflowSha");
    let archive_sha = digest(&root.join("dist/codexy-marketplace-plugin.tar.gz"))?;
    let manifest_sha = digest(&root.join("dist/candidate/plugins/codexy-devtools/runtime-candidate.json"))?;
    assert_eq!(receipt["schema"], "codexy-runtime-pr-head-candidate-receipt/v1");
    assert_eq!(receipt["admission"]["pullRequestNumber"], 7);
    assert_eq!(receipt["admission"]["sourceSha"], source);
    assert_eq!(receipt["admission"]["workflowSha"], workflow);
    assert_eq!(receipt["provenance"]["artifactName"], "runtime-pr-head-1-1");
    assert_eq!(receipt["artifact"]["sha256"], archive_sha);
    assert_eq!(receipt["artifact"]["payloadManifestSha256"], manifest_sha);
    Ok(())
}
#[cfg(unix)]
fn run_helper(source: &str, pr_number: &str, response: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let temporary = tempdir()?;
    let bin = temporary.path().join("bin");
    fs::create_dir(&bin)?;
    let gh = bin.join("gh");
    let args_log = temporary.path().join("gh-args");
    fs::write(&gh, "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$@\" >\"$GH_ARGS_LOG\"\ntest \"$1\" = api\ntest \"$2\" = --method\ntest \"$3\" = GET\ntest \"$4\" = --header\ntest \"$6\" = \"repos/$GITHUB_REPOSITORY/pulls/$EXACT_PR_NUMBER\"\nprintf '%s\\n' \"$FAKE_PR_JSON\"\n")?;
    support::make_executable(&gh)?;
    let mut paths = vec![bin];
    paths.extend(env::split_paths(&env::var_os("PATH").ok_or("PATH")?));
    let output = Command::new("sh")
        .arg("scripts/verify-runtime-candidate-source.sh")
        .current_dir(codexy_runtime::paths::repository_root())
        .env("SOURCE_COMMIT", source)
        .env("EXACT_PR_NUMBER", pr_number)
        .env("GITHUB_REPOSITORY", REPOSITORY)
        .env("GH_TOKEN", "test-token")
        .env("FAKE_PR_JSON", response)
        .env("GH_ARGS_LOG", &args_log)
        .env("PATH", env::join_paths(paths)?)
        .output()?;
    if source.len() == 40 && pr_number == "7" {
        assert_eq!(fs::read_to_string(&args_log)?, "api\n--method\nGET\n--header\nAccept: application/vnd.github+json\nrepos/eunsoogi/codexy/pulls/7\n");
    } else {
        assert!(!args_log.exists(), "invalid inputs must not call GitHub");
    }
    Ok(output)
}
#[cfg(unix)]
fn pr_json(
    number: u64, head_repository: &str, base_repository: &str, base_ref: &str,
    state: &str, merged_at: Option<&str>, head_sha: &str,
) -> String {
    json!({"number": number, "state": state, "merged_at": merged_at, "base": {"ref": base_ref, "repo": {"full_name": base_repository}}, "head": {"sha": head_sha, "repo": {"full_name": head_repository}}}).to_string()
}
fn stderr(output: &std::process::Output) -> String { String::from_utf8_lossy(&output.stderr).into_owned() }
