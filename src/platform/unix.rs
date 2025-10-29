use crate::logging::debug;
use std::io;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// 安全地准备子进程的执行环境
///
/// 这个函数使用更安全的方式设置进程组和死亡信号
pub fn prepare_command(cmd: &mut Command) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        // 使用RAII模式包装unsafe操作
        unsafe {
            cmd.pre_exec(|| {
                // 安全地设置进程组ID
                if set_process_group() != 0 {
                    return Err(io::Error::last_os_error());
                }

                // 在Linux上设置父进程死亡信号
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

/// 检查进程是否存活
///
/// 使用更安全的系统调用包装器
pub fn process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        let c_pid = pid as libc::pid_t;
        match unsafe_send_signal(c_pid, 0) {
            Ok(_) => true,                      // 信号发送成功，进程存在
            Err(errno) => errno == libc::EPERM, // EPERM表示进程存在但没有权限
        }
    }
    #[cfg(not(unix))]
    {
        false // 非Unix系统的后备实现
    }
}

/// 终止进程
///
/// 首先尝试优雅地终止（SIGTERM），如果失败则强制终止（SIGKILL）
pub fn terminate_process(pid: u32) {
    #[cfg(unix)]
    {
        let c_pid = pid as libc::pid_t;

        // 首先检查进程是否存在
        if !process_alive(pid) {
            return;
        }

        // 优雅终止
        if unsafe_send_signal(c_pid, libc::SIGTERM).is_ok() {
            thread::sleep(Duration::from_millis(500));

            // 检查是否已经终止
            if !process_alive(pid) {
                return;
            }
        }

        // 强制终止
        if unsafe_send_signal(c_pid, libc::SIGKILL).is_ok() {
            debug(format!("pid={} sent SIGKILL", pid));
        }
    }

    #[cfg(not(unix))]
    {
        // 非Unix系统的实现（如果需要的话）
        // 目前是空实现
    }
}

/// 安全地设置进程组ID
///
/// 封装了unsafe的setpgid调用
#[cfg(unix)]
unsafe fn set_process_group() -> libc::c_int {
    unsafe { libc::setpgid(0, 0) }
}

/// 安全地设置父进程死亡信号
///
/// 封装了unsafe的prctl调用
#[cfg(target_os = "linux")]
unsafe fn set_parent_death_signal() -> libc::c_int {
    unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) }
}

/// 安全地发送信号
///
/// 封装了unsafe的kill调用，并返回Result而不是原始的错误码
#[cfg(unix)]
fn unsafe_send_signal(pid: libc::pid_t, signal: libc::c_int) -> Result<(), libc::c_int> {
    let result = unsafe { libc::kill(pid, signal) };
    if result == 0 {
        Ok(())
    } else {
        Err(get_last_errno())
    }
}

/// 获取最后的错误码
///
/// 封装了unsafe的errno访问
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
        // 其他Unix系统的后备实现
        0
    }
}
