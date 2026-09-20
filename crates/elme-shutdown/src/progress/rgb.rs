// !- Rgb

#[derive(Debug, Copy, Clone)]
pub(crate) struct Rgb(pub(crate) u8, pub(crate) u8, pub(crate) u8);
impl Rgb {
    pub(crate) fn ansi_fg(self) -> String {
        format!("\x1b[38;2;{};{};{}m", self.0, self.1, self.2)
    }
    pub(crate) fn _ansi_bg(self) -> String {
        format!("\x1b[48;2;{};{};{}m", self.0, self.1, self.2)
    }
    pub(crate) fn _from(self, start: Self) -> DirectTransition {
        DirectTransition {
            start,
            end: self,
        }
    }
    pub(crate) fn to(self, end: Self) -> DirectTransition {
        DirectTransition {
            start: self,
            end,
        }
    }
}

// !- Transition

pub(crate) trait Transition {
    fn res(self, progress: u64, total: u64) -> Rgb;

    #[allow(clippy::pedantic)]
    fn channel_pos(start: u8, end: u8, progress: u64, total: u64) -> u8 {
        let range = i16::from(end) - i16::from(start);
        let factor = (progress as f64)/(total as f64);
        let delta = ((range as f64) * factor) as i16;
        ((start as i16) + delta) as u8
    }
}

// !- Direct transition

#[derive(Debug, Copy, Clone)]
pub(crate) struct DirectTransition {
    start: Rgb,
    end: Rgb,
}
impl DirectTransition {
    pub(crate) fn _new(start: Rgb, end: Rgb) -> Self {
        Self {
            start,
            end,
        }
    }
    pub(crate) fn to(self, end: Rgb) -> MidpointTransition {
        MidpointTransition {
            start: self.start,
            mid: self.end,
            end,
        }
    }
    pub(crate) fn _from(self, start: Rgb) -> MidpointTransition {
        MidpointTransition {
            start,
            mid: self.start,
            end: self.end,
        }
    }
}
impl Transition for DirectTransition {
    fn res(self, progress: u64, total: u64) -> Rgb {
        Rgb(
            Self::channel_pos(self.start.0, self.end.0, progress, total),
            Self::channel_pos(self.start.1, self.end.1, progress, total),
            Self::channel_pos(self.start.2, self.end.2, progress, total),
        )
    }
}

// !- Midpoint transition

#[derive(Debug, Copy, Clone)]
pub(crate) struct MidpointTransition {
    start: Rgb,
    mid: Rgb,

    end: Rgb,
}
impl MidpointTransition {
    pub(crate) fn _new(start: Rgb, mid: Rgb, end: Rgb) -> Self {
        Self {
            start,
            mid,
            end,
        }
    }
}
impl Transition for MidpointTransition {
    fn res(self, progress: u64, total: u64) -> Rgb {
        let half = total/2;
        let mut sub_progress = progress;
        let transition = if progress <= half {
            self.start.to(self.mid)
        } else {
            sub_progress -= half;
            self.mid.to(self.end)
        };
        transition.res(sub_progress, half)
    }
}
