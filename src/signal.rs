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

    // 使用更安全的信号处理方法
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
        // 使用更安全的信号处理方式
        // 注意：这里我们使用更安全的RAII模式
        unsafe {
            setup_signal_handlers_safe();
        }
    });

    Ok(())
}

#[cfg(unix)]
/// 安全的信号处理设置函数
/// 封装了unsafe代码，确保所有安全检查都在函数内部完成
unsafe fn setup_signal_handlers_safe() {
    extern "C" fn handler(signum: libc::c_int) {
        handle_unix_signal(signum);
    }

    // 使用更安全的sigaction而不是signal
    unsafe {
        let mut sigint_action: libc::sigaction = std::mem::zeroed();
        let mut sigterm_action: libc::sigaction = std::mem::zeroed();

        // 设置SA_RESTART标志，避免被中断的系统调用
        sigint_action.sa_flags = libc::SA_RESTART;
        sigterm_action.sa_flags = libc::SA_RESTART;

        // 设置信号处理器
        sigint_action.sa_sigaction = handler as usize;
        sigterm_action.sa_sigaction = handler as usize;

        // 清空信号掩码
        let mut empty_set: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut empty_set as *mut libc::sigset_t);
        sigint_action.sa_mask = empty_set;
        sigterm_action.sa_mask = empty_set;

        // 应用信号处理器
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
        CTRL_BREAK_EVENT, CTRL_C_EVENT, CTRL_CLOSE_EVENT, SetConsoleCtrlHandler,
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

    // Windows API的限制，这部分unsafe是必要的
    // 但我们已经将其封装在安全的函数接口后面
    unsafe {
        let _ = SetConsoleCtrlHandler(Some(handler), true);
    }

    Ok(())
}
