use crate::{
    task::{
        InstanceCount,
        list::TrackedTaskList,
        map::TrackedTaskMap,
    },
};

use tokio::sync::broadcast;

// !- Task Registry

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RegistrationMessage {
    Register(&'static str),
    Unregister(&'static str),
}

#[derive(Debug)]
pub(crate) struct TaskRegistry {
    tasks: TrackedTaskMap,

    /// inform on changes to active task set
    registration_tx: broadcast::Sender<RegistrationMessage>,
}
impl Default for TaskRegistry {
    fn default() -> Self {
        Self {
            tasks: TrackedTaskMap::default(),
            registration_tx: Self::new_registration_channel(),
        }
    }
}
impl TaskRegistry {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub fn task_map(&self) -> &TrackedTaskMap {
        &self.tasks
    }
    pub fn as_task_list(&self) -> TrackedTaskList {
        self.tasks.as_list()
    }

    pub fn active_task_count(&self) -> usize {
        self.tasks.active_task_count()
    }
    pub fn has_active_tasks(&self) -> bool {
        self.tasks.has_active_tasks()
    }
    pub fn total_instance_count(&self) -> InstanceCount {
        self.tasks.instance_count()
    }
    pub fn task_instance_count(&self, task_name: &'static str) -> InstanceCount {
        self.tasks.task_instance_count(task_name)
    }

    pub fn registration_rx(&self) -> broadcast::Receiver<RegistrationMessage> {
        self.registration_tx.subscribe()
    }
    pub(crate) fn rebuild_registration_channel(&mut self) {
        self.registration_tx = Self::new_registration_channel();
    }
    fn new_registration_channel() -> broadcast::Sender<RegistrationMessage> {
        let (tx, _) = broadcast::channel(8);
        tx
    }

    pub(crate) fn register_task(&mut self, task_name: &'static str) {
        let prev_count = self.task_instance_count(task_name);
        let new_count = prev_count + 1.into();
        tracing::trace!(
            ?task_name,
            %prev_count,
            %new_count,
            "Registering task: {task_name} ({prev_count}->{new_count} instances)",
        );

        self.tasks.0.entry(task_name).or_default().0 += 1;

        // inform task was registered
        tracing::trace!("Notifying for registered task");
        // will return err if there aren't any receivers, which is fine.
        // should only have receivers during teardown
        let _ = self.registration_tx.send(RegistrationMessage::Register(task_name));
    }
    pub(crate) fn unregister_task(&mut self, task_name: &'static str) {
        let prev_count = self.task_instance_count(task_name);
        assert!(prev_count.0 >= 1, "cannot unregister task with count <= 1");
        let new_count = prev_count - 1.into();

        tracing::trace!(
            ?task_name,
            %prev_count,
            %new_count,
            "Unregistering task: {task_name} ({prev_count}->{new_count} instances)",
        );

        *self.tasks.0.entry(task_name).or_default() = new_count;

        // inform task was unregistered
        tracing::trace!("Notifying for unregistered task");
        // will return err if there aren't any receivers, which is fine.
        // should only have receivers during teardown
        let _ = self.registration_tx.send(RegistrationMessage::Unregister(task_name));
    }
}
