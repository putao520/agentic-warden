use crate::logging::debug;
use std::io;
use std::os::windows::io::AsRawHandle;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE, STILL_ACTIVE};
use windows::Win32::System::Console::{
    GetConsoleMode, GetStdHandle, SetConsoleMode, CONSOLE_MODE, ENABLE_VIRTUAL_TERMINAL_PROCESSING,
    STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, TerminateProcess, WaitForSingleObject,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
};

pub fn prepare_command(_cmd: &mut std::process::Command) -> io::Result<()> {
    Ok(())
}

pub fn enable_virtual_terminal_processing() -> io::Result<()> {
    unsafe {
        for kind in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
            let handle = match GetStdHandle(kind) {
                Ok(handle) => handle,
                Err(_) => continue,
            };
            if handle == HANDLE(0) || handle == INVALID_HANDLE_VALUE {
                continue;
            }
            let mut mode = CONSOLE_MODE(0);
            if GetConsoleMode(handle, &mut mode).is_err() {
                continue;
            }
            if mode.0 & ENABLE_VIRTUAL_TERMINAL_PROCESSING.0 == 0 {
                let new_mode = CONSOLE_MODE(mode.0 | ENABLE_VIRTUAL_TERMINAL_PROCESSING.0);
                let _ = SetConsoleMode(handle, new_mode);
            }
        }
    }
    Ok(())
}

pub fn process_alive(pid: u32) -> bool {
    unsafe {
        let handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(handle) => handle,
            Err(_) => return false,
        };
        let mut exit_code = 0u32;
        let ok = GetExitCodeProcess(handle, &mut exit_code).is_ok();
        let _ = CloseHandle(handle);
        ok && exit_code == STILL_ACTIVE.0 as u32
    }
}

pub fn terminate_process(pid: u32) {
    unsafe {
        let handle = match OpenProcess(
            PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            pid,
        ) {
            Ok(handle) => handle,
            Err(_) => return,
        };
        if TerminateProcess(handle, 1).is_ok() {
            let _ = WaitForSingleObject(handle, 5_000);
            debug(format!("Terminated Codex child pid={pid}"));
        }
        let _ = CloseHandle(handle);
    }
}

pub fn after_spawn(child: &std::process::Child) -> io::Result<Option<JobHandle>> {
    unsafe {
        let job = match CreateJobObjectW(None, PCWSTR::null()) {
            Ok(job) => job,
            Err(err) => return Err(io::Error::from(err)),
        };

        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if let Err(err) = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        ) {
            let _ = CloseHandle(job);
            return Err(io::Error::from(err));
        }

        let process_handle = HANDLE(child.as_raw_handle() as isize);
        if let Err(err) = AssignProcessToJobObject(job, process_handle) {
            let _ = CloseHandle(job);
            return Err(io::Error::from(err));
        }

        Ok(Some(JobHandle(job)))
    }
}

pub struct JobHandle(HANDLE);

impl Drop for JobHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
