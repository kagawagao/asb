#[allow(
    clippy::collapsible_if,
    clippy::too_many_arguments,
    clippy::unnecessary_sort_by
)]
pub mod aapt2;
pub mod aar;
#[allow(
    clippy::collapsible_if,
    clippy::unnecessary_map_or,
    clippy::single_char_add_str,
    clippy::useless_asref
)]
pub mod builder;
pub mod cache;
#[allow(clippy::cmp_owned, clippy::unwrap_or_default)]
pub mod dependency;
pub mod error;
pub mod merge;
#[allow(
    clippy::collapsible_if,
    clippy::new_without_default,
    clippy::unwrap_or_default
)]
pub mod resource_priority;
#[allow(
    clippy::collapsible_if,
    clippy::needless_borrow,
    clippy::ptr_arg,
    clippy::too_many_arguments
)]
pub mod types;
