use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

#[cfg(target_os = "linux")]
use std::ffi::CString;
#[cfg(target_os = "linux")]
use std::os::unix::ffi::OsStrExt as _;

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

#[test]
fn publishes_each_script_at_a_distinct_executable_path() -> TestResult {
    let temp = tempfile::tempdir()?;
    let first = make_script(temp.path(), "exit 11\n")?;
    let second = make_script(temp.path(), "exit 22\n")?;

    assert_ne!(first, second);
    assert!(std::fs::read_to_string(first)?.contains("exit 11"));
    assert!(std::fs::read_to_string(second)?.contains("exit 22"));
    Ok(())
}

#[test]
fn publishes_a_closed_staging_file_to_a_distinct_executable_path() -> TestResult {
    let temp = tempfile::tempdir()?;
    let (script, staging) = make_script_with_publication_paths(temp.path(), "exit 0\n")?;

    assert_ne!(script, staging);
    assert!(
        !staging.exists(),
        "staging path must be removed before execution"
    );
    let status = Command::new(&script).status()?;

    assert!(status.success());
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn linux_rejects_a_published_script_while_its_staging_file_is_writable() -> TestResult {
    let temp = tempfile::tempdir()?;
    let file = stage_script(temp.path(), "exit 0\n")?;
    let staging = file.path().to_path_buf();
    let executable = executable_path_for(&staging)?;

    std::fs::rename(&staging, &executable)?;
    let executable = CString::new(executable.as_os_str().as_bytes())?;
    let child = unsafe { libc::fork() };
    assert_ne!(child, -1, "fork must succeed");
    if child == 0 {
        unsafe {
            let arguments = [executable.as_ptr(), std::ptr::null()];
            libc::execv(executable.as_ptr(), arguments.as_ptr());
            libc::_exit(*libc::__errno_location());
        }
    }
    let mut status = 0;
    assert_eq!(unsafe { libc::waitpid(child, &mut status, 0) }, child);
    assert!(libc::WIFEXITED(status));
    assert_eq!(libc::WEXITSTATUS(status), libc::ETXTBSY);

    drop(file);
    assert!(Command::new(executable.to_str()?).status()?.success());
    Ok(())
}

fn make_script(root: &Path, body: &str) -> TestResult<PathBuf> {
    make_script_with_publication_paths(root, body).map(|(script, _)| script)
}

fn make_script_with_publication_paths(root: &Path, body: &str) -> TestResult<(PathBuf, PathBuf)> {
    let staging = stage_script(root, body)?.into_temp_path();
    let executable = executable_path_for(&staging)?;
    if executable.exists() {
        return Err(format!(
            "refusing to replace existing probe: {}",
            executable.display()
        )
        .into());
    }
    std::fs::rename(&staging, &executable)?;
    std::fs::File::open(root)?.sync_all()?;
    Ok((executable, staging.to_path_buf()))
}

fn stage_script(root: &Path, body: &str) -> TestResult<tempfile::NamedTempFile> {
    let mut file = tempfile::Builder::new()
        .prefix(".probe-staging-")
        .suffix(".tmp")
        .tempfile_in(root)?;
    file.write_all(format!("#!/bin/sh\n{body}").as_bytes())?;
    file.as_file().sync_all()?;
    let mut permissions = file.as_file().metadata()?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(file.path(), permissions)?;
    Ok(file)
}

fn executable_path_for(staging: &Path) -> TestResult<PathBuf> {
    let suffix = staging
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix(".probe-staging-"))
        .and_then(|name| name.strip_suffix(".tmp"))
        .ok_or("invalid probe staging path")?;
    Ok(staging.with_file_name(format!(".probe-{suffix}.sh")))
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
