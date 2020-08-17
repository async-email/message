//! General types for Email messages.

use std::ffi::OsStr;
use std::fmt;
use std::str::FromStr;

pub use email::{Address, Header, Mailbox, MimeMessage, MimeMultipartType};
use fast_chemail::is_valid_email;
#[cfg(feature = "serde")]
use serde_crate::{Deserialize, Serialize};

/// Email address
#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(
    feature = "serde",
    derive(Deserialize, Serialize),
    serde(crate = "serde_crate")
)]
pub struct EmailAddress(String);

/// Error values for `EmailAddress` parsing.
#[derive(Copy, Clone, Debug, thiserror::Error)]
pub enum EmailAddressError {
    /// Missing to in the envelope.
    #[error("invalid email address")]
    Invalid,
}

impl EmailAddress {
    /// Constructs a new `EmailAddress`, validtating the incoming string.
    pub fn new(address: String) -> Result<EmailAddress, EmailAddressError> {
        if !is_valid_email(&address) && !address.ends_with("localhost") {
            return Err(EmailAddressError::Invalid);
        }

        Ok(EmailAddress(address))
    }
}

impl FromStr for EmailAddress {
    type Err = EmailAddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        EmailAddress::new(s.to_string())
    }
}

impl fmt::Display for EmailAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for EmailAddress {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<OsStr> for EmailAddress {
    fn as_ref(&self) -> &OsStr {
        &self.0.as_ref()
    }
}

/// Represents a message id
pub type MessageId = String;

/// Simple email representation
#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(
    feature = "serde",
    derive(Deserialize, Serialize),
    serde(crate = "serde_crate")
)]
pub struct Email {
    /// Message
    pub message: Vec<u8>,
    /// Envelope
    pub envelope: Envelope,
    /// Message-ID
    pub message_id: String,
}

impl Email {
    /// Creates a new email builder
    pub fn builder() -> crate::EmailBuilder {
        crate::EmailBuilder::new()
    }

    /// Creates a string version of the actual message.
    pub fn message_to_string(self) -> Result<String, std::string::FromUtf8Error> {
        std::string::String::from_utf8(self.message)
    }
}

/// Simple email envelope representation
///
/// We only accept mailboxes, and do not support source routes (as per RFC).
#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(
    feature = "serde",
    derive(Deserialize, Serialize),
    serde(crate = "serde_crate")
)]
pub struct Envelope {
    /// The envelope recipients' addresses
    ///
    /// This can not be empty.
    forward_path: Vec<EmailAddress>,
    /// The envelope sender address
    reverse_path: Option<EmailAddress>,
}

/// Error values for `Envelope` construction.
#[derive(Copy, Clone, Debug, thiserror::Error)]
pub enum EnvelopeError {
    /// Missing to in the envelope.
    #[error("missing destination address")]
    MissingTo,
    /// Missing from in the envelope.
    #[error("missing from address")]
    MissingFrom,
}

impl Envelope {
    /// Creates a new envelope, which may fail if `to` is empty.
    pub fn new(
        from: Option<EmailAddress>,
        to: Vec<EmailAddress>,
    ) -> Result<Envelope, EnvelopeError> {
        if to.is_empty() {
            return Err(EnvelopeError::MissingTo);
        }
        Ok(Envelope {
            forward_path: to,
            reverse_path: from,
        })
    }

    /// Destination addresses of the envelope
    pub fn to(&self) -> &[EmailAddress] {
        self.forward_path.as_slice()
    }

    /// Source address of the envelope
    pub fn from(&self) -> Option<&EmailAddress> {
        self.reverse_path.as_ref()
    }
}
