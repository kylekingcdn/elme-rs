<p align='center'>
  <img src='https://raw.githubusercontent.com/kylekingcdn/elme-rs/refs/heads/main/assets/elme-rs.png?raw=true' width=400 />
</p>

# `elme-rs`

A set of crates providing glue code for Rust applications.

<div class="warning">
  <b>This and associated crates are currently a work in progress.</b>
  <br/><br/>
  Until stabilized, usage in production code is <b>strongly discouraged</b>.
</div>

[**crates.io**](https://crates.io/crates/elme)
|
[**Docs**](https://docs.rs/elme/latest)
|
[**GitHub**](https://github.com/kylekingcdn/elme-rs)

---

***Certified Non AI Slop***

Each crate in the `elme` framework was crafted by ~~hand~~ keyboard with care.

I take personal credit for the (hopefully) rare case where you believe some code to be of a sloppy nature.

For these cases, please open an issue or submit a PR - **it's very much appreciated!**

## Jump to module

- [`elme`](#elme)
- [`elme-config`](#elme-config)
- [`elme-db`](#elme-db)

## Installation

By default, `elme` ships with 0 dependencies and all modules disabled.

Modules can be enabled via feature flags:

```shell
cargo add elme --features [module_name]
```

**`Cargo.toml` example**

```toml
[dependencies]
elme = { version = "0.0.1", features = ["config","shutdown"] }
```

> ***Note***
>
> ---
>
> Opting into modules using feature flags via the `elme` crate provides
> identical performance to manually depending on each module's respective crate.
>
> Therefore, depending on the `elme` crate as opposed to depending on each respective
> module crate is the recommended versioning strategy.

## Breakdown of modules/crates

### `elme`

Module re-exports - each module is gated by a feature flag.

This is the the recommended method of integrating `elme` modules.

[**crates.io**](https://crates.io/crates/elme)
|
[**Docs**](https://docs.rs/elme/latest)
|
[**GitHub**](https://github.com/kylekingcdn/elme-rs/tree/main/crates/elme)

#### Feature flags

See the following list for feature flags relevant to each module.

### `elme-config`

`elme-config` is a very small convenience module.

Supported features:

- A `ConfigureApp` trait for an app's root config struct
  - Contains default impl of a `load()` method, reading vals from `.env` files + env vars
  - Replaces the need for any and all config logic in standard cases
- Pulls in `config` and `dotenvy` deps, removing one-off dependency burden

[**crates.io**](https://crates.io/crates/elme-config)
|
[**Docs**](https://docs.rs/elme-config/latest)
|
[**GitHub**](https://github.com/kylekingcdn/elme-rs/tree/main/crates/elme-config)

#### Relevant `elme` feature flags

**Flag** | **Description**
--------------- | ---------------
**`config`** | Enables this module

### `elme-db`

Provides types supporting:

- Managed/unmanaged database wrapper structs
  - Managed db's are a superset of unmanaged w/ migration support
  - Can optionally be used to scope query availability in associated `Repo` impl's, e.g.
    1. Restrict repos backed by an `UnmanagedDatabase` to read-only queries (select, count, etc)
    2. Provide additional access to mutable queries when the `Repo` is backed with a `ManagedDatabase`
- Managed/unmanaged db config with types automatically mapped to `PgPoolOptions`

And provides utility traits supporting:

- `Repo` trait provides repository pattern on a per-table basis, with associated `Row` types and a clean interface
- Common query operations (insert one, insert many, etc) [*`db-op-traits` feature only*]
  - Boilerplate logic auto impl'd, simply provide the statement and `query.bind()` call
  - Allows for inline chaining of queries - with optional, automatic tx commit/rollback handling

<div class="warning">
<b>This module currently only supports <a href="https://docs.rs/sqlx/latest/)"><code>sqlx</code></a> used with <i>PostgreSQL</i> databases.</b>
<br><br>
Support may be extended in the future.
</div>

[**crates.io**](https://crates.io/crates/elme-db)
|
[**Docs**](https://docs.rs/elme-db/latest)
|
[**GitHub**](https://github.com/kylekingcdn/elme-rs/tree/main/crates/elme-db)

#### Relevant `elme` feature flags

**Flag** | **Description**
--------------- | ---------------
**`db`** | Enables this module
**`db-op-traits`** | Enables the `op-traits` feature of `elme-db` <br> This feature includes traits used for repo operations (insert, batch insert, delete, etc) <br> [Feature docs](https://docs.rs/elme-db/latest/elme_db/repo/ops/)
