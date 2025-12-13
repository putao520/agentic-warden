use std::process::Command;

pub fn current_pid() -> u32 {
    std::process::id()
}

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

#[cfg(not(any(unix, windows)))]
compile_error!("agentic-warden platform module is not supported on this operating system");

pub fn prepare_command(cmd: &mut Command) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        unix::prepare_command(cmd)?;
    }
    #[cfg(windows)]
    {
        windows::prepare_command(cmd)?;
    }
    Ok(())
}

pub fn after_spawn(child: &std::process::Child) -> std::io::Result<ChildResources> {
    #[cfg(unix)]
    {
        let _ = child;
        Ok(ChildResources::new())
    }
    #[cfg(windows)]
    {
        let job = windows::after_spawn(child)?;
        Ok(ChildResources::with_job(job))
    }
}

pub fn init_platform() {
    #[cfg(windows)]
    {
        let _ = windows::enable_virtual_terminal_processing();
    }
}

pub struct ChildResources {
    #[cfg(windows)]
    #[allow(dead_code)]
    job: Option<windows::JobHandle>,
}

#[cfg(unix)]
impl ChildResources {
    pub fn new() -> Self {
        ChildResources {}
    }
}

#[cfg(windows)]
impl ChildResources {
    pub fn with_job(job: Option<windows::JobHandle>) -> Self {
        ChildResources { job }
    }
}
