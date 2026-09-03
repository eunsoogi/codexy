use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use super::*;

const TARGET_VERSION: &str = "1.6.3";
const PRIOR_VERSION: &str = "1.6.2";

#[test]
fn public_smoke_receives_prior_version_in_a_one_commit_checkout() -> TestResult {
    let temp = tempfile::tempdir()?;
    let missing_input = create_fixture(temp.path().join("missing-input"))?;
    let history = git_output(
        &missing_input,
        &["log", "--format=%H", "--", "packages/getcodexy/pyproject.toml"],
    )?;
    assert!(history.status.success());
    assert_eq!(String::from_utf8(history.stdout)?.lines().count(), 1);

    let failed = run_smoke(&missing_input, None)?;
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr)
        .contains("previous package version is unavailable"));

    let explicit_input = create_fixture(temp.path().join("explicit-input"))?;
    let passed = run_smoke(&explicit_input, Some(PRIOR_VERSION))?;
    assert!(
        passed.status.success(),
        "one-commit public smoke failed: {}",
        String::from_utf8_lossy(&passed.stderr)
    );
    assert!(
        fs::read_to_string(explicit_input.join("public-upgrade.json"))?.contains("\"command\":\"update\"")
    );
    Ok(())
}

#[test]
fn release_workflows_resolve_and_forward_the_explicit_upgrade_version() -> TestResult {
    let publisher = document("publish-version-release.yml")?;
    let verifier = document("verify-version-release.yml")?;
    let job = "publish-release";
    let resolve = step_index(&publisher, job, "Resolve previous package version from full history")?;
    let smoke = step_index(&publisher, job, "Smoke exact final package before publication")?;
    assert!(resolve < smoke);
    let resolve_step = &steps(&publisher, job)?[resolve];
    assert_eq!(resolve_step["id"], "resolve_previous_package_version");
    let resolve_run = resolve_step["run"].as_str().ok_or("resolve run")?;
    for required in [
        "git log --format=%H \"$ACTIVATION_COMMIT\" -- packages/getcodexy/pyproject.toml",
        "git show \"$revision:packages/getcodexy/pyproject.toml\"",
        "printf 'version=%s\\n' \"$previous_version\" >> \"$GITHUB_OUTPUT\"",
        "previous package version is unavailable",
    ] {
        assert!(resolve_run.contains(required), "missing history gate: {required}");
    }
    assert_eq!(
        steps(&publisher, job)?[smoke]["env"]["UPGRADE_FROM_VERSION"],
        "${{ steps.resolve_previous_package_version.outputs.version }}"
    );
    assert_eq!(
        publisher["jobs"]["publish-release"]["outputs"]["upgrade_from_version"],
        "${{ steps.resolve_previous_package_version.outputs.version }}"
    );
    assert_eq!(
        publisher["jobs"]["verify-public-release"]["with"]["upgrade_from_version"],
        "${{ needs.publish-release.outputs.upgrade_from_version }}"
    );

    let inputs = verifier["on"]["workflow_call"]["inputs"]
        .as_mapping()
        .ok_or("verifier inputs")?;
    assert_eq!(inputs["upgrade_from_version"]["required"], true);
    assert_eq!(inputs["upgrade_from_version"]["type"], "string");
    let public_smoke = run(
        &verifier,
        "verify-public-release",
        "Smoke public release without a token",
    )?;
    assert_eq!(
        steps(&verifier, "verify-public-release")?
            .iter()
            .find(|step| step["name"] == "Smoke public release without a token")
            .ok_or("public smoke step")?["env"]["UPGRADE_FROM_VERSION"],
        "${{ inputs.upgrade_from_version }}"
    );
    assert_eq!(public_smoke, "scripts/smoke-public-getcodexy-release.sh");
    let smoke_script = fs::read_to_string(
        codexy_runtime::paths::repository_root().join("scripts/smoke-public-getcodexy-release.sh"),
    )?;
    assert!(smoke_script.contains("previous_version=${UPGRADE_FROM_VERSION:-}"));
    assert!(!smoke_script.contains("git log --format=%H"));
    assert!(!smoke_script.contains("git show \"$revision:packages/getcodexy/pyproject.toml\""));
    Ok(())
}

