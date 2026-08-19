<p align='center'>
  <img src='https://raw.githubusercontent.com/kylekingcdn/elme-rs/refs/heads/main/assets/elme-rs.png?raw=true' width=400 />
</p>

# `elme-db`

An `elme` module which provides types supporting:

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

## Relevant `elme` feature flags

**Flag for `elme`** | **Description**
--------------- | ---------------
**`db`** | Enables this module
**`db-op-traits`** | Enables the `op-traits` feature of `elme-db`<br><br>This feature includes traits used for repo operations (insert, batch insert, delete, etc) <br> [Feature docs](https://docs.rs/elme-db/latest/elme_db/repo/ops/)
