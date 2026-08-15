use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

const FIRST_LINE_TIMEOUT: Duration = Duration::from_secs(2);

#[test]
fn gate_streams_workload_output_before_the_child_completes()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let bin = root.path().join("bin");
    std::fs::create_dir(&bin)?;
    let release = root.path().join("release-workload");
    let cargo = bin.join("cargo");
    std::fs::write(
        &cargo,
        "#!/bin/sh\nprintf '%s\\n' workload-begin\nwhile [ ! -f \"$PROFILE_RELEASE\" ]; do sleep 0.01; done\nprintf '%s\\n' 'Finished `test` profile [unoptimized] target(s) in 0.01s' 'test result: ok. 1802 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s'\n",
    )?;
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(&cargo)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&cargo, permissions)?;

    let path = std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(&std::env::var_os("PATH").ok_or("PATH")?)),
    )?;
    let mut child = Command::new("python3")
        .arg(codexy_runtime::paths::repository_root().join("scripts/profile_rust_tests.py"))
        .current_dir(codexy_runtime::paths::repository_root())
        .env("PATH", path)
        .env("PROFILE_RELEASE", &release)
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().ok_or("gate stdout")?;
    let (sender, receiver) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let _ = reader.read_line(&mut line);
        let _ = sender.send(line);
        let _ = reader.read_to_end(&mut Vec::new());
    });

    // The child remains blocked on `release`, so this only bounds host startup;
    // it cannot let buffered output satisfy the pre-completion assertion.
    let first_line = receiver.recv_timeout(FIRST_LINE_TIMEOUT);
    std::fs::write(&release, [])?;
    assert!(child.wait()?.success());
    reader.join().map_err(|_| "gate stdout reader panicked")?;
    assert_eq!(first_line?.trim(), "workload-begin");
    Ok(())
}

#[test]
fn gate_spools_large_crlf_utf8_output_without_losing_live_order()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let bin = root.path().join("bin");
    std::fs::create_dir(&bin)?;
    let release = root.path().join("release-workload");
    let cargo = bin.join("cargo");
    std::fs::write(
        &cargo,
        "#!/bin/sh\nprintf 'workload-begin\\r\\n'\npython3 -c 'import sys; sys.stdout.buffer.write((b\"second\\r\\n\" * 300000) + \"lambda=λ\\r\\n\".encode())'\nwhile [ ! -f \"$PROFILE_RELEASE\" ]; do sleep 0.01; done\nprintf '%s\\n' 'Finished `test` profile [unoptimized] target(s) in 0.01s' 'test result: ok. 1802 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s'\n",
    )?;
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(&cargo)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&cargo, permissions)?;

    let path = std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(&std::env::var_os("PATH").ok_or("PATH")?)),
    )?;
    let mut child = Command::new("python3")
        .arg(codexy_runtime::paths::repository_root().join("scripts/profile_rust_tests.py"))
        .current_dir(codexy_runtime::paths::repository_root())
        .env("PATH", path)
        .env("PROFILE_RELEASE", &release)
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().ok_or("gate stdout")?;
    let (sender, receiver) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut first = String::new();
        let _ = reader.read_line(&mut first);
        let _ = sender.send(first);
        let mut tail = Vec::new();
        let _ = reader.read_to_end(&mut tail);
        tail
    });

    let first_line = receiver.recv_timeout(FIRST_LINE_TIMEOUT)?;
    std::fs::write(&release, [])?;
    assert!(child.wait()?.success());
    let tail = reader.join().map_err(|_| "gate stdout reader panicked")?;

    let mut expected = b"workload-begin\r\n".to_vec();
    expected.extend_from_slice(&b"second\r\n".repeat(300000));
    expected.extend_from_slice("lambda=λ\r\n".as_bytes());
    expected.extend_from_slice(b"Finished `test` profile [unoptimized] target(s) in 0.01s\n");
    expected.extend_from_slice(b"test result: ok. 1802 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n");
    let mut observed = first_line.into_bytes();
    observed.extend_from_slice(&tail);
    assert!(observed.starts_with(&expected));
    Ok(())
}
