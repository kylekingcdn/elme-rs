<p align='center'>
  <img src='https://raw.githubusercontent.com/kylekingcdn/elme-rs/refs/heads/main/assets/elme-rs.png?raw=true' width=400 />
</p>

# `elme-error`

An `elme` module providing types for error propagation, spantraces, and backtraces.

Features similar functionality to `eyre`, `color-eyre`, `anyhow`, etc - with the following additions:
- Configuration of backtrace/spantrace format & visibility on a per-destination basis.
- Configuration of ANSI/color output on a per-destination basis.

For example, this could allow for the following (simultaneous) handling of errors:
- Output error messages with color backtraces in full to terminal
- Output error messages with non-color backtraces in full to a file
- Output error message strings to OTEL - with backtraces embedded in structured attra/metadata as opposed to the body

<div class="warning">
  <b>Docs and module impls will be published soon</b>
</div>

[**crates.io**](https://crates.io/crates/elme-error)
|
[**Docs**](https://docs.rs/elme-error/latest)
|
[**GitHub**](https://github.com/kylekingcdn/elme-rs/tree/main/crates/elme-error)

## Relevant `elme` feature flags

**Flag for `elme`** | **Description**
--------------- | ---------------
**`error`** | Enables this module
