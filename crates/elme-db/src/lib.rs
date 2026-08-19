#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(rustdoc::broken_intra_doc_links)]
#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/kylekingcdn/elme-rs/refs/heads/main/assets/elme-rs.png?raw=true")]

pub mod conf;
pub mod db;
pub mod error;
pub mod repo;
pub mod util;

pub use conf::{ManagedDbConf, UnmanagedDbConf};
pub use db::{ManagedDb, UnmanagedDb};
pub use error::DbError;
pub use repo::Repo;
pub use util::{PartialVec, SortOrder};
