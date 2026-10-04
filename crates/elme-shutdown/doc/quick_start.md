## Configure workers

#### Add task handle to worker struct(s)

First, add a `TaskHandle` field to your worker struct

```rust
# pub mod elme {
#     pub mod shutdown {
#         pub use elme_shutdown::{ShutdownManager, TaskHandle};
#     }
# }
use elme::shutdown::TaskHandle;

pub struct MyWorker {
    // ..,
    task_handle: TaskHandle,
}
```

Then add the handle to the worker's constructor

```rust
# use elme_shutdown::TaskHandle;
#
# pub struct MyWorker {
#     task_handle: TaskHandle,
# }
impl MyWorker {
    pub fn new(
        // ..,
        task_handle: TaskHandle,
    ) -> Self {
        Self {
            // ..,
            task_handle,
        }
    }
}
```

#### Handle worker graceful stop

There are numerous variations of worker run loops.
This example operates on a '*schedule*'-style worker.
It performs a short-running action at some interval.

See the crate
[`examples`](https://github.com/kylekingcdn/elme-rs/tree/main/crates/elme-shutdown/examples) for
implementations based on other common worker patterns.

---

For context, let's assume your worker's `run()` fn initially looked like:

```rust
# use elme_shutdown::TaskHandle;
use std::time::Duration;

# pub struct MyWorker {
#     task_handle: TaskHandle,
# }
impl MyWorker {
    # pub fn new(task_handle: TaskHandle) -> Self {
    #     Self { task_handle }
    # }
    pub async fn run(self) {
        let mut interval = tokio::time::interval(Duration::from_mins(5));
        loop {
            interval.tick().await;
            self.do_work().await;
        }
    }
    async fn do_work(&self) {
        // ..
    }
}
```

Graceful shutdown support is added by using a [`tokio::select`].
If teardown begins during the '*cooldown*' period, we return immediately.

The `run()` fn takes ownership of the worker, so it (and it's handle) drop immediately after.

```rust
# use elme_shutdown::TaskHandle;
use std::time::Duration;

# pub struct MyWorker {
#     task_handle: TaskHandle,
# }
impl MyWorker {
    # pub fn new(task_handle: TaskHandle) -> Self {
    #     Self { task_handle }
    # }
    pub async fn run(self) {
        let mut interval = tokio::time::interval(Duration::from_mins(5));
        loop {
            // we use biased here so that the teardown check is always evaluated first.
            // that way, we avoid working if teardown had already begun prior to the loop
            tokio::select! {
                biased;
                () = self.task_handle.wait_for_teardown_start() => { return; }
                _ = interval.tick() => self.do_work().await
            }
        }
    }
    async fn do_work(&self) {
        // ..
    }
}
```

## Setup manager

#### Configure manager

Next, we setup the [`ShutdownManager`] in the application's `main()` fn.

You may want to start with a timeout higher than the default of 10s. This helps avoid timeouts
off-the-bat, allowing you to get an idea of how long your teardown will typically take.

See [`ShutdownConfigBuilder`] for additional configuration.

```rust
# pub mod elme {
#     pub mod shutdown {
#         pub use elme_shutdown::ShutdownManager;
#     }
# }
use elme::shutdown::ShutdownManager;
use std::error::Error;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // other one-time/global init here (e.g. eyre, tracing, otel, etc.)
    // ...

    // init shutdown manager with custom config
    let shutdown_config = ShutdownManager::config_builder()
        .timeout(Duration::from_secs(30))
        .build();
    let shutdown_mgr = ShutdownManager::init(shutdown_config);

    // run code
    // ..

    Ok(())
}
```

#### Task registration

The [`TaskHandle`] needed to initialize `MyWorker` can be acquired with
[`register_task()`](ShutdownManager::register_task).

```rust
# use elme_shutdown::{ShutdownManager, TaskHandle};
# use std::error::Error;
# use std::time::Duration;
#
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // ...

    # // init shutdown manager with custom config
    # let shutdown_config = ShutdownManager::config_builder()
    #     .timeout(Duration::from_secs(30))
    #     .build();
    # let shutdown_mgr = ShutdownManager::init(shutdown_config);
    #
    let worker_handle = shutdown_mgr.register_task("My worker")?;
    let worker = MyWorker::new(worker_handle);

    // run code
    // ..
    tokio::spawn(async move { worker.run().await; });

    Ok(())
}
# pub struct MyWorker {
#     task_handle: TaskHandle,
# }
# impl MyWorker {
#     pub fn new(task_handle: TaskHandle) -> Self {
#         Self { task_handle }
#     }
#     pub async fn run(self) {
#         let mut interval = tokio::time::interval(Duration::from_mins(5));
#         loop {
#             tokio::select! {
#                 biased;
#                 () = self.task_handle.wait_for_teardown_start() => { return; }
#                 _ = interval.tick() => self.do_work().await
#             }
#         }
#     }
#     async fn do_work(&self) { }
# }
```

#### Lifecycle control

All that's left to do is integrate `elme-shutdown`'s built-in lifecycle features.

To do this:
1. Wrap any non-global init and all '*run*' code in a
   [`app_should_start()`](ShutdownManager::app_should_start) `while` loop block
    1. An [`inform_starting()`](ShutdownManager::inform_starting) call is added at the start of the
       loop to let `ShutdownManager` know when startup has begun
    1. An [`inform_started()`](ShutdownManager::inform_started) call is added after our setup/init
       code to let `ShutdownManager` know when startup has finished
    1. At the end of the loop, a
       [`wait_for_teardown_done()`](ShutdownManager::wait_for_teardown_done) call is added to delay
       exit/reload until all workers have gracefully stopped (or the time-out is reached)
1. We add a return call (and return type) to use the `exit_code` that is passed to
   [`stop()`](ShutdownManager::stop) calls

```rust
# use elme_shutdown::{ShutdownManager, TaskHandle};
# use std::error::Error;
use std::process::ExitCode
# use std::time::Duration;

#[tokio::main]
async fn main() -> Result<ExitCode, Box<dyn Error>> {
    // ...

    // init shutdown manager with custom config
    # let shutdown_config = ShutdownManager::config_builder()
    #     .timeout(Duration::from_secs(30))
    #     .build();
    // ..
    let shutdown_mgr = ShutdownManager::init(shutdown_config);

    // startup + run + teardown loop
    while shutdown_mgr.app_should_start() { // loops until a stop command is received
        // – startup
        shutdown_mgr.inform_starting()?;
        let worker_handle = shutdown_mgr.register_task("My worker")?;
        let worker = MyWorker::new(worker_handle);

        // – running
        shutdown_mgr.inform_started()?;
        tokio::spawn(async move { worker.run().await; });

        // - teardown
        shutdown_mgr.wait_for_teardown_done().await; // wait for tasks to end gracefully
    }

    Ok(shutdown_mgr.exit_code().unwrap_or(0).into())
}
# pub struct MyWorker {
#     task_handle: TaskHandle,
# }
# impl MyWorker {
#     pub fn new(task_handle: TaskHandle) -> Self {
#         Self { task_handle }
#     }
#     pub async fn run(self) {
#         let mut interval = tokio::time::interval(Duration::from_mins(5));
#         loop {
#             tokio::select! {
#                 biased;
#                 () = self.task_handle.wait_for_teardown_start() => { return; }
#                 _ = interval.tick() => self.do_work().await
#             }
#         }
#     }
#     async fn do_work(&self) {
#         let _ = self.task_handle.manager().stop(0);
#     }
# }
```

#### Add `tracing` writer

<div class="warning">Coming soon</div>
