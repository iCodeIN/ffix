//! Rust FFI utilities
#![warn(missing_docs)]

mod error;

/// Array-related utilities
pub mod array;

/// String-related utilities
pub mod string;

pub use self::error::{Error, Result};
