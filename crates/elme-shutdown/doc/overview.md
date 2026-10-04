This is a high-level overview of the core concepts that make up the foundation of `elme-shutdown`.

## Tasks

For an application to handle gracefully stopping background tasks/workers,
the following conditions must be met:

1. The task must be made aware that teardown (graceful shutdown) has been requested
1. The task must inform the shutdown system when it has finished
1. The shutdown system must keep track of unfinished tasks to delay exit until:
   - All tasks have finished, or
   - A timeout is reached (preventing indefinite waiting on stalled tasks)

`elme-shutdown` provides *`TaskHandle`* to uphold these requirements.

#### Task handles

A [`TaskHandle`] is used to represent a single instance of a background worker.

A handle can be acquired by registering a task via [`ShutdownManager::register_task`].

Tasks are registered with a name to aid in identification. For example:

```rust
# use std::error::Error;
# #[tokio::main]
# async fn main() -> Result<(), Box<dyn Error>> {
# pub mod elme {
#     pub mod shutdown {
#         pub use elme_shutdown::ShutdownManager;
#     }
# }
use elme::shutdown::ShutdownManager;

let shutdown_mgr = ShutdownManager::default();
// ..

// setup worker
let worker_handle = shutdown_mgr.register_task("My custom worker")?;
let worker = MyWorker::new(worker_handle);

// run worker
tokio::spawn(async move { worker.run().await; });
# Ok(())
# }
# use elme_shutdown::TaskHandle;
# pub struct MyWorker { task_handle: TaskHandle }
# impl MyWorker {
#     pub fn new(task_handle: TaskHandle) -> Self { Self { task_handle } }
#     pub async fn run(self) { }
#     async fn do_work(&self) { }
# }
```

Upon registration, a counter for the number of running instances for a given task (by name) is
incremented.

Once a `TaskHandle` is dropped, the instance counter associated with the task name is decremented.

##### Additional uses

A [`TaskHandle`] can also be used to:
- Check whether or not teardown has begun
- Provide a future that can be used to wait until teardown begins
- Access the associated [`ShutdownManager`] (e.g. to trigger shutdown from a task)
- Register additional tasks

## Teardown

'***Teardown***' (or '*tearing down*') is the term used to refer to the 'graceful shutdown' period of
the application's lifecycle.

Conceptually, it is the opposite of '***start-up***' (or '*starting up*').

This distinction is made as teardown plays an equally important role in not just `Stop` commands,
but `Reload` commands too.

To refer to this period as "shutting down", when the application is actually restarting, would
be confusing to say the least.

<!--
Teardown is considered complete once all task handles are dropped (all instance counters == 0).
-->

## Lifecycle

`elme-shutdown` classifies an application's lifecycle using the 3 following stages:

1. **Startup**
   - The 'init' stage
   - The application is getting ready to serve its intended purpose
1. **Running**
   - The 'ready' stage
   - After starting up, the application is serving its intended purpose
1. **Teardown**
   - The 'cleanup' stage
   - All work stopped, all non-globals dropped, control returns to `main`

In code, these states are provided by the [`LifecycleStage`] enum.

