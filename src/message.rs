use std::collections::HashMap;

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

use crate::header::{Header, HeaderMap};
use crate::mimeheader::{MimeContentType, MimeContentTypeHeader};
use crate::rfc5322::Rfc5322Builder;

const BOUNDARY_LENGTH: usize = 30;

/// Marks the type of a multipart message
#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum MimeMultipartType {
    /// Entries which are independent.
    ///
    /// This value is the default.
    ///
    /// As defined by Section 5.1.3 of RFC 2046
    Mixed,
    /// Entries which are interchangeable, such that the system can choose
    /// whichever is "best" for its use.
    ///
    /// As defined by Section 5.1.4 of RFC 2046
    Alternative,
    /// Entries are (typically) a collection of messages.
    ///
    /// As defined by Section 5.1.5 of RFC 2046
    Digest,
    /// Two entries, the first of which explains the decryption process for
    /// the second body part.
    ///
    /// As defined by Section 2.2 of RFC 1847
    Encrypted,
    /// Entry order does not matter, and could be displayed simultaneously.
    ///
    /// As defined by Section 5.1.6 of RFC 2046
    Parallel,
    /// Two entries, the first of which is the content, the second is a
    /// digital signature of the first, including MIME headers.
    ///
    /// As defined by Section 2.1 of RFC 1847
    Signed,
}

impl MimeMultipartType {
    /// Returns the appropriate `MimeMultipartType` for the given MimeContentType
    pub fn from_content_type(ct: MimeContentType) -> Option<MimeMultipartType> {
        let (major, minor) = ct;
        match (&major[..], &minor[..]) {
            ("multipart", "alternative") => Some(MimeMultipartType::Alternative),
            ("multipart", "digest") => Some(MimeMultipartType::Digest),
            ("multipart", "encrypted") => Some(MimeMultipartType::Encrypted),
            ("multipart", "parallel") => Some(MimeMultipartType::Parallel),
            ("multipart", "signed") => Some(MimeMultipartType::Signed),
            ("multipart", "mixed") | ("multipart", _) => Some(MimeMultipartType::Mixed),
            _ => None,
        }
    }

    /// Returns a MimeContentType that represents this multipart type.
    pub fn to_content_type(self) -> MimeContentType {
        let multipart = "multipart".to_string();
        match self {
            MimeMultipartType::Mixed => (multipart, "mixed".to_string()),
            MimeMultipartType::Alternative => (multipart, "alternative".to_string()),
            MimeMultipartType::Digest => (multipart, "digest".to_string()),
            MimeMultipartType::Encrypted => (multipart, "encrypted".to_string()),
            MimeMultipartType::Parallel => (multipart, "parallel".to_string()),
            MimeMultipartType::Signed => (multipart, "signed".to_string()),
        }
    }
}

/// Represents a MIME message
/// [unstable]
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct MimeMessage {
    /// The headers for this message
    pub headers: HeaderMap,

    /// The content of this message
    ///
    /// Keep in mind that this is the undecoded form, so may be quoted-printable
    /// or base64 encoded.
    pub body: String,

    /// The MIME multipart message type of this message, or `None` if the message
    /// is not a multipart message.
    pub message_type: Option<MimeMultipartType>,

    /// Any additional parameters of the MIME multipart header, not including the boundary.
    pub message_type_params: Option<HashMap<String, String>>,

    /// The sub-messages of this message
    pub children: Vec<MimeMessage>,

    /// The boundary used for MIME multipart messages
    ///
    /// This will always be set, even if the message only has a single part
    pub boundary: String,
}

impl MimeMessage {
    fn random_boundary() -> String {
        let mut rng = thread_rng();
        std::iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .take(BOUNDARY_LENGTH)
            .collect()
    }

    /// [unstable]
    pub fn new(body: String) -> MimeMessage {
        let mut message = MimeMessage::new_blank_message();
        message.body = body;
        message.update_headers();
        message
    }

    pub fn new_with_children(
        body: String,
        message_type: MimeMultipartType,
        children: Vec<MimeMessage>,
    ) -> MimeMessage {
        let mut message = MimeMessage::new_blank_message();
        message.body = body;
        message.message_type = Some(message_type);
        message.children = children;
        message.update_headers();
        message
    }

    pub fn new_with_boundary(
        body: String,
        message_type: MimeMultipartType,
        children: Vec<MimeMessage>,
        boundary: String,
    ) -> MimeMessage {
        let mut message = MimeMessage::new_blank_message();
        message.body = body;
        message.message_type = Some(message_type);
        message.children = children;
        message.boundary = boundary;
        message.update_headers();
        message
    }

    pub fn new_with_boundary_and_params(
        body: String,
        message_type: MimeMultipartType,
        children: Vec<MimeMessage>,
        boundary: String,
        message_type_params: Option<HashMap<String, String>>,
    ) -> MimeMessage {
        let mut message = MimeMessage::new_blank_message();
        message.body = body;
        message.message_type = Some(message_type);
        message.children = children;
        message.boundary = boundary;
        message.message_type_params = message_type_params;
        message.update_headers();
        message
    }

    pub fn new_blank_message() -> MimeMessage {
        MimeMessage {
            headers: HeaderMap::new(),
            body: "".to_string(),
            message_type: None,
            message_type_params: None,
            children: Vec::new(),

            boundary: MimeMessage::random_boundary(),
        }
    }

