//! Module with helpers for dealing with RFC 5322.

pub const MIME_LINE_LENGTH: usize = 78;

trait Rfc5322Character {
    /// Is considered a special character by RFC 5322 Section 3.2.3
    fn is_special(&self) -> bool;
    /// Is considered to be a VCHAR by RFC 5234 Appendix B.1
    fn is_vchar(&self) -> bool;
    /// Is considered to be field text as defined by RFC 5322 Section 3.6.8
    fn is_ftext(&self) -> bool;

    fn is_atext(&self) -> bool {
        self.is_vchar() && !self.is_special()
    }
}

impl Rfc5322Character for char {
    fn is_ftext(&self) -> bool {
        match *self {
            '!'..='9' | ';'..='~' => true,
            _ => false,
        }
    }

    fn is_special(&self) -> bool {
        match *self {
            '(' | ')' | '<' | '>' | '[' | ']' | ':' | ';' | '@' | '\\' | ',' | '.' | '\"' | ' ' => {
                true
            }
            _ => false,
        }
    }

    fn is_vchar(&self) -> bool {
        match *self {
            '!'..='~' => true,
            _ => false,
        }
    }
}

/// Type for constructing RFC 5322 messages
pub struct Rfc5322Builder {
    result: String,
}

impl Rfc5322Builder {
    /// Make a new builder, with an empty string
    pub fn new() -> Rfc5322Builder {
        Rfc5322Builder {
            result: "".to_string(),
        }
    }

    pub fn result(&self) -> &String {
        &self.result
    }

    pub fn emit_raw(&mut self, s: &str) {
        self.result.push_str(s);
    }

    pub fn emit_folded(&mut self, s: &str) {
        let mut cur_len = 0;
        let mut last_space = 0;
        let mut last_cut = 0;

        for (pos, c) in s.char_indices() {
            match c {
                ' ' => {
                    last_space = pos;
                }
                '\r' => {
                    cur_len = 0;
                }
                '\n' => {
                    cur_len = 0;
                }
                _ => {}
            }

            cur_len += 1;
            // We've reached our line length, so
            if cur_len >= MIME_LINE_LENGTH && last_space > 0 {
                // Emit the string from the last place we cut it to the
                // last space that we saw
                self.emit_raw(&s[last_cut..last_space]);
                // ... and get us ready to put out the continuation
                self.emit_raw("\r\n\t");

                // Reset our counters
                cur_len = 0;
                last_cut = last_space + s[last_space..].chars().next().unwrap().len_utf8();
                last_space = 0;
            }
        }

        // Finally, emit everything left in the string
        self.emit_raw(&s[last_cut..]);
    }
}

impl Default for Rfc5322Builder {
    fn default() -> Self {
        Rfc5322Builder::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_folding() {
        struct BuildFoldTest<'s> {
            input: &'s str,
            expected: &'s str,
        }

        let tests = vec![
            BuildFoldTest {
                input: "A long line that should get folded on a space at some point around here, possibly at this point.",
                expected: "A long line that should get folded on a space at some point around here,\r\n\
                \tpossibly at this point.",
            },
            BuildFoldTest {
                input: "A long line that should get folded on a space at some point around here, possibly at this point. And yet more content that will get folded onto another line.",
                expected: "A long line that should get folded on a space at some point around here,\r\n\
                \tpossibly at this point. And yet more content that will get folded onto another\r\n\
                \tline.",
            },
        ];

        for test in tests.into_iter() {
            let mut gen = Rfc5322Builder::new();
            gen.emit_folded(test.input);
            assert_eq!(gen.result(), &test.expected.to_string());
        }
    }
}
