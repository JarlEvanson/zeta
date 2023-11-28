//! Iterators over various TOML string types.

use core::{
    fmt::{Display, Write},
    str::Chars,
};

use super::{is_newline, is_whitespace};

/// An iterator over the resolved characters of a TOML basic string.
#[derive(Clone, Debug)]
pub struct BasicStringIterator<'str>(Chars<'str>);

impl<'str> BasicStringIterator<'str> {
    /// Validates that `str` is a valid TOML basic string.
    pub fn parse_from_str(str: &'str str) -> Option<BasicStringIterator<'str>> {
        let mut chars = str.chars();

        if !matches!(chars.next(), Some('"')) {
            return None;
        }

        while let Some(ch) = chars.next() {
            if ('\u{0}'..='\u{8}').contains(&ch)
                || ('\n'..='\u{1F}').contains(&ch)
                || ch == '\u{7F}'
            {
                // Illegal control characters inside basic string.
                return None;
            } else if ch == '\\' {
                // Escape characters.
                match chars.next() {
                    Some('b' | 't' | 'n' | 'f' | 'r' | '"' | '\\') => {}
                    Some('u') => {
                        let descriptor = chars.as_str().get(0..4)?;

                        let value = u32::from_str_radix(descriptor, 16).ok()?;

                        char::from_u32(value)?;
                    }
                    Some('U') => {
                        let descriptor = chars.as_str().get(0..8)?;

                        let value = u32::from_str_radix(descriptor, 16).ok()?;

                        char::from_u32(value)?;
                    }
                    _ => return None,
                }
            } else if ch == '"' {
                // End of string.
                let k = str.strip_suffix(chars.as_str())?.strip_suffix('"')?.len();

                let basic_string = &str[1..k];

                return Some(Self(basic_string.chars()));
            }
        }

        None
    }
}

impl Iterator for BasicStringIterator<'_> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let current_char = self.0.next()?;

        if current_char != '\\' {
            return Some(current_char);
        }

        let actual_char = match self.0.next() {
            // Backspace
            Some('b') => '\u{0008}',
            // Tab
            Some('t') => '\t',
            // Newline
            Some('n') => '\n',
            // Form feed
            Some('f') => '\u{000C}',
            // Carriage return
            Some('r') => '\r',
            // Quote
            Some('"') => '"',
            // Backslash
            Some('\\') => '\\',
            // U+XXXX
            Some('u') => {
                // PANIC SAFETY:
                // The operation below was previously tested when the underlying string was parsed.
                // Therefore, this operation should never panic.
                let descriptor = self.0.as_str().get(0..4).unwrap();

                for _ in 0..4 {
                    let _ = self.0.next();
                }

                // PANIC SAFETY:
                // The operation below was previously tested when the underlying string was parsed.
                // Therefore, this operation should never panic.
                let value = u32::from_str_radix(descriptor, 16).unwrap();

                // PANIC SAFETY:
                // The operation below was previously tested when the underlying string was parsed.
                // Therefore, this operation should never panic.
                char::from_u32(value).unwrap()
            }
            // U+XXXXXXXX
            Some('U') => {
                // PANIC SAFETY:
                // The operation below was previously tested when the underlying string was parsed.
                // Therefore, this operation should never panic.
                let descriptor = self.0.as_str().get(0..8).unwrap();

                // PANIC SAFETY:
                // The operation below was previously tested when the underlying string was parsed.
                // Therefore, this operation should never panic.
                let value = u32::from_str_radix(descriptor, 16).unwrap();

                for _ in 0..8 {
                    let _ = self.0.next();
                }

                // PANIC SAFETY:
                // The operation below was previously tested when the underlying string was parsed.
                // Therefore, this operation should never panic.
                char::from_u32(value).unwrap()
            }
            Some(_) | None => {
                // PANIC SAFETY:
                // The operation below was previously tested when the underlying string was parsed.
                // Therefore, this operation should never panic.
                unreachable!("a valid basic string must not include a broken escape")
            }
        };

        Some(actual_char)
    }
}

impl<I: Iterator<Item = char> + Clone> PartialEq<I> for BasicStringIterator<'_> {
    fn eq(&self, other: &I) -> bool {
        <Self as Iterator>::eq(self.clone(), other.clone())
    }
}

impl Eq for BasicStringIterator<'_> {}

impl PartialEq<str> for BasicStringIterator<'_> {
    fn eq(&self, other: &str) -> bool {
        <Self as Iterator>::eq(self.clone(), other.chars())
    }
}

impl Display for BasicStringIterator<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for ch in self.clone() {
            f.write_char(ch)?;
        }

        Ok(())
    }
}

/// An iterator over the resolved characters of a TOML multi-line basic string.
#[derive(Clone, Debug)]
pub struct MultiLineBasicStringIterator<'str>(Chars<'str>);

