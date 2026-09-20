mod worker;
use worker::WorkerManager;

use elme_shutdown::{Command, ShutdownConfig, ShutdownManager, StopCommand};
use std::time::Duration;
use std::process::ExitCode;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> color_eyre::Result<ExitCode> {
    // - begin one-time init
    color_eyre::install()?;

    // setup elme-shutdown
    let shutdown_config = ShutdownConfig::builder()
        .timeout(Duration::from_secs(15))
        .build();
    let shutdown_mgr = ShutdownManager::init(shutdown_config);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_ansi_sanitization(false);

    // prevent broken tracing logs w/ progress
    #[cfg(feature = "progress-writer")]
    let fmt_layer = fmt_layer.with_writer(shutdown_mgr.progress_writer());

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    // init + run loop
    while shutdown_mgr.app_should_start() {

        // – init app - //
        tracing::info!("Handling app startup...");
        shutdown_mgr.inform_starting().unwrap();

        let workers = WorkerManager::new(shutdown_mgr.register_task("Worker manager").unwrap());
        std::thread::sleep(Duration::from_millis(500)); // simulate longer startup with 0,5s delay

        // – run app - //
        shutdown_mgr.inform_started().unwrap();

        // the worker manager's run loop ends upon teardown, so thats all we need to wait on
        workers.run().await?;

        // - teardown app -- //
        tracing::info!("Teardown has started... waiting for graceful stop");
        // now wait for tasks to end gracefully
        shutdown_mgr.wait_for_teardown_done().await;
        tracing::info!("Tasks gracefully stopped\n\n----\n");
    }

    // - this will only be reached at the end of teardown
    //   if we are stopping (as opposed to reloading)
    let code = match shutdown_mgr.issued_command() {
        Some(Command::Stop(StopCommand { exit_code })) => exit_code,
        _ => -1
    };

    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    Ok(ExitCode::from(code as u8))
}