fn create_fixture(root: PathBuf) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let repository = codexy_runtime::paths::repository_root();
    fs::create_dir_all(root.join("scripts"))?;
    for name in [
        "smoke-public-getcodexy-release.sh",
        "fake_public_codex_host.py",
    ] {
        let destination = root.join("scripts").join(name);
        fs::copy(repository.join("scripts").join(name), &destination)?;
        make_executable(&destination)?;
    }

    let package = root.join("packages/getcodexy");
    fs::create_dir_all(&package)?;
    fs::write(
        package.join("pyproject.toml"),
        format!("[project]\nversion = \"{TARGET_VERSION}\"\n"),
    )?;
    git(&root, &["init", "-q"])?;
    git(&root, &["config", "user.email", "codexy@example.invalid"])?;
    git(&root, &["config", "user.name", "Codexy test"])?;
    git(&root, &["add", "packages/getcodexy/pyproject.toml"])?;
    git(&root, &["commit", "-qm", "activated release"])?;

    let bundle = root.join("bundle-source");
    for name in ["codexy", "codexy-github", "codexy-devtools"] {
        let manifest = bundle.join(format!("plugins/{name}/.codex-plugin"));
        fs::create_dir_all(&manifest)?;
        fs::write(
            manifest.join("plugin.json"),
            format!("{{\"name\":\"{name}\",\"version\":\"{TARGET_VERSION}\"}}\n"),
        )?;
    }
    let inspect_manifest = root.join("public-inspect/plugins/codexy-devtools/.codex-plugin");
    fs::create_dir_all(&inspect_manifest)?;
    fs::write(
        inspect_manifest.join("plugin.json"),
        format!("{{\"version\":\"{TARGET_VERSION}\"}}\n"),
    )?;
    let archive = root.join("public-bundle.tar.gz");
    assert!(
        Command::new("tar")
            .args(["-czf"])
            .arg(&archive)
            .args(["-C"])
            .arg(&bundle)
            .arg(".")
            .status()?
            .success()
    );

    let stubs = root.join("stubs");
    let runner_temp = root.join("runner-temp");
    fs::create_dir_all(&stubs)?;
    fs::create_dir_all(&runner_temp)?;
    let python = runner_temp.join("python");
    fs::write(
        &python,
        "#!/bin/sh\nset -eu\nif test \"${1:-}\" = \"-m\" && test \"${2:-}\" = \"venv\"; then\n  mkdir -p \"$3/bin\"\n  cp \"$CODEXY_TEST_STUB_ROOT/python-in-venv\" \"$3/bin/python\"\n  cp \"$CODEXY_TEST_STUB_ROOT/codexy-mcp-runtime\" \"$3/bin/codexy-mcp-runtime\"\n  cp \"$CODEXY_TEST_STUB_ROOT/getcodexy\" \"$3/bin/getcodexy\"\n  chmod 755 \"$3/bin/python\" \"$3/bin/codexy-mcp-runtime\" \"$3/bin/getcodexy\"\nfi\n",
    )?;
    fs::write(stubs.join("python-in-venv"), "#!/bin/sh\nexit 0\n")?;
    fs::write(
        stubs.join("codexy-mcp-runtime"),
        "#!/bin/sh\nexit 0\n",
    )?;
    fs::write(stubs.join("getcodexy"), fake_getcodexy())?;
    make_executable(&python)?;
    for name in ["python-in-venv", "codexy-mcp-runtime", "getcodexy"] {
        make_executable(&stubs.join(name))?;
    }
    Ok(root)
}

fn fake_getcodexy() -> &'static str {
    r##"#!/bin/sh
set -eu

write_state() {
  mkdir -p "$CODEX_HOME"
  jq -n --arg version "$TARGET_VERSION" \
    '{selection:["core","devtools","github"],versions:{core:$version,devtools:$version,github:$version}}' \
    >"$CODEX_HOME/.codexy-public-proof.json"
  touch "$CODEX_HOME/.codexy-public-marketplace-present"
}

case "${1:-}" in
install)
  write_state
  printf '%s\n' '{"schema":"getcodexy.operation-receipt.v1","outcome":"completed","errors":[],"selection_after":["core","devtools","github"]}'
  ;;
update)
  write_state
  printf '%s\n' '{"schema":"getcodexy.operation-receipt.v1","command":"update","outcome":"completed","errors":[],"selection_after":["core","devtools","github"]}'
  ;;
status)
  printf '%s\n' '{"schema":"getcodexy.status.v1","outcome":"completed","inventory_consistency":"consistent","errors":[],"installed_components":["core","devtools","github"]}'
  ;;
doctor)
  jq -n --arg version "$TARGET_VERSION" '{schema:"getcodexy.doctor.v1",outcome:"completed",inventory_consistency:"consistent",host_readiness:{state:"ready"},errors:[],component_health:( ["core","devtools","github"] | map({healthy:true,state:"healthy",observed:{plugin:{version:$version},runtime:{version:$version}}}) )}'
  ;;
*)
  echo "unexpected getcodexy command" >&2
  exit 1
  ;;
esac
"##
}

fn run_smoke(
    root: &Path,
    upgrade_from_version: Option<&str>,
) -> Result<Output, Box<dyn std::error::Error>> {
    let runner_temp = root.join("runner-temp");
    let stubs = root.join("stubs");
    let mut path = runner_temp.into_os_string();
    path.push(":");
    path.push(std::env::var_os("PATH").ok_or("PATH")?);
    let mut command = Command::new(root.join("scripts/smoke-public-getcodexy-release.sh"));
    command
        .current_dir(root)
        .env("TARGET_VERSION", TARGET_VERSION)
        .env("RUNNER_TEMP", root.join("runner-temp"))
        .env("CODEXY_TEST_STUB_ROOT", stubs)
        .env("PATH", path);
    if let Some(version) = upgrade_from_version {
        command.env("UPGRADE_FROM_VERSION", version);
    } else {
        command.env_remove("UPGRADE_FROM_VERSION");
    }
    Ok(command.output()?)
}

fn make_executable(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

fn git(root: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("git").current_dir(root).args(args).status()?;
    assert!(status.success(), "git command failed: git {args:?}");
    Ok(())
}

fn git_output(root: &Path, args: &[&str]) -> Result<Output, Box<dyn std::error::Error>> {
    Ok(Command::new("git").current_dir(root).args(args).output()?)
}