impl<'str> MultiLineBasicStringIterator<'str> {
    /// Validates that `str` is a valid TOML multi-line basic string.
    pub fn parse_from_str(str: &'str str) -> Option<MultiLineBasicStringIterator<'str>> {
        let mut chars = str.chars();

        if !matches!(chars.next(), Some('"'))
            || !matches!(chars.next(), Some('"'))
            || !matches!(chars.next(), Some('"'))
        {
            return None;
        }

        let underlying = chars.as_str();

        while let Some(ch) = chars.next() {
            if ('\u{0}'..='\u{8}').contains(&ch)
                || ch == '\u{B}'
                || ch <= '\u{C}'
                || ('\u{E}'..='\u{1F}').contains(&ch)
                || ch == '\u{7F}'
            {
                // Illegal control characters inside basic string.
                return None;
            } else if ch == '\\' {
                // Escape characters.
                match chars.next() {
                    Some('b' | 't' | 'n' | 'f' | 'r' | '"' | '\\') => {}
                    Some('u') => {
                        let descriptor = chars.as_str().get(0..4)?;

                        let value = u32::from_str_radix(descriptor, 16).ok()?;

                        char::from_u32(value)?;
                    }
                    Some('U') => {
                        let descriptor = chars.as_str().get(0..8)?;

                        let value = u32::from_str_radix(descriptor, 16).ok()?;

                        char::from_u32(value)?;
                    }
                    Some(ch)
                        if ch == '\n' || (ch == '\r' && chars.clone().next() == Some('\n')) => {}
                    Some(ch)
                        if is_whitespace(ch) && {
                            while let Some(ch) = chars.clone().next() {
                                if is_whitespace(ch) {
                                    let _ = chars.next();
                                } else {
                                    break;
                                }
                            }

                            is_newline(chars.as_str())
                        } => {}
                    _ => return None,
                }
            } else if ch == '"' && chars.next() == Some('"') && chars.next() == Some('"') {
                // End of string.

                let underlying_str = underlying
                    .strip_suffix(chars.as_str())?
                    .strip_suffix("\"\"\"")?;

                return Some(Self(underlying_str.chars()));
            }
        }

        None
    }
}

impl Iterator for MultiLineBasicStringIterator<'_> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let current_char = self.0.next()?;

        if current_char != '\\' {
            return Some(current_char);
        }

        loop {
            match self.0.next() {
                // Backspace
                Some('b') => return Some('\u{0008}'),
                // Tab
                Some('t') => return Some('\t'),
                // Newline
                Some('n') => return Some('\n'),
                // Form feed
                Some('f') => return Some('\u{000C}'),
                // Carriage return
                Some('r') => return Some('\r'),
                // Quote
                Some('"') => return Some('"'),
                // Backslash
                Some('\\') => return Some('\\'),
                // U+XXXX
                Some('u') => {
                    // PANIC SAFETY:
                    // The operation below was previously tested when the underlying string was parsed.
                    // Therefore, this operation should never panic.
                    let descriptor = self.0.as_str().get(0..4).unwrap();

                    // PANIC SAFETY:
                    // The operation below was previously tested when the underlying string was parsed.
                    // Therefore, this operation should never panic.
                    let value = u32::from_str_radix(descriptor, 16).unwrap();

                    // PANIC SAFETY:
                    // The operation below was previously tested when the underlying string was parsed.
                    // Therefore, this operation should never panic.
                    return char::from_u32(value);
                }
                // U+XXXXXXXX
                Some('U') => {
                    // PANIC SAFETY:
                    // The operation below was previously tested when the underlying string was parsed.
                    // Therefore, this operation should never panic.
                    let descriptor = self.0.as_str().get(0..8).unwrap();

                    // PANIC SAFETY:
                    // The operation below was previously tested when the underlying string was parsed.
                    // Therefore, this operation should never panic.
                    let value = u32::from_str_radix(descriptor, 16).unwrap();

                    // PANIC SAFETY:
                    // The operation below was previously tested when the underlying string was parsed.
                    // Therefore, this operation should never panic.
                    return char::from_u32(value);
                }
                Some(w)
                    if is_whitespace(w)
                        || w == '\n'
                        || (w == '\r' && self.0.clone().next() == Some('\n')) =>
                {
                    let mut peeker = self.0.clone();

                    while let Some(peeked) = peeker.next() {
                        if is_whitespace(peeked) || is_newline(peeker.as_str()) {
                            self.0.next();
                        }
                    }
                }
                Some(_) | None => {
                    // PANIC SAFETY:
                    // The operation below was previously tested when the underlying string was parsed.
                    // Therefore, this operation should never panic.
                    unreachable!("a valid basic string must not include a broken escape")
                }
            };
        }
    }
}

