#![allow(dead_code)]

use elme_shutdown::TaskHandle;
use tokio::time::{interval, MissedTickBehavior};
use std::time::Duration;

const SHORT_WORK_MIN: u64 = 1;
const SHORT_WORK_MAX: u64 = 3;

const LONG_WORK_MIN: u64 = 5;
const LONG_WORK_MAX: u64 = 20;

// !- Busy/long-running worker

/// An example of a worker that could work constantly, or be working for long periods of time
///
/// > [!WARNING]
/// > If your loop may be evaluated often (1 or more times per sec),
/// > you should instead use `wait_for_teardown_started()` with a join handle.
/// >
/// > See [`IntermittentWorker`]
pub struct BusyWorker {
    // task handle could alternatively be added as a field if workers are init'd during startup
}
impl BusyWorker {
    /// primary worker loop, consumes self to indicate completion
    pub async fn run(&self, handle: TaskHandle) {
        while !handle.should_teardown() {
            // simulate long running, blocking batch work
            let _ = tokio::task::spawn_blocking(Self::work).await;
        }
        tracing::info!("BusyWorker stopping");
    }
    /// blocking work that takes between 5s and 20s
    // #[tracing::instrument(skip_all, fields(worker="BusyWorker"))]
    fn work() {
        tracing::info!(handle=%"Busy worker", "working on job");

        std::thread::sleep(util::rand_long_duration());
    }
}

// !- One shot worker

/// An example of a worker that is created to process a single job and return
pub struct OneShotWorker {
    work_time: Duration,

    /// [`TaskHandle`] can also be stored in the worker as a field.
    ///
    /// This guarantees tasks are fully dropped, and reduces noise in your run worker logic
    ///
    /// Storing the handle as a member field also improves tracing spans,
    /// as you will have access to the task name from the run fn
    handle: TaskHandle,
}
impl OneShotWorker {
    pub fn new(handle: TaskHandle) -> Self {
        Self {
            handle,
            work_time: util::rand_short_duration(),
        }
    }
    // #[tracing::instrument(skip(self), fields(worker=self.handle.task_name(), work_time=?self.work_time.as_secs()))]
    pub async fn run(self) {
        // simulate short work
        tokio::time::sleep(self.work_time).await;

        tracing::debug!("OneShotWorker finished it's job");
    }
}

// !- Periodic worker

/// An example of a background task that ocassionally does some work.
///
/// This is commonly used for tasks that are done on a schedule, interval, or cooldown.
pub struct IntermittentWorker {
    handle: TaskHandle,
}
impl IntermittentWorker {
    // does brief work on a schedule/interval of 30s
    const WORK_INTERVAL: u64 = 30;

    /// primary work loop
    ///
    /// every 30s, it will work on a task that takes 1-3 seconds to finish
    pub async fn run(self) {
        let mut interval = interval(Duration::from_secs(Self::WORK_INTERVAL));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        // run scheduled tasks until we want to teardown
        while !self.handle.should_teardown() {
            // within the main loop, we use a select! branch with
            // an interval to immediately bail out if we are in the
            // "cooldown" period and teardown is invoked
            tokio::select! {
                biased;

                () = self.handle.wait_for_teardown_start() => {},

                // simulate work with cooldown
                _ = interval.tick() => self.work().await,
            }
        }

        tracing::info!("IntermittentWorker stopping");
    }

    /// non-blocking work that takes between 1s and 3s
    // #[tracing::instrument(skip(self), fields(worker=self.handle.task_name()))]
    async fn work(&self) {
        tracing::info!(handle=%"Intermittent worker", "working on job");
        // self.handle.manager().task_list().dump_tasks();
        tokio::time::sleep(util::rand_short_duration()).await;
    }
}

// !- Dispatched/delegated jobs

/// An example of a worker that has "jobs" issued to it.
///
/// This can be implemented with a channel, message broker, pub/sub, etc.
pub struct DelegatedWorker {
    handle: TaskHandle,

