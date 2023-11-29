//! A low-level TOML lexer.
//!
//! This is meant to seperate out lexing concerns from other concerns,
//! such as parsing, and thereby simplify the overall parsing of the TOML
//! file.

use core::str::Chars;

use super::{
    is_newline, is_whitespace,
    strings::{BasicStringIterator, MultiLineBasicStringIterator, StringIterator},
};

/// The character used to signal that the string has been exhausted.
pub const EOF_CHAR: char = '\0';

/// A utility struct meant to help produce a labeled input stream
/// that can somewhat more easily be parsed into a TOML.
///
/// This struct is meant to seperate the concerns of recognizing tokens
/// and parsing the TOML file.
pub struct Lexer<'str> {
    /// The untokenized string.
    untokenized_str: &'str str,
    /// An iterator containing the characters that have not been included as part of a token.
    chars: Chars<'str>,
    /// The next token.
    next: Token<'str>,
}

impl<'str> Lexer<'str> {
    /// Creates a new `Lexer` from `str`.
    pub fn new(str: &'str str) -> Lexer<'str> {
        let mut x = Lexer {
            untokenized_str: str,
            chars: str.chars(),
            next: Token::Eof,
        };

        let _ = x.next();

        x
    }

    /// Returns the remaining character as a `str`.
    fn as_str(&self) -> &'str str {
        self.chars.as_str()
    }

    /// Peeks the next symbol from the input stream without consuming it.
    ///
    /// If requested position doesn't exist, [`EOF_CHAR`] is returned.
    /// However, getting [`EOF_CHAR`] doesn't always mean actual end of file,
    /// it should be checked with [`is_eof`][e] method.
    ///
    /// [e]: Self::is_eof
    fn first(&self) -> char {
        self.chars.clone().next().unwrap_or(EOF_CHAR)
    }

    /// Peeks the second symbol from the input stream without consuming it.
    ///
    /// If requested position doesn't exist, [`EOF_CHAR`] is returned.
    /// However, getting [`EOF_CHAR`] doesn't always mean actual end of file,
    /// it should be checked with [`is_eof`][e] method.
    ///
    /// [e]: Self::is_eof
    fn second(&self) -> char {
        let mut iter = self.chars.clone();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    /// Peeks the symbol `index` characters away from the current place in the
    /// input stream without consuming it.
    ///
    /// If requested position doesn't exist, [`EOF_CHAR`] is returned.
    /// However, getting [`EOF_CHAR`] doesn't always mean actual end of file,
    /// it should be checked with [`is_eof`][e] method.
    ///
    /// [e]: Self::is_eof
    #[allow(dead_code)]
    fn nth(&self, index: usize) -> char {
        self.chars.clone().nth(index).unwrap_or(EOF_CHAR)
    }

    /// Checks if there is nothing more to consume.
    fn is_eof(&self) -> bool {
        self.chars.as_str().is_empty()
    }

    /// Returns the string representing the symbols consumed between
    /// now and the last time [`reset_pos_within_token`][r] was called.
    ///
    /// [r]: Self::reset_pos_within_token
    fn get_token_str(&self) -> &'str str {
        &self.untokenized_str[..self.pos_within_token()]
    }

    /// Returns the string representing the remaining symbols that have not
    /// been tokenized yet.
    fn get_untokenized_str(&self) -> &'str str {
        self.untokenized_str
    }

    /// Returns amount of already consumed symbols.
    fn pos_within_token(&self) -> usize {
        self.untokenized_str.len() - self.chars.as_str().len()
    }

    /// Resets the number of bytes consumed to 0.
    fn reset_pos_within_token(&mut self) {
        self.untokenized_str = self.chars.as_str();
    }

    /// Advances the iterator, returning the next character.
    fn bump(&mut self) -> Option<char> {
        self.chars.next()
    }

    /// Advances the iterator while `predicate` returns `true` and the
    /// lexer has not encountered [`EOF_CHAR`].
    ///
    /// # Panics
    /// This function may panic if and only if `predicate` can panic.
    fn eat_while<F>(&mut self, predicate: F)
    where
        F: Fn(char) -> bool,
    {
        while predicate(self.first()) && !self.is_eof() {
            self.bump();
        }
    }

    /// Advances the iterator while `predicate` returns `true` and the
    /// lexer has not encountered [`EOF_CHAR`].
    ///
    /// # Panics
    /// This function may panic if and only if `predicate` can panic.
    fn eat_while_advanced<F, E>(&mut self, mut predicate: F) -> Result<(), E>
    where
        F: FnMut(&mut Self, char) -> Result<bool, E>,
    {
        while predicate(self, self.first())? && !self.is_eof() {
            self.bump();
        }

        Ok(())
    }
}

impl<'str> Lexer<'str> {
    /// Returns the next [`Token`] in the token stream.
    #[must_use]
    pub fn next(&mut self) -> Token<'str> {
        let Some(first_char) = self.bump() else {
            return Token::Eof;
        };

