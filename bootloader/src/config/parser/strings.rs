//! Iterators over various TOML string types.

use core::{
    fmt::{Display, Write},
    str::Chars,
};

use super::{is_newline, is_whitespace, lexer::Lexer};

/// An iterator over the resolved characters of a TOML basic string.
#[derive(Clone, Debug)]
pub struct BasicStringIterator<'str>(Chars<'str>);

impl<'str> BasicStringIterator<'str> {
    /// Parses a valid TOML basic string from `lexer`.
    pub fn parse(
        lexer: &mut Lexer<'str>,
    ) -> Result<BasicStringIterator<'str>, ParseBasicStringError> {
        /// The state of the finite machine.
        enum State {
            /// Last character was not a '\\'.
            Normal,
            /// Last character was a '\\'.
            Escaped,
        }
        if lexer.first() != '"' {
            let err = ParseBasicStringError {
                kind: ParseBasicStringErrorKind::MissingOpeningQuotationMark,
                line: lexer.current_line(),
                column: lexer.current_column(),
            };
            return Err(err);
        }
        let _ = lexer.bump();

        lexer.flush_token();

        let mut state = State::Normal;

        while !lexer.is_eof() {
            let c = lexer.first();

            // Illegal control characters inside basic string.
            if ('\u{0}'..='\u{8}').contains(&c) || ('\n'..='\u{1F}').contains(&c) || c == '\u{7F}' {
                let err = ParseBasicStringError {
                    kind: ParseBasicStringErrorKind::IllegalCharacter { c },
                    line: lexer.current_line(),
                    column: lexer.current_column(),
                };
                return Err(err);
            }

            match state {
                State::Normal if c == '"' => {
                    let basic_string = lexer.token_str();

                    let _ = lexer.bump();

                    return Ok(BasicStringIterator(basic_string.chars()));
                }
                State::Normal if c == '\\' => {
                    state = State::Escaped;
                    let _ = lexer.bump();
                }
                State::Escaped if matches!(c, 'b' | 't' | 'n' | 'f' | 'r' | '"' | '\\') => {
                    let _ = lexer.bump();

                    state = State::Normal;
                }
                State::Escaped if c == 'u' => {
                    let _ = lexer.bump();

                    parse_unicode_scalar(lexer, 4)?;

                    state = State::Normal;
                }
                State::Escaped if c == 'U' => {
                    let _ = lexer.bump();

                    parse_unicode_scalar(lexer, 8)?;

                    state = State::Normal;
                }
                State::Escaped => {
                    let err = ParseBasicStringError {
                        kind: ParseBasicStringErrorKind::IllegalEscape { c },
                        line: lexer.current_line(),
                        column: lexer.current_column(),
                    };

                    return Err(err);
                }
                State::Normal => {
                    let _ = lexer.bump();
                }
            }
        }

        let err = ParseBasicStringError {
            kind: ParseBasicStringErrorKind::UnexpectedEof,
            line: lexer.current_line(),
            column: lexer.current_column(),
        };

        Err(err)
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

/// Various errors that can occur while parsing a [`BasicStringIterator`].
///
/// Includes data to help pinpoint the error's cause.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseBasicStringError {
    /// The type of the error.
    pub kind: ParseBasicStringErrorKind,
    /// The line on which it occurred.
    pub line: usize,
    /// The column in which it occurred.
    pub column: usize,
}

impl From<ParseUnicodeScalarError> for ParseBasicStringError {
    fn from(value: ParseUnicodeScalarError) -> Self {
        let kind = match value.kind {
            ParseUnicodeScalarErrorKind::IllegalCharacter { c } => {
                ParseBasicStringErrorKind::IllegalCharacter { c }
            }
            ParseUnicodeScalarErrorKind::IllegalUnicodeScalar { value } => {
                ParseBasicStringErrorKind::IllegalUnicodeScalar { value }
            }
            ParseUnicodeScalarErrorKind::UnexpectedEof => ParseBasicStringErrorKind::UnexpectedEof,
        };

        Self {
            kind,
            line: value.line,
            column: value.column,
        }
    }
}

impl Display for ParseBasicStringError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "error ocurred at {}:{}: {}",
            self.line, self.column, self.kind
        )
    }
}

/// Various types of errors that can occur while parsing a [`BasicStringIterator`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseBasicStringErrorKind {
    /// The opening quotation mark is missing.
    MissingOpeningQuotationMark,
    /// An illegal character was encountered while parsing.
    IllegalCharacter {
        /// The illegal character that was encountered.
        c: char,
    },
    /// An invalid escape sequence was found inside the parsed string.
    IllegalEscape {
        /// The character that revealed the invalid escape sequence.
        c: char,
    },
    /// The parsed `u32` scalar was not a valid unicode codepoint.
    IllegalUnicodeScalar {
        /// The `u32` scalar that was not a valid unicode codepoint.
        value: u32,
    },
    /// The lexer reached `eof` unexpectedly.
    UnexpectedEof,
}