impl<I: Iterator<Item = char> + Clone> PartialEq<I> for MultiLineBasicStringIterator<'_> {
    fn eq(&self, other: &I) -> bool {
        <Self as Iterator>::eq(self.clone(), other.clone())
    }
}

impl Eq for MultiLineBasicStringIterator<'_> {}

impl PartialEq<str> for MultiLineBasicStringIterator<'_> {
    fn eq(&self, other: &str) -> bool {
        <Self as Iterator>::eq(self.clone(), other.chars())
    }
}

impl Display for MultiLineBasicStringIterator<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for ch in self.clone() {
            f.write_char(ch)?;
        }

        Ok(())
    }
}

/// A wrapper around [`Chars`] that makes it comparable.
#[derive(Clone, Debug)]
pub struct StringIterator<'str>(Chars<'str>);

impl<'str> StringIterator<'str> {
    /// Constructs a new [`StringIterator`].
    pub fn new(chars: Chars<'str>) -> StringIterator<'str> {
        Self(chars)
    }
}

impl Iterator for StringIterator<'_> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<I: Iterator<Item = char> + Clone> PartialEq<I> for StringIterator<'_> {
    fn eq(&self, other: &I) -> bool {
        <Self as Iterator>::eq(self.clone(), other.clone())
    }
}

impl Eq for StringIterator<'_> {}

impl PartialEq<str> for StringIterator<'_> {
    fn eq(&self, other: &str) -> bool {
        <Self as Iterator>::eq(self.clone(), other.chars())
    }
}

impl Display for StringIterator<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for ch in self.clone() {
            f.write_char(ch)?;
        }

        Ok(())
    }
}

/// A wrapper around the various TOML string iterators.
#[derive(Clone, Debug)]
pub enum MultiplexedStringIterator<'str> {
    /// Simple wrapper are [`Chars`] to make it comparable.
    Simple(StringIterator<'str>),
    /// A TOML basic string.
    Basic(BasicStringIterator<'str>),
    /// A TOML multi-line basic string.
    MultiLineBasic(MultiLineBasicStringIterator<'str>),
}

impl<'str> From<BasicStringIterator<'str>> for MultiplexedStringIterator<'str> {
    fn from(value: BasicStringIterator<'str>) -> Self {
        MultiplexedStringIterator::Basic(value)
    }
}

impl<'str> From<MultiLineBasicStringIterator<'str>> for MultiplexedStringIterator<'str> {
    fn from(value: MultiLineBasicStringIterator<'str>) -> Self {
        MultiplexedStringIterator::MultiLineBasic(value)
    }
}

impl<'str> From<StringIterator<'str>> for MultiplexedStringIterator<'str> {
    fn from(value: StringIterator<'str>) -> Self {
        MultiplexedStringIterator::Simple(value)
    }
}

impl Iterator for MultiplexedStringIterator<'_> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            MultiplexedStringIterator::Basic(iter) => iter.next(),
            MultiplexedStringIterator::MultiLineBasic(iter) => iter.next(),
            MultiplexedStringIterator::Simple(iter) => iter.next(),
        }
    }
}

impl<I: Iterator<Item = char> + Clone> PartialEq<I> for MultiplexedStringIterator<'_> {
    fn eq(&self, other: &I) -> bool {
        <Self as Iterator>::eq(self.clone(), other.clone())
    }
}

impl PartialEq<str> for MultiplexedStringIterator<'_> {
    fn eq(&self, other: &str) -> bool {
        <Self as Iterator>::eq(self.clone(), other.chars())
    }
}

impl Eq for MultiplexedStringIterator<'_> {}

impl Display for MultiplexedStringIterator<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for ch in self.clone() {
            f.write_char(ch)?;
        }

        Ok(())
    }
}

/// `StringLike` is a trait representing the basic expectations for TOML strings, namely that
/// they can be compared to `str`s and have a [`Display`] implementation.
pub(super) trait StringLike: PartialEq<str> + Display {}

impl<T> StringLike for T where T: PartialEq<str> + Display {}

#[cfg(test)]
mod test {
    use super::BasicStringIterator;

    #[test]
    fn basic_string() {
        const TEST_STR: &str =
            "\"I'm a string. \\\"You can quote me\\\". Name\\tJos\\u00E9\\nLocation\\tSF.\\nEmoji:\\t\\U0001F602\"";
        const EXPECTED_STR: &str =
            "I'm a string. \"You can quote me\". Name\tJos\u{00E9}\nLocation\tSF.\nEmoji:\t\u{1F602}";

        let result = BasicStringIterator::parse_from_str(TEST_STR).unwrap();

        if !<BasicStringIterator as PartialEq<str>>::eq(&result, EXPECTED_STR) {
            panic!("Expected: {}\nActual: {}", EXPECTED_STR, result);
        }
    }
}
