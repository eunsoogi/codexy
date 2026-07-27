use std::{env, fs, path::PathBuf, process};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executable = env::current_exe()?;
    let configuration = fs::read_to_string(sidecar_path(&executable))?;
    let mut lines = configuration.lines();
    match lines.next() {
        Some("argv") => echo_arguments(),
        Some("uname") => emulate_uname(lines.next(), lines.next()),
        _ => Err("unknown fixture probe mode".into()),
    }
}

fn sidecar_path(executable: &std::path::Path) -> PathBuf {
    executable.with_extension("fixture")
}

fn echo_arguments() -> Result<(), Box<dyn std::error::Error>> {
    for argument in env::args().skip(1) {
        println!("{argument}");
    }
    if let Ok(stderr) = env::var("CODEXY_FIXTURE_PROBE_STDERR") {
        eprintln!("{stderr}");
    }
    let status = env::var("CODEXY_FIXTURE_PROBE_EXIT")
        .ok()
        .map(|value| value.parse::<i32>())
        .transpose()?
        .unwrap_or(0);
    process::exit(status);
}

fn emulate_uname(
    operating_system: Option<&str>,
    architecture: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    match env::args().nth(1).as_deref() {
        Some("-s") => println!("{}", operating_system.ok_or("missing uname OS")?),
        Some("-m") => println!("{}", architecture.ok_or("missing uname architecture")?),
        _ => process::exit(2),
    }
    Ok(())
}
