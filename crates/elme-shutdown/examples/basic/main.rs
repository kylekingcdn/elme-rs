mod worker;
use worker::WorkerManager;

use elme_shutdown::{ShutdownConfig, ShutdownManager};
use std::process::ExitCode;
use std::time::Duration;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> color_eyre::Result<ExitCode> {

    // ---- begin one-time init

    color_eyre::install()?;

    // - setup elme-shutdown
    let shutdown_config = ShutdownConfig::builder()
        .timeout(Duration::from_secs(15))
        .build();
    let shutdown_mgr = ShutdownManager::init(shutdown_config);

    // - setup tracing
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))?;
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_ansi_sanitization(false);

    // prevent broken tracing logs w/ elme-shutdown progress bars
    #[cfg(feature = "progress-writer")] // gated due to tracing-subscriber dependancy
    let fmt_layer = fmt_layer.with_writer(shutdown_mgr.progress_writer());

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    // ---- end one-time init

    // - init + run loop
    //   returns control upon stop command teardown completion
    while shutdown_mgr.app_should_start() { // provides support for live-reload

        // – init app
        //   any init that is not 'one-time' should happen here
        shutdown_mgr.inform_starting().unwrap();
        let workers = WorkerManager::new(shutdown_mgr.register_task("Worker manager")?);

        // – run app
        shutdown_mgr.inform_started()?;
        workers.run().await?; // the worker manager's run loop ends once teardown
                              // starts, so thats all we need to wait on
        // - teardown
        shutdown_mgr.wait_for_teardown_done().await; // now wait for tasks to end gracefully
    }

    // retrieve exit code provided by user calls to `stop()`
    let exit_code = shutdown_mgr.exit_code().unwrap_or(0);

    Ok(exit_code.into())
}
