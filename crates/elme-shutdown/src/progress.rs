mod rgb;

#[cfg(feature = "progress-writer")]
pub(crate) mod writer;

use self::rgb::{Rgb, MidpointTransition, Transition};
use crate::task::map::TransitioningTaskMap;

use chrono::{DateTime, Utc};
use console::{colors_enabled_stderr, true_colors_enabled_stderr};
use indicatif::{MultiProgress, ProgressBar, ProgressFinish, ProgressState, ProgressStyle};
use std::cmp::max;
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use tokio::time::{interval, MissedTickBehavior};

// !- Statics

static GREEN:  Rgb = Rgb( 26, 188, 156);
static YELLOW: Rgb = Rgb(252, 188,   9);
static RED:    Rgb = Rgb(244,  64,  64);

#[allow(dead_code)]
static ANSI_GREEN: &str = "\x1b[32m";
#[allow(dead_code)]
static ANSI_YELLOW: &str = "\x1b[33m";
#[allow(dead_code)]
static ANSI_RED: &str = "\x1b[31m";

static GREEN_TO_RED: LazyLock<MidpointTransition> = LazyLock::new(||
    GREEN.to(YELLOW).to(RED)
);

static TICK_FACTOR: u64 = 10;

// !- Shared-lifetime bars

/// Ensures lifetime of spawned progress bars is identical
///
/// Otherwise some bars will disappear after finish, pending insert order
#[allow(dead_code, clippy::struct_field_names)]
pub(crate) struct Bars {
    mp: MultiProgress,
    task_bars: HashMap<&'static str, ProgressBar>,
    instance_bar: ProgressBar,
    timeout_bar: ProgressBar,
}

// !- Progress hook

pub(crate) struct TeardownProgressHook {
    bars: Arc<Bars>,
    max_instances: usize,
    bar_value_width: usize,
}
#[allow(unused, clippy::unused_self)]
impl TeardownProgressHook {
    pub fn new(
        progress_bars: MultiProgress,
        transition_map: &TransitioningTaskMap,
        started_at: DateTime<Utc>,
        timeout: Duration,
    ) -> Self {
        let instances = transition_map.instance_count();
        let mut task_list = transition_map.as_list().into_active_filtered();
        task_list.sort_tasks();
        let mut max_instances = 0;
        for task in task_list.inner() {
            max_instances = max(max_instances, usize::from(task.total_count()));
        }

        let timeout_value = AlignedValue::new_timeout(timeout);
        let instances_value = AlignedValue::new_instances(instances.into());
        let bar_value_width = max(timeout_value.len(), instances_value.len());

        // init task spinners
        let mut task_bars = HashMap::new();
        for task in task_list.inner() {
            let bar = progress_bars.add(ProgressBar::new(usize::from(task.total_count()) as u64))
                .with_style(task_style(max_instances as u64, false))
                .with_prefix("✔")
                .with_message(task.task_name())
                .with_finish(ProgressFinish::AndClear);
            bar.enable_steady_tick(Duration::from_millis(50));

            task_bars.insert(task.task_name(), bar);
        }

        // init main instance progress bar
        let instance_bar = progress_bars.add(ProgressBar::new(usize::from(instances) as u64))
            .with_style(instances_style(bar_value_width, false))
            .with_prefix("Progress")
            .with_finish(ProgressFinish::AndLeave);

        // init timeout bar
        let timeout_bar = progress_bars.add(ProgressBar::new(duration_to_ticks(timeout)))
            .with_style(timeout_style(bar_value_width))
            .with_prefix("Time-out")
            .with_finish(ProgressFinish::AndLeave);

        // build shared-lifetime bars wrapper
        let bars = Arc::new(Bars {
            mp: progress_bars,
            task_bars,
            instance_bar,
            timeout_bar,
        });

        // spawn timeout bar update loop
        let bars_ = bars.clone();
        tokio::spawn(async move {
            Self::update_timeout_bar(bars_, timeout).await;
        });

        // tick bars post-init
        bars.timeout_bar.tick();
        bars.instance_bar.tick();
        for bar in bars.task_bars.values() {
            bar.tick();
        }

        Self {
            bars,
            max_instances,
            bar_value_width,
        }
    }

    async fn update_timeout_bar(bars: Arc<Bars>, timeout: Duration) {
        let start = Instant::now();
        let mut interval = interval(Duration::from_millis(TICK_FACTOR));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        while start.elapsed() < timeout && !bars.timeout_bar.is_finished() {
            interval.tick().await;
            bars.timeout_bar.set_position(duration_to_ticks(start.elapsed()));
        }
    }

    pub fn on_task_unregistered(
        &self,
        transition_map: &TransitioningTaskMap,
        task_name: &'static str,
    ) {
        let task = transition_map.inner().get(task_name).unwrap();
        self.bars.instance_bar.inc(1);

        let bar = self.bars.task_bars.get(task_name).unwrap();
        bar.set_position(usize::from(task.transitioned()) as u64);
        if task.is_transitioned() {
            bar.set_style(task_style(self.max_instances as u64, true));
        }
    }

    pub fn on_finished(
        &self,
        transition_map: &TransitioningTaskMap,
        started_at: DateTime<Utc>,
        finished_at: DateTime<Utc>,
        timeout: Duration,
    ) {
        for bar in self.bars.task_bars.values() {
            if !bar.is_finished() {
                bar.set_style(task_style(self.max_instances as u64, true));
                bar.finish_using_style();
            }
        }
        self.bars.instance_bar.set_style(instances_style(self.bar_value_width, true));
        self.bars.instance_bar.finish_using_style();
        self.bars.timeout_bar.abandon();
    }

