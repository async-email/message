//! General types for Email messages.

#[cfg(feature = "serde")]
use serde_crate::{Deserialize, Serialize};

pub use crate::{Address, Header, Mailbox, MimeMessage, MimeMultipartType};

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
    forward_path: Vec<Address>,
    /// The envelope sender address
    reverse_path: Option<Address>,
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
    pub fn new(from: Option<Address>, to: Vec<Address>) -> Result<Envelope, EnvelopeError> {
        if to.is_empty() {
            return Err(EnvelopeError::MissingTo);
        }
        Ok(Envelope {
            forward_path: to,
            reverse_path: from,
        })
    }

    /// Destination addresses of the envelope
    pub fn to(&self) -> &[Address] {
        self.forward_path.as_slice()
    }

    /// Source address of the envelope
    pub fn from(&self) -> Option<&Address> {
        self.reverse_path.as_ref()
    }
}
