use std::ffi::c_void;
use std::os::windows::io::AsRawHandle;
use std::process::Child;

use anyhow::{Result, bail};

const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: i32 = 9;
const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x2000;

type Handle = *mut c_void;

pub(super) struct Job(Handle);

impl Job {
    pub(super) fn new() -> Result<Self> {
        // SAFETY: CreateJobObjectW has no pointer inputs; the returned handle is owned below.
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            bail!("creating Windows hook job object failed");
        }
        let job = Self(handle);
        // SAFETY: zero initialization is valid for this Windows POD structure.
        let mut limits: JobObjectExtendedLimitInformation = unsafe { std::mem::zeroed() };
        limits.basic.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: limits is a valid structure for JOB_OBJECT_EXTENDED_LIMIT_INFORMATION.
        if unsafe {
            SetInformationJobObject(
                job.0,
                JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
                (&limits as *const JobObjectExtendedLimitInformation).cast(),
                std::mem::size_of::<JobObjectExtendedLimitInformation>() as u32,
            )
        } == 0
        {
            bail!("configuring Windows hook job object failed");
        }
        Ok(job)
    }

    pub(super) fn assign(&self, child: &Child) -> Result<()> {
        // SAFETY: child owns a live process handle and job is configured to terminate its tree.
        if unsafe { AssignProcessToJobObject(self.0, child.as_raw_handle().cast()) } == 0 {
            bail!("assigning hook child to Windows job object failed");
        }
        Ok(())
    }

    pub(super) fn resume(&self, child: &Child) -> Result<()> {
        // SAFETY: child owns the process handle for a process created suspended below.
        if unsafe { NtResumeProcess(child.as_raw_handle().cast()) } != 0 {
            bail!("resuming suspended Windows hook process failed");
        }
        Ok(())
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        // SAFETY: the handle was created by CreateJobObjectW and is owned exactly once here.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

#[repr(C)]
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
struct IoCounters {
    values: [u64; 6],
}

#[repr(C)]
struct JobObjectExtendedLimitInformation {
    basic: BasicLimitInformation,
    io: IoCounters,
    process_memory_limit: usize,
    job_memory_limit: usize,
    peak_process_memory_used: usize,
    peak_job_memory_used: usize,
}

unsafe extern "system" {
    fn CreateJobObjectW(attributes: *const c_void, name: *const u16) -> Handle;
    fn SetInformationJobObject(job: Handle, class: i32, info: *const c_void, size: u32) -> i32;
    fn AssignProcessToJobObject(job: Handle, process: Handle) -> i32;
    fn CloseHandle(handle: Handle) -> i32;
}

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtResumeProcess(process: Handle) -> i32;
}
