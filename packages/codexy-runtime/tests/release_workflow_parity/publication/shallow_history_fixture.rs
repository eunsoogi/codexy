use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const TARGET_VERSION: &str = "1.6.3";
type FixtureResult<T> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn create_fixture(root: PathBuf, getcodexy_stub: &str) -> FixtureResult<PathBuf> {
    // One tagged commit reproduces the shallow activation checkout used by public smoke.
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
    let digest = "0".repeat(64);
    fs::write(
        root.join("public-package-artifacts.tsv"),
        format!(
            "bdist_wheel\thttps://files.pythonhosted.org/wheel\t{digest}\tgetcodexy-{TARGET_VERSION}-py3-none-any.whl\nsdist\thttps://files.pythonhosted.org/sdist\t{digest}\tgetcodexy-{TARGET_VERSION}.tar.gz\n",
        ),
    )?;

    let stubs = root.join("stubs");
    let runner_temp = root.join("runner-temp");
    fs::create_dir_all(&stubs)?;
    fs::create_dir_all(&runner_temp)?;
    fs::write(stubs.join("simple-index-count"), "0\n")?;
    let python = runner_temp.join("python");
    fs::write(
        &python,
        "#!/bin/sh\nset -eu\nif test \"${1:-}\" = \"-m\" && test \"${2:-}\" = \"venv\"; then\n  mkdir -p \"$3/bin\"\n  cp \"$CODEXY_TEST_STUB_ROOT/python-in-venv\" \"$3/bin/python\"\n  cp \"$CODEXY_TEST_STUB_ROOT/codexy-mcp-runtime\" \"$3/bin/codexy-mcp-runtime\"\n  cp \"$CODEXY_TEST_STUB_ROOT/getcodexy\" \"$3/bin/getcodexy\"\n  cp \"$CODEXY_TEST_STUB_ROOT/getcodexy\" \"$3/bin/codexy-github-install\"\n  chmod 755 \"$3/bin/python\" \"$3/bin/codexy-mcp-runtime\" \"$3/bin/getcodexy\"\nfi\n",
    )?;
    fs::write(
        stubs.join("python-in-venv"),
        r##"#!/bin/sh
set -eu

log_event() { printf '%s\n' "$1" >>"$CODEXY_TEST_EVENTS"; }
case "$*" in
  *"--no-index --find-links"*) log_event local-package-install ;;
  *"--index-url https://pypi.org/simple"*)
    log_event public-package-install
    if test "$CODEXY_TEST_FAIL_PUBLIC_INSTALL" = 1; then
      echo "injected unrelated pip failure" >&2
      exit 23
    fi
    ;;
  *uv) log_event bootstrap-pip ;;
  *)
    echo "unexpected pip invocation: $*" >&2
    exit 90
    ;;
esac
"##,
    )?;
    fs::write(stubs.join("codexy-mcp-runtime"), "#!/bin/sh\nexit 0\n")?;
    fs::write(stubs.join("getcodexy"), getcodexy_stub)?;
    // Return two valid stale snapshots before exposing the verified wheel and sdist.
    fs::write(
        stubs.join("curl"),
        r##"#!/bin/sh
set -eu

output= accept= write_out= url=
connect_timeout= max_time=
while test "$#" -gt 0; do
  case "$1" in
    --fail|--silent|--show-error|--location) shift ;;
    --connect-timeout) connect_timeout=$2; shift 2 ;;
    --max-time) max_time=$2; shift 2 ;;
    --header|-H) accept=$2; shift 2 ;;
    --output|-o) output=$2; shift 2 ;;
    --write-out|-w) write_out=$2; shift 2 ;;
    *) url=$1; shift ;;
  esac
done

read -r count <"$CODEXY_TEST_STUB_ROOT/simple-index-count"
count=$((count + 1))
printf '%s\n' "$count" >"$CODEXY_TEST_STUB_ROOT/simple-index-count"
printf 'simple-index:%s\n' "$count" >>"$CODEXY_TEST_EVENTS"
if test "$connect_timeout" != 5 || test "$max_time" != 5 || \
  test "$accept" != 'Accept: application/vnd.pypi.simple.v1+json' || \
  test "$write_out" != '%{content_type}' || \
  test "$url" != 'https://pypi.org/simple/getcodexy/' || test -z "$output"; then
  echo "unexpected Simple Index request" >&2
  exit 88
fi

