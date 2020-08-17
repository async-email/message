//! Build email messages in Rust.

#![deny(
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces
)]

mod email_builder;
    
pub mod email;

pub use self::email_builder::*;

