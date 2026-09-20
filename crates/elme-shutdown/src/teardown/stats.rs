use crate::task::{
    list::{TrackedTaskList, TransitioningTaskList},
    TransitioningTask,
};

use chrono::{DateTime, Local, Utc};
use console::Style;
use std::time::Duration;

// !- Statics

static WIDTH: usize = 40;

// !- Common stats interface

#[derive(Debug, Clone)]
pub(crate) struct CommonStats {
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub timeout: Duration,

    pub tasks: TransitioningTaskList,
}
impl CommonStats {
    // a panic is only possible if finished_at < started_at
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn duration(&self) -> Duration {
        let delta = self.finished_at - self.started_at;
        delta.to_std().unwrap()
    }
    /// Formats the duration as seconds with ms decimals (e.g. `"X.XXXs"`)
    #[must_use]
    pub fn duration_text(&self) -> String {
        let dur = self.duration();
        let mut secs = dur.as_secs();
        let mins = secs / 60;
        secs -= mins * 60;
        let mins = if mins > 0 {
            format!("{mins}m ")
        } else {
            String::new()
        };
        format!("{mins}{secs:02}.{:0>3}s", dur.subsec_millis())
    }
    /// Formats the duration as seconds with ms decimals (e.g. `"X.XXXs"`)
    #[must_use]
    pub fn timeout_text(&self) -> String {
        let mut secs = self.timeout.as_secs();
        let mins = secs / 60;
        secs -= mins * 60;
        let mins = if mins > 0 {
            format!("{mins}m ")
        } else {
            String::new()
        };
        format!("{mins}{secs}s")
    }
    fn task_line(name: &str, val: usize, total: usize, sty: &Style, params: &TaskListParams) -> String {
        let TaskListParams { value_w, name_w } = params;
        let name = Style::new().apply_to(format!("{name:<name_w$}"));
        let value = format!("{val:>value_w$}");
        let total = format!("{total:<value_w$}");
        let val = sty.apply_to(format!("{value}/{total}"));
        format!(" - {name}  [{val}]")
    }
    fn task_list_params(tasks: &Vec<&TransitioningTask>) -> TaskListParams {
        let name_w = tasks
            .iter().map(|t| t.task_name().len())
            .max().unwrap_or_default();
        let mut total_max = tasks
            .iter().map(|t| usize::from(t.total_count()))
            .max().unwrap_or_default();
        let mut value_w = 1;
        while total_max > 10 {
            total_max /= 10;
            value_w += 1;
        }
        TaskListParams {
            name_w,
            value_w,
        }
    }
    fn graceful_tasks_list(&self) -> Option<Vec<String>> {
        let tasks: Vec<_> = self.tasks.0
            .iter()
            .filter(|t| usize::from(t.transitioned_count()) > 0)
            .collect();
        if tasks.is_empty() {
            None
        } else {
            let params = Self::task_list_params(&tasks);
            let value_sty = Style::new().green();

            Some(tasks.iter().map(|t| {
                let transitioned = t.transitioned_count().into();
                let total = t.total_count().into();
                Self::task_line(t.task_name(), transitioned, total, &value_sty, &params)
            }).collect())
        }
    }
    fn timeout_tasks_list(&self) -> Option<Vec<String>> {
        let tasks: Vec<_> = self.tasks.0
            .iter()
            .filter(|t| t.is_active())
            .collect();
        if tasks.is_empty() {
            None
        } else {
            let params = Self::task_list_params(&tasks);
            let value_sty = Style::new().red();

            Some(tasks.iter().map(|t| {
                let remaining = t.remaining_count().into();
                let total = t.total_count().into();
                Self::task_line(t.task_name(), remaining, total, &value_sty, &params)
            }).collect())
        }
    }
    fn _build_header(text: &str, sty: &Style, fill: char) -> String {
        let title_w_right = (WIDTH - text.len())/2;
        let title_w_left = WIDTH - text.len() - title_w_right;
        let title_l = sty.apply_to(fill.to_string().repeat(title_w_left));
        let title_r = sty.apply_to(fill.to_string().repeat(title_w_right));
        format!("{title_l} {} {title_r}", Style::new().bold().apply_to(text))
    }
    fn format_date(datetime: DateTime<Utc>) -> String {
        static DATETIME_FMT: &str = "%Y-%m-%d %H:%M:%S";
        datetime.with_timezone(&Local).format(DATETIME_FMT).to_string()


    }
    #[must_use]
    pub fn report_text(&self) -> String {
        let timed_out = self.tasks.has_active_tasks();
        let (
            title,
            finished_at_name,
            title_sty,
        ) = if timed_out {(
            "Timed-out Teardown Stats",
            "Timed-out at",
            Style::new().red(),
        )} else {(
            "Teardown Stats",
            "Finished at",
            Style::new().bold().cyan(),
        )};
        let separator = Style::new().dim().apply_to("-".repeat(WIDTH)).to_string();
        let name_sty = Style::new();
        let value_sty = Style::new().bold();
        let title_w_right = (WIDTH - title.len())/2;
        let title_w_left = WIDTH - title.len() - title_w_right;
        let title_l = title_sty.apply_to("=".repeat(title_w_left - 1));
        let title_r = title_sty.apply_to("=".repeat(title_w_right - 1));
        let mut table_parts = vec![
            separator.clone(),
            // Self::build_header(title, &title_sty, ' '),
            format!("{title_l} {} {title_r}", Style::new().bold().apply_to(title)),
            //title_sty.apply_to(title).to_string(),
            separator.clone(),
            format!("{}{}",
                Self::field_name("Started at", &name_sty),
                Self::field_value(Self::format_date(self.started_at), &value_sty)
            ),
            format!("{}{}",
                Self::field_name(finished_at_name, &name_sty),
                Self::field_value(Self::format_date(self.finished_at), &value_sty)
            ),
            String::new(),
            format!("{}{}",
                Self::field_name("Duration", &name_sty),
                Self::field_value(self.duration_text(), &value_sty)
            ),
            format!("{}{}",
                Self::field_name("Timeout", &name_sty),
                Self::field_value(self.timeout_text(), &value_sty)
            ),
        ];
        if let Some(list) = self.graceful_tasks_list() {
            table_parts.extend(vec![
                String::new(),
                Style::new().bold().apply_to("Gracefully stopped tasks").to_string(),
            ]);
            table_parts.extend(list);
        }
        if let Some(list) = self.timeout_tasks_list() {
            table_parts.extend(vec![
                String::new(),
                Style::new().bold().apply_to("Timed-out tasks").to_string(),
            ]);
            table_parts.extend(list);
        }
        table_parts.extend(vec![
            separator,
            String::new(),
        ]);

        table_parts.join("\n")
    }
    fn field_name(name: &str, style: &Style) -> String {
        let name = format!("{name}:");
        style.apply_to(format!("{name:<15}")).to_string()
    }
    fn field_value(value: String, style: &Style) -> String {
        style.apply_to(value).to_string()
    }
}

