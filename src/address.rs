use std::fmt;
use std::str::FromStr;

#[cfg(feature = "serde")]
use serde_crate::{Deserialize, Serialize};

use crate::ToFoldedHeader;

/// Represents an RFC 5322 Address
#[derive(PartialEq, Eq, Debug, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(Deserialize, Serialize),
    serde(crate = "serde_crate")
)]
pub enum Address {
    /// A "regular" email address
    Mailbox(Mailbox),
    /// A named group of mailboxes
    Group(String, Vec<Mailbox>),
}

impl FromStr for Address {
    type Err = mailparse::MailParseError;

    fn from_str(val: &str) -> Result<Self, mailparse::MailParseError> {
        let addrs = mailparse::addrparse(val)?.into_inner();
        if addrs.len() != 1 {
            return Err(mailparse::MailParseError::Generic(
                "expected a single address",
            ));
        }

        match addrs.into_iter().next().unwrap() {
            mailparse::MailAddr::Group(group) => Ok(Address::new_group(
                group.group_name,
                group
                    .addrs
                    .into_iter()
                    .map(|i| Mailbox {
                        name: i.display_name,
                        address: i.addr,
                    })
                    .collect(),
            )),
            mailparse::MailAddr::Single(i) => Ok(Address::Mailbox(Mailbox {
                name: i.display_name,
                address: i.addr,
            })),
        }
    }
}

impl Address {
    /// Attempts to parse a given email address.
    pub fn new(addr: impl AsRef<str>) -> Result<Self, mailparse::MailParseError> {
        addr.as_ref().parse()
    }

    /// Shortcut function to make a new Mailbox with the given address
    /// [unstable]
    pub fn new_mailbox(address: String) -> Address {
        Address::Mailbox(Mailbox::new(address))
    }

    /// Shortcut function to make a new Mailbox with the address and given-name
    /// [unstable]
    pub fn new_mailbox_with_name(name: String, address: String) -> Address {
        Address::Mailbox(Mailbox::new_with_name(name, address))
    }

    /// Shortcut function to make a new Group with a collection of mailboxes
    /// [unstable]
    pub fn new_group(name: String, mailboxes: Vec<Mailbox>) -> Address {
        Address::Group(name, mailboxes)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Address::Mailbox(ref mbox) => mbox.fmt(fmt),
            Address::Group(ref name, ref mboxes) => {
                let mut mailbox_list = String::new();
                for mbox in mboxes.iter() {
                    if !mailbox_list.is_empty() {
                        // Insert the separator if there's already things in this list
                        mailbox_list.push_str(", ");
                    }
                    mailbox_list.push_str(&mbox.to_string()[..]);
                }
                write!(fmt, "{}: {};", name, mailbox_list)
            }
        }
    }
}

/// Represents an RFC 5322 mailbox
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Mailbox {
    /// The given name for this address
    pub name: Option<String>,
    /// The mailbox address
    pub address: String,
}

impl Mailbox {
    /// Create a new Mailbox without a display name
    pub fn new(address: String) -> Mailbox {
        Mailbox {
            name: None,
            address,
        }
    }

    /// Create a new Mailbox with a display name
    pub fn new_with_name(name: String, address: String) -> Mailbox {
        Mailbox {
            name: Some(name),
            address,
        }
    }
}

impl fmt::Display for Mailbox {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self.name {
            Some(ref name) => {
                if name.chars().all(|c| c.is_ascii_alphanumeric() || c == ' ') {
                    write!(fmt, "{} <{}>", name, self.address)
                } else {
                    let s = encoded_words::encode(
                        name,
                        None,
                        encoded_words::EncodingFlag::Shortest,
                        None,
                    );
                    write!(fmt, "{} <{}>", s, self.address)
                }
            }
            None => write!(fmt, "<{}>", self.address),
        }
    }
}

impl<'a> From<&'a str> for Mailbox {
    fn from(mailbox: &'a str) -> Mailbox {
        Mailbox::new(mailbox.into())
    }
}

impl From<String> for Mailbox {
    fn from(mailbox: String) -> Mailbox {
        Mailbox::new(mailbox)
    }
}

impl<S: Into<String>, T: Into<String>> From<(S, T)> for Mailbox {
    fn from(header: (S, T)) -> Mailbox {
        let (address, alias) = header;
        Mailbox::new_with_name(alias.into(), address.into())
    }
}

impl FromStr for Mailbox {
    type Err = mailparse::MailParseError;

