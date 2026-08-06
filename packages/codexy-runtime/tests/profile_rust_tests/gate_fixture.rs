use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) struct GateFixture {
    pub(crate) temp: tempfile::TempDir,
    pub(crate) marker: PathBuf,
    pub(crate) cwd_marker: PathBuf,
    bin_dir: PathBuf,
    pub(crate) workflow: PathBuf,
}

const WORKFLOW_ROOT: &str =
    "name: Rust tests\n\non:\n  pull_request:\n  push:\n    branches: [main]\n\npermissions:\n  contents: read\n\n";

const SHARDED_WORKFLOW: &str = "jobs:\n  rust-test:\n    name: Rust shard (Ubuntu, ${{ matrix.shard }})\n    runs-on: ubuntu-latest\n    timeout-minutes: 6\n    strategy:\n      fail-fast: false\n      max-parallel: 7\n      matrix:\n        shard: [support, agent, child, orchestration, governance, system, archive]\n    steps:\n      - uses: actions/checkout@v7\n        with:\n          fetch-depth: 0\n          persist-credentials: false\n      - run: sudo apt-get update && sudo apt-get install --yes ripgrep\n      - run: scripts/profile-rust-tests --shard ${{ matrix.shard }} --receipt receipts/posix-${{ matrix.shard }}.json\n      - if: always()\n        uses: actions/upload-artifact@v7\n        with:\n          name: rust-receipt-posix-${{ matrix.shard }}\n          path: receipts/posix-${{ matrix.shard }}.json\n          if-no-files-found: error\n  windows-rust-test:\n    name: Rust shard (Windows, ${{ matrix.shard }})\n    runs-on: windows-latest\n    timeout-minutes: 20\n    strategy:\n      fail-fast: false\n      max-parallel: 7\n      matrix:\n        shard: [support, agent, child, orchestration, governance, system, archive]\n    steps:\n      - uses: actions/checkout@v7\n        with:\n          fetch-depth: 0\n          persist-credentials: false\n      - shell: pwsh\n        run: scripts/install-windows-test-prerequisites.ps1\n      - shell: pwsh\n        run: rustup toolchain install; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }; cargo fetch --locked; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }\n      - run: python scripts/profile-rust-tests --windows --shard ${{ matrix.shard }} --receipt receipts/windows-${{ matrix.shard }}.json\n      - if: always()\n        uses: actions/upload-artifact@v7\n        with:\n          name: rust-receipt-windows-${{ matrix.shard }}\n          path: receipts/windows-${{ matrix.shard }}.json\n          if-no-files-found: error\n  rust-test-aggregate:\n    needs: [rust-test, windows-rust-test]\n    if: always()\n    runs-on: ubuntu-latest\n    timeout-minutes: 6\n    steps:\n      - uses: actions/checkout@v7\n        with:\n          fetch-depth: 0\n          persist-credentials: false\n      - uses: actions/download-artifact@v8\n        with:\n          pattern: rust-receipt-*\n          merge-multiple: true\n          path: receipts\n      - run: scripts/profile-rust-tests --aggregate-receipts receipts\n";

impl GateFixture {
    pub(crate) fn new(
        exit: i32,
        passed: usize,
        ignored: usize,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let bin_dir = temp.path().join("bin");
        std::fs::create_dir(&bin_dir)?;
        let marker = temp.path().join("workloads");
        let cwd_marker = temp.path().join("workload-cwds");
        let cargo = bin_dir.join("cargo");
        write_executable(
            &cargo,
            &format!(
                "#!/bin/sh\nif [ \"$1\" = metadata ]; then\n  printf '%s\\n' '{{\"packages\":[{{\"targets\":[{{\"kind\":[\"test\"]}},{{\"kind\":[\"test\"]}}]}}]}}'\n  exit 0\nfi\nprintf '%s\\n' \"$*\" >> \"$PROFILE_MARKER\"\nprintf '%s\\n' \"$(pwd)\" >> \"$PROFILE_CWD_MARKER\"\nprintf '%s\\n' 'Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 2.00s'\nprintf '%s\\n' 'test result: ok. {passed} passed; 0 failed; {ignored} ignored; 0 measured; 0 filtered out; finished in 1.00s'\nexit {exit}\n"
            ),
        )?;
        let workflow = temp.path().join("rust-test.yml");
        let checkout = "          ref: ${{ github.event.pull_request.head.sha }}\n          fetch-depth: 0\n          persist-credentials: false";
        std::fs::write(
            &workflow,
            format!(
                "{WORKFLOW_ROOT}{}",
                SHARDED_WORKFLOW.replace(
                    "cargo fetch --locked",
                    "cargo fetch --manifest-path packages/codexy-runtime/Cargo.toml --locked",
                ).replace(
                    "          fetch-depth: 0\n          persist-credentials: false",
                    checkout,
                )
            ),
        )?;
        Ok(Self {
            temp,
            marker,
            cwd_marker,
            bin_dir,
            workflow,
        })
    }

    pub(crate) fn run(
        &self,
        environment: &[(&str, &std::ffi::OsStr)],
    ) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        self.run_from_root(codexy_runtime::paths::repository_root(), environment)
    }

    pub(crate) fn run_from_root(
        &self,
        root: &Path,
        environment: &[(&str, &std::ffi::OsStr)],
    ) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        self.run_from_root_with_required_windows_job(root, environment, true)
    }

    pub(crate) fn run_without_required_windows_job(
        &self,
        environment: &[(&str, &std::ffi::OsStr)],
    ) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        self.run_from_root_with_required_windows_job(
            codexy_runtime::paths::repository_root(),
            environment,
            false,
        )
    }

    fn run_from_root_with_required_windows_job(
        &self,
        root: &Path,
        environment: &[(&str, &std::ffi::OsStr)],
        include_windows_job: bool,
    ) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        if !include_windows_job {
            let workflow = std::fs::read_to_string(&self.workflow)?;
            if let Some((producers, remainder)) = workflow.split_once("  windows-rust-test:\n") {
                let (_, aggregate) = remainder
                    .split_once("  rust-test-aggregate:\n")
                    .ok_or("missing aggregate fixture job")?;
                std::fs::write(
                    &self.workflow,
                    format!("{producers}  rust-test-aggregate:\n{aggregate}"),
                )?;
            }
        }
        let path = format!("{}:{}", self.bin_dir.display(), std::env::var("PATH")?);
        let mut command = Command::new("python3");
        command
            .arg(codexy_runtime::paths::repository_root().join("scripts/profile-rust-tests"))
            .args(["--root", root.to_str().ok_or("profile root")?, "--workflow-file"])
            .arg(&self.workflow)
            .env("PATH", path)
            .env("PROFILE_MARKER", &self.marker)
            .env("PROFILE_CWD_MARKER", &self.cwd_marker);
        for (key, value) in environment {
            if *key == "EXTRA_ARGUMENT" {
                command.arg(value);
            } else {
                command.env(key, value);
            }
        }
        Ok(command.output()?)
    }
}

fn write_executable(path: &Path, contents: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::write(path, contents)?;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}
