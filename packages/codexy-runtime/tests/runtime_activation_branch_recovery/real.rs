use std::{
    cell::Cell,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use crate::support::{
    FixtureCommand, write_posix_fixture_command,
};

mod metadata;
mod receipt;
mod shell_runner;
mod command;

use command::run as command;
use receipt::receipt_value;

#[test]
fn real_pre_671_committed_tree_authenticates_retry_and_metadata_matrix()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let candidate = metadata::current_candidate_version()?;
    metadata::assert_canonical_default_prompt(&fixture.repo)?;
    metadata::assert_canonical_preserved_eol(&fixture.repo)?;
    assert_result(fixture.verify("main", &candidate)?, true, "exact retry");
    assert_eq!(
        fixture.cargo_invocations()?,
        0,
        "the verifier must use the injected prebuilt sync binary instead of cargo",
    );
    assert_eq!(
        fixture.external_activation_process_invocations(),
        1,
        "real matrix must retain only the successful verifier activation process",
    );
    Ok(())
}

#[test]
fn real_base_activator_preserves_candidate_bytes_with_autocrlf()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let candidate = metadata::current_candidate_version()?;
    metadata::enable_autocrlf(&fixture.repo)?;
    metadata::assert_canonical_preserved_eol(&fixture.repo)?;
    assert_result(fixture.verify("main", &candidate)?, true, "autocrlf retry");
    Ok(())
}