        let token = match first_char {
            '.' => Token::Dot,
            '=' => Token::Equals,
            '[' => Token::LeftSquareBracket,
            ']' => Token::RightSquareBracket,
            '{' => Token::LeftCurlyBracket,
            '}' => Token::RightCurlyBracket,
            '#' => {
                self.eat_until_line_end();
                Token::Comment
            }
            ',' => Token::Comma,
            '"' => {
                if self.first() == '"' && self.second() == '"' {
                    MultiLineBasicStringIterator::parse_from_str(self.get_untokenized_str())
                        .inspect(|val| {
                            let _ = self.bump();
                            let _ = self.bump();
                            for _ in 0..val.clone().count() {
                                let _ = self.bump();
                            }
                            let _ = self.bump();
                            let _ = self.bump();
                            let _ = self.bump();
                        })
                        .map_or(Token::Error, Token::MultiLineBasicString)
                } else {
                    BasicStringIterator::parse_from_str(self.get_untokenized_str())
                        .inspect(|val| {
                            for _ in 0..val.clone().count() {
                                let _ = self.bump();
                            }
                            let _ = self.bump();
                        })
                        .map_or(Token::Error, Token::BasicString)
                }
            }
            '\'' => {
                if self.first() == '\'' && self.second() == '\'' {
                    let res = self.eat_while_advanced(|s, ch| {
                        match s.second() {
                            '\u{0}'..='\u{8}' | '\u{A}'..='\u{1F}' => return Err(()),
                            _ => {}
                        }

                        Ok(ch != '\'' && s.second() == '\'' && s.nth(2) == '\'')
                    });

                    if res.is_err() {
                        return Token::Error;
                    }

                    let token = Token::MultiLineLiteralString(StringIterator::new(
                        self.get_token_str().chars(),
                    ));

                    self.bump();

                    token
                } else {
                    self.eat_while(|ch| ch != '\'');

                    let token =
                        Token::LiteralString(StringIterator::new(self.get_token_str().chars()));

                    self.bump();

                    token
                }
            }
            c if c.is_ascii_alphanumeric() || c == '_' || c == '-' => {
                self.eat_while(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-');

                Token::BareString(StringIterator::new(self.get_token_str().chars()))
            }
            '\u{9}' | '\u{20}' => {
                self.eat_while(is_whitespace);
                Token::Whitespace
            }
            '\n' => Token::Newline,
            '\r' => match self.bump() {
                Some('\n') => Token::Newline,
                _ => Token::Error,
            },
            _ => Token::Error,
        };

        self.reset_pos_within_token();

        core::mem::replace(&mut self.next, token)
    }

    /// Returns the next [`Token`] in the token stream.
    pub fn peek(&self) -> Token<'str> {
        self.next.clone()
    }

    /// Advances the token stream if running `predicate` on the next [`Token`] in the token stream
    /// returns true.
    ///
    /// Returns the next [`Token`] otherwise.
    pub fn consume<F>(&mut self, predicate: F) -> Result<(), Token<'str>>
    where
        F: Fn(Token<'str>) -> bool,
    {
        if predicate(self.next.clone()) {
            let _ = self.next();
            Ok(())
        } else {
            Err(self.next.clone())
        }
    }

    /// Skips all TOML whitespace and resets the token position.
    ///
    /// TOML whitespace is defined as tabs (U+0009) and spaces (U+0020).
    pub fn skip_whitespace(&mut self) {
        while self.peek() == Token::Whitespace {
            let _ = self.next();
        }
    }

    /// Skips until the next non whitespace or newline character is found.
    ///
    /// TOML whitespace is defined as tabs (U+0009) and spaces (U+0020).
    /// TOML newlines are defined as either U+000A or the sequence U+000D, U+000A.
    #[allow(dead_code)]
    fn skip_whitespace_newline(&mut self) {
        while self.peek() == Token::Whitespace || self.peek() == Token::Newline {
            let _ = self.next();
        }
    }

    /// Skips until the next non-whitespace, non-newline, and non-comment
    /// character is found.
    ///
    /// TOML whitespace is defined as tabs (U+0009) and spaces (U+0020).
    /// TOML newlines are defined as either U+000A or the sequence U+000D, U+000A.
    /// A TOML comment is defined as a '#' character and anything following it
    /// on the same line.
    pub fn skip_noncontent(&mut self) {
        while self.peek() == Token::Whitespace
            || self.peek() == Token::Newline
            || self.peek() == Token::Comment
        {
            let _ = self.next();
        }
    }

    /// Skips until the new line characters.
    ///
    /// TOML newlines are defined as either U+000A or the sequence U+000D, U+000A.
    fn eat_until_line_end(&mut self) {
        let _ = self
            .eat_while_advanced::<_, core::convert::Infallible>(|s, _| Ok(!is_newline(s.as_str())));
    }

    /// If the next one or two characters make up a newline, then eat it.
    fn eat_newline(&mut self) {
        if self.first() == '\n' {
            self.bump();
        } else if self.first() == '\r' && self.second() == '\n' {
            self.bump();
            self.bump();
        }
    }
}

/// All the possible tokens produced by lexing a toml file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token<'str> {
    /// A TOML bare key.
    BareString(StringIterator<'str>),
    /// A TOML basic string.
    BasicString(BasicStringIterator<'str>),
    /// A TOML multi-line basic string.
    MultiLineBasicString(MultiLineBasicStringIterator<'str>),
    /// A TOML literal string.
    LiteralString(StringIterator<'str>),
    /// A TOML multi-line literal string.
    MultiLineLiteralString(StringIterator<'str>),

    /// A TOML comment.
    Comment,

    /// One or more consecutive TOML newlines.
    Newline,
    /// One or more consecutive TOML whitespaces.
    Whitespace,

    /// A dot.
    Dot,
    /// An equals sign.
    Equals,

    /// A comma.
    Comma,
    /// A left square bracket ([).
    LeftSquareBracket,
    /// A right square bracket (]).
    RightSquareBracket,
    /// A left curly bracket ({).
    LeftCurlyBracket,
    /// A right curly bracket (}).
    RightCurlyBracket,

    /// An error that occurred while parsing a toml token.
    Error,
    /// A token signalling the end of the toml file.
    Eof,
}
