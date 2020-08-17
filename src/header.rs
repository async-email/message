use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;
use std::slice::Iter as SliceIter;
use std::sync::Arc;

/// Trait for converting from a Rust type into a Header value.
pub trait ToHeader {
    type Error;

    /// Turn the `value` into a String suitable for being used in
    /// a message header.
    ///
    /// Returns None if the value cannot be stringified.
    fn to_header(value: Self) -> Result<String, Self::Error>;
}

/// Trait for converting from a Rust time into a Header value
/// that handles its own folding.
///
/// Be mindful that this trait does not mean that the value will
/// not be folded later, rather that the type returns a value that
/// should not be folded, given that the header value starts so far
/// in to a line.
pub trait ToFoldedHeader {
    type Error;
    fn to_folded_header(start_pos: usize, value: Self) -> Result<String, Self::Error>;
}

impl<T: ToHeader> ToFoldedHeader for T {
    type Error = <T as ToHeader>::Error;

    fn to_folded_header(_: usize, value: T) -> Result<String, Self::Error> {
        // We ignore the start_position because the thing will fold anyway.
        ToHeader::to_header(value)
    }
}

impl ToHeader for String {
    type Error = ();

    fn to_header(value: String) -> Result<String, ()> {
        Ok(value)
    }
}

impl<'a> ToHeader for &'a str {
    type Error = ();

    fn to_header(value: &'a str) -> Result<String, ()> {
        Ok(value.to_string())
    }
}

/// Represents an RFC 822 Header
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct Header {
    /// The name of this header
    pub name: String,
    value: String,
}

impl<S: Into<String>, T: Into<String>> From<(S, T)> for Header {
    fn from(header: (S, T)) -> Self {
        let (name, value) = header;
        Header::new(name.into(), value.into())
    }
}

impl Header {
    /// Creates a new Header for the given `name` and `value`
    pub fn new(name: String, value: String) -> Header {
        Header { name, value }
    }

    /// Creates a new Header for the given `name` and `value`,
    /// as converted through the `ToHeader` or `ToFoldedHeader` trait.
    ///
    /// Returns None if the value failed to be converted.
    pub fn new_with_value<T: ToFoldedHeader>(name: String, value: T) -> Result<Header, T::Error> {
        let header_len = name.len() + 2;
        ToFoldedHeader::to_folded_header(header_len, value)
            .map(|val| Header::new(name.clone(), val))
    }

    /// Get the value represented by this header.
    pub fn get_value(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for Header {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}: {}", self.name, self.value)
    }
}

#[derive(Debug)]
pub struct HeaderIter<'s> {
    iter: SliceIter<'s, Arc<Header>>,
}

impl<'s> HeaderIter<'s> {
    fn new(iter: SliceIter<'s, Arc<Header>>) -> HeaderIter<'s> {
        HeaderIter { iter }
    }
}

impl<'s> Iterator for HeaderIter<'s> {
    type Item = &'s Header;

    fn next(&mut self) -> Option<&'s Header> {
        match self.iter.next() {
            Some(s) => Some(s.deref()),
            None => None,
        }
    }
}

/// A collection of Headers
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct HeaderMap {
    // We store headers "twice" inside the HeaderMap.
    //
    // The first is as an ordered list of headers,
    // which is used to iterate over.
    ordered_headers: Vec<Arc<Header>>,
    // The second is as a mapping between header names
    // and all of the headers with that name.
    //
    // This allows quick retrival of a header by name.
    headers: HashMap<String, Vec<Arc<Header>>>,
}

impl HeaderMap {
    pub fn new() -> HeaderMap {
        HeaderMap {
            ordered_headers: Vec::new(),
            headers: HashMap::new(),
        }
    }