fn assert_result(output: Output, success: bool, case: &str) {
    assert_eq!(
        output.status.success(),
        success,
        "unexpected {case} result\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

struct Fixture {
    _temp: tempfile::TempDir,
    repo: PathBuf,
    receipt: PathBuf,
    bin: PathBuf,
    activator: PathBuf,
    command_trace: PathBuf,
    runner: PathBuf,
    external_activation_process_invocations: Cell<usize>,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path().join("repo with spaces");
        let archive = temp.path().join("repo.tar");
        let pre_activation_revision = metadata::pre_activation_revision()?;
        fs::create_dir(&repo)?;
        command(
            Command::new("git")
                .args(["archive", "--format=tar", &pre_activation_revision])
                .arg("-o")
                .arg(&archive)
                .current_dir(codexy_runtime::paths::repository_root()),
        )?;
        command(
            Command::new("tar")
                .arg("-xf")
                .arg(&archive)
                .arg("-C")
                .arg(&repo),
        )?;
        let runtime = repo.join("packages/codexy-runtime");
        fs::create_dir_all(runtime.join("src/version"))?;
        let suite = runtime.join("tests/suites/all.rs");
        fs::create_dir_all(suite.parent().ok_or("suite parent")?)?;
        fs::copy(
            codexy_runtime::paths::runtime_package_root().join("tests/suites/all.rs"),
            suite,
        )?;
        for relative in ["Cargo.toml", "Cargo.lock"] {
            let path = repo.join(relative);
            if path.exists() {
                fs::remove_file(path)?;
            }
        }
        metadata::synchronize_current_plugin_validation_inputs(&repo)?;
        metadata::make_uv_lock_stale(&repo)?;
        let workflow = ".github/workflows/plugin-runtime-binaries.yml";
        let workflow_target = repo.join(workflow);
        fs::create_dir_all(workflow_target.parent().ok_or("workflow parent")?)?;
        fs::copy(
            codexy_runtime::paths::repository_root().join(workflow),
            workflow_target,
        )?;
        for relative in [
            "scripts/activate-runtime-contract.sh",
            "scripts/sync-plugin-version.sh",
            "scripts/verify-runtime-activation-branch",
        ] {
            fs::copy(
                codexy_runtime::paths::repository_root().join(relative),
                repo.join(relative),
            )?;
        }
        git(&repo, &["init", "-b", "main"])?;
        git(&repo, &["config", "user.name", "test"])?;
        git(&repo, &["config", "user.email", "test@example.com"])?;
        git(&repo, &["add", "."])?;
        git(&repo, &["commit", "-m", "base"])?;
        git(&repo, &["switch", "-c", "activation"])?;
        metadata::select_current_bootstrap(&repo)?;
        let candidate = metadata::current_candidate_version()?;
        let receipt = temp.path().join("receipt.json");
        fs::write(&receipt, serde_json::to_vec(&core_aware_receipt_value()?)?)?;
        let bin = temp.path().join("bin");
        fs::create_dir(&bin)?;
        let activator = bin.join("activate-current-bootstrap");
        write_posix_fixture_command(
            &activator,
            "#!/bin/sh\nset -eu\nroot=\nprevious=\nfor argument in \"$@\"; do\n  if [ \"$previous\" = --repo-root ]; then root=\"$argument\"; break; fi\n  previous=\"$argument\"\ndone\ntest -n \"$root\"\npython3 - \"$root\" <<'PY'\nimport json\nimport os\nimport pathlib\nimport shutil\nimport sys\nroot = pathlib.Path(sys.argv[1])\ncontract = root / '.agents/plugins/release-publish-contract.json'\nbootstrap_source = pathlib.Path(os.environ['CODEXY_TEST_BOOTSTRAP_SOURCE'])\nbootstrap = bootstrap_source.read_text()\nselected_version = next(\n    line.split('= ', 1)[1].strip().strip(';').strip(chr(34))\n    for line in bootstrap.splitlines()\n    if line.startswith('pub(super) const VERSION: &str = ')\n)\nruntime_release = json.loads((root / 'plugins/codexy-devtools/runtime-release.json').read_text())\nselected_tag = runtime_release['artifact']['tag']\ndata = json.loads(contract.read_text())\ndata['bootstrap']['selectedVersion'] = selected_version\ndata['runtime']['selectedTag'] = selected_tag\ncontract.write_text(json.dumps(data, indent=2) + '\\n')\nshutil.copyfile(bootstrap_source, root / 'packages/codexy-runtime/src/version/bootstrap.rs')\nPY\nif ! test -d \"$root/.git\"; then\n  git -C \"$root\" init -b main >/dev/null\n  git -C \"$root\" config user.name test\n  git -C \"$root\" config user.email test@example.com\n  git -C \"$root\" add .\n  git -C \"$root\" commit -m pre-671-archive >/dev/null\nfi\nexec \"$CODEXY_TEST_ACTIVATE_RUNTIME_BINARY\" \"$@\"\n",
        )?;
        let command_trace = temp.path().join("command-trace");
        write_posix_fixture_command(
            &bin.join("cargo"),
            "#!/bin/sh\nprintf 'cargo\\n' >> \"$CODEXY_FIXTURE_COMMAND_TRACE\"\nexit 97\n",
        )?;
        let external_activation_process_invocations = Cell::new(0);
        codexy_runtime::version::activation::activate(&repo, &candidate, &receipt)?;
        let mut sync = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"));
        sync.args(["--version", &candidate])
            .current_dir(&repo)
            .env("CODEXY_REPO_ROOT", &repo);
        command(&mut sync)?;
        git(
            &repo,
            &[
                "add",
                ".agents/plugins",
                "plugins/codexy",
                "plugins/codexy-devtools",
                "plugins/codexy-github",
                "packages/getcodexy/pyproject.toml",
                "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json",
                "packages/getcodexy/uv.lock",
                "packages/codexy-runtime/src/version/bootstrap.rs",
            ],
        )?;
        git(
            &repo,
            &[
                "add",
                "packages/codexy-runtime/Cargo.toml",
                "packages/codexy-runtime/Cargo.lock",
            ],
        )?;
        git(&repo, &["commit", "-m", "activation"])?;
        write_posix_fixture_command(&bin.join("gh"), "#!/bin/sh\nprintf 'OPEN\\n'\n")?;
        let runner = shell_runner::write_activation_verifier_runner(temp.path())?;
        Ok(Self {
            _temp: temp,
            repo,
            receipt,
            bin,
            activator,
            command_trace,
            runner,
            external_activation_process_invocations,
        })
    }

    fn verify(&self, base: &str, version: &str) -> Result<Output, Box<dyn std::error::Error>> {
        let mut path = vec![self.bin.clone()];
        path.extend(std::env::split_paths(&std::env::var_os("PATH").ok_or("PATH")?));
        let mut command = FixtureCommand::new(&self.runner);
        command
            .args(["activation", base, version])
            .arg(&self.receipt)
            .current_dir(&self.repo);
        command
            .env("CODEXY_TEST_MODE", "1")
            .env("GIT_CONFIG_COUNT", "2")
            .env("GIT_CONFIG_KEY_0", "maintenance.auto")
            .env("GIT_CONFIG_VALUE_0", "false")
            .env("GIT_CONFIG_KEY_1", "gc.auto")
            .env("GIT_CONFIG_VALUE_1", "0")
            .env_path(
                "CODEXY_FIXTURE_VERIFY_RUNTIME_ACTIVATION_BRANCH",
                self.repo.join("scripts/verify-runtime-activation-branch"),
            )
            .env_path("CODEXY_FIXTURE_GH", self.bin.join("gh"))
            .env_path("CODEXY_FIXTURE_COMMAND_TRACE", &self.command_trace)
            .env_path(
                "CODEXY_TEST_ACTIVATE_RUNTIME_BINARY",
                env!("CARGO_BIN_EXE_codexy-activate-runtime"),
            )
            .env_path("CODEXY_TEST_ACTIVATE_RUNTIME", &self.activator)
            .env_path(
                "CODEXY_TEST_BOOTSTRAP_SOURCE",
                codexy_runtime::paths::runtime_package_root().join("src/version/bootstrap.rs"),
            )
            .env_path(
                "CODEXY_TEST_SYNC_VERSION_BINARY",
                env!("CARGO_BIN_EXE_codexy-sync-version"),
            )
            .env_path_list("PATH", path);
        let output = command.output()?;
        self.record_external_activation_process_invocation();
        Ok(output)
    }

    fn external_activation_process_invocations(&self) -> usize {
        self.external_activation_process_invocations.get()
    }

    fn cargo_invocations(&self) -> Result<usize, Box<dyn std::error::Error>> {
        match fs::read_to_string(&self.command_trace) {
            Ok(trace) => Ok(trace.lines().filter(|command| *command == "cargo").count()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
            Err(error) => Err(error.into()),
        }
    }

    fn record_external_activation_process_invocation(&self) {
        self.external_activation_process_invocations
            .set(self.external_activation_process_invocations.get() + 1);
    }
}

fn core_aware_receipt_value() -> Result<Value, Box<dyn std::error::Error>> {
    let mut receipt = receipt_value();
    let candidate = receipt
        .get_mut("candidate")
        .and_then(Value::as_object_mut)
        .ok_or("candidate object")?;
    candidate
        .get_mut("source")
        .and_then(Value::as_object_mut)
        .ok_or("candidate source object")?
        .insert("tree".to_owned(), Value::String("b".repeat(40)));

    let mut handoff_platforms = serde_json::Map::new();
    for platform in ["darwin-arm64", "linux-x86_64", "windows-x86_64"] {
        let extension = if platform == "windows-x86_64" { "exe" } else { "bin" };
        let kind = match platform {
            "darwin-arm64" => "mach-o",
            "linux-x86_64" => "elf",
            "windows-x86_64" => "pe",
            _ => unreachable!(),
        };
        handoff_platforms.insert(
            platform.to_owned(),
            json!({
                "path": format!("runtime/codexy-handoff-validate-{platform}.{extension}"),
                "sha256": "d".repeat(64),
                "kind": kind,
            }),
        );
    }
    let platforms = candidate
        .get("platforms")
        .cloned()
        .ok_or("candidate platforms")?;
    candidate.insert(
        "classes".to_owned(),
        json!({
            "devtoolsMcp": {"platforms": platforms},
            "coreHandoff": {
                "manifest": {"path": "handoff-runtime.json", "sha256": "c".repeat(64)},
                "platforms": handoff_platforms,
            },
        }),
    );
    let canonical_candidate = canonical(Value::Object(candidate.clone()));
    let candidate_bytes = serde_json::to_vec(&canonical_candidate)?;
    receipt["artifact"]["payloadManifestSha256"] = Value::String(
        format!("{:x}", Sha256::digest(candidate_bytes)),
    );
    Ok(receipt)
}

fn canonical(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries = map.into_iter().collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, canonical(value)))
                    .collect(),
            )
        }
        Value::Array(values) => Value::Array(values.into_iter().map(canonical).collect()),
        other => other,
    }
}

fn git(root: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    command(Command::new("git").args(args).current_dir(root))
}
