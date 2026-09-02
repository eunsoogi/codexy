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
    fs::write(artifacts.join("codexy-runtime-package.tar.gz"), b"baseline")?;
    let gh = bin.join("gh");
    fs::write(&gh, r#"#!/bin/sh
repo=eunsoogi/codexy
header='X-GitHub-Api-Version: 2026-03-10'
route="repos/$repo/attestations/sha256:${EXPECTED_DIGEST:?}?per_page=100"
case "$1" in
  api)
    case "$2" in
      --include)
        test "$3" = -H && test "$4" = "$header" && test "$5" = "$route" || exit 2
        case "${ATTESTATION_STATE:?}" in absent) printf '%s\n' 'HTTP/2 404 Not Found' ;; *) printf '%s\n' 'HTTP/2 200 OK' ;; esac ;;
      --paginate)
        test "$3" = --slurp && test "$4" = -H && test "$5" = "$header" && test "$6" = "$route" || exit 2
        case "${ATTESTATION_STATE:?}" in
          many-unrelated) jq -n '[{attestations: ([range(31) | {kind: "unrelated"}] + [{kind: "matching"}])}]' ;;
          *) printf '%s\n' '[{"attestations":[{}]},{"attestations":[{}]}]' ;;
        esac ;;
      *) exit 2 ;;
    esac ;;
  attestation)
    test "$2" = verify && test "$4" = --repo && test "$5" = "$repo" && \
      test "$6" = --signer-workflow && test "$7" = "$repo/.github/workflows/${EXPECTED_WORKFLOW:?}" && \
      test "$8" = --source-ref && test "$9" = refs/heads/main && \
      test "${10}" = --source-digest && test "${11}" = "${EXPECTED_SOURCE_DIGEST:?}" && \
      test "${12}" = --deny-self-hosted-runners && test "${13}" = --limit && test "${14}" = 1000 && test "${15}" = --format && test "${16}" = json || exit 2
    case "${ATTESTATION_STATE:?}" in
      mismatch) printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"one"},{"name":"two"}]}}}]' ;;
      many-unrelated)
        # The matching runtime attestation follows 31 unrelated records.
        unrelated_count=31
        case "$*" in
          *"--limit 1000"*) test "$unrelated_count" -gt 30 || exit 2; printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"codexy-marketplace-plugin.tar.gz"},{"name":"runtime-staging-receipt.json"}]}}}]' ;;
          *) printf '%s\n' '[]' ;;
        esac ;;
      *)
        case "${ATTESTATION_SUBJECTS:-single}" in
          runtime-valid) printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"codexy-marketplace-plugin.tar.gz"},{"name":"runtime-staging-receipt.json"}]}}}]' ;;
          runtime-missing) printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"codexy-marketplace-plugin.tar.gz"}]}}}]' ;;
          runtime-extra) printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"codexy-marketplace-plugin.tar.gz"},{"name":"runtime-staging-receipt.json"},{"name":"extra"}]}}}]' ;;
          runtime-duplicate) printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"codexy-marketplace-plugin.tar.gz"},{"name":"codexy-marketplace-plugin.tar.gz"}]}}}]' ;;
          runtime-renamed) printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"codexy-marketplace-plugin.tar.gz"},{"name":"renamed-receipt.json"}]}}}]' ;;
          runtime-arbitrary) printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"arbitrary-one"},{"name":"arbitrary-two"}]}}}]' ;;
          runtime-malformed-top-level) printf '%s\n' '{"attestation":{"verificationResult":{"statement":{"subject":[{"name":"codexy-marketplace-plugin.tar.gz"},{"name":"runtime-staging-receipt.json"}]}}}}' ;;
          runtime-malformed-subject-object) printf '%s\n' '[{"verificationResult":{"statement":{"subject":{"first":{"name":"codexy-marketplace-plugin.tar.gz"},"second":{"name":"runtime-staging-receipt.json"}}}}}]' ;;
          *) printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"one"}]}}}]' ;;
        esac ;;
    esac ;;
  *) exit 2 ;;
