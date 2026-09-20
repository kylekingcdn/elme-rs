use crate::task::{
    InstanceCount,
    Itemize,
    TaskData,
    TransitionInstanceCount,
};

// ! Task List

#[derive(Debug, Clone, Default)]
pub struct TaskList<I: Itemize>(
    pub(crate) Vec<TaskData<I>>,
);
impl<I: Itemize> TaskList<I> {
    /// Borrows the underlying vec
    #[must_use]
    pub fn inner(&self) -> &Vec<TaskData<I>> {
        &self.0
    }

    /// Converts into the underlying Vec
    #[must_use]
    pub fn into_inner(self) -> Vec<TaskData<I>> {
        self.into()
    }

    /// Number of distinct tasks (by name). Does not take into account instances.
    #[must_use]
    pub fn task_total(&self) -> usize {
        self.0.len()
    }
    /// If the list contains any active tasks
    #[must_use]
    pub fn has_active_tasks(&self) -> bool {
        self.0.iter().any(TaskData::is_active)
    }
    /// For `TrackedTaskList`, number of tasks. For `TransitioningTaskList`, number of tasks that aren't fully transitioned
    #[must_use]
    pub fn active_task_total(&self) -> usize {
        self.0.iter().filter(|i| i.is_active()).count()
    }
    /// Removes task entries with 0 instances
    pub fn filter_active_only(&mut self) {
        self.0 = self.0.iter().copied().filter(TaskData::is_active).collect();
    }
    /// Returns a list with active tasks only
    #[must_use]
    pub fn into_active_filtered(self) -> Self {
        Self(self.0.into_iter().filter(TaskData::is_active).collect())
    }

    /// Sorts tasks by name
    pub fn sort_tasks(&mut self) {
        self.0.sort();
    }

    pub fn dump_tasks(&self) {
        if !self.0.is_sorted() {
            tracing::warn!("dump_tasks() called with unsorted task list");
        }
        for t in &self.0 {
            println!("- {t}");
        }
    }
}
impl<I: Itemize> From<TaskList<I>> for Vec<TaskData<I>> {
    fn from(task_list: TaskList<I>) -> Self {
        task_list.0
    }
}

// !- InstanceCount task list

pub type TrackedTaskList = TaskList<InstanceCount>;

impl TrackedTaskList {
    /// Sum of each task's instance count
    #[must_use]
    pub fn instances_total(&self) -> InstanceCount {
        self.0.iter().map(|t| t.inner).sum()
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.active_task_total() == 0
    }
    #[must_use]
    pub fn has_tasks(&self) -> bool {
        !self.is_empty()
    }
}

// ! TransitionInstanceCount task list

pub type TransitioningTaskList = TaskList<TransitionInstanceCount>;

impl TransitioningTaskList {
    #[must_use]
    pub fn instances_transitioned_total(&self) -> InstanceCount {
        self.0.iter().map(|t| t.inner.transitioned()).sum()
    }
    #[must_use]
    pub fn instances_remaining_total(&self) -> InstanceCount {
        self.0.iter().map(|t| t.inner.remaining).sum()
    }
    /// Sum of each task's instance count
    #[must_use]
    pub fn instances_total(&self) -> InstanceCount {
        self.0.iter().map(|t| t.inner.total).sum()
    }

    /// Number of tasks with 0 instance transitions remaining
    #[must_use]
    pub fn transitioned_task_total(&self) -> usize {
        self.0.iter().filter(|t| t.is_fully_transitioned()).count()
    }

    /// Returns the list of tasks with the number of instances transitioned (excludes tasks with 0 instances transitioned)
    #[must_use]
    pub fn as_transitioned_tasks(&self) -> TrackedTaskList {
        TaskList(self.0
            .iter()
            .filter(|t| usize::from(t.inner.transitioned()) > 0)
            .map(|t| TaskData::new_with_data(t.task_name(), t.inner().transitioned()))
            .collect()
        )
    }
    /// Returns the list of tasks with the number of instances remaining (excludes tasks with all instances fully transitioned)
    #[must_use]
    pub fn as_untransitioned_tasks(&self) -> TrackedTaskList {
        TaskList(self.0
            .iter()
            .filter(|t| t.inner.is_active())
            .map(|t| TaskData::new_with_data(t.task_name(), t.inner().remaining()))
            .collect()
        )
    }
}
