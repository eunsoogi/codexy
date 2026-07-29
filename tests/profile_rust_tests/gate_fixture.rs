use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) struct GateFixture {
    pub(crate) temp: tempfile::TempDir,
    pub(crate) marker: PathBuf,
    bin_dir: PathBuf,
    pub(crate) workflow: PathBuf,
}

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
        let cargo = bin_dir.join("cargo");
        write_executable(
            &cargo,
            &format!(
                "#!/bin/sh\nif [ \"$1\" = metadata ]; then\n  printf '%s\\n' '{{\"packages\":[{{\"targets\":[{{\"kind\":[\"test\"]}},{{\"kind\":[\"test\"]}}]}}]}}'\n  exit 0\nfi\nprintf '%s\\n' \"$*\" >> \"$PROFILE_MARKER\"\nprintf '%s\\n' 'Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 2.00s'\nprintf '%s\\n' 'test result: ok. {passed} passed; 0 failed; {ignored} ignored; 0 measured; 0 filtered out; finished in 1.00s'\nexit {exit}\n"
            ),
        )?;
        let workflow = temp.path().join("rust-test.yml");
        std::fs::write(
            &workflow,
            "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n",
        )?;
        Ok(Self {
            temp,
            marker,
            bin_dir,
            workflow,
        })
    }

    pub(crate) fn run(
        &self,
        environment: &[(&str, &std::ffi::OsStr)],
    ) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        self.run_with_required_windows_job(environment, true)
    }

    pub(crate) fn run_without_required_windows_job(
        &self,
        environment: &[(&str, &std::ffi::OsStr)],
    ) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        self.run_with_required_windows_job(environment, false)
    }

    fn run_with_required_windows_job(
        &self,
        environment: &[(&str, &std::ffi::OsStr)],
        include_windows_job: bool,
    ) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        if include_windows_job {
            let mut workflow = std::fs::read_to_string(&self.workflow)?;
            if !workflow.contains("runs-on: ubuntu-latest") {
                workflow = workflow.replacen(
                    "  rust-test:\n",
                    "  rust-test:\n    runs-on: ubuntu-latest\n",
                    1,
                );
            }
            if !workflow.contains("windows-rust-test:") {
                workflow.push_str(
                    "  windows-rust-test:\n    runs-on: windows-latest\n    timeout-minutes: 10\n    steps:\n      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: python scripts/profile-rust-tests --windows\n",
                );
            }
            std::fs::write(&self.workflow, workflow)?;
        }
        let path = format!("{}:{}", self.bin_dir.display(), std::env::var("PATH")?);
        let mut command = Command::new("python3");
        command
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-rust-tests"))
            .args(["--root", env!("CARGO_MANIFEST_DIR"), "--workflow-file"])
            .arg(&self.workflow)
            .env("PATH", path)
            .env("PROFILE_MARKER", &self.marker);
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