esac
"#)?;
    crate::support::make_executable(&gh)?;
    let environment = temp.path().join("release.env");
    let base_path = std::env::var("PATH")?;
    let run = |state: &str, workflow: &str, source_digest: &str, name: &str, subjects: &str| {
        Command::new("sh")
            .arg(&script)
            .arg(&artifacts)
            .args(["ATTEST_ORIGINAL", name])
            .current_dir(temp.path())
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("ACTIVATION_COMMIT", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            .env("EXPECTED_DIGEST", "8ba8496a2525ae171ffd104d632dede6ef418d9b95962a9d88e2fcdbc8d48d24")
            .env("EXPECTED_WORKFLOW", workflow)
            .env("EXPECTED_SOURCE_DIGEST", source_digest)
            .env("STAGING_SOURCE_COMMIT", "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
            .env("GITHUB_ENV", &environment)
            .env("ATTESTATION_STATE", state)
            .env("ATTESTATION_SUBJECTS", subjects)
            .env("PATH", format!("{}:{base_path}", bin.display()))
            .output()
    };
    let absent = run("absent", "publish-version-release.yml", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "release-baseline.json", "single")?;
    assert!(absent.status.success(), "stdout: {} stderr: {}", String::from_utf8_lossy(&absent.stdout), String::from_utf8_lossy(&absent.stderr));
    assert_eq!(fs::read_to_string(&environment)?, "ATTEST_ORIGINAL=true\n");
    fs::write(&environment, "")?;
    let existing = run("existing", "publish-version-release.yml", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "release-baseline.json", "single")?;
    assert!(existing.status.success(), "stdout: {} stderr: {}", String::from_utf8_lossy(&existing.stdout), String::from_utf8_lossy(&existing.stderr));
    assert_eq!(fs::read_to_string(&environment)?, "ATTEST_ORIGINAL=false\n");
    fs::write(&environment, "")?;
    let mismatch = run("mismatch", "publish-version-release.yml", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "release-baseline.json", "single")?;
    assert!(!mismatch.status.success(), "multi-subject attestation was admitted");
    assert_eq!(fs::read_to_string(&environment)?, "");
    let runtime = run(
        "existing",
        "runtime-candidate.yml",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "codexy-runtime-package.tar.gz",
        "runtime-valid",
    )?;
    assert!(runtime.status.success(), "runtime candidate policy rejected: stdout: {} stderr: {}", String::from_utf8_lossy(&runtime.stdout), String::from_utf8_lossy(&runtime.stderr));
    let many_unrelated = run(
        "many-unrelated",
        "runtime-candidate.yml",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "codexy-runtime-package.tar.gz",
        "single",
    )?;
    assert!(many_unrelated.status.success(), "runtime candidate after 31 unrelated attestations was rejected: stdout: {} stderr: {}", String::from_utf8_lossy(&many_unrelated.stdout), String::from_utf8_lossy(&many_unrelated.stderr));
    for subjects in ["runtime-missing", "runtime-extra", "runtime-duplicate", "runtime-renamed", "runtime-arbitrary", "runtime-malformed-top-level", "runtime-malformed-subject-object"] {
        fs::write(&environment, "")?;
        let rejected = run(
            "existing",
            "runtime-candidate.yml",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "codexy-runtime-package.tar.gz",
            subjects,
        )?;
        assert!(!rejected.status.success(), "runtime subject set {subjects} was admitted");
        assert_eq!(fs::read_to_string(&environment)?, "");
    }
    fs::write(&environment, "")?;
    let missing_runtime = run(
        "absent",
        "runtime-candidate.yml",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "codexy-runtime-package.tar.gz",
        "single",
    )?;
    assert!(!missing_runtime.status.success(), "missing runtime candidate attestation was admitted");
    assert_eq!(fs::read_to_string(&environment)?, "");
    Ok(())
}