    id: u16,
    rx: async_channel::Receiver<JobMessage>
}
impl DelegatedWorker {
    /// primary work loop
    ///
    /// receives messages and works on jobs as they are sent
    ///
    /// ## `TaskHandle usage`
    /// We use `async_channel` here, allowing jobs to be split up among workers in a first available fashion.
    /// Unlike other workers, we don't actually
    /// handle any teardown logic/checks.
    ///
    /// Instead, the job dispatcher will close the channel and stop sending jobs
    /// to the delegated workers. We work on the jobs in the channel until it's exhausted,
    /// and then exit.
    ///
    /// Had we used the same pattern as above, jobs woud be accepted by the dispatcher,
    /// queued in the bounded channel and then abandoned, as the worker would also bail out when
    /// the teardown notification goes out.
    ///
    /// ## So why even provide a `TaskHandle` to the `DelegatedWorker`?
    ///
    /// We store the `TaskHandle` here as it is what allows us to include the worker in the graceful shutdown logs
    /// and progress tracker.
    ///
    /// In the event of a teardown timeout, this will provide valuable statistics on what tasks timed out.
    ///
    /// Once this worker is dropped, it's stored [`TaskHandle`] will be dropped as well
    /// - thus notifying elme's Shutdown handler of task completion.
    ///
    /// > [!WARNING]
    /// > channel queue size and average job worker duration must be taken into account when
    /// > determining an elme-shutdown timeout value.
    /// >
    /// > As a starting point, you may use a timeout such as:
    /// > ```text
    /// > ((worker_count + channel_size) * avg_job_time_secs / worker_count) * 4
    /// > ```
    /// >
    /// > **The logic behind this formula is**
    /// >
    /// > - **max job**: `(worker_count + channel_size)`:
    /// >   - The max number of jobs that may need processing at any given time
    /// >   - `worker_count` is included for in-progress jobs
    /// > - **The trailing "` * 4`"**
    /// >   - This is a multiplier to serve as a grace period. E.g. we want to be done these
    /// >     jobs in 1/4 the total timeout, to still allow us to handle significantly
    /// >     worse-case scenarios
    /// > Simplifying to `max_jobs * avg_job_time_secs / worker_count * grace_factor`
    pub async fn run(self) {
        while let Ok(msg) = self.rx.recv().await {
            tracing::info!(handle=%self.handle.task_name(), id=%self.id, "working on job (queued: {})", self.rx.len()+1);
            tokio::time::sleep(msg.work_time).await; // simulate work
        }
    }
}

/// Simulated job message
///
/// we're using `work_time` to provide random range work durations, better simulating
/// dynamic worker/job timings.
struct JobMessage {
    pub work_time: Duration,
}

struct Dispatcher {
    handle: TaskHandle,
    tx: async_channel::Sender<JobMessage>,
}
impl Dispatcher {
    pub async fn run(self) {
        // constantly saturate the channel until shutdown is issued
        loop {
            let msg = JobMessage { work_time: util::rand_short_duration() };
            tokio::select! {
                biased;
                () = self.handle.wait_for_teardown_start() => { tracing::info!("Dispatcher stopping"); return; }
                _ = self.tx.send(msg) => { tracing::trace!("Dispatched job"); },
            };
        }
    }
}

// !- Worker manager

/// Constructs/spawns workers and provides error handling
pub struct WorkerManager {
    handle: TaskHandle,

}
impl WorkerManager {
    // for dispatcher -> deletegated worker channel
    const WORKER_COUNT: usize = 4;
    const CHANNEL_SIZE: usize = 10;

    pub fn new(handle: TaskHandle) -> Self {
        Self {
            handle,
        }
    }

    /// Consumes self to ensure that the owned handle field is dropped immediately after call to [`run()`](Self::run)
    pub async fn run(self) -> color_eyre::Result<()> {
        let inter = IntermittentWorker {
            handle: self.handle.register_task("Intermittent worker")?,
        };

        let busy = BusyWorker { };
        let busy_handle = self.handle.register_task("Busy worker")?;

        // init dispatcher + delegated workers
        // we nest this to ensure that the initial, uncloned `rx`, is dropped
        let dispatcher = {
            let (tx, rx) = async_channel::bounded(Self::CHANNEL_SIZE); // formula gives timeout of 48s for avg 2s
            let dispatcher = Dispatcher {
                handle: self.handle.register_task("Dispatcher")?,
                tx,
            };
            tracing::info!("Spawning delegated workers");
            for i in 0..Self::WORKER_COUNT {
                // spawn these, but dont join on them.
                // in reality, you should use a join set and push the tasks to it
                let delegated = DelegatedWorker {
                    handle: self.handle.register_task("Delegated worker")?,
                    id: u16::try_from(i).unwrap(),
                    rx: rx.clone(),
                };

                tokio::task::spawn(async move { delegated.run().await; });
            }
            dispatcher
        };

        // random one shot worker
        let one_shot = OneShotWorker {
            work_time: util::rand_long_duration(),
            handle: self.handle.register_task("One shot worker")?,
        };

        tokio::join!(
            inter.run(),
            busy.run(busy_handle),
            dispatcher.run(),
            one_shot.run(),
        );

        tracing::info!("WorkerManager finished");

        Ok(())
    }
}

// !- utils


/// Utility fn's that aren't relevant to elme-shutdown example
mod util {
    use super::Duration;
    pub(crate) fn rand_duration(min_s: u64, max_s: u64) -> Duration {
        let min = Duration::from_secs(min_s);
        let max = Duration::from_secs(max_s);
        rand::random_range(min..max)
    }
    pub(crate) fn rand_short_duration() -> Duration {
        rand_duration(super::SHORT_WORK_MIN, super::SHORT_WORK_MAX)
    }
    pub(crate) fn rand_long_duration() -> Duration {
        rand_duration(super::LONG_WORK_MIN, super::LONG_WORK_MAX)
    }
}
