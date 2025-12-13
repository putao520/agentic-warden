//! 日志系统
//!
//! 提供统一的日志记录功能

use anyhow::Result;
use std::path::PathBuf;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// 初始化日志系统
///
/// # Arguments
/// * `log_level` - 日志级别 (trace, debug, info, warn, error)，如果为 None 则使用环境变量 RUST_LOG
/// * `log_file` - 日志文件路径，如果为 None 则只输出到标准输出
///
/// # Examples
/// ```no_run
/// use aiw::utils::logger::init_logger;
///
/// // 使用默认配置（从环境变量读取）
/// init_logger(None, None).unwrap();
///
/// // 指定日志级别
/// init_logger(Some("debug"), None).unwrap();
///
/// // 同时写入文件
/// use std::path::PathBuf;
/// init_logger(Some("info"), Some(PathBuf::from("app.log"))).unwrap();
/// ```
pub fn init_logger(log_level: Option<&str>, log_file: Option<PathBuf>) -> Result<()> {
    // 构建 EnvFilter，优先使用参数指定的级别，其次使用 RUST_LOG 环境变量
    let env_filter = if let Some(level) = log_level {
        EnvFilter::try_new(level)?
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // 默认级别：库代码 info，应用代码 debug
            EnvFilter::new("info,aiw=debug")
        })
    };

    // 基础的格式化层（输出到标准输出）
    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_level(true)
        .with_ansi(true) // 启用 ANSI 颜色输出
        .compact();

    // 如果指定了日志文件，添加文件输出层
    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer);

    if let Some(log_path) = log_file {
        // 确保日志文件的父目录存在
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // 创建日志文件
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        // 添加文件输出层
        let file_layer = fmt::layer()
            .with_writer(std::sync::Arc::new(file))
            .with_target(true)
            .with_ansi(false) // 文件中不使用 ANSI 颜色
            .with_level(true);

        registry.with(file_layer).init();
    } else {
        registry.init();
    }

    tracing::info!("Logger initialized");
    Ok(())
}
