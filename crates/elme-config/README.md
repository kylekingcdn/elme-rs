<p align='center'>
  <img src='https://raw.githubusercontent.com/kylekingcdn/elme-rs/refs/heads/main/assets/elme-rs.png?raw=true' width=400 />
</p>

# `elme-config`

`elme-config` is a very small convenience module for `elme`.

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

## Relevant `elme` feature flags

**Flag for `elme`** | **Description**
--------------- | ---------------
**`config`** | Enables this module