Handling of an application's lifecycle is done via:
- [`inform_starting`](ShutdownManager::inform_starting)
- [`inform_started`](ShutdownManager::inform_started)
- [Commands](#Commands)

<!--
### Run state

There is an additional (and less crucial) set of states, [`RunState`],
that are more command/action oriented.

Without diving into the details here, it is mentioned to note the following:

[`RunState`] is not the same as [`LifecycleStage`].
-->

## Commands

`elme-shutdown` has two commands used to handle 'exit' procedures: ***`Stop`*** and ***`Reload`***.

In code, commands are provided by the [`Command`](command::Command) enum.

#### Stop

`Stop` is the standard 'graceful shutdown' command.

After issuing `Stop`, the following sequence of steps occur:
1. Background tasks gracefully finish or cancel their work (teardown)
2. The application has its control returned to `main()`
3. The application exits

<!--`Stop` can be issued with [`ShutdownManager::stop()`](ShutdownManager::stop)-->

#### Reload

The `Reload` command is similar to `Stop`, with the *only change occurring in step #3*.

After issuing `Reload`, the following sequence of steps occur:
1. Background tasks gracefully finish or cancel their work (teardown)
2. The application has its control returned to `main()`
3. Instead of exiting, the application begins its startup and run sequence again

Because the first 2 steps are identical, implementing `Reload` support typically only requires
a single additional line in your `main()` function.

A helper method, [`app_should_start`](ShutdownManager::app_should_start), is provided for
proper handling of step #3 in either command.

<!--`Reload` can be issued with [`ShutdownManager::reload()`](ShutdownManager::reload)-->

Reload support is enabled by default. It can be disabled through [`ShutdownConfig::reload_enabled`].

## Signal handling

`elme-shutdown` ships with optional support for signal handling.
This allows, for example, triggering a `stop` command on `ctrl`+`c`.

The following table lists supported signals and the command associated with their invocation.

| Signal | Command |
| ------ | ------- |
| `INT` | `Stop` |
| `TERM` | `Stop` |
| `HUP` | `Reload` (when enabled, otherwise `Stop`) |

<!--
**Note:** If reload support has been disabled (via [`ShutdownConfig`]),
a `HUP` signal invokes `Stop` instead of `Reload`.
-->

Signal handling is enabled by default.
It can be disabled through [`ShutdownConfig::handle_signals`].

#### Repeat signal invocations

By default, if a signal is received more than once, the application will be terminated immediately.

This behaviour can be disabled through [`ShutdownConfig::terminate_on_second_signal`].

## Teardown futures

Polling a teardown [`Future`] is an efficient strategy for reacting to teardown start/end events.

#### Teardown started future

[`ShutdownManager`] provides a [`wait_for_teardown_start()`] method, returning a `Future`
which will be resolved once teardown starts.
If teardown has already started, the future will resolve immediately.

This is typically used by tasks to trigger their graceful stop logic.

For convenience, [`TaskHandle`] also contains a
[`wait_for_teardown_start()`](TaskHandle::wait_for_teardown_start) method providing identical
functionality.

#### Teardown completed future

[`ShutdownManager`] also provides a [`wait_for_teardown_done()`] method that returns a `Future`
which will be resolved once teardown completes successfully (all `TaskHandle`'s dropped).
If teardown has already finished, the future will resolve immediately.

This is typically used from `main()` to delay exit/restart until all tasks have finished
gracefully.

Unlike `wait_for_teardown_start()`, `TaskHandle` does not provide a `wait_for_teardown_done()`
method, as this usage would almost always be an anti-pattern (the future will never complete
because the associated handle likely remains in scope, preventing teardown from finishing).

## Teardown progress bars

Enabling the `progress` feature provides automatic support for teardown progress bars.

#### Tracing compatibility

If you are using the [`tracing`](::tracing) crate, it's highly recommended to use
[`ProgressWriter`] to avoid broken output. This requires the `progress-writer` feature.

See the [`ProgressWriter`] docs for more information.

#### Example output

<pre> [0/1] <span style="color:green;">⠁</span> Busy worker
 [0/4] <span style="color:green;">⠤</span> Delegated worker
 [1/1] <span style="color:green;">✔</span> Dispatcher
 [1/1] <span style="color:green;">✔</span> Intermittent worker
 [0/1] <span style="color:green;">⠒</span> One shot worker
 [0/1] <span style="color:green;">⠲</span> Worker manager
 <span style="font-weight:bold;">Progress</span> [<span style="color:green;">#######</span ><span style="color:grey;">----------------------</span>][ 2/9 ]
 <span style="font-weight:bold;">Time-out</span> [<span style="color:#FCBC09;">===============></span ><span style="color:grey;">-------------</span>][-0:07]</pre>

## Teardown reports

For improved monitoring and simpler diagnosing of timeouts,
`elme-shutdown` will automatically output reports containing teardown and timeout stats.

#### Successful teardown report

The [`tracing::Level`] used for successful teardown reports can be changed
via [`log_teardown_stats_level`](ShutdownConfig::log_teardown_stats_level). The default
is `Level::INFO`.

Alternatively, successful report output can be entirely disabled with
[`log_teardown_stats`](ShutdownConfig::log_teardown_stats).

For custom reports, disable output and set a custom callback that includes custom rendering
via [`ShutdownManager::on_teardown`].

**Example output:**

<pre>----------------------------------------
<span style="font-weight:bold;color:teal;">============</span> <span style="font-weight:bold;">Teardown Stats</span> <span style="font-weight:bold;color:teal;">============</span>
----------------------------------------
Started at:    <span style="font-weight:bold;">2026-10-01 07:31:12</span>
Finished at:   <span style="font-weight:bold;">2026-10-01 07:31:22</span>

Duration:      <span style="font-weight:bold;">10.589s</span>
Timeout:       <span style="font-weight:bold;">15s</span>

<span style="font-weight:bold;">Gracefully stopped tasks</span>
 - Busy worker          [<span style="color:green;">1/1</span>]
 - Delegated worker     [<span style="color:green;">4/4</span>]
 - Dispatcher           [<span style="color:green;">1/1</span>]
 - Intermittent worker  [<span style="color:green;">1/1</span>]
 - One shot worker      [<span style="color:green;">1/1</span>]
 - Worker manager       [<span style="color:green;">1/1</span>]
----------------------------------------</pre>

#### Timed-out teardown report

The [`tracing::Level`] used for timed-out teardown reports can be changed
via [`log_timeout_stats_level`](ShutdownConfig::log_timeout_stats_level). The default
is `Level::ERROR`.

Alternatively, timeout report output can be entirely disabled with
[`log_timeout_stats`](ShutdownConfig::log_timeout_stats).

For custom reports, disable output and set a custom callback that includes custom rendering
via [`ShutdownManager::on_timeout`].

**Example output:**

<pre>----------------------------------------
<span style="color:red;">=======</span> <span style="font-weight:bold;">Timed-out Teardown Stats</span> <span style="color:red;">=======</span>
----------------------------------------
Started at:    <span style="font-weight:bold;">2026-10-01 07:46:51</span>
Timed-out at:  <span style="font-weight:bold;">2026-10-01 07:47:06</span>

Duration:      <span style="font-weight:bold;">15.003s</span>
Timeout:       <span style="font-weight:bold;">15s</span>

<span style="font-weight:bold;">Gracefully stopped tasks</span>
 - Delegated worker     [<span style="color:#FCBC09;">1/4</span>]
 - Dispatcher           [<span style="color:green;">1/1</span>]
 - Intermittent worker  [<span style="color:green;">1/1</span>]

<span style="font-weight:bold;">Timed-out tasks</span>
 - Busy worker       [<span style="color:red;">1/1</span>]
 - Delegated worker  [<span style="color:red;">3/4</span>]
 - One shot worker   [<span style="color:red;">1/1</span>]
 - Worker manager    [<span style="color:red;">1/1</span>]
----------------------------------------</pre>

[`wait_for_teardown_start()`]: ShutdownManager::wait_for_teardown_start
[`wait_for_teardown_done()`]: ShutdownManager::wait_for_teardown_done