// ! Task list params

#[derive(Debug, Copy, Clone)]
struct TaskListParams {
    name_w: usize,
    value_w: usize,
}

// !- Teardown stats

#[derive(Debug, Clone)]
pub struct TeardownStats {
    common: CommonStats,
}
impl TeardownStats {
    #[must_use]
    pub fn started_at(&self) -> DateTime<Utc> {
        self.common.started_at
    }
    #[must_use]
    pub fn finished_at(&self) -> DateTime<Utc> {
        self.common.finished_at
    }
    #[must_use]
    pub fn timeout(&self) -> Duration {
        self.common.timeout
    }
    #[must_use]
    pub fn timeout_text(&self) -> String {
        self.common.timeout_text()
    }
    #[must_use]
    pub fn duration(&self) -> Duration {
        self.common.duration()
    }
    #[must_use]
    pub fn duration_text(&self) -> String {
        self.common.duration_text()
    }
    #[must_use]
    pub fn total_tasks(&self) -> usize {
        self.common.tasks.task_total()
    }
    #[must_use]
    pub fn total_instances(&self) -> usize {
        self.common.tasks.instances_total().into()
    }
    #[must_use]
    pub fn report_text(&self) -> String {
        self.common.report_text()
    }

    /// Returns the list of tasks with the number of instances transitioned (excludes tasks with 0 instances transitioned)
    #[must_use]
    pub fn tasks(&self) -> TrackedTaskList {
        self.common.tasks.as_transitioned_tasks()
    }
}
impl From<CommonStats> for TeardownStats {
    fn from(common: CommonStats) -> Self {
        Self { common }
    }
}

// !- Teardown timeout stats

#[derive(Debug, Clone)]
pub struct TeardownTimeoutStats {
    common: CommonStats,
}
impl TeardownTimeoutStats {
    #[must_use]
    pub fn started_at(&self) -> DateTime<Utc> {
        self.common.started_at
    }
    #[must_use]
    pub fn timed_out_at(&self) -> DateTime<Utc> {
        self.common.finished_at
    }
    #[must_use]
    pub fn timeout(&self) -> Duration {
        self.common.timeout
    }
    #[must_use]
    pub fn timeout_text(&self) -> String {
        self.common.timeout_text()
    }
    #[must_use]
    pub fn duration(&self) -> Duration {
        self.common.duration()
    }
    #[must_use]
    pub fn duration_text(&self) -> String {
        self.common.duration_text()
    }
    #[must_use]
    pub fn total_tasks(&self) -> usize {
        self.common.tasks.task_total()
    }
    #[must_use]
    pub fn total_instances(&self) -> usize {
        self.common.tasks.instances_total().into()
    }
    #[must_use]
    pub fn total_instances_finished(&self) -> usize {
        self.common.tasks.instances_transitioned_total().into()
    }
    #[must_use]
    pub fn total_instances_timed_out(&self) -> usize {
        self.common.tasks.instances_remaining_total().into()
    }
    #[must_use]
    pub fn report_text(&self) -> String {
        self.common.report_text()
    }

    /// Returns the list of tasks, each containing the number of transitioned instances as
    /// well as untransitioned instances
    #[must_use]
    pub fn tasks(&self) -> &TransitioningTaskList {
        &self.common.tasks
    }
    /// Returns the list of tasks with the number of instances transitioned (excludes tasks with 0 instances transitioned)
    #[must_use]
    pub fn transitioned_tasks(&self) -> TrackedTaskList {
        self.common.tasks.as_transitioned_tasks()
    }
    /// Returns the list of tasks with the number of instances remaining (excludes tasks with all instances fully transitioned)
    #[must_use]
    pub fn untransitioned_tasks(&self) -> TrackedTaskList {
        self.common.tasks.as_untransitioned_tasks()
    }
}
impl From<CommonStats> for TeardownTimeoutStats {
    fn from(common: CommonStats) -> Self {
        Self { common }
    }
}
