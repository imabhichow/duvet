#[macro_use]
pub mod attribute;

mod marker;

pub mod coverage;
pub mod db;
pub mod entity;
pub mod fs;
pub mod html;
pub mod notification;
pub mod region;
pub mod schema;
pub mod source;
pub mod types;

#[cfg(feature = "highlight")]
pub mod highlight;

#[cfg(feature = "rust-src")]
pub mod rust_src;
