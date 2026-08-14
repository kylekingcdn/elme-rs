<p align='center'>
  <img src='https://raw.githubusercontent.com/kylekingcdn/elme-rs/refs/heads/main/assets/elme-rs.png?raw=true' width=400 />
</p>

# `elme-rs`

A set of crates providing glue code for Rust applications.

## About

> [!WARNING]
> This crate set is currently WIP, and only the crate scaffolding is currently
provided.

## Installation

By default, `elme` ships with 0 dependencies and all modules disabled.

Modules can be enabled via feature flags:

```shell
cargo add elme --features [module_name]
```

> [!NOTE]
> Opting into modules using feature flags via the `elme` crate provides
identical performance to manually depending on each module's respective crate.
>
> Therefore, depending on `elme` and not the module crates is recommended
versioning strategy.

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