    pub fn on_timeout(
        &self,
        transition_map: &TransitioningTaskMap,
        started_at: DateTime<Utc>,
        timeout_at: DateTime<Utc>,
        timeout: Duration,
    ) {
        for bar in self.bars.task_bars.values() {
            if !bar.is_finished() {
                bar.abandon();
            }
        }
        self.bars.instance_bar.set_style(instances_style(self.bar_value_width, true));
        self.bars.instance_bar.abandon();
        self.bars.timeout_bar.finish_using_style();
    }

    fn duration_text(duration: Duration) -> String {
        let mins = duration.as_secs() / 60;
        let secs = duration.as_secs() - (mins * 60);
        format!("{mins}:{secs:02}")
    }
}

// !- Styles

pub fn timeout_style(width: usize) -> ProgressStyle {
    ProgressStyle::with_template(
        " {prefix:.bold} [{bar_color}{wide_bar:./white.dim}{bar_color_end}][{timeout}]"
    )
    .unwrap()
    .with_key("timeout", move |state: &ProgressState, w: &mut dyn Write| {
        let total_s = ticks_to_duration(state.len().unwrap()).as_secs();
        let elapsed_s = ticks_to_duration(state.pos()).as_secs();
        let mut remain_s = total_s - elapsed_s;
        let remain_m = remain_s / 60;
        remain_s -= remain_m * 60;
        let value = format!("-{remain_m}:{remain_s:02}");
        write!(w, "{value:>width$}").unwrap();
    })
    .with_key("bar_color", |state: &ProgressState, w: &mut dyn Write| {
        let ansi =
        if true_colors_enabled_stderr() {
            Some(GREEN_TO_RED.res(state.pos(), state.len().unwrap()).ansi_fg())
        } else if colors_enabled_stderr() {
            let len = state.len().unwrap();
            if state.pos() * 3 < len {
                Some(ANSI_GREEN.to_string())
            } else if state.pos() * 3 / 2 < len {
                Some(ANSI_YELLOW.to_string())
            } else {
                Some(ANSI_RED.to_string())
            }
        } else {
            None
        };
        if let Some(ansi) = ansi {
            write!(w, "{ansi}").unwrap();
        }
    })
    .with_key("bar_color_end", |_state: &ProgressState, w: &mut dyn Write| {
        if colors_enabled_stderr() || true_colors_enabled_stderr() {
            write!(w, "\x1B[0m").unwrap();
        }
    })
    .progress_chars("=>-")
}
pub fn instances_style(width: usize, finished: bool) -> ProgressStyle {
    let mut template = " {prefix:.bold} [{wide_bar:.green/".to_string();
    if finished {
        template.push_str("red");
    } else {
        template.push_str("white.dim");
    }
    template.push_str("}][{instances}]");
    let chars = if finished {
        "#X"
    } else {
        "#-"
    };
    ProgressStyle::with_template(&template).unwrap()
    .with_key("instances", move |state: &ProgressState, w: &mut dyn Write| {
        let pos = state.pos();
        let len = state.len().unwrap();
        let value = format!("{pos}/{len}");
        write!(w, "{value:^width$}").unwrap();
    })
    .progress_chars(chars)
}
pub fn task_style(max_total: u64, finished: bool) -> ProgressStyle {
    let mut digits = 1;
    let mut total = max_total;
    while total >= 10  {
        digits += 1;
        total /= 10;
    }
    let icon = if finished {
        "prefix"
    } else {
        "spinner"
    };
    let template = format!(" [{{pos:>{digits}}}/{{len:{digits}}}] {{{icon}:.green}} {{msg}}");
    ProgressStyle::with_template(&template).unwrap()
}

// ! Value alignment

#[derive(Debug, Copy, Clone)]
pub struct AlignedValue {
    pos_width: usize, // width required for pos at max value
    value_len: usize,
}
impl AlignedValue {
    pub fn new(pos_width: usize, value_len: usize) -> Self {
        Self {
            pos_width,
            value_len,
        }
    }
    pub fn new_instances(count: usize) -> Self {
        let mut pos_width = 1;
        let mut scale = count;
        while scale >= 10 {
            pos_width += 1;
            scale /= 10;
        }
        Self::new(pos_width, pos_width + 1)
    }
    pub fn new_timeout(timeout: Duration) -> Self {
        let mut pos_width = 1; // always include at least 1 minute digit
        let mut mins = timeout.as_secs()/60;
        while mins >= 10 {
            pos_width += 1;
            mins /= 10;
        }
        Self::new(pos_width, 4) // 3 for ':xx', 1 for '-'
    }
    pub fn len(&self) -> usize {
        self.pos_width + self.value_len
    }
}

// !- Tick scaling

fn ms_to_ticks(ms: u64) -> u64 {
    ms / TICK_FACTOR
}
fn ticks_to_ms(ticks: u64) -> u64 {
    ticks * TICK_FACTOR
}
fn duration_to_ticks(duration: Duration) -> u64 {
    ms_to_ticks(duration.as_millis().try_into().unwrap())
}
fn ticks_to_duration(ticks: u64) -> Duration {
    Duration::from_millis(ticks_to_ms(ticks))
}
