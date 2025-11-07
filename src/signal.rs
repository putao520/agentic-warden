use crate::platform;
use std::io;
use std::sync::atomic::{AtomicU32, Ordering};

static CHILD_PID: AtomicU32 = AtomicU32::new(0);

pub struct SignalGuard;

impl Drop for SignalGuard {
    fn drop(&mut self) {
        CHILD_PID.store(0, Ordering::SeqCst);
    }
}

pub fn install(child_pid: u32) -> io::Result<SignalGuard> {
    CHILD_PID.store(child_pid, Ordering::SeqCst);

    // Use safer signal handling methods
    #[cfg(unix)]
    {
        setup_unix_signal_handlers()?;
    }

    #[cfg(windows)]
    {
        setup_windows_signal_handler()?;
    }

    Ok(SignalGuard)
}

#[cfg(unix)]
fn setup_unix_signal_handlers() -> io::Result<()> {
    use std::sync::Once;

    static INIT: Once = Once::new();

    INIT.call_once(|| {
        // Use safer signal handling approach
        // Note: We use safer RAII pattern here
        unsafe {
            setup_signal_handlers_safe();
        }
    });

    Ok(())
}

#[cfg(unix)]
/// Safe signal handling setup function
/// Encapsulates unsafe code to ensure all safety checks are completed within the function
unsafe fn setup_signal_handlers_safe() {
    extern "C" fn handler(signum: libc::c_int) {
        handle_unix_signal(signum);
    }

    // Use safer sigaction instead of signal
    unsafe {
        let mut sigint_action: libc::sigaction = std::mem::zeroed();
        let mut sigterm_action: libc::sigaction = std::mem::zeroed();

        // Set SA_RESTART flag to avoid interrupted system calls
        sigint_action.sa_flags = libc::SA_RESTART;
        sigterm_action.sa_flags = libc::SA_RESTART;

        // Set signal handler
        sigint_action.sa_sigaction = handler as usize;
        sigterm_action.sa_sigaction = handler as usize;

        // Clear signal mask
        let mut empty_set: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut empty_set as *mut libc::sigset_t);
        sigint_action.sa_mask = empty_set;
        sigterm_action.sa_mask = empty_set;

        // Apply signal handlers
        libc::sigaction(libc::SIGINT, &sigint_action, std::ptr::null_mut());
        libc::sigaction(libc::SIGTERM, &sigterm_action, std::ptr::null_mut());
    }
}

#[cfg(unix)]
fn handle_unix_signal(signum: libc::c_int) {
    match signum {
        libc::SIGINT | libc::SIGTERM => {
            let pid = CHILD_PID.load(Ordering::SeqCst);
            if pid != 0 {
                platform::terminate_process(pid);
            }
        }
        _ => {}
    }
}

#[cfg(windows)]
fn setup_windows_signal_handler() -> io::Result<()> {
    use windows::Win32::Foundation::BOOL;
    use windows::Win32::System::Console::{
        SetConsoleCtrlHandler, CTRL_BREAK_EVENT, CTRL_CLOSE_EVENT, CTRL_C_EVENT,
    };

    unsafe extern "system" fn handler(ctrl_type: u32) -> BOOL {
        match ctrl_type {
            CTRL_C_EVENT | CTRL_BREAK_EVENT | CTRL_CLOSE_EVENT => {
                let pid = CHILD_PID.load(Ordering::SeqCst);
                if pid != 0 {
                    platform::terminate_process(pid);
                }
                BOOL(1)
            }
            _ => BOOL(0),
        }
    }

    // Windows API limitations, this unsafe part is necessary
    // But we have encapsulated it behind a safe function interface
    unsafe {
        let _ = SetConsoleCtrlHandler(Some(handler), true);
    }

    Ok(())
}
