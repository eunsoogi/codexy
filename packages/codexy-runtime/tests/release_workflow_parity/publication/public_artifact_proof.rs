use std::fs;
use std::path::Path;
use std::process::Output;

use super::{bash_command, document, run};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn public_artifact_proof_retries_incomplete_json_and_fails_closed() -> TestResult {
    let bootstrap = document("bootstrap-package.yml")?;
    let proof = run(
        &bootstrap,
        "publish-bootstrap",
        "Prove public wheel and source distribution availability",
    )?;
    let version = codexy_runtime::version::runtime_version();
    let incomplete = artifact_json(version, false);
    let complete = artifact_json(version, true);
    for (mode, expected_attempts, expected_json) in
        [("propagate", 2, complete.as_str()), ("exhaust", 12, incomplete.as_str())]
    {
        let root = tempfile::tempdir()?;
        let result = run_public_artifact_preflight(
            proof,
            root.path(),
            version,
            &incomplete,
            expected_json,
            mode,
        )?;
        let attempts = fs::read_to_string(root.path().join("pypi-attempts"))?;
        assert_eq!(
            attempts.trim().parse::<u32>()?,
            expected_attempts,
            "status={:?}\nstdout={}\nstderr={}",
            result.status,
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        if mode == "propagate" {
            assert!(result.status.success());
            assert_eq!(
                fs::read_to_string(root.path().join("public-bdist_wheel"))?,
                "wheel-bytes"
            );
            assert_eq!(
                fs::read_to_string(root.path().join("public-sdist"))?,
                "sdist-bytes"
            );
        } else {
            assert_eq!(result.status.code(), Some(1));
            assert!(
                String::from_utf8_lossy(&result.stderr).contains("expected one public sdist")
            );
        }
    }
    Ok(())
}

fn artifact_json(version: &str, include_sdist: bool) -> String {
    let sdist = include_sdist
        .then(|| {
            format!(
                r#",{{"packagetype":"sdist","url":"https://files.pythonhosted.org/getcodexy-{version}.tar.gz","digests":{{"sha256":"3493dfe12f9879d916893954eb5c64591ab724bd752d2d79a7b55e15b2417239"}}}}"#
            )
        })
        .unwrap_or_default();
    format!(
        r#"{{"info":{{"version":"{version}"}},"urls":[{{"packagetype":"bdist_wheel","url":"https://files.pythonhosted.org/getcodexy-{version}-py3-none-any.whl","digests":{{"sha256":"9ceb18f15662bb87e54af2f5953c0484d2ef76f5444d87913360b9ef87d7296d"}}}}{sdist}]}}"#
    )
}

fn run_public_artifact_preflight(
    run: &str,
    root: &Path,
    version: &str,
    incomplete_json: &str,
    complete_json: &str,
    mode: &str,
) -> Result<Output, Box<dyn std::error::Error>> {
    let (preflight, artifact_body) = run
        .split_once("\nwhile IFS=\"$(printf '\\t')\" read -r package_type url digest; do")
        .ok_or("public artifact preflight")?;
    let artifact_loop = format!(
        "while IFS=\"$(printf '\\t')\" read -r package_type url digest; do{artifact_body}"
    );
    let script = format!(
        r#"curl() {{
    case "$*" in
        *pypi.org/pypi*)
            if test -f pypi-attempts; then
                count=$(cat pypi-attempts)
            else
                count=0
            fi
            count=$((count + 1))
            printf '%s\n' "$count" > pypi-attempts
            if test "$count" -eq 1 || test "$MODE" = exhaust; then
                printf '%s\n' "$INCOMPLETE_JSON" > pypi.json
            else
                printf '%s\n' "$COMPLETE_JSON" > pypi.json
            fi
            ;;
        *public-bdist_wheel*) printf 'wheel-bytes' > public-bdist_wheel ;;
        *public-sdist*) printf 'sdist-bytes' > public-sdist ;;
        *) return 90 ;;
    esac
}}
sleep() {{ :; }}
python3() {{
    local output status
    if output=$(command python3 "$@"); then
        status=0
    else
        status=$?
    fi
    printf '%s\n' "$output" | awk '{{ printf "%s%c%c", $0, 13, 10 }}'
    return "$status"
}}
{preflight}
tr -d '\r' < public-artifacts.tsv > public-artifacts.lf
mv public-artifacts.lf public-artifacts.tsv
{artifact_loop}"#
    );
    Ok(bash_command()?
        .args(["-euo", "pipefail", "-c", &script])
        .current_dir(root)
        .env("BOOTSTRAP_VERSION", version)
        .env("INCOMPLETE_JSON", incomplete_json)
        .env("COMPLETE_JSON", complete_json)
        .env("MODE", mode)
        .output()?)
}
