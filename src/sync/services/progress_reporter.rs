//! Progress Reporter - Common progress reporting for sync operations
//!
//! Eliminates repeated progress reporting logic across sync operations,
//! providing consistent progress updates and user feedback.

use super::sync_service::{ProgressReporter, SyncResult};

/// Multi-channel progress reporter that reports to multiple sources
pub struct MultiProgressReporter {
    reporters: Vec<Box<dyn ProgressReporter>>,
}

impl MultiProgressReporter {
    pub fn new() -> Self {
        Self {
            reporters: Vec::new(),
        }
    }

    pub fn add_reporter(mut self, reporter: Box<dyn ProgressReporter>) -> Self {
        self.reporters.push(reporter);
        self
    }
}

#[async_trait::async_trait]
impl ProgressReporter for MultiProgressReporter {
    async fn report(&mut self, percent: u8, message: String) {
        for reporter in &mut self.reporters {
            reporter.report(percent, message.clone()).await;
        }
    }

    async fn error(&mut self, error: String) {
        for reporter in &mut self.reporters {
            reporter.error(error.clone()).await;
        }
    }

    async fn complete(&mut self, result: SyncResult) {
        for reporter in &mut self.reporters {
            reporter.complete(result.clone()).await;
        }
    }

    async fn should_continue(&self) -> bool {
        for reporter in &self.reporters {
            if !reporter.should_continue().await {
                return false;
            }
        }
        true
    }
}

/// Logging progress reporter
pub struct LoggingProgressReporter {
    operation_name: String,
    start_time: std::time::Instant,
    log_level: tracing::Level,
}

impl LoggingProgressReporter {
    pub fn new(operation_name: impl Into<String>) -> Self {
        Self {
            operation_name: operation_name.into(),
            start_time: std::time::Instant::now(),
            log_level: tracing::Level::INFO,
        }
    }

    pub fn with_log_level(mut self, level: tracing::Level) -> Self {
        self.log_level = level;
        self
    }
}

#[async_trait::async_trait]
impl ProgressReporter for LoggingProgressReporter {
    async fn report(&mut self, percent: u8, message: String) {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        match self.log_level {
            tracing::Level::DEBUG => {
                tracing::debug!(
                    "{}: {}% - {} ({:.1}s elapsed)",
                    self.operation_name,
                    percent,
                    message,
                    elapsed
                );
            }
            tracing::Level::INFO => {
                tracing::info!("{}: {}% - {}", self.operation_name, percent, message);
            }
            tracing::Level::WARN => {
                tracing::warn!("{}: {}% - {}", self.operation_name, percent, message);
            }
            tracing::Level::ERROR => {
                tracing::error!("{}: {}% - {}", self.operation_name, percent, message);
            }
            _ => {}
        }
    }

    async fn error(&mut self, error: String) {
        tracing::error!("{} error: {}", self.operation_name, error);
    }

    async fn complete(&mut self, result: SyncResult) {
        let duration = self.start_time.elapsed();
        let rate = if duration.as_secs() > 0 {
            result.bytes_transferred / duration.as_secs()
        } else {
            0
        };

        if result.success {
            tracing::info!(
                "{} completed successfully in {:.2}s: {} files, {} bytes ({}/s)",
                self.operation_name,
                duration.as_secs_f32(),
                result.files_processed,
                result.bytes_transferred,
                rate
            );
        } else {
            tracing::warn!(
                "{} completed with errors in {:.2}s: {}",
                self.operation_name,
                duration.as_secs_f32(),
                result.message
            );
        }
    }

    async fn should_continue(&self) -> bool {
        true
    }
}

/// TUI progress reporter for terminal interfaces
pub struct TuiProgressReporter {
    cancelled: bool,
    last_percent: u8,
    last_message: String,
}

impl TuiProgressReporter {
    pub fn new() -> Self {
        Self {
            cancelled: false,
            last_percent: 0,
            last_message: String::new(),
        }
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    pub fn get_current_progress(&self) -> (u8, &str) {
        (self.last_percent, &self.last_message)
    }
}

impl Default for TuiProgressReporter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ProgressReporter for TuiProgressReporter {
    async fn report(&mut self, percent: u8, message: String) {
        self.last_percent = percent;
        self.last_message = message.clone();

        // This would typically update TUI state
        // For now, just log at debug level
        tracing::debug!("Progress: {}% - {}", percent, message);
    }

    async fn error(&mut self, error: String) {
        self.last_message = error.clone();
        tracing::error!("Progress error: {}", error);
    }

    async fn complete(&mut self, result: SyncResult) {
        self.last_percent = 100;
        self.last_message = result.message.clone();
        tracing::info!("Progress completed: {}", result.message);
    }

    async fn should_continue(&self) -> bool {
        !self.cancelled
    }
}

/// Callback progress reporter for custom progress handling
pub struct CallbackProgressReporter<F>
where
    F: Fn(u8, String) + Send + Sync,
{
    on_progress: F,
    cancelled: bool,
}

impl<F> CallbackProgressReporter<F>
where
    F: Fn(u8, String) + Send + Sync,
{
    pub fn new(on_progress: F) -> Self {
        Self {
            on_progress,
            cancelled: false,
        }
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
}

#[async_trait::async_trait]
impl<F> ProgressReporter for CallbackProgressReporter<F>
where
    F: Fn(u8, String) + Send + Sync,
{
    async fn report(&mut self, percent: u8, message: String) {
        (self.on_progress)(percent, message);
    }

    async fn error(&mut self, error: String) {
        (self.on_progress)(0, error);
    }

    async fn complete(&mut self, result: SyncResult) {
        (self.on_progress)(100, result.message);
    }

    async fn should_continue(&self) -> bool {
        !self.cancelled
    }
}

/// Progress reporter factory
pub struct ProgressReporterFactory;

impl ProgressReporterFactory {
    /// Create default logging reporter
    pub fn logging(operation: impl Into<String>) -> Box<dyn ProgressReporter> {
        Box::new(LoggingProgressReporter::new(operation))
    }

    /// Create TUI reporter
    pub fn tui() -> Box<dyn ProgressReporter> {
        Box::new(TuiProgressReporter::new())
    }

    /// Create callback reporter
    pub fn callback<F>(callback: F) -> Box<dyn ProgressReporter>
    where
        F: Fn(u8, String) + Send + Sync + 'static,
    {
        Box::new(CallbackProgressReporter::new(callback))
    }

    /// Create multi-reporter that reports to all provided reporters
    pub fn multi(reporters: Vec<Box<dyn ProgressReporter>>) -> Box<dyn ProgressReporter> {
        Box::new(MultiProgressReporter { reporters })
    }

    /// Create combined logging and TUI reporter
    pub fn logging_and_tui(operation: impl Into<String>) -> Box<dyn ProgressReporter> {
        Self::multi(vec![Self::logging(operation), Self::tui()])
    }
}
