use std::{
    cell::Cell,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::support::write_posix_fixture_command;

use super::{
    command, metadata, real_fixture_seed, real_source_pointer, receipt::receipt_value, shell_runner,
};

pub(super) struct Fixture {
    _temp: tempfile::TempDir,
    pub(super) repo: PathBuf,
    pub(super) receipt: PathBuf,
    pub(super) bin: PathBuf,
    pub(super) activator: PathBuf,
    pub(super) command_trace: PathBuf,
    pub(super) runner: PathBuf,
    external_activation_process_invocations: Cell<usize>,
}

impl Fixture {
    pub(super) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let prepared = real_fixture_seed::materialize()?;
        let temp = prepared.temp;
        let repo = prepared.repo;
        let candidate = prepared.candidate;
        git(&repo, &["switch", "-c", "activation"])?;
        metadata::select_current_bootstrap(&repo)?;
        let receipt = temp.path().join("receipt.json");
        fs::write(&receipt, serde_json::to_vec(&receipt_value())?)?;
        let bin = temp.path().join("bin");
        fs::create_dir(&bin)?;
        let activator = bin.join("activate-current-bootstrap");
        write_posix_fixture_command(&activator, ACTIVATOR)?;
        let command_trace = temp.path().join("command-trace");
        write_posix_fixture_command(
            &bin.join("cargo"),
            "#!/bin/sh\nprintf 'cargo\\n' >> \"$CODEXY_FIXTURE_COMMAND_TRACE\"\nexit 97\n",
        )?;
        let external_activation_process_invocations = Cell::new(0);
        codexy_runtime::version::activation::activate(&repo, &candidate, &receipt)?;
        real_source_pointer::assert_activated_source_pointer(&repo, &candidate)?;
        let mut sync = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"));
        sync.args(["--version", &candidate])
            .current_dir(&repo)
            .env("CODEXY_REPO_ROOT", &repo);
        command::run(&mut sync)?;
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

    pub(super) fn external_activation_process_invocations(&self) -> usize {
        self.external_activation_process_invocations.get()
    }

    pub(super) fn cargo_invocations(&self) -> Result<usize, Box<dyn std::error::Error>> {
        match fs::read_to_string(&self.command_trace) {
            Ok(trace) => Ok(trace.lines().filter(|command| *command == "cargo").count()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
            Err(error) => Err(error.into()),
        }
    }

    pub(super) fn record_external_activation_process_invocation(&self) {
        self.external_activation_process_invocations
            .set(self.external_activation_process_invocations.get() + 1);
    }
}

fn git(root: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    run(Command::new("git").args(args).current_dir(root))
}

fn run(process: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    command::run(process)
}

const ACTIVATOR: &str = "#!/bin/sh\nset -eu\nroot=\nprevious=\nfor argument in \"$@\"; do\n  if [ \"$previous\" = --repo-root ]; then root=\"$argument\"; break; fi\n  previous=\"$argument\"\ndone\ntest -n \"$root\"\npython3 - \"$root\" <<'PY'\nimport json\nimport os\nimport pathlib\nimport shutil\nimport sys\nroot = pathlib.Path(sys.argv[1])\ncontract = root / '.agents/plugins/release-publish-contract.json'\nbootstrap_source = pathlib.Path(os.environ['CODEXY_TEST_BOOTSTRAP_SOURCE'])\nbootstrap = bootstrap_source.read_text()\nselected_version = next(\n    line.split('= ', 1)[1].strip().strip(';').strip(chr(34))\n    for line in bootstrap.splitlines()\n    if line.startswith('pub(super) const VERSION: &str = ')\n)\nruntime_release = json.loads((root / 'plugins/codexy-devtools/runtime-release.json').read_text())\nselected_tag = runtime_release['artifact']['tag']\ndata = json.loads(contract.read_text())\ndata['bootstrap']['selectedVersion'] = selected_version\ndata['runtime']['selectedTag'] = selected_tag\ncontract.write_text(json.dumps(data, indent=2) + '\\n')\nshutil.copyfile(bootstrap_source, root / 'packages/codexy-runtime/src/version/bootstrap.rs')\nPY\nif ! test -d \"$root/.git\"; then\n  git -C \"$root\" init -b main >/dev/null\n  git -C \"$root\" config user.name test\n  git -C \"$root\" config user.email test@example.com\n  git -C \"$root\" add .\n  git -C \"$root\" commit -m pre-671-archive >/dev/null\nfi\nexec \"$CODEXY_TEST_ACTIVATE_RUNTIME_BINARY\" \"$@\"\n";