impl Display for ParseBasicStringErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParseBasicStringErrorKind::MissingOpeningQuotationMark => {
                f.write_str("missing opening quotation mark")
            }
            ParseBasicStringErrorKind::IllegalCharacter { c } => {
                write!(f, "illegal character {c:?}")
            }
            ParseBasicStringErrorKind::IllegalEscape { c } => write!(f, "illegal escape \'{c}\'"),
            ParseBasicStringErrorKind::IllegalUnicodeScalar { value } => {
                if *value > 0xFFFF {
                    write!(f, "{value:08X} is not a valid unicode scalar value")
                } else {
                    write!(f, "{value:04X} is not a valid unicode scalar value")
                }
            }
            ParseBasicStringErrorKind::UnexpectedEof => write!(f, "lexer unexpectedly reached EOF"),
        }
    }
}

/// An iterator over the resolved characters of a TOML multi-line basic string.
#[derive(Clone, Debug)]
pub struct MultiLineBasicStringIterator<'str>(Chars<'str>);

impl<'str> MultiLineBasicStringIterator<'str> {
    /// Parses a valid TOML multi-line basic string from `lexer`.
    pub fn parse(
        lexer: &mut Lexer<'str>,
    ) -> Result<MultiLineBasicStringIterator<'str>, ParseMultiLineBasicStringError> {
        /// The state of the finite machine.
        enum State {
            /// Last character was not a '\\'.
            Normal,
            /// Last character was a '\\'.
            Escaped,
        }

        for _ in 0..3 {
            if lexer.first() != '"' {
                let err = ParseMultiLineBasicStringError {
                    kind: ParseMultiLineBasicStringErrorKind::MissingOpeningQuotationMark,
                    line: lexer.current_line(),
                    column: lexer.current_column(),
                };
                return Err(err);
            }
            let _ = lexer.bump();
        }

        lexer.flush_token();

        let mut state = State::Normal;

        while !lexer.is_eof() {
            let c = lexer.first();

            // Illegal control characters inside multi-line basic string.
            if ('\u{0}'..='\u{8}').contains(&c)
                || c == '\u{B}'
                || c == '\u{C}'
                || ('\u{E}'..='\u{1F}').contains(&c)
                || c == '\u{7F}'
            {
                let err = ParseMultiLineBasicStringError {
                    kind: ParseMultiLineBasicStringErrorKind::IllegalCharacter { c },
                    line: lexer.current_line(),
                    column: lexer.current_column(),
                };
                return Err(err);
            }

            match state {
                State::Normal if c == '"' && lexer.second() == '"' && lexer.nth(2) == '"' => {
                    let multi_line = lexer.token_str();

                    let _ = lexer.bump();
                    let _ = lexer.bump();
                    let _ = lexer.bump();

                    return Ok(MultiLineBasicStringIterator(multi_line.chars()));
                }
                State::Normal if c == '\\' => {
                    state = State::Escaped;
                    let _ = lexer.bump();
                }
                State::Escaped if matches!(c, 'b' | 't' | 'n' | 'f' | 'r' | '"' | '\\') => {
                    let _ = lexer.bump();

                    state = State::Normal;
                }
                State::Escaped if c == 'u' => {
                    let _ = lexer.bump();

                    parse_unicode_scalar(lexer, 4)?;

                    state = State::Normal;
                }
                State::Escaped if c == 'U' => {
                    let _ = lexer.bump();

                    parse_unicode_scalar(lexer, 8)?;

                    state = State::Normal;
                }
                State::Escaped if is_whitespace(c) || is_newline(lexer.untokenized_str()) => {
                    let _ = lexer.eat_while_advanced::<_, core::convert::Infallible>(|lexer, k| {
                        Ok(is_whitespace(k) || is_newline(lexer.untokenized_str()))
                    });

                    state = State::Normal;
                }
                State::Escaped => {
                    let err = ParseMultiLineBasicStringError {
                        kind: ParseMultiLineBasicStringErrorKind::IllegalEscape { c },
                        line: lexer.current_line(),
                        column: lexer.current_column(),
                    };

                    return Err(err);
                }
                State::Normal => {
                    let _ = lexer.bump();
                }
            }
        }
        let err = ParseMultiLineBasicStringError {
            kind: ParseMultiLineBasicStringErrorKind::UnexpectedEof,
            line: lexer.current_line(),
            column: lexer.current_column(),
        };

