use std::process::{Child, Output};
use std::time::{Duration, Instant};

pub(crate) fn wait_for_wrapper_output(
    mut child: Child,
    description: String,
    timeout: Duration,
) -> Result<Output, Box<dyn std::error::Error>> {
    let started = Instant::now();

    loop {
        if child.try_wait()?.is_some() {
            terminate_wrapper_process_tree(&mut child);
            return Ok(child.wait_with_output()?);
        }
        if started.elapsed() >= timeout {
            terminate_wrapper_process_tree(&mut child);
            let output = child.wait_with_output()?;
            let message = format!(
                "wrapper subprocess timed out after {}s: {description}\nstdout:\n{}\nstderr:\n{}",
                timeout.as_secs(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
            return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, message).into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn terminate_wrapper_process_tree(child: &mut Child) {
    #[cfg(unix)]
    {
        let process_group = -(child.id() as i32);
        // SAFETY: the child is the process-group leader configured above; a failed kill means
        // the process group has already exited and the subsequent child reaping remains safe.
        unsafe {
            let _ = libc::kill(process_group, libc::SIGKILL);
        }
    }
    // On non-Unix targets this is the portable fallback. On Unix it also handles the direct
    // child if the process-group operation raced with process exit.
    let _ = child.kill();
}
