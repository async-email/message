use std::collections::HashMap;

use crate::header::ToHeader;

/// Content-Type string, major/minor as the first and second elements
/// respectively.
pub type MimeContentType = (String, String);

/// Special header type for the Content-Type header.
#[derive(Debug, Clone)]
pub struct MimeContentTypeHeader {
    /// The content type presented by this header
    pub content_type: MimeContentType,
    /// Parameters of this header
    pub params: HashMap<String, String>,
}

impl ToHeader for MimeContentTypeHeader {
    type Error = ();

    fn to_header(value: MimeContentTypeHeader) -> Result<String, ()> {
        let (mime_major, mime_minor) = value.content_type;
        let mut result = format!("{}/{}", mime_major, mime_minor);
        for (key, val) in value.params.iter() {
            result = format!("{}; {}={}", result, key, val);
        }
        Ok(result)
    }
}

/// Special header type for the Content-Transfer-Encoding header.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MimeContentTransferEncoding {
    /// Message content is not encoded in any way.
    Identity,
    /// Content transfered using the quoted-printable encoding.
    ///
    /// This encoding is defined in RFC 2045 Section 6.7
    QuotedPrintable,
    /// Content transfered as BASE64
    ///
    /// This encoding is defined in RFC 2045 Section 6.8
    Base64,
}