        Err(err)
    }
}

impl Iterator for MultiLineBasicStringIterator<'_> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let current_char = self.0.next()?;

            if current_char != '\\' {
                return Some(current_char);
            }

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
                    return char::from_u32(value);
                }
                // U+XXXXXXXX
                Some('U') => {
                    // PANIC SAFETY:
                    // The operation below was previously tested when the underlying string was parsed.
                    // Therefore, this operation should never panic.
                    let descriptor = self.0.as_str().get(0..8).unwrap();

                    for _ in 0..8 {
                        let _ = self.0.next();
                    }

                    // PANIC SAFETY:
                    // The operation below was previously tested when the underlying string was parsed.
                    // Therefore, this operation should never panic.
                    let value = u32::from_str_radix(descriptor, 16).unwrap();

                    // PANIC SAFETY:
                    // The operation below was previously tested when the underlying string was parsed.
                    // Therefore, this operation should never panic.
                    return char::from_u32(value);
                }
                Some(w) if is_whitespace(w) || is_newline(self.0.as_str()) => {
                    let mut peeker = self.0.clone();

                    while let Some(peeked) = peeker.next() {
                        if is_whitespace(peeked) || is_newline(peeker.as_str()) {
                            self.0.next();
                        } else {
                            break;
                        }
                    }
                }
                Some(_) | None => {
                    // PANIC SAFETY:
                    // The operation below was previously tested when the underlying string was parsed.
                    // Therefore, this operation should never panic.
                    unreachable!("a valid multi-line basic string must not include a broken escape")
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

/// Various errors that can occur while parsing a [`MultiLineBasicStringIterator`].
///
/// Includes data to help pinpoint the error's cause.
pub struct ParseMultiLineBasicStringError {
    /// The type of the error.
    pub kind: ParseMultiLineBasicStringErrorKind,
    /// The line on which it occurred.
    pub line: usize,
    /// The column in which it occurred.
    pub column: usize,
}

impl From<ParseUnicodeScalarError> for ParseMultiLineBasicStringError {
    fn from(value: ParseUnicodeScalarError) -> Self {
        let kind = match value.kind {
            ParseUnicodeScalarErrorKind::IllegalCharacter { c } => {
                ParseMultiLineBasicStringErrorKind::IllegalCharacter { c }
            }
            ParseUnicodeScalarErrorKind::IllegalUnicodeScalar { value } => {
                ParseMultiLineBasicStringErrorKind::IllegalUnicodeScalar { value }
            }
            ParseUnicodeScalarErrorKind::UnexpectedEof => {
                ParseMultiLineBasicStringErrorKind::UnexpectedEof
            }
        };

        Self {
            kind,
            line: value.line,
            column: value.column,
        }
    }
}

impl Display for ParseMultiLineBasicStringError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "error ocurred at {}:{}: {}",
            self.line, self.column, self.kind
        )
    }
}

/// Various types of errors that can occur while parsing a [`MultiLineBasicStringIterator`].
pub enum ParseMultiLineBasicStringErrorKind {
    /// At least on of the three opening quotation marks is missing.
    MissingOpeningQuotationMark,
    /// An illegal character was encountered while parsing.
    IllegalCharacter {
        /// The illegal character that was encountered.
        c: char,
    },
    /// An invalid escape sequence was found inside the parsed string.
    IllegalEscape {
        /// The character that revealed the invalid escape sequence.
        c: char,
    },
    /// The parsed `u32` scalar was not a valid unicode codepoint.
    IllegalUnicodeScalar {
        /// The `u32` scalar that was not a valid unicode codepoint.
        value: u32,
    },
    /// The lexer reached `eof` unexpectedly.
    UnexpectedEof,
}

impl Display for ParseMultiLineBasicStringErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParseMultiLineBasicStringErrorKind::MissingOpeningQuotationMark => {
                f.write_str("missing opening quotation mark")
            }
            ParseMultiLineBasicStringErrorKind::IllegalCharacter { c } => {
                write!(f, "illegal character {c:?}")
            }
            ParseMultiLineBasicStringErrorKind::IllegalEscape { c } => {
                write!(f, "illegal escape \'{c}\'")
            }
            ParseMultiLineBasicStringErrorKind::IllegalUnicodeScalar { value } => {
                if *value > 0xFFFF {
                    write!(f, "{value:08X} is not a valid unicode scalar value")
                } else {
                    write!(f, "{value:04X} is not a valid unicode scalar value")
                }
            }
            ParseMultiLineBasicStringErrorKind::UnexpectedEof => {
                write!(f, "lexer unexpectedly reached EOF")
            }
        }
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

