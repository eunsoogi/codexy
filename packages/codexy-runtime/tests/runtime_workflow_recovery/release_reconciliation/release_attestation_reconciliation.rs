use std::{fs, process::Command};

#[test]
fn attestation_reconciliation_admits_only_absent_or_exact_authenticated_state()
-> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let temp = tempfile::tempdir()?;
    let scripts = temp.path().join("scripts");
    let artifacts = temp.path().join("dist");
    let bin = temp.path().join("bin");
    fs::create_dir_all(&scripts)?;
    fs::create_dir_all(&artifacts)?;
    fs::create_dir_all(&bin)?;
    let script = scripts.join("reconcile-release-attestations");
    fs::copy(root.join("scripts/reconcile-release-attestations"), &script)?;
    fs::write(artifacts.join("release-baseline.json"), b"baseline")?;
    let gh = bin.join("gh");
    fs::write(&gh, r#"#!/bin/sh
repo=eunsoogi/codexy
header='X-GitHub-Api-Version: 2026-03-10'
route="repos/$repo/attestations/sha256:8ba8496a2525ae171ffd104d632dede6ef418d9b95962a9d88e2fcdbc8d48d24?per_page=100"
case "$1" in
  api)
    case "$2" in
      --include)
        test "$3" = -H && test "$4" = "$header" && test "$5" = "$route" || exit 2
        case "${ATTESTATION_STATE:?}" in absent) printf '%s\n' 'HTTP/2 404 Not Found' ;; *) printf '%s\n' 'HTTP/2 200 OK' ;; esac ;;
      --paginate)
        test "$3" = --slurp && test "$4" = -H && test "$5" = "$header" && test "$6" = "$route" || exit 2
        printf '%s\n' '[{"attestations":[{}]},{"attestations":[{}]}]' ;;
      *) exit 2 ;;
    esac ;;
  attestation)
    test "$2" = verify && test "$4" = --repo && test "$5" = "$repo" && \
      test "$6" = --signer-workflow && test "$7" = "$repo/.github/workflows/publish-version-release.yml" && \
      test "$8" = --source-ref && test "$9" = refs/heads/main && \
      test "${10}" = --source-digest && test "${11}" = aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa && \
      test "${12}" = --deny-self-hosted-runners && test "${13}" = --format && test "${14}" = json || exit 2
    case "${ATTESTATION_STATE:?}" in
      mismatch) printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"one"}]}}}]' ;;
      *) printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"one"}]}}},{"verificationResult":{"statement":{"subject":[{"name":"two"}]}}}]' ;;
    esac ;;
  *) exit 2 ;;
esac
"#)?;
    crate::support::make_executable(&gh)?;
    let environment = temp.path().join("release.env");
    let base_path = std::env::var("PATH")?;
    let run = |state: &str| {
        Command::new("sh")
            .arg(&script)
            .arg(&artifacts)
            .args(["ATTEST_ORIGINAL", "release-baseline.json"])
            .current_dir(temp.path())
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("ACTIVATION_COMMIT", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            .env("GITHUB_ENV", &environment)
            .env("ATTESTATION_STATE", state)
            .env("PATH", format!("{}:{base_path}", bin.display()))
            .output()
    };
    let absent = run("absent")?;
    assert!(absent.status.success(), "stdout: {} stderr: {}", String::from_utf8_lossy(&absent.stdout), String::from_utf8_lossy(&absent.stderr));
    assert_eq!(fs::read_to_string(&environment)?, "ATTEST_ORIGINAL=true\n");
    fs::write(&environment, "")?;
    let existing = run("existing")?;
    assert!(existing.status.success(), "stdout: {} stderr: {}", String::from_utf8_lossy(&existing.stdout), String::from_utf8_lossy(&existing.stderr));
    assert_eq!(fs::read_to_string(&environment)?, "ATTEST_ORIGINAL=false\n");
    fs::write(&environment, "")?;
    let mismatch = run("mismatch")?;
    assert!(!mismatch.status.success(), "count mismatch was admitted");
    assert_eq!(fs::read_to_string(&environment)?, "");
    Ok(())
}
