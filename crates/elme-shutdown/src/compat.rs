use std::env;
use std::io::{self, IsTerminal};
use supports_color::Stream;

pub struct ColorSupport {
    force: Option<bool>,
    color_term: Option<String>,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OutputType {
    StdErr,
    StdOut,
}
impl OutputType {
    pub fn is_terminal(&self) -> bool {
        match self {
            Self::StdErr => io::stderr().is_terminal(),
            Self::StdOut => io::stdout().is_terminal(),
        }
    }
}
pub(crate) fn init_color_support() {
    // for ty in [OutputType::StdErr, OutputType::StdOut] {
    //     init_term_color_support(ty);
    //     init_term_true_color_support(ty);
    // }
    init_color_support_stream(Stream::Stdout);
    init_color_support_stream(Stream::Stderr);
}
pub(crate) fn init_color_support_stream(stream: Stream) {
    let (color, truecolor) = if let Some(support) = supports_color::on(stream) {
        (support.has_basic, support.has_16m)
    } else {
        (false, false)
    };

    match stream {
        Stream::Stderr => {
            console::set_colors_enabled_stderr(color);
            console::set_true_colors_enabled_stderr(truecolor);
        }
        Stream::Stdout => {
            console::set_colors_enabled(color);
            console::set_true_colors_enabled(truecolor);
        }
    }
}
fn init_term_color_support(ty: OutputType) {

}

fn term_color_support() -> Option<bool> {
    if env::var_os("NO_COLOR").is_some() {
        Some(false)
    }
    else if let Some(val) = env::var_os("CLICOLOR_FORCE") && val != "0" {
        Some(true)
    }
    else if let Some(val) = env::var_os("FORCE_COLOR") && val != "0" {
        Some(true)
    }
    else if let Some(val) = env::var_os("CLICOLOR") && val == "0" {
        Some(false)
    } else {
        None
    }
}

fn init_term_true_color_support(ty: OutputType) {
}

/// NO_COLOR -> force off
/// CLICOLOR_FORCE -> force on
///
fn term_true_color_support() -> Option<bool> {
    match term_color_support() {
        Some(false) => Some(false),
        Some(true) | None => {
            if let Some(v) = env::var_os("COLORTERM") && v == "truecolor" {
                Some(true)
            } else {
                Some(false)
            }
        }
    }
}
