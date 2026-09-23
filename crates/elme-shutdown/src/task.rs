pub(crate) mod handle;
pub(crate) mod list;
pub(crate) mod map;
pub(crate) mod registry;

pub use {
    handle::TaskHandle,
    list::{TaskList, TrackedTaskList, TransitioningTaskList},
};

use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Sub, SubAssign};

// !- Errors

#[derive(Debug, thiserror::Error)]
pub enum RegisterError {
    #[error("Failed to register task '{0}'. Currently tearing down.")]
    TearingDown(&'static str),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum TransitionError {
    #[error("Task not found in registry: {0}")]
    TaskNotFound(&'static str),

    #[error("Task transition instance remaining count must be <= total. (value: {value}, total: {total}")]
    AboveTotal { value: usize, total: usize },

    //#[error("Task transition instance count can not be increased. (from: {current}, to: {update}")]
    //DecrementOnly { current: usize, update: usize },
}

// !- Itemize trait

/// Keeps track of one or more groups of instances for a given task
pub trait Itemize: fmt::Debug + fmt::Display + Copy + Clone + Default + PartialEq + Eq + PartialOrd + Ord {
    fn is_active(&self) -> bool;
}

// !- Task instance count newtype

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct InstanceCount(usize);

impl Add for InstanceCount {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}
impl AddAssign for InstanceCount {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}
impl Sub for InstanceCount {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}
impl SubAssign for InstanceCount {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}
impl Sum for InstanceCount {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.map(|i| i.0).sum::<usize>().into()
    }
}
impl fmt::Display for InstanceCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl From<usize> for InstanceCount {
    fn from(val: usize) -> Self {
        Self(val)
    }
}
impl From<InstanceCount> for usize {
    fn from(val: InstanceCount) -> Self {
        val.0
    }
}
impl Itemize for InstanceCount {
    fn is_active(&self) -> bool {
        self.0 > 0
    }
}

// !- Task transitioning instance counts

/// [`TransitionInstanceCount`] represents the distribution of a task's instances between 2 possible states.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct TransitionInstanceCount {
    /// number of instances that still need to be transitioned
    remaining: InstanceCount,
    /// total instances to be transitioned
    total: InstanceCount,
}
impl TransitionInstanceCount {
    pub(crate) fn new(total: InstanceCount) -> Self {
        Self {
            remaining: total,
            total,
        }
    }
    #[must_use]
    pub fn remaining(&self) -> InstanceCount {
        self.remaining
    }
    #[must_use]
    pub fn total(&self) -> InstanceCount {
        self.total
    }
    /// # of instances transitioned so far
    #[must_use]
    pub fn transitioned(&self) -> InstanceCount {
        self.total - self.remaining
    }
    #[must_use]
    pub fn is_transitioned(&self) -> bool {
        self.remaining.0 == 0
    }
    pub(crate) fn try_deduct_remaining(&mut self) -> Result<(), TransitionError> {
        if self.remaining.0 == 0 {
            Err(TransitionError::AboveTotal { value: self.total.0 + 1, total: self.total.0 })
        } else {
            self.remaining.0 -= 1;
            Ok(())
        }
    }

    /// Returns a string representing the progress as a fraction
    ///
    /// ```text
    /// "{transitioned}/{total}"
    /// ```
    ///
    /// E.g. where `n == total`, transitions through:
    ///
    /// ```text
    /// 0/n, 1/n, ..., n-1/n, n/n
    /// ```
    #[must_use]
    pub fn as_transitioned_fraction(&self) -> String {
        format!("{}/{}", self.transitioned(), self.total)
    }
    /// Returns a string representing the progress as a fraction
    ///
    /// ```text
    /// "{remaining}/{total}"
    /// ```
    ///
    /// E.g. where `n == total`, transitions through:
    ///
    /// ```text
    /// n/n, n-1/n, ..., 1/n, 0/n
    /// ```
    #[must_use]
    pub fn as_remaining_fraction(&self) -> String {
        format!("{}/{}", self.remaining, self.total)
    }

    pub(crate) fn invert(&mut self) {
        self.remaining = self.transitioned();
    }
    pub(crate) fn as_inverted(&self) -> Self {
        let mut res = *self;
        res.invert();
        res
    }
}
impl fmt::Display for TransitionInstanceCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.transitioned(), self.total)
    }
}
impl From<usize> for TransitionInstanceCount {
    fn from(val: usize) -> Self {
        Self::new(val.into())
    }
}
impl From<InstanceCount> for TransitionInstanceCount {
    fn from(val: InstanceCount) -> Self {
        Self::new(val)
    }
}
impl From<&InstanceCount> for TransitionInstanceCount {
    fn from(val: &InstanceCount) -> Self {
        Self::new(*val)
    }
}
impl Itemize for TransitionInstanceCount {
    fn is_active(&self) -> bool {
        self.remaining.0 > 0
    }
}
// !- Task Data

/// Associates a task (by name) with an Itemize struct
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskData<I: Itemize> {
    task_name: &'static str,
    inner: I,
}
impl<I: Itemize> TaskData<I> {
    #[allow(dead_code)]
    pub(crate) fn new(task_name: &'static str) -> Self {
        Self {
            task_name,
            inner: I::default(),
        }
    }
    pub(crate) fn new_with_data(task_name: &'static str, inner: I) -> Self {
        Self {
            task_name,
            inner,
        }
    }
    pub fn task_name(&self) -> &'static str {
        self.task_name
    }
    pub fn is_active(&self) -> bool {
        self.inner.is_active()
    }
    pub fn inner(&self) -> I {
        self.inner
    }
}
impl<I: Itemize> fmt::Display for TaskData<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [{}]", self.task_name, self.inner)
    }
}
impl<I: Itemize> From<(&'static str, I)> for TaskData<I> {
    fn from((name, data): (&'static str, I)) -> Self {
        Self::new_with_data(name, data)
    }
}

// ! Tracked task

pub type TrackedTask = TaskData<InstanceCount>;

impl TrackedTask {
    #[must_use]
    pub fn instance_count(&self) -> InstanceCount {
        self.inner
    }
}

// ! Transitioning task

pub type TransitioningTask = TaskData<TransitionInstanceCount>;

impl TransitioningTask {
    #[must_use]
    pub fn transitioned_count(&self) -> InstanceCount {
        self.inner.transitioned()
    }
    #[must_use]
    pub fn remaining_count(&self) -> InstanceCount {
        self.inner.remaining
    }
    #[must_use]
    pub fn total_count(&self) -> InstanceCount {
        self.inner.total
    }
    #[must_use]
    pub fn is_fully_transitioned(&self) -> bool {
        !self.is_active()
    }
    #[must_use]
    pub fn as_inverted(&self) -> Self {
        Self::new_with_data(self.task_name, self.inner.as_inverted())
    }

}
