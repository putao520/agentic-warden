use crate::logging::debug;
use std::io;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Safely prepare the execution environment for child processes
///
/// This function uses safer methods to set process group and death signals
pub fn prepare_command(cmd: &mut Command) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        // Use RAII pattern to wrap unsafe operations
        unsafe {
            cmd.pre_exec(|| {
                // Safely set process group ID
                if set_process_group() != 0 {
                    return Err(io::Error::last_os_error());
                }

                // Set parent death signal on Linux
                #[cfg(target_os = "linux")]
                {
                    if set_parent_death_signal() != 0 {
                        return Err(io::Error::last_os_error());
                    }
                }

                Ok(())
            });
        }
    }

    Ok(())
}

/// Check if process is alive
///
/// Use safer system call wrappers
pub fn process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        let c_pid = pid as libc::pid_t;
        match unsafe_send_signal(c_pid, 0) {
            Ok(_) => true,                      // Signal sent successfully, process exists
            Err(errno) => errno == libc::EPERM, // EPERM means process exists but no permission
        }
    }
    #[cfg(not(unix))]
    {
        false // Fallback implementation for non-Unix systems
    }
}

/// Terminate process
///
/// First try graceful termination (SIGTERM), force termination (SIGKILL) if it fails
pub fn terminate_process(pid: u32) {
    #[cfg(unix)]
    {
        let c_pid = pid as libc::pid_t;

        // First check if process exists
        if !process_alive(pid) {
            return;
        }

        // Graceful termination
        if unsafe_send_signal(c_pid, libc::SIGTERM).is_ok() {
            thread::sleep(Duration::from_millis(500));

            // Check if already terminated
            if !process_alive(pid) {
                return;
            }
        }

        // Force termination
        if unsafe_send_signal(c_pid, libc::SIGKILL).is_ok() {
            debug(format!("pid={} sent SIGKILL", pid));
        }
    }

    #[cfg(not(unix))]
    {
        // Implementation for non-Unix systems (if needed)
        // Currently empty implementation
    }
}

/// Safely set process group ID
///
/// Encapsulates unsafe setpgid call
#[cfg(unix)]
unsafe fn set_process_group() -> libc::c_int {
    unsafe { libc::setpgid(0, 0) }
}

/// Safely set parent death signal
///
/// Encapsulates unsafe prctl call
#[cfg(target_os = "linux")]
unsafe fn set_parent_death_signal() -> libc::c_int {
    unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) }
}

/// Safely send signal
///
/// Encapsulates unsafe kill call and returns Result instead of raw error code
#[cfg(unix)]
fn unsafe_send_signal(pid: libc::pid_t, signal: libc::c_int) -> Result<(), libc::c_int> {
    let result = unsafe { libc::kill(pid, signal) };
    if result == 0 {
        Ok(())
    } else {
        Err(get_last_errno())
    }
}

/// Get last error code
///
/// Encapsulates unsafe errno access
#[cfg(unix)]
fn get_last_errno() -> libc::c_int {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        unsafe { *libc::__errno_location() }
    }

    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
    {
        unsafe { *libc::__error() }
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd"
    )))]
    {
        // Fallback implementation for other Unix systems
        0
    }
}
