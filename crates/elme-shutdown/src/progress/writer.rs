// adapted from emersonford's excellent work on tracing-indicatif:
// https://github.com/emersonford/tracing-indicatif/blob/main/src/writer.rs

use indicatif::MultiProgress;
use tracing_subscriber::fmt::MakeWriter;
use std::io;

#[derive(Clone)]
pub struct ProgressWriter {
    mp: MultiProgress,
}
impl ProgressWriter {
    #[must_use]
    pub fn new(mp: MultiProgress) -> Self {
        Self { mp }
    }
}
impl io::Write for ProgressWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.mp.suspend(|| io::stderr().write(buf))
    }
    fn flush(&mut self) -> io::Result<()> {
        self.mp.suspend(|| io::stderr().flush())
    }
    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        self.mp.suspend(|| io::stderr().write_vectored(bufs))
    }
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.mp.suspend(|| io::stderr().write_all(buf))
    }
    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> io::Result<()> {
        self.mp.suspend(|| io::stderr().write_fmt(fmt))
    }
}
impl<'a> MakeWriter<'a> for ProgressWriter {
    type Writer = ProgressWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}