    /// Adds a header to the collection
    pub fn insert(&mut self, header: Header) {
        let header_name = header.name.clone();
        let rc = Arc::new(header);
        // Add to the ordered list of headers
        self.ordered_headers.push(rc.clone());

        // and to the mapping between header names and values.
        match self.headers.entry(header_name) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().push(rc);
            }
            Entry::Vacant(entry) => {
                // There haven't been any headers with this name
                // as of yet, so make a new list and push it in.
                let mut header_list = Vec::new();
                header_list.push(rc);
                entry.insert(header_list);
            }
        };
    }

    pub fn replace(&mut self, header: Header) {
        let header_name = header.name.clone();
        let rc = Arc::new(header);
        // Remove existing
        let mut i = 0;
        let mut have_inserted = false;
        while i < self.ordered_headers.len() {
            if self.ordered_headers[i].name == header_name {
                if have_inserted {
                    // Just remove the header, as we've already updated
                    self.ordered_headers.remove(i);
                } else {
                    // Update the header in-place
                    self.ordered_headers[i] = rc.clone();
                    have_inserted = true;
                }
            } else {
                i += 1;
            }
        }
        let mut header_list = Vec::new();
        header_list.push(rc.clone());
        // Straight up replace the header in the map
        self.headers.insert(header_name, header_list);
    }

    /// Get an Iterator over the collection of headers.
    pub fn iter(&self) -> HeaderIter {
        HeaderIter::new(self.ordered_headers.iter())
    }

    /// Get the last value of the header with `name`
    pub fn get(&self, name: String) -> Option<&Header> {
        self.headers
            .get(&name)
            .map(|headers| headers.last().unwrap())
            .map(|rc| rc.deref())
    }

    /// Get the number of headers within this map.
    pub fn len(&self) -> usize {
        self.ordered_headers.len()
    }

    /// Returns true if there are no headers in this map.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Find a list of headers of `name`, `None` if there
    /// are no headers with that name.
    pub fn find(&self, name: &str) -> Option<Vec<&Header>> {
        self.headers
            .get(name)
            .map(|rcs| rcs.iter().map(|rc| rc.deref()).collect())
    }
}

impl Default for HeaderMap {
    fn default() -> Self {
        HeaderMap::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    static SAMPLE_HEADERS: [(&'static str, &'static str); 4] = [
        ("Test", "Value"),
        ("Test", "Value 2"),
        ("Test-2", "Value 3"),
        ("Test-Multiline", "Foo\nBar"),
    ];

    fn make_sample_headers() -> Vec<Header> {
        SAMPLE_HEADERS
            .iter()
            .map(|&(name, value)| Header::new(name.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn test_header_to_string() {
        let header = Header::new("Test".to_string(), "Value".to_string());
        assert_eq!(header.to_string(), "Test: Value".to_string());
    }

    #[test]
    fn test_string_get_value() {
        struct HeaderTest<'s> {
            input: &'s str,
            result: Option<&'s str>,
        }

        let tests = vec![
            HeaderTest {
                input: "Value",
                result: Some("Value"),
            },
            HeaderTest {
                input: "=?ISO-8859-1?Q?Test=20text?=",
                result: Some("Test text"),
            },
            HeaderTest {
                input: "=?ISO-8859-1?Q?Multiple?= =?utf-8?b?ZW5jb2Rpbmdz?=",
                result: Some("Multiple encodings"),
            },
            HeaderTest {
                input: "Some things with =?utf-8?b?ZW5jb2Rpbmdz?=, other things without.",
                result: Some("Some things with encodings, other things without."),
            },
            HeaderTest {
                input: "Encoding =?utf-8?q?fail",
                result: Some("Encoding =?utf-8?q?fail"),
            },
        ];

        for test in tests.into_iter() {
            let header = Header::new("Test".to_string(), test.input.to_string());
            let string_value = header.get_value();
            assert_eq!(string_value, test.result.unwrap());
        }
    }

    #[test]
    fn test_to_header_string() {
        let header = Header::new_with_value("Test".to_string(), "Value".to_string()).unwrap();
        let header_value = header.get_value();
        assert_eq!(header_value, "Value");
    }

    #[test]
    fn test_header_map_len() {
        let mut headers = HeaderMap::new();
        for (i, header) in make_sample_headers().into_iter().enumerate() {
            headers.insert(header);
            assert_eq!(headers.len(), i + 1);
        }
    }
    #[test]
    fn test_header_map_iter() {
        let mut headers = HeaderMap::new();
        let mut expected_headers = HashSet::new();
        for header in make_sample_headers().into_iter() {
            headers.insert(header.clone());
            expected_headers.insert(header);
        }

        let mut count = 0;
        // Ensure all the headers returned are expected
        for header in headers.iter() {
            assert!(expected_headers.contains(header));
            count += 1;
        }
        // And that there is the right number of them
        assert_eq!(count, expected_headers.len());
    }
}
