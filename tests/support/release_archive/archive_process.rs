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
    #[cfg(windows)]
    {
        return create_windows_archive_with_commands(
            root,
            archive,
            tar_command,
            gzip_command,
            timeout,
        );
    }
    let archive_file = File::create(archive)?;
    let mut tar = Command::new(tar_command)
        .env("COPYFILE_DISABLE", "1")
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

#[cfg(windows)]
fn create_windows_archive_with_commands(
    root: &std::path::Path,
    archive: &std::path::Path,
    tar_command: &str,
    gzip_command: &str,
    timeout: Duration,
) -> std::io::Result<()> {
    let temporary = tempfile::NamedTempFile::new_in(
        archive
            .parent()
            .ok_or_else(|| std::io::Error::other("archive requires a parent directory"))?,
    )?;
    let temporary_path = temporary.into_temp_path();
    let wrappers = governed_wrapper_paths(root)?;

    let mut tar = Command::new(tar_command);
    tar.env("COPYFILE_DISABLE", "1")
        .arg("-C")
        .arg(root)
        .arg("-cf");
    tar.arg(&temporary_path);
    for wrapper in &wrappers {
        tar.arg(format!("--exclude={wrapper}"));
    }
    tar.arg("plugins/codexy");
    let mut tar = tar.spawn()?;
    let tar_status = wait_for_archive_process(&mut tar, "tar", timeout)?;
    if !tar_status.success() {
        return Err(std::io::Error::other(format!("tar failed: {tar_status}")));
    }

    if !wrappers.is_empty() {
        let mode = super::governed_archive_mode(true, true, 0o755)
            .ok_or_else(|| std::io::Error::other("Windows wrapper archive mode unavailable"))?;
        let mut wrapper_tar = Command::new(tar_command);
        wrapper_tar
            .env("COPYFILE_DISABLE", "1")
            .arg("-C")
            .arg(root)
            .arg("--append")
            .arg("--file")
            .arg(&temporary_path)
            .arg(format!("--mode={mode:o}"));
        wrapper_tar.args(&wrappers);
        let mut wrapper_tar = wrapper_tar.spawn()?;
        let status = wait_for_archive_process(&mut wrapper_tar, "tar", timeout)?;
        if !status.success() {
            return Err(std::io::Error::other(format!("tar failed: {status}")));
        }
    }

    let archive_file = File::create(archive)?;
    let mut gzip = Command::new(gzip_command)
        .args(["-1", "-c"])
        .stdin(Stdio::from(
            std::fs::OpenOptions::new()
                .read(true)
                .open(&temporary_path)?,
        ))
        .stdout(archive_file)
        .spawn()?;
    let status = wait_for_archive_process(&mut gzip, "gzip", timeout)?;
    if !status.success() {
        return Err(std::io::Error::other(format!("gzip failed: {status}")));
    }
    Ok(())
}

#[cfg(windows)]
fn governed_wrapper_paths(root: &std::path::Path) -> std::io::Result<Vec<String>> {
    let directory = root.join("plugins/codexy/mcp");
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut wrappers = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("codexy-mcp-")
            && path.is_file()
            && std::fs::read(&path)?.starts_with(b"#!")
        {
            wrappers.push(format!("plugins/codexy/mcp/{name}"));
        }
    }
    wrappers.sort();
    Ok(wrappers)
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
