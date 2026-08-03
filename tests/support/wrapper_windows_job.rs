use std::ffi::c_void;
use std::io;
use std::mem::size_of;
use std::os::windows::io::RawHandle;
use std::ptr::{null, null_mut};

pub(crate) const CREATE_SUSPENDED: u32 = 0x0000_0004;

const ERROR_NO_MORE_FILES: i32 = 18;
const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;
const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: u32 = 9;
const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x0000_2000;
const THREAD_SUSPEND_RESUME: u32 = 0x0002;
const TH32CS_SNAPTHREAD: u32 = 0x0000_0004;
const WAIT_OBJECT_0: u32 = 0;
const WAIT_TIMEOUT: u32 = 258;
const TERMINATION_WAIT_MS: u32 = 5_000;
type Handle = *mut c_void;

#[repr(C)]
#[derive(Default)]
struct IoCounters {
    read_operation_count: u64,
    write_operation_count: u64,
    other_operation_count: u64,
    read_transfer_count: u64,
    write_transfer_count: u64,
    other_transfer_count: u64,
}

#[repr(C)]
#[derive(Default)]
struct BasicLimitInformation {
    per_process_user_time_limit: i64,
    per_job_user_time_limit: i64,
    limit_flags: u32,
    minimum_working_set_size: usize,
    maximum_working_set_size: usize,
    active_process_limit: u32,
    affinity: usize,
    priority_class: u32,
    scheduling_class: u32,
}

#[repr(C)]
#[derive(Default)]
struct ExtendedLimitInformation {
    basic: BasicLimitInformation,
    io_info: IoCounters,
    process_memory_limit: usize,
    job_memory_limit: usize,
    peak_process_memory_used: usize,
    peak_job_memory_used: usize,
}

#[repr(C)]
struct ThreadEntry32 {
    size: u32,
    usage: u32,
    thread_id: u32,
    owner_process_id: u32,
    base_priority: i32,
    delta_priority: i32,
    flags: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn AssignProcessToJobObject(job: Handle, process: Handle) -> i32;
    fn CloseHandle(handle: Handle) -> i32;
    fn CreateJobObjectW(attributes: Handle, name: *const u16) -> Handle;
    fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> Handle;
    fn GetLastError() -> u32;
    fn OpenThread(access: u32, inherit: i32, thread_id: u32) -> Handle;
    fn ResumeThread(thread: Handle) -> u32;
    fn SetInformationJobObject(job: Handle, class: u32, info: Handle, length: u32) -> i32;
    fn TerminateJobObject(job: Handle, exit_code: u32) -> i32;
    fn Thread32First(snapshot: Handle, entry: *mut ThreadEntry32) -> i32;
    fn Thread32Next(snapshot: Handle, entry: *mut ThreadEntry32) -> i32;
    fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
}

pub(crate) struct JobObject {
    handle: Handle,
}

impl JobObject {
    pub(crate) fn for_process(process: RawHandle) -> io::Result<Self> {
        let handle = unsafe { CreateJobObjectW(null_mut(), null()) };
        if handle.is_null() {
            return Err(last_error("CreateJobObjectW"));
        }
        let mut job = Self { handle };
        let mut limits = ExtendedLimitInformation::default();
        limits.basic.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let set_result = unsafe {
            SetInformationJobObject(
                job.handle,
                JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
                (&mut limits as *mut ExtendedLimitInformation).cast(),
                size_of::<ExtendedLimitInformation>() as u32,
            )
        };
        if set_result == 0 {
            return Err(job.failure(last_error("SetInformationJobObject")));
        }
        if unsafe { AssignProcessToJobObject(job.handle, process) } == 0 {
            return Err(job.failure(last_error("AssignProcessToJobObject")));
        }
        Ok(job)
    }

    pub(crate) fn resume_primary_thread(&self, process_id: u32) -> io::Result<()> {
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(last_error("CreateToolhelp32Snapshot"));
        }
        let mut entry = ThreadEntry32 {
            size: size_of::<ThreadEntry32>() as u32,
            usage: 0,
            thread_id: 0,
            owner_process_id: 0,
            base_priority: 0,
            delta_priority: 0,
            flags: 0,
        };
        let mut thread_id = None;
        let mut enumeration_error = None;
        if unsafe { Thread32First(snapshot, &mut entry) } == 0 {
            enumeration_error = Some(last_error("Thread32First"));
        } else {
            loop {
                if entry.owner_process_id == process_id {
                    thread_id = Some(entry.thread_id);
                    break;
                }
                if unsafe { Thread32Next(snapshot, &mut entry) } == 0 {
                    let error = last_error("Thread32Next");
                    if error.raw_os_error() != Some(ERROR_NO_MORE_FILES) {
                        enumeration_error = Some(error);
                    }
                    break;
                }
            }
        }
        let snapshot_close = close_handle(snapshot, "CloseHandle(thread snapshot)");
        if let Some(error) = enumeration_error {
            return Err(with_close_error(error, snapshot_close));
        }
        snapshot_close?;
        let thread_id = thread_id.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("primary thread not found for suspended process {process_id}"),
            )
        })?;
        let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, thread_id) };
        if thread.is_null() {
            return Err(last_error("OpenThread"));
        }
        let resumed = unsafe { ResumeThread(thread) };
        let resume_error = if resumed == u32::MAX {
            Some(last_error("ResumeThread"))
        } else if resumed != 1 {
            Some(io::Error::other(format!(
                "ResumeThread returned unexpected suspend count {resumed}"
            )))
        } else {
            None
        };
        let thread_close = close_handle(thread, "CloseHandle(primary thread)");
        if let Some(error) = resume_error {
            return Err(with_close_error(error, thread_close));
        }
        thread_close
    }

    pub(crate) fn terminate_and_wait(&self) -> io::Result<()> {
        if unsafe { TerminateJobObject(self.handle, 124) } == 0 {
            return Err(last_error("TerminateJobObject"));
        }
        match unsafe { WaitForSingleObject(self.handle, TERMINATION_WAIT_MS) } {
            WAIT_OBJECT_0 => Ok(()),
            WAIT_TIMEOUT => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Job Object did not reach active-process-zero after termination",
            )),
            _ => Err(last_error("WaitForSingleObject")),
        }
    }

    pub(crate) fn close_checked(&mut self) -> io::Result<()> {
        if self.handle.is_null() {
            return Ok(());
        }
        if unsafe { CloseHandle(self.handle) } == 0 {
            return Err(last_error("CloseHandle(job)"));
        }
        self.handle = null_mut();
        Ok(())
    }

    fn failure(&mut self, error: io::Error) -> io::Error {
        with_close_error(error, self.close_checked())
    }
}

impl Drop for JobObject {
    fn drop(&mut self) {
        let _ = self.close_checked();
    }
}

fn close_handle(handle: Handle, action: &str) -> io::Result<()> {
    if unsafe { CloseHandle(handle) } == 0 {
        return Err(last_error(action));
    }
    Ok(())
}

fn last_error(action: &str) -> io::Error {
    let code = unsafe { GetLastError() };
    io::Error::from_raw_os_error(code as i32).into_with_context(action)
}

fn with_close_error(error: io::Error, close: io::Result<()>) -> io::Error {
    match close {
        Ok(()) => error,
        Err(close_error) => io::Error::new(
            error.kind(),
            format!("{error}; close handle: {close_error}"),
        ),
    }
}

trait IoErrorContext {
    fn into_with_context(self, action: &str) -> io::Error;
}

impl IoErrorContext for io::Error {
    fn into_with_context(self, action: &str) -> io::Error {
        io::Error::new(self.kind(), format!("{action}: {self}"))
    }
}