/// Parses a unicode scalar of `count` hex digits.
fn parse_unicode_scalar(
    lexer: &mut Lexer,
    hex_digit_count: usize,
) -> Result<char, ParseUnicodeScalarError> {
    let remaining = lexer.untokenized_str();
    let mut count = 0;

    while !lexer.is_eof() {
        if !lexer.first().is_ascii_hexdigit() {
            let err = ParseUnicodeScalarError {
                kind: ParseUnicodeScalarErrorKind::IllegalCharacter { c: lexer.first() },
                line: lexer.current_line(),
                column: lexer.current_column(),
            };
            // Illegal control characters inside basic string.
            return Err(err);
        }

        let _ = lexer.bump();
        count += 1;

        if count == hex_digit_count {
            let value = u32::from_str_radix(&remaining[0..hex_digit_count], 16).unwrap();

            return char::from_u32(value).ok_or(ParseUnicodeScalarError {
                kind: ParseUnicodeScalarErrorKind::IllegalUnicodeScalar { value },
                line: lexer.current_line(),
                column: lexer.current_column() - hex_digit_count,
            });
        }
    }

    let err = ParseUnicodeScalarError {
        kind: ParseUnicodeScalarErrorKind::UnexpectedEof,
        line: lexer.current_line(),
        column: lexer.current_column(),
    };

    Err(err)
}

/// Various errors that can occur while parsing a unicode scalar value.
///
/// Includes data to help pinpoint the error's cause.
pub struct ParseUnicodeScalarError {
    /// The type of the error.
    kind: ParseUnicodeScalarErrorKind,
    /// The line on which it occurred.
    line: usize,
    /// The column in which it occurred.
    column: usize,
}

/// Various error types that can occur when parsing a unicode scalar.
enum ParseUnicodeScalarErrorKind {
    /// An non-hex digit was encountered while parsing a unicode scalar.
    IllegalCharacter {
        /// The illegal character that was encountered.
        c: char,
    },
    /// The parsed `u32` scalar was not a valid unicode codepoint.
    IllegalUnicodeScalar {
        /// The `u32` scalar that was not a valid unicode codepoint.
        value: u32,
    },
    /// The lexer reached `eof` unexpectedly.
    UnexpectedEof,
}

#[cfg(test)]
mod test {
    use super::{BasicStringIterator, MultiLineBasicStringIterator};

    use crate::config::parser::Lexer;

    #[test]
    fn basic_string() {
        const TEST_STR: &str =
            "\"I'm a string. \\\"You can quote me\\\". Name\\tJos\\u00E9\\nLocation\\tSF.\\nEmoji:\\t\\U0001F602\"";
        const EXPECTED_STR: &str =
            "I'm a string. \"You can quote me\". Name\tJos\u{00E9}\nLocation\tSF.\nEmoji:\t\u{1F602}";

        let mut lexer = Lexer::new_testing(TEST_STR);

        let result = match BasicStringIterator::parse(&mut lexer) {
            Ok(result) => result,
            Err(err) => {
                panic!("{}\nRemaining: {:?}", err, lexer.untokenized_str());
            }
        };

        if !<BasicStringIterator as PartialEq<str>>::eq(&result, EXPECTED_STR) {
            panic!("Expected: {}\nActual: {}", EXPECTED_STR, result,);
        }
    }

    #[test]
    fn multi_line_basic_string() {
        const TEST_STR: &str =
            "\"\"\"I'm a string. \"You can quote me\". Name\\tJos\\u00E9\\nLocation\\tSF.\\nEmoji:\\t\\U0001F602\nNew Line: \"\"Double Quotes\"\\\" \\ Whitespace deletion\"\"\"";
        const EXPECTED_STR: &str =
            "I'm a string. \"You can quote me\". Name\tJos\u{00E9}\nLocation\tSF.\nEmoji:\t\u{1F602}\nNew Line: \"\"Double Quotes\"\" Whitespace deletion";

        let mut lexer = Lexer::new_testing(TEST_STR);

        let result = match MultiLineBasicStringIterator::parse(&mut lexer) {
            Ok(result) => result,
            Err(err) => {
                panic!("{}\nRemaining: {:?}", err, lexer.untokenized_str());
            }
        };

        if !<MultiLineBasicStringIterator as PartialEq<str>>::eq(&result, EXPECTED_STR) {
            panic!("Expected: {}\nActual: {}", EXPECTED_STR, result);
        }
    }
}
