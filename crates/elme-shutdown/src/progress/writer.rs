// adapted from emersonford's excellent work on tracing-indicatif:
// https://github.com/emersonford/tracing-indicatif/blob/main/src/writer.rs

use indicatif::MultiProgress;
use tracing_subscriber::fmt::MakeWriter;
use std::io;

// !- ProgressWriter - any writer

/// A custom [`std::io::Write`] impl that fixes broken (simultaneous) log + progress output
///
/// # Background
///
/// Console output will likely become out-of-sync if an additional source writes output while progress bars are visible.
/// This will appear as messages and progress bars overwriting eachother, duplicate/stale progress lines, etc.
///
/// This custom (wrapper) writer fixes this issue by temporarily suspending rendering of progress bars
/// whenever output is sent to the underlying `io::Write` type
pub struct ProgressWriter<W: io::Write> {
    mp: MultiProgress,
    writer: W,
}

impl<W: io::Write> ProgressWriter<W> {
    /// Creates a new `ProgressWriter`
    pub fn new(multi_progress: MultiProgress, writer: W) -> Self {
        Self {
            mp: multi_progress,
            writer,
        }
    }
    /// Returns the inner [`std::io::Write`] writer
    pub fn writer(&self) -> &W {
        &self.writer
    }
}
impl<W: io::Write> io::Write for ProgressWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.mp.suspend(|| self.writer.write(buf))
    }
    fn flush(&mut self) -> io::Result<()> {
        self.mp.suspend(|| self.writer.flush())
    }
    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        self.mp.suspend(|| self.writer.write_vectored(bufs))
    }
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.mp.suspend(|| self.writer.write_all(buf))
    }
    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> io::Result<()> {
        self.mp.suspend(|| self.writer.write_fmt(fmt))
    }
}

// ! ProgressWriter Stdout

impl ProgressWriter<io::Stdout> {
    /// Creates a new `ProgressWriter` with output forced to `stdout`
    ///
    /// This disregards the any writer configuration present in the [`tracing_subscriber::fmt::Layer`].
    #[must_use]
    pub fn new_stdout(multi_progress: MultiProgress) -> ProgressWriter<io::Stdout> {
        Self::new(multi_progress, io::stdout())
    }
}
impl<'a> MakeWriter<'a> for ProgressWriter<io::Stdout> {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        ProgressWriter::new(self.mp.clone(), io::stdout())
    }
}

// ! ProgressWriter Stderr

impl ProgressWriter<io::Stderr> {
    /// Creates a new `ProgressWriter` with output forced to `stderr`
    ///
    /// This disregards the any writer configuration present in the [`tracing_subscriber::fmt::Layer`].
    #[must_use]
    pub fn new_stderr(multi_progress: MultiProgress) -> ProgressWriter<io::Stderr> {
        Self::new(multi_progress, io::stderr())
    }
}
impl<'a> MakeWriter<'a> for ProgressWriter<io::Stderr> {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        ProgressWriter::new(self.mp.clone(), io::stderr())
    }
}

/// Intermediate type used to provide map support for any [`fmt::Layer`](tracing_subscriber::fmt::Layer) writer
pub struct MappedProgressWriter<M> {
    mp: MultiProgress,
    make: M,
}
impl<'a, M> MappedProgressWriter<M>
where
    M: MakeWriter<'a>,
    <M as MakeWriter<'a>>::Writer: io::Write,
{
    /// Creates a new [`MappedProgressWriter`] for use with
    /// [`fmt::Layer::map_writer`](tracing_subscriber::fmt::Layer::map_writer)
    pub fn new(multi_progress: MultiProgress, make_writer: M) -> Self {
        Self {
            mp: multi_progress,
            make: make_writer,
        }
    }
}
impl<'a, M> MakeWriter<'a> for MappedProgressWriter<M>
where
    M: MakeWriter<'a>,
    <M as MakeWriter<'a>>::Writer: io::Write,
{
    type Writer = ProgressWriter<<M as MakeWriter<'a>>::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        ProgressWriter::new(self.mp.clone(), self.make.make_writer())
    }
}
