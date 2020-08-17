//! Build email messages in Rust.

#![deny(
    // missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces
)]

mod email_builder;
mod address;
mod header;
mod mimeheader;
mod message;
mod rfc5322;

    
pub mod email;

pub use self::message::*;
pub use self::mimeheader::*;
pub use self::email_builder::*;
pub use self::address::*;
pub use self::header::*;

