use std::io;
use std::process::{Child, ChildStdin, Command, Output};
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::io::AsRawHandle as _;
#[cfg(windows)]
#[path = "wrapper_windows_job.rs"]
mod windows_job;

pub(crate) struct WrapperChild {
    child: Child,
    pub(crate) stdin: Option<ChildStdin>,
    #[cfg(windows)]
    job: windows_job::JobObject,
}

pub(crate) fn spawn_wrapper_child(command: &mut Command) -> io::Result<WrapperChild> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;

        // Keep the process suspended until the Job Object owns it. This closes the
        // spawn-to-assign window in which a shell could create an unassigned descendant.
        command.creation_flags(windows_job::CREATE_SUSPENDED);
        let child = command.spawn()?;
        return WrapperChild::from_suspended_child(child);
    }
    #[cfg(not(windows))]
    {
        let mut child = command.spawn()?;
        let stdin = child.stdin.take();
        Ok(WrapperChild { child, stdin })
    }
}

#[cfg(windows)]
impl WrapperChild {
    fn from_suspended_child(mut child: Child) -> io::Result<Self> {
        let job = match windows_job::JobObject::for_process(child.as_raw_handle()) {
            Ok(job) => job,
            Err(error) => {
                let cleanup = kill_and_reap(&mut child);
                return Err(with_cleanup_error(error, cleanup));
            }
        };
        if let Err(error) = job.resume_primary_thread(child.id()) {
            let cleanup = job
                .terminate_and_wait()
                .and_then(|()| child.wait().map(|_| ()))
                .or_else(|termination| {
                    let _ = child.kill();
                    let _ = child.wait();
                    Err(termination)
                });
            return Err(with_cleanup_error(error, cleanup));
        }
        let stdin = child.stdin.take();
        Ok(Self { child, stdin, job })
    }
}

impl WrapperChild {
    fn try_wait(&mut self) -> io::Result<Option<std::process::ExitStatus>> {
        self.child.try_wait()
    }

    fn wait_with_output(mut self) -> io::Result<Output> {
        drop(self.stdin.take());
        let output = self.child.wait_with_output();
        #[cfg(windows)]
        {
            let close = self.job.close_checked();
            return match (output, close) {
                (Ok(output), Ok(())) => Ok(output),
                (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
                (Err(error), Err(close_error)) => Err(io::Error::new(
                    error.kind(),
                    format!("{error}; CloseHandle(job): {close_error}"),
                )),
            };
        }
        output
    }

    fn terminate_tree(&mut self) -> io::Result<()> {
        #[cfg(windows)]
        {
            return self.job.terminate_and_wait();
        }
        #[cfg(unix)]
        {
            let process_group = -(self.child.id() as i32);
            // SAFETY: the child is the process-group leader configured above; a failed kill
            // means the process group has already exited and child reaping remains safe.
            unsafe {
                let _ = libc::kill(process_group, libc::SIGKILL);
            }
            let _ = self.child.kill();
            return Ok(());
        }
        #[cfg(all(not(unix), not(windows)))]
        {
            let _ = self.child.kill();
            Ok(())
        }
    }

    #[cfg(windows)]
    fn close_job(&mut self) -> io::Result<()> {
        self.job.close_checked()
    }
}

#[cfg(windows)]
fn kill_and_reap(child: &mut Child) -> io::Result<()> {
    let kill = child.kill();
    let wait = child.wait().map(|_| ());
    match (kill, wait) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(error), Err(wait_error)) => Err(io::Error::new(
            error.kind(),
            format!("{error}; wait: {wait_error}"),
        )),
    }
}

#[cfg(windows)]
fn with_cleanup_error(error: io::Error, cleanup: io::Result<()>) -> io::Error {
    match cleanup {
        Ok(()) => error,
        Err(cleanup_error) => {
            io::Error::new(error.kind(), format!("{error}; cleanup: {cleanup_error}"))
        }
    }
}

pub(crate) fn wait_for_wrapper_output(
    mut child: WrapperChild,
    description: String,
    timeout: Duration,
) -> Result<Output, Box<dyn std::error::Error>> {
    let started = Instant::now();

    loop {
        if child.try_wait()?.is_some() {
            return Ok(child.wait_with_output()?);
        }
        if started.elapsed() >= timeout {
            if let Err(error) = child.terminate_tree() {
                #[cfg(windows)]
                let close = child.close_job();
                #[cfg(not(windows))]
                let close: io::Result<()> = Ok(());
                let close_note = close
                    .err()
                    .map(|close_error| format!("; CloseHandle(job): {close_error}"))
                    .unwrap_or_default();
                let message = format!(
                    "wrapper subprocess timed out after {}s: {description}; process-tree termination failed: {error}{close_note}",
                    timeout.as_secs(),
                );
                return Err(io::Error::new(io::ErrorKind::TimedOut, message).into());
            }
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
