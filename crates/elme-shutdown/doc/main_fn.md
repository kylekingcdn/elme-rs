There are strict checks in place to ensure application state remains consistent.

Sticking to the following pattern should avoid all possible error scenarios:

```rust
use elme_shutdown::{ShutdownManager, TaskHandle};
use std::error::Error;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> Result<ExitCode, Box<dyn Error>> { //
    // one-time/global init here (e.g. eyre, tracing, otel, etc.)
    // ...

    // init shutdown manager
    let shutdown_mgr = ShutdownManager::default();
    #
    # // add callback for printing stats during doctest
    # shutdown_mgr.on_teardown(|stats| {
    #     println!("Teardown finished:\n\n{}", stats.report_text());
    # });

    // startup + run + teardown loop
    while shutdown_mgr.app_should_start() { // loops until a stop command is received
        // – startup
        shutdown_mgr.inform_starting()?;
        let api = Api::new(shutdown_mgr.register_task("API")?);
        let notifier = Notifier::new(shutdown_mgr.register_task("Notifier")?);

        // – running
        shutdown_mgr.inform_started()?;
        tokio::spawn(async move { notifier.run().await; });
        api.run().await;

        // - teardown
        shutdown_mgr.wait_for_teardown_done().await; // wait for tasks to end gracefully
    }

    Ok(shutdown_mgr.exit_code().unwrap_or(0).into())
}

pub struct Api {
     task_handle: TaskHandle,
}
impl Api {
     pub fn new(task_handle: TaskHandle) -> Self {
         Self { task_handle }
     }
     pub async fn run(self) {
         // ...
         # let _ = self.task_handle.manager().stop(0);
         # self.task_handle.wait_for_teardown_start().await;
     }
}

pub struct Notifier {
     task_handle: TaskHandle,
}
impl Notifier {
     pub fn new(task_handle: TaskHandle) -> Self {
         Self { task_handle }
     }
     pub async fn run(self) {
         // ...
         # self.task_handle.wait_for_teardown_start().await;
     }
}
```
