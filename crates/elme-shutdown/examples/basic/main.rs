mod worker;
use worker::WorkerManager;

use elme_shutdown::ShutdownManager;
use std::process::ExitCode;
use std::time::Duration;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// `elme-shutdown` timeout
const TIMEOUT_SEC: u64 = 15;

/// Artificial delay added to startup
const STARTUP_DELAY_SEC: u64 = 3;

#[tokio::main]
async fn main() -> color_eyre::Result<ExitCode> {
    //
    // ---- begin one-time init
    //
    color_eyre::install()?;

    // - setup elme-shutdown
    let shutdown_config = ShutdownManager::config_builder()
        .timeout(Duration::from_secs(TIMEOUT_SEC))
        .build();
    let shutdown_mgr = ShutdownManager::init(shutdown_config);

    // - setup tracing
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))?;
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_ansi_sanitization(false);

    // prevent broken tracing logs w/ elme-shutdown progress bars
    #[cfg(feature = "progress-writer")]
    let fmt_layer = fmt_layer.map_writer(|w| shutdown_mgr.mapped_progress_writer(w));
    //              ^^^^^^^^^^^^^^^^^^^^
    // NOTE: you do not need a feature gate or separate call for `map_writer`.
    //       it can be inlined into the `fmt::layer()` builder (but it must be last)

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();
    //
    // ---- end one-time init
    //

    // - init + run loop
    //   returns control upon stop command teardown completion
    while shutdown_mgr.app_should_start() { // provides support for live-reload

        // – init app
        //   any init that is not 'one-time' should happen here
        shutdown_mgr.inform_starting()?;
        let worker_mgr_handle = shutdown_mgr.register_task("Worker manager")?;
        let workers = WorkerManager::try_new(worker_mgr_handle)?;
        tracing::warn!("Delaying startup for {STARTUP_DELAY_SEC}");
        tokio::time::sleep(Duration::from_secs(STARTUP_DELAY_SEC)).await;

        // – run app
        shutdown_mgr.inform_started()?;
        workers.run();  // manager spawns all tasks (non-async), and returns
                        // ..maybe manager isn't the best name to use..
        // - teardown
        shutdown_mgr.wait_for_teardown_done().await; // now wait for tasks to end gracefully
    }

    // retrieve exit code provided by user calls to `stop()`
    let exit_code = shutdown_mgr.exit_code().unwrap_or(0);

    Ok(exit_code.into())
}
