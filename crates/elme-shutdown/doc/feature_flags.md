Features for the corresponding crates (`elme`, `elme-shutdown`) are listed below.

**`elme`** | **`elme-shutdown`** | **Description**
-|-|-
**`shutdown`** | *n/a* | Enables this module
**`shutdown-progress`** | **`progress`** | Enables task + timeout progress bars during teardown
**`shutdown-progress-writer`** | **`progress-writer`** | Provides a [`tracing-subscriber`](https://docs.rs/tracing-subscriber/latest) writer for clean `tracing` + `progress` output
**`shutdown-instance-ids`** | **`instance-ids`** | Enables assigning arbitrary IDs to task instances
**`full`** | **`full`** | If the `shutdown` module is enabled, enables `progress`, `progress-writer`, `instance-ids`, `serde`
**`serde`** | **`serde`** | Enables `serde` support, providing `Deserialize` support for runtime config ([`ShutdownOverrideConfig`](crate::ShutdownOverrideConfig))
