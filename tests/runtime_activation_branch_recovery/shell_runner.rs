use std::{io, path::{Path, PathBuf}};

use crate::support::write_single_posix_fixture_shell_runner;

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
