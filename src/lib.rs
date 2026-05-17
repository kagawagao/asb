#![allow(
    clippy::too_many_arguments,
    clippy::collapsible_if,
    clippy::cmp_owned,
    clippy::unwrap_or_default
)]

pub mod aapt2;
pub mod aar;
pub mod builder;
pub mod cache;
pub mod dependency;
pub mod error;
pub mod merge;
pub mod resource_priority;
pub mod types;