    fn from_str(s: &str) -> Result<Mailbox, mailparse::MailParseError> {
        let addrs = mailparse::addrparse(s)?;
        if let Some(info) = addrs.extract_single_info() {
            Ok(Mailbox {
                name: info.display_name,
                address: info.addr,
            })
        } else {
            Err(mailparse::MailParseError::Generic(
                "expected only one address",
            ))
        }
    }
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum AddressFoldingError {
    #[error("Header value cannot be empty")]
    EmtpyHeader,
}

impl ToFoldedHeader for Vec<Address> {
    type Error = AddressFoldingError;

    fn to_folded_header(
        start_pos: usize,
        value: Vec<Address>,
    ) -> Result<String, AddressFoldingError> {
        if value.is_empty() {
            return Err(AddressFoldingError::EmtpyHeader);
        }

        let mut header = String::new();

        let mut line_len = start_pos;

        for addr in value.iter() {
            let addr_str = format!("{}, ", addr);

            if line_len + addr_str.len() > crate::rfc5322::MIME_LINE_LENGTH {
                // Adding this would cause a wrap, so wrap before!
                header.push_str("\r\n\t");
                line_len = 0;
            }
            line_len += addr_str.len();
            header.push_str(&addr_str[..]);
        }

        // Clear up the final ", "
        let real_len = header.len() - 2;
        header.truncate(real_len);

        Ok(header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::header::Header;

    #[test]
    fn test_address_to_string() {
        let addr = Mailbox::new("foo@example.org".to_string());
        assert_eq!(addr.to_string(), "<foo@example.org>".to_string());

        let name_addr =
            Mailbox::new_with_name("Joe Blogs".to_string(), "foo@example.org".to_string());
        assert_eq!(name_addr.to_string(), "Joe Blogs <foo@example.org>");
    }

    #[test]
    fn test_address_from_string() {
        let addr = "\"Joe Blogs\" <joe@example.org>"
            .parse::<Mailbox>()
            .unwrap();
        assert_eq!(addr.name.unwrap(), "Joe Blogs".to_string());
        assert_eq!(addr.address, "joe@example.org".to_string());

        assert!("Not an address".parse::<Mailbox>().is_err());
    }

    #[test]
    fn test_address_group_to_string() {
        let addr = Address::new_group("undisclosed recipients".to_string(), vec![]);
        assert_eq!(addr.to_string(), "undisclosed recipients: ;".to_string());

        let addr = Address::new_group(
            "group test".to_string(),
            vec![
                Mailbox::new("joe@example.org".to_string()),
                Mailbox::new_with_name("John Doe".to_string(), "john@example.org".to_string()),
            ],
        );
        assert_eq!(
            addr.to_string(),
            "group test: <joe@example.org>, John Doe <john@example.org>;".to_string()
        );
    }

    #[test]
    fn test_to_header_generation() {
        let addresses = vec![
            Address::new_mailbox_with_name("Joe Blogs".to_string(), "joe@example.org".to_string()),
            Address::new_mailbox_with_name("John Doe".to_string(), "john@example.org".to_string()),
        ];

        let header = Header::new_with_value("From:".to_string(), addresses).unwrap();
        assert_eq!(
            header.get_value(),
            "Joe Blogs <joe@example.org>, John Doe <john@example.org>",
        );
    }

    #[test]
    fn test_to_header_line_wrap() {
        let addresses = vec![
            Address::new_mailbox_with_name("Joe Blogs".to_string(), "joe@example.org".to_string()),
            Address::new_mailbox_with_name("John Doe".to_string(), "john@example.org".to_string()),
            Address::new_mailbox_with_name(
                "Mr Black".to_string(),
                "mafia_black@example.org".to_string(),
            ),
        ];

        let header = Header::new_with_value("To".to_string(), addresses).unwrap();
        assert_eq!(
            &header.to_string()[..],
            "To: Joe Blogs <joe@example.org>, John Doe <john@example.org>, \r\n\tMr Black <mafia_black@example.org>"
        );
    }

    #[test]
    fn test_to_header_empty() {
        let header = Header::new_with_value("To".to_string(), vec![]);
        assert!(header.is_err());
    }

    #[test]
    fn test_escape_email_address() {
        let display_name = "ä space";
        let addr = "x@y.org";

        assert!(!display_name.is_ascii());

        let s = format!(
            "{}",
            Address::new_mailbox_with_name(display_name.to_string(), addr.to_string())
        );

        println!("{}", s);

        assert_eq!(s, "=?utf-8?q?=C3=A4_space?= <x@y.org>");
    }
}
