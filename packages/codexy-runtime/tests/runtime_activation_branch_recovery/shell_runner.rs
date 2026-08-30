use std::{
    io,
    path::{Path, PathBuf},
    process::Output,
};

use crate::support::{FixtureCommand, write_single_posix_fixture_shell_runner};

pub(super) fn write_activation_verifier_runner(temp_root: &Path) -> io::Result<PathBuf> {
    let runner = temp_root.join("bound-verify-runtime-activation-branch.sh");
    write_single_posix_fixture_shell_runner(
        &runner,
        "CODEXY_FIXTURE_VERIFY_RUNTIME_ACTIVATION_BRANCH",
        "gh",
        "CODEXY_FIXTURE_GH",
    )?;
    Ok(runner)
}

impl super::Fixture {
    pub(super) fn verify(
        &self,
        base: &str,
        version: &str,
    ) -> Result<Output, Box<dyn std::error::Error>> {
        let mut path = vec![self.bin.clone()];
        path.extend(std::env::split_paths(
            &std::env::var_os("PATH").ok_or("PATH")?,
        ));
        let mut command = FixtureCommand::new(&self.runner);
        command
            .args(["activation", base, version])
            .arg(&self.receipt)
            .current_dir(&self.repo);
        command
            .env("CODEXY_TEST_MODE", "1")
            .envs([
                ("GIT_CONFIG_COUNT", "2"),
                ("GIT_CONFIG_KEY_0", "maintenance.auto"),
                ("GIT_CONFIG_VALUE_0", "false"),
                ("GIT_CONFIG_KEY_1", "gc.auto"),
                ("GIT_CONFIG_VALUE_1", "0"),
            ])
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
}