mode=$CODEXY_TEST_INDEX_MODE
if test "$mode" = delayed && test "$count" -lt 3; then
  mode=missing
fi
digest=$(printf '%064d' 0)
case "$mode" in
  ready|delayed|digest-mismatch)
    if test "$mode" = digest-mismatch; then digest=$(printf '%064d' 1); fi
    jq -n --arg version "$TARGET_VERSION" --arg digest "$digest" '{meta:{"api-version":"1.4"},name:"getcodexy",files:[{filename:("getcodexy-"+$version+"-py3-none-any.whl"),url:"https://files.pythonhosted.org/wheel",hashes:{sha256:$digest}},{filename:("getcodexy-"+$version+".tar.gz"),url:"https://files.pythonhosted.org/sdist",hashes:{sha256:$digest}}]}' >"$output"
    ;;
  missing)
    jq -n --arg digest "$digest" '{meta:{"api-version":"1.4"},name:"getcodexy",files:[{filename:"getcodexy-1.6.2.tar.gz",url:"https://files.pythonhosted.org/old",hashes:{sha256:$digest}}]}' >"$output"
    ;;
  malformed)
    printf '%s\n' '{"meta":{"api-version":"not-an-api-version"},"name":"getcodexy","files":[]}' >"$output"
    ;;
  *)
    echo "unknown Simple Index fixture mode" >&2
    exit 91
    ;;
esac
printf '%s' 'application/vnd.pypi.simple.v1+json'
"##,
    )?;
    fs::write(
        stubs.join("sleep"),
        "#!/bin/sh\nset -eu\nprintf 'sleep:%s\\n' \"$1\" >>\"$CODEXY_TEST_EVENTS\"\n",
    )?;
    for name in [
        "python-in-venv",
        "codexy-mcp-runtime",
        "getcodexy",
        "curl",
        "sleep",
    ] {
        make_executable(&stubs.join(name))?;
    }
    make_executable(&python)?;
    Ok(root)
}

pub(super) fn run_smoke(
    root: &Path,
    upgrade_from_version: Option<&str>,
    index_mode: &str,
    fail_public_install: bool,
    local_distribution: bool,
) -> FixtureResult<Output> {
    let runner_temp = root.join("runner-temp");
    let stubs = root.join("stubs");
    let mut path = runner_temp.into_os_string();
    path.push(":");
    path.push(stubs.into_os_string());
    path.push(":");
    path.push(std::env::var_os("PATH").ok_or("PATH")?);
    let mut command = Command::new(root.join("scripts/smoke-public-getcodexy-release.sh"));
    command
        .current_dir(root)
        .env("TARGET_VERSION", TARGET_VERSION)
        .env("RUNNER_TEMP", root.join("runner-temp"))
        .env("CODEXY_TEST_STUB_ROOT", root.join("stubs"))
        .env("CODEXY_TEST_EVENTS", root.join("events"))
        .env("CODEXY_TEST_INDEX_MODE", index_mode)
        .env(
            "CODEXY_TEST_FAIL_PUBLIC_INSTALL",
            if fail_public_install { "1" } else { "0" },
        )
        .env("PATH", path);
    if local_distribution {
        let distribution = root.join("getcodexy-dist");
        fs::create_dir_all(&distribution)?;
        command.env("GETCODEXY_DIST", distribution);
    } else {
        command.env_remove("GETCODEXY_DIST");
    }
    if let Some(version) = upgrade_from_version {
        command.env("UPGRADE_FROM_VERSION", version);
    } else {
        command.env_remove("UPGRADE_FROM_VERSION");
    }
    Ok(command.output()?)
}

pub(super) fn index_request_count(root: &Path) -> FixtureResult<usize> {
    Ok(fs::read_to_string(root.join("stubs/simple-index-count"))?
        .trim()
        .parse()?)
}

pub(super) fn smoke_events(root: &Path) -> FixtureResult<Vec<String>> {
    Ok(fs::read_to_string(root.join("events"))?
        .lines()
        .map(str::to_owned)
        .collect())
}

fn make_executable(path: &Path) -> FixtureResult<()> {
    Ok(fs::set_permissions(
        path,
        fs::Permissions::from_mode(0o755),
    )?)
}

fn git(root: &Path, args: &[&str]) -> FixtureResult<()> {
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(args)
            .status()?
            .success(),
        "git command failed: git {args:?}"
    );
    Ok(())
}