    /// Update the headers on this message based on the internal state.
    ///
    /// When certain properties of the message are modified, the headers
    /// used to represent them are not automatically updated.
    /// Call this if these are changed.
    pub fn update_headers(&mut self) {
        if !self.children.is_empty() && self.message_type.is_none() {
            // This should be a multipart message, so make it so!
            self.message_type = Some(MimeMultipartType::Mixed);
        }

        if let Some(message_type) = self.message_type {
            // We are some form of multi-part message, so update our
            // Content-Type header.
            let mut params = match &self.message_type_params {
                Some(p) => p.clone(),
                None => HashMap::new(),
            };
            params.insert("boundary".to_string(), self.boundary.clone());
            let ct_header = MimeContentTypeHeader {
                content_type: message_type.to_content_type(),
                params,
            };
            self.headers
                .replace(Header::new_with_value("Content-Type".to_string(), ct_header).unwrap());
        }
    }

    pub fn as_string(&self) -> String {
        let mut builder = Rfc5322Builder::new();

        for header in self.headers.iter() {
            builder.emit_folded(&header.to_string()[..]);
            builder.emit_raw("\r\n");
        }
        builder.emit_raw("\r\n");

        self.as_string_without_headers_internal(builder)
    }

    pub fn as_string_without_headers(&self) -> String {
        let builder = Rfc5322Builder::new();

        self.as_string_without_headers_internal(builder)
    }

    fn as_string_without_headers_internal(&self, mut builder: Rfc5322Builder) -> String {
        builder.emit_raw(&format!("{}\r\n", self.body)[..]);

        if !self.children.is_empty() {
            for part in self.children.iter() {
                builder.emit_raw(&format!("--{}\r\n{}\r\n", self.boundary, part.as_string())[..]);
            }

            builder.emit_raw(&format!("--{}--\r\n", self.boundary)[..]);
        }

        builder.result().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MultipartParseTest<'s> {
        mime_type: (&'s str, &'s str),
        result: Option<MimeMultipartType>,
    }

    #[test]
    fn test_multipart_type_type_parsing() {
        let tests = vec![
            MultipartParseTest {
                mime_type: ("multipart", "mixed"),
                result: Some(MimeMultipartType::Mixed),
            },
            MultipartParseTest {
                mime_type: ("multipart", "alternative"),
                result: Some(MimeMultipartType::Alternative),
            },
            MultipartParseTest {
                mime_type: ("multipart", "digest"),
                result: Some(MimeMultipartType::Digest),
            },
            MultipartParseTest {
                mime_type: ("multipart", "parallel"),
                result: Some(MimeMultipartType::Parallel),
            },
            // Test fallback on multipart/mixed
            MultipartParseTest {
                mime_type: ("multipart", "potato"),
                result: Some(MimeMultipartType::Mixed),
            },
            // Test failure state
            MultipartParseTest {
                mime_type: ("text", "plain"),
                result: None,
            },
        ];

        for test in tests.into_iter() {
            let (major_type, minor_type) = test.mime_type;
            assert_eq!(
                MimeMultipartType::from_content_type((
                    major_type.to_string(),
                    minor_type.to_string()
                )),
                test.result
            );
        }
    }

    #[test]
    fn test_multipart_type_to_content_type() {
        let multipart = "multipart".to_string();

        assert_eq!(
            MimeMultipartType::Mixed.to_content_type(),
            (multipart.clone(), "mixed".to_string())
        );
        assert_eq!(
            MimeMultipartType::Alternative.to_content_type(),
            (multipart.clone(), "alternative".to_string())
        );
        assert_eq!(
            MimeMultipartType::Digest.to_content_type(),
            (multipart.clone(), "digest".to_string())
        );
        assert_eq!(
            MimeMultipartType::Parallel.to_content_type(),
            (multipart.clone(), "parallel".to_string())
        );
    }

    #[test]
    fn test_boundary_generation() {
        let message = MimeMessage::new("Body".to_string());
        // This is random, so we can only really check that it's the expected length
        assert_eq!(message.boundary.len(), super::BOUNDARY_LENGTH);
    }
}

#[cfg(all(feature = "nightly", test))]
mod bench {
    extern crate test;
    use self::test::Bencher;

    use super::*;

    macro_rules! bench_parser {
        ($name:ident, $test:expr) => {
            #[bench]
            fn $name(b: &mut Bencher) {
                let s = $test;
                b.iter(|| {
                    let _ = MimeMessage::parse(s);
                });
            }
        };
    }

    bench_parser!(
        bench_simple,
        "From: joe@example.org\r\nTo: john@example.org\r\n\r\nHello!"
    );
    bench_parser!(
        bench_simple_multipart,
        "From: joe@example.org\r\n\
         To: john@example.org\r\n\
         Content-Type: multipart/alternative; boundary=foo\r\n\
         \r\n\
         Parent\r\n\
         --foo\r\n\
         Hello!\r\n\
         --foo\r\n\
         Other\r\n\
         --foo"
    );
    bench_parser!(
        bench_deep_multipart,
        "From: joe@example.org\r\n\
         To: john@example.org\r\n\
         Content-Type: multipart/mixed; boundary=foo\r\n\
         \r\n\
         Parent\r\n\
         --foo\r\n\
         Content-Type: multipart/alternative; boundary=bar\r\n\
         \r\n\
         --bar\r\n\
         Hello!\r\n\
         --bar\r\n\
         Other\r\n\
         --foo\r\n\
         Outside\r\n\
         --foo\r\n"
    );
}
