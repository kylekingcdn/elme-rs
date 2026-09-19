use crate::{
    //listener::TeardownStartListener,
    manager::ShutdownManager,
    state::{LockingSharedState, RegisterError, SharedState},
};

// !- TODO: add support for instance ids (for logging/tracing spans, etc)
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
    pub fn register_task(&self, task_name: &'static str) -> Result<TaskHandle, RegisterError> {
        SharedState::register_task(&self.shared, task_name)
    }

    #[must_use]
    pub fn task_name(&self) -> &'static str {
        self.task_name
    }

    #[must_use]
    pub fn manager(&self) -> ShutdownManager {
        self.shared.clone().into()
    }
    #[allow(clippy::missing_panics_doc)]
    pub fn wait_for_teardown_start(&self) -> impl Future<Output=()> {
        let token = self.shared.lock().unwrap().teardown_start_token();
        token.cancelled_owned()
    }

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
