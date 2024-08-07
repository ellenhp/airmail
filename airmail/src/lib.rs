#![forbid(unsafe_code)]
#![warn(clippy::missing_panics_doc)]

#[macro_use]
extern crate lazy_static;

pub mod error;
pub mod index;
pub mod poi;
pub mod query;
pub mod substitutions;
