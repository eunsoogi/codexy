use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use super::process::{MAX_HOOK_OUTPUT_BYTES, output_with_timeout};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn bounds_hook_execution() -> TestResult {
    let temp = tempfile::tempdir()?;
    let script = make_script(temp.path(), "sleep 30\n")?;
    let started = Instant::now();
    let output = output_with_timeout(&script, "timeout probe", &[], Duration::from_millis(100))?;
    assert!(started.elapsed() < Duration::from_secs(3));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("timed out"));
    Ok(())
}

#[test]
fn bounds_output_collection_from_background_descendants() -> TestResult {
    let temp = tempfile::tempdir()?;
    let script = make_script(
        temp.path(),
        "(trap '' TERM; sleep 30) &\nprintf 'complete\\n'\n",
    )?;
    let started = Instant::now();
    let output = output_with_timeout(&script, "pipe probe", &[], Duration::from_secs(2))?;
    assert!(output.status.success());
    assert!(started.elapsed() < Duration::from_secs(3));
    Ok(())
}

#[test]
fn kills_lingering_background_descendants() -> TestResult {
    let temp = tempfile::tempdir()?;
    let marker = format!("codexy-hook-descendant-{}", std::process::id());
    let script = make_script(
        temp.path(),
        &format!(
            "sh -c 'trap \"\" TERM HUP; while :; do sleep 1; done' {marker} &\nprintf 'complete\\n'\n"
        ),
    )?;
    let output = output_with_timeout(&script, "descendant probe", &[], Duration::from_secs(2))?;
    assert!(output.status.success());
    let pids = matching_pids(&marker)?;
    kill_all(&pids);
    assert!(pids.is_empty(), "hook descendants remain: {pids:?}");
    Ok(())
}

#[test]
fn kills_redirected_background_descendants() -> TestResult {
    let temp = tempfile::tempdir()?;
    let marker = format!("codexy-hook-redirected-descendant-{}", std::process::id());
    let script = make_script(
        temp.path(),
        &format!(
            "sh -c 'trap \"\" TERM HUP; : {marker}; exec 1<&- 2<&-; while :; do sleep 1; done' &\nprintf 'complete\\n'\n"
        ),
    )?;
    let output = output_with_timeout(&script, "redirected probe", &[], Duration::from_secs(2))?;
    assert!(output.status.success());
    let pids = matching_pids(&marker)?;
    kill_all(&pids);
    assert!(
        pids.is_empty(),
        "redirected hook descendants remain: {pids:?}"
    );
    Ok(())
}

#[test]
fn bounds_continuous_hook_output() -> TestResult {
    let temp = tempfile::tempdir()?;
    let script = make_script(temp.path(), "yes noisy-output\n")?;
    let output = output_with_timeout(&script, "output probe", &[], Duration::from_secs(2))?;
    assert!(!output.status.success());
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(
        message.contains("output exceeded") || message.contains("timed out"),
        "unexpected error: {message}"
    );
    assert!(MAX_HOOK_OUTPUT_BYTES <= 1024 * 1024);
    Ok(())
}

fn make_script(root: &Path, body: &str) -> TestResult<PathBuf> {
    let script = root.join("probe.sh");
    let staging = root.join(".probe.sh.tmp");
    let mut file = std::fs::File::create(&staging)?;
    file.write_all(format!("#!/bin/sh\n{body}").as_bytes())?;
    file.sync_all()?;
    drop(file);
    let mut permissions = std::fs::metadata(&staging)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&staging, permissions)?;
    std::fs::rename(staging, &script)?;
    std::fs::File::open(root)?.sync_all()?;
    Ok(script)
}

#[cfg(target_os = "linux")]
#[test]
fn replaces_a_running_script_without_writing_its_executable_path() -> TestResult {
    use std::io::BufRead as _;
    use std::process::Stdio;

    let temp = tempfile::tempdir()?;
    let script = make_script(temp.path(), "printf 'ready\\n'\nread _ || true\n")?;
    let mut child = Command::new(&script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let mut ready = String::new();
    std::io::BufReader::new(child.stdout.take().ok_or("missing child stdout")?)
        .read_line(&mut ready)?;
    assert_eq!(ready, "ready\n");

    let replacement = make_script(temp.path(), "exit 0\n")?;
    drop(child.stdin.take());
    assert!(child.wait()?.success());
    assert!(Command::new(replacement).status()?.success());
    Ok(())
}

fn matching_pids(marker: &str) -> TestResult<Vec<i32>> {
    let output = Command::new("pgrep").args(["-f", marker]).output()?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse::<i32>().ok())
        .filter(|pid| *pid != std::process::id() as i32)
        .collect())
}

fn kill_all(pids: &[i32]) {
    for pid in pids {
        unsafe {
            let _ = libc::kill(*pid, libc::SIGKILL);
        }
    }
}
