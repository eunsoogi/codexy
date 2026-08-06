use std::process::Command;

pub(super) fn audit(
    input: &std::path::Path,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-session-audit"))
        .arg("--input")
        .arg(input)
        .output()?)
}

pub(super) fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
