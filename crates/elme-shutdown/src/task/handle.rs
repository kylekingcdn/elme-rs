use crate::{
    manager::ShutdownManager,
    state::{LockingSharedState, SharedState},
    task::RegisterError,
};

// !- TODO: add support for instance ids (for logging/tracing spans, etc)
/// A [`TaskHandle`] is used to represent a single instance of a background worker.
///
/// A handle can be acquired by registering a task via
/// [`ShutdownManager::register_task`](crate::ShutdownManager::register_task).
///
/// Tasks are registered with a name to aid in identification.
///
/// ## Behavior
///
/// Upon registration, a counter of the number of running instances of the task (mapped by name),
/// is incremented.
///
/// Once a `TaskHandle` is dropped, the instance counter associated with the task
/// name is decremented.
///
/// Cloning a `TaskHandle` is identical to registering a new task of the same name.
///
/// ## Examples
///
/// Registering a task and passing the `TaskHandle` to some custom worker:
///
/// ```
#[doc = include_str!("../../doc/elme_proxy.rs")]
/// use elme::shutdown::{ShutdownManager, TaskHandle};
///
/// pub struct MyWorker {
///      task_handle: TaskHandle,
/// }
/// impl MyWorker {
///      pub fn new(task_handle: TaskHandle) -> Self {
///          Self { task_handle }
///      }
///      pub async fn run(self) {
///          // ...
///          # self.task_handle.wait_for_teardown_start().await;
///          # let _ = self.task_handle.manager().stop(0);
///      }
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let shutdown_mgr = ShutdownManager::default();
///     shutdown_mgr.inform_starting();
///
///     // since task is registered during startup (in the same thread) we can safely unwrap
///     let my_worker_handle = shutdown_mgr.register_task("My worker").unwrap();
///     let my_worker = MyWorker::new(my_worker_handle);
///
///     // done startup
///     shutdown_mgr.inform_started();
///     // start worker
///     tokio::spawn(async move { my_worker.run().await; });
///
///     // wait for teardown completion
///     shutdown_mgr.wait_for_teardown_done().await;
/// }
/// ```
pub struct TaskHandle {
    task_name: &'static str,
    shared: LockingSharedState,
}
impl TaskHandle {
    pub(crate) fn new(task_name: &'static str, shared: LockingSharedState) -> Self {
        Self {
            task_name,
            shared,
        }
    }

    /// This is identical to
    /// [`ShutdownManager::register_task`](crate::ShutdownManager::register_task).
    ///
    /// See the
    /// [`ShutdownManager` method](crate::ShutdownManager::register_task) for more information.
    pub fn register_task(&self, task_name: &'static str) -> Result<TaskHandle, RegisterError> {
        SharedState::register_task(&self.shared, task_name)
    }

    /// The task name assigned during registration
    #[must_use]
    pub fn task_name(&self) -> &'static str {
        self.task_name
    }

    /// The task name assigned during registration
    #[must_use]
    pub fn manager(&self) -> ShutdownManager {
        self.shared.clone().into()
    }

    /// Returns a [`Future`] which will be resolved once teardown starts.
    ///
    /// If teardown has already started, the future will resolve immediately.
    ///
    /// This is typically used by tasks to trigger their graceful stop logic.
    ///
    /// This is identical to
    /// [`ShutdownManager::wait_for_teardown_start`](crate::ShutdownManager::wait_for_teardown_start).
    #[allow(clippy::missing_panics_doc)]
    pub fn wait_for_teardown_start(&self) -> impl Future<Output=()> {
        let token = self.shared.lock().unwrap().teardown_start_token();
        token.cancelled_owned()
    }

    /// Returns true if teardown has started.
    ///
    /// This can be used as a simpler graceful stop condition check
    /// for workers which run loops very frequently - where it can guarantee check-in multiple
    /// times within a single span of the timeout duration.
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn should_teardown(&self) -> bool {
        self.shared.lock().unwrap().teardown_started()
    }
}
impl Clone for TaskHandle {
    fn clone(&self) -> Self {
        Self::new(self.task_name, self.shared.clone())
    }
}
impl Drop for TaskHandle {
    fn drop(&mut self) {
        SharedState::unregister_task(&self.shared, self.task_name);
    }
}
