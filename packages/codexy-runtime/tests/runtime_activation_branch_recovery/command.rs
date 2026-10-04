use std::process::Command;

// Include stderr when reporting a fixture command failure so callers retain its actionable diagnostics.
pub(super) fn run(command: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    let output = command.output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned().into())
    }
}
