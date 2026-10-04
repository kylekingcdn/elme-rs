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

/// Possible error types for calls to [`register_task()`](crate::ShutdownManager::register_task)
#[derive(Debug, thiserror::Error)]
pub enum RegisterError {
    /// Failed because the application is currently tearing down
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
    /// Whether this group of instances is considered active
    ///
    /// The interpretation of 'active' is of the implementing type's choice.
    fn is_active(&self) -> bool;
}

// !- Task instance count newtype

/// `usize` new-type representing an instance count for a task/worker
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
    /// A task instance count is considered active when greater than `0`
    fn is_active(&self) -> bool {
        self.0 > 0
    }
}

// !- Task transitioning instance counts

/// [`TransitionInstanceCount`] represents the distribution of a task's instances between 2 possible states.
///
/// Internally, this is used during teardown to keep track of how many instances of a task have
/// gracefully stopped vs. how many are still running.
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
    /// The number of instances that have not yet transitioned
    #[must_use]
    pub fn remaining(&self) -> InstanceCount {
        self.remaining
    }
    /// The total number of instances
    ///
    /// Equivalent to `remaining + transitioned`.
    #[must_use]
    pub fn total(&self) -> InstanceCount {
        self.total
    }
    /// The number of instances transitioned so far
    #[must_use]
    pub fn transitioned(&self) -> InstanceCount {
        self.total - self.remaining
    }
    /// Returns true when there are no instances left to transition
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

    /// Returns a string representing the progress in the form of a fraction
    ///
    /// ```text
    /// "{transitioned}/{total}"
    /// ```
    ///
    /// E.g. where `n == total`, transitions through:
    ///
    /// ```text
    /// 0/n, 1/n, 2/n, ..., (n-1)/n, n/n
    /// ```
    #[must_use]
    pub fn as_transitioned_fraction(&self) -> String {
        format!("{}/{}", self.transitioned(), self.total)
    }

    /// Returns a string representing the remainder of progress in the form of a fraction
    ///
    /// ```text
    /// "{remaining}/{total}"
    /// ```
    ///
    /// E.g. where `n == total`, transitions through:
    ///
    /// ```text
    /// n/n, (n-1)/n, ..., 2/n, 1/n, 0/n
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
///
/// Internally, this is used to provide support for conversion from Maps of
/// `[ task name->instance counts ]` to Lists of `[ TaskData(task name, instance counts) ]`
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
    /// The name of the task
    pub fn task_name(&self) -> &'static str {
        self.task_name
    }
    /// Whether the underlying type is active
    ///
    /// The definition of `active` is up to the inner type.
    ///
    /// See the underlying type for the definition.
    pub fn is_active(&self) -> bool {
        self.inner.is_active()
    }

    /// Returns the inner instance count type
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

/// Informally, a pairing containing a task name and number of instances
pub type TrackedTask = TaskData<InstanceCount>;

impl TrackedTask {
    /// The inner [`InstanceCount`]
    #[must_use]
    pub fn instance_count(&self) -> InstanceCount {
        self.inner
    }
}

// ! Transitioning task

/// Informally, a pairing of a task name with a (remaining, total) instance count fraction
pub type TransitioningTask = TaskData<TransitionInstanceCount>;

impl TransitioningTask {
    /// The number of instances transitioned so far
    #[must_use]
    pub fn transitioned_count(&self) -> InstanceCount {
        self.inner.transitioned()
    }
    /// The number of instances that have not yet transitioned
    #[must_use]
    pub fn remaining_count(&self) -> InstanceCount {
        self.inner.remaining
    }
    /// The total number of instances
    ///
    /// Equivalent to `remaining_count + transitioned_count`.
    #[must_use]
    pub fn total_count(&self) -> InstanceCount {
        self.inner.total
    }
    /// Returns true when there are no instances left to transition
    #[must_use]
    pub fn is_fully_transitioned(&self) -> bool {
        !self.is_active()
    }
    /// Returns a new `TransitioningTask` with `remaining` and `transitioned` swapped
    #[must_use]
    pub fn as_inverted(&self) -> Self {
        Self::new_with_data(self.task_name, self.inner.as_inverted())
    }

}
