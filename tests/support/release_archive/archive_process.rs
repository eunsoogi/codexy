use std::{
    fs::File,
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

const ARCHIVE_PROCESS_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) fn create_archive(
    root: &std::path::Path,
    archive: &std::path::Path,
) -> std::io::Result<()> {
    create_archive_with_commands(root, archive, "tar", "gzip", ARCHIVE_PROCESS_TIMEOUT)
}

pub(crate) fn create_archive_with_commands(
    root: &std::path::Path,
    archive: &std::path::Path,
    tar_command: &str,
    gzip_command: &str,
    timeout: Duration,
) -> std::io::Result<()> {
    let archive_file = File::create(archive)?;
    let mut tar = Command::new(tar_command)
        .args(["-C"])
        .arg(root)
        .args(["-cf", "-", "plugins/codexy"])
        .stdout(Stdio::piped())
        .spawn()?;
    let tar_stdout = match tar.stdout.take() {
        Some(stdout) => stdout,
        None => {
            reap_archive_process(&mut tar);
            return Err(std::io::Error::other("tar stdout unavailable"));
        }
    };
    let mut gzip = match Command::new(gzip_command)
        .args(["-1", "-c"])
        .stdin(Stdio::from(tar_stdout))
        .stdout(archive_file)
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            reap_archive_process(&mut tar);
            return Err(error);
        }
    };
    let gzip_status = match wait_for_archive_process(&mut gzip, "gzip", timeout) {
        Ok(status) => status,
        Err(error) => {
            reap_archive_process(&mut tar);
            return Err(error);
        }
    };
    let tar_status = wait_for_archive_process(&mut tar, "tar", timeout)?;
    if !gzip_status.success() {
        return Err(std::io::Error::other(format!("gzip failed: {gzip_status}")));
    }
    if !tar_status.success() {
        return Err(std::io::Error::other(format!("tar failed: {tar_status}")));
    }
    Ok(())
}

fn wait_for_archive_process(
    child: &mut Child,
    name: &str,
    timeout: Duration,
) -> std::io::Result<ExitStatus> {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if started.elapsed() >= timeout {
            reap_archive_process(child);
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("{name} timed out after {} seconds", timeout.as_secs_f32()),
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn reap_archive_process(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}
