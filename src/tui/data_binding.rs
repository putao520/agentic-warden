use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Error, Result};

use crate::{
    logging::debug, provider::manager::ProviderManager, registry_factory::create_cli_registry,
    registry_factory::CliRegistry,
};

use super::app_state::AppState;
use crate::common::constants::duration::PROVIDER_REFRESH_INTERVAL_SECS;

const TASK_REFRESH_INTERVAL_MS: u64 = 750;

/// Background controller that keeps [`AppState`] in sync with disk/registry state.
pub struct DataBindingController {
    shutdown: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl DataBindingController {
    pub fn start() -> Self {
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_shutdown = shutdown.clone();
        let handle = thread::Builder::new()
            .name("tui-data-binding".into())
            .spawn(move || {
                let app_state = AppState::global();
                let mut registry: Option<CliRegistry> = None;
                let mut last_provider_refresh = Instant::now()
                    .checked_sub(Duration::from_secs(PROVIDER_REFRESH_INTERVAL_SECS))
                    .unwrap_or_else(Instant::now);

                while !thread_shutdown.load(Ordering::Relaxed) {
                    if last_provider_refresh.elapsed()
                        >= Duration::from_secs(PROVIDER_REFRESH_INTERVAL_SECS)
                    {
                        if let Err(err) = Self::refresh_providers(app_state) {
                            debug(format!("provider snapshot refresh failed: {err:?}"));
                        }
                        last_provider_refresh = Instant::now();
                    }

                    if let Err(err) = Self::refresh_tasks(app_state, &mut registry) {
                        debug(format!("task snapshot refresh failed: {err:?}"));
                        registry = None;
                    }

                    thread::sleep(Duration::from_millis(TASK_REFRESH_INTERVAL_MS));
                }
            })
            .ok();

        Self { shutdown, handle }
    }

    fn refresh_providers(app_state: &'static AppState) -> Result<()> {
        let manager = ProviderManager::new().context("failed to load providers config")?;
        let providers = manager
            .list_providers()
            .into_iter()
            .map(|(id, provider)| (id.clone(), provider.clone()))
            .collect::<Vec<_>>();
        app_state.set_providers(providers, Some(manager.default_provider_name().to_string()));
        Ok(())
    }

    fn refresh_tasks(
        app_state: &'static AppState,
        registry: &mut Option<CliRegistry>,
    ) -> Result<()> {
        if registry.is_none() {
            match create_cli_registry() {
                Ok(connected) => *registry = Some(connected),
                Err(err) => {
                    return Err(Error::new(err).context("failed to connect CliRegistry"));
                }
            }
        }

        if let Some(registry) = registry {
            let entries = registry.entries().context("failed to read task entries")?;
            app_state.replace_tasks_from_registry(entries);
        }

        Ok(())
    }
}

impl Default for DataBindingController {
    fn default() -> Self {
        Self::start()
    }
}

impl Drop for DataBindingController {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
