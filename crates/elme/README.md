<p align='center'>
  <img src='https://raw.githubusercontent.com/kylekingcdn/elme-rs/refs/heads/main/assets/elme-rs.png?raw=true' width=150 />
</p>

# `elme-rs`

A set of crates providing glue code for Rust applications.

## About

> [!WARNING]
> This crate set is currently WIP, and only the crate scaffolding is currently
provided.

## Installation

To simplifiy versioning, modules can be selected by using feature flags with the
`elme` crate.

By default, `elme` provides no elme modules and has 0 dependencies. Modules are
opt-in and can be enabled via feature flags

```shell
cargo add elme --features [module_name]
```

Depending on `elme` and opting into modules using feature flags provides
identical performance to manually adding each module's respective  on each
module's respective crate, and is therefore the recommended versioning strategy.

**`Cargo.toml` example**

```toml
[dependencies]
elme = { version = "0.0.1", features = ["config","db"] }
```

## Basic Usage

> [!NOTE]
> Docs coming soon

For detailed usage, please see the [crate docs](https://docs.rs/elme/latest/elme/).

## Breakdown of modules/crates

### [`elme`](https://github.com/kylekingcdn/elme-rs/tree/main/crates/elme)

> [!NOTE]
> Docs coming soon

### [`elme-config`](https://github.com/kylekingcdn/elme-rs/tree/main/crates/elme-config)

> [!NOTE]
> Docs coming soon

### [`elme-db`](https://github.com/kylekingcdn/elme-rs/tree/main/crates/elme-db)

> [!NOTE]
> Docs coming soon

### [`elme-db-repo-traits`](https://github.com/kylekingcdn/elme-rs/tree/main/crates/elme-db-repo-traits)

> [!NOTE]
> Docs coming soon
