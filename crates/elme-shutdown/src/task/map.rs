use crate::task::{
    InstanceCount,
    Itemize,
    list::TaskList,
    TransitionError,
    TransitionInstanceCount,
};

use std::collections::HashMap;

// ! Task map

#[derive(Debug, Clone, Default)]
pub(crate) struct TaskMap<I: Itemize>(
    pub(crate) HashMap<&'static str, I>,
);
impl<I: Itemize> TaskMap<I> {
    #[allow(dead_code)] // feature-dependant
    pub fn inner(&self) -> &HashMap<&'static str, I> {
        &self.0
    }

    pub fn get(&self, task_name: &'static str) -> Option<I> {
        self.0.get(task_name).copied()
    }

    pub fn has_active_tasks(&self) -> bool {
        self.0.iter().any(|(_,i)| i.is_active())
    }
    pub fn active_task_count(&self) -> usize {
        self.0.iter().filter(|(_,i)| i.is_active()).count()
    }
    pub fn _filter_active_only(&mut self) {
        self.0 = self.0.iter().filter(|(_,i)| i.is_active()).map(|(k,v)| (*k,*v)).collect();
    }
    pub fn into_active_filtered(self) -> Self {
        Self(self.0.into_iter().filter(|(_,i)| i.is_active()).collect())
    }

    pub fn as_list(&self) -> TaskList<I> {
        self.into()
    }
    pub fn _into_list(self) -> TaskList<I> {
        self.into()
    }
}
impl<I: Itemize> From<HashMap<&'static str, I>> for TaskMap<I> {
    fn from(map: HashMap<&'static str, I>) -> Self {
        Self(map)
    }
}
impl<I: Itemize> From<&TaskMap<I>> for TaskList<I> {
    fn from(map: &TaskMap<I>) -> Self {
        Self(map.0.iter().map(|(k,v)|(*k,*v).into()).collect())
    }
}
impl<I: Itemize> From<TaskMap<I>> for TaskList<I> {
    fn from(map: TaskMap<I>) -> Self {
        Self(map.0.into_iter().map(Into::into).collect())
    }
}

// !- InstanceCount task map

pub(crate) type TrackedTaskMap = TaskMap<InstanceCount>;

impl TrackedTaskMap {
    pub fn instance_count(&self) -> InstanceCount {
        self.0.values().copied().sum()
    }
    pub fn task_instance_count(&self, task_name: &'static str) -> InstanceCount {
        self.get(task_name).unwrap_or_default()
    }
}

// !- TransitionInstanceCount task map

pub(crate) type TransitioningTaskMap = TaskMap<TransitionInstanceCount>;

impl TransitioningTaskMap {
    pub fn try_deduct_remaining(
        &mut self,
        task_name: &'static str,
    ) -> Result<(), TransitionError> {
        if let Some(data) = self.0.get_mut(task_name) {
            data.try_deduct_remaining()
        }
        else {
            Err(TransitionError::TaskNotFound(task_name))
        }
    }
    #[allow(dead_code)] // feature-dependant
    pub fn instance_count(&self) -> InstanceCount {
        self.0.values().map(|t| t.total).sum::<InstanceCount>()
    }
    pub fn _total_progress(&self) -> (InstanceCount, InstanceCount) {
        self.0.values()
            .map(|i| (i.remaining, i.total))
            .fold(
                (0.into(), 0.into()),
                |acc, val|
                (acc.0 + val.0, acc.1 + val.1)
            )
    }
}
impl From<TrackedTaskMap> for TransitioningTaskMap {
    fn from(map: TrackedTaskMap) -> Self {
        Self(map.0.into_iter().map(|(t,i)| (t,i.into())).collect())
    }
}
