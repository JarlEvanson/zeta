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
    unflushed_str: &'str str,
    /// An iterator whose difference between `untokenized_str` represents the current state of a token.
    token_chars: Chars<'str>,
    /// The current line.
    current_line: usize,
    /// The current number of codepoints since the last newline.
    current_column: usize,
    /// The next token.
    next: Token<'str>,
}

impl<'str> Lexer<'str> {
    /// Creates a new `Lexer` from `str`.
    pub fn new(str: &'str str) -> Lexer<'str> {
        let mut x = Lexer {
            unflushed_str: str,
            token_chars: str.chars(),
            current_line: 1,
            current_column: 0,
            next: Token {
                kind: TokenKind::Eof,
                line: 0,
                column: 0,
            },
        };

        let _ = x.next();

        x
    }

    /// Does not call `self.next()` for ease of testing.
    ///
    /// Calling [`peek()`] will return erroneous results.
    #[cfg(test)]
    pub(super) fn new_testing(str: &'str str) -> Lexer<'str> {
        Lexer {
            unflushed_str: str,
            token_chars: str.chars(),
            current_line: 1,
            current_column: 0,
            next: Token {
                kind: TokenKind::Eof,
                line: 0,
                column: 0,
            },
        }
    }

    /// Peeks the next symbol from the input stream without consuming it.
    ///
    /// If requested position doesn't exist, [`EOF_CHAR`] is returned.
    /// However, getting [`EOF_CHAR`] doesn't always mean actual end of file,
    /// it should be checked with [`is_eof`][e] method.
    ///
    /// [e]: Self::is_eof
    pub(super) fn first(&self) -> char {
        self.token_chars.clone().next().unwrap_or(EOF_CHAR)
    }

    /// Peeks the second symbol from the input stream without consuming it.
    ///
    /// If requested position doesn't exist, [`EOF_CHAR`] is returned.
    /// However, getting [`EOF_CHAR`] doesn't always mean actual end of file,
    /// it should be checked with [`is_eof`][e] method.
    ///
    /// [e]: Self::is_eof
    pub(super) fn second(&self) -> char {
        let mut iter = self.token_chars.clone();
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
    pub(super) fn nth(&self, index: usize) -> char {
        self.token_chars.clone().nth(index).unwrap_or(EOF_CHAR)
    }

    /// Checks if there is nothing more to consume.
    pub(super) fn is_eof(&self) -> bool {
        self.token_chars.as_str().is_empty()
    }

    /// Returns the remaining untokenized characters as a `str`.
    pub(super) fn untokenized_str(&self) -> &'str str {
        self.token_chars.as_str()
    }

    /// Returns the string representing the symbols consumed between
    /// now and the last time [`flush_token()`][r] was called.
    ///
    /// [r]: Self::flush_token
    pub(super) fn token_str(&self) -> &'str str {
        &self.unflushed_str[..self.token_char_count()]
    }

    /// Returns the amount of characters consumed since the last flush.
    pub(super) fn token_char_count(&self) -> usize {
        self.unflushed_str.len() - self.token_chars.as_str().len()
    }

    /// Resets the number of bytes consumed to 0.
    pub(super) fn flush_token(&mut self) {
        self.unflushed_str = self.token_chars.as_str();
    }

    /// Resets the progress of the tokenizer to the last flush point.
    ///
    /// Must not be used between lines.
    pub(super) fn reset_token(&mut self) {
        self.current_column -= self.token_char_count();
        self.token_chars = self.unflushed_str.chars();
    }

    /// The current number of lines we have parsed.
    pub(super) fn current_line(&self) -> usize {
        self.current_line
    }

    /// The number of characters since the last newline.
    pub(super) fn current_column(&self) -> usize {
        self.current_column
    }

    /// Advances the iterator, returning the next character.
    pub(super) fn bump(&mut self) -> Option<char> {
        self.token_chars.next().inspect(|&c| {
            if c == '\u{A}'
                || c == '\u{B}'
                || c == '\u{C}'
                || c == '\u{D}'
                || c == '\u{85}'
                || c == '\u{2028}'
                || c == '\u{2029}'
            {
                self.current_line += 1;
                self.current_column = 0;
            } else {
                self.current_column += 1;
            }
        })
    }

    /// Advances the iterator while `predicate` returns `true` and the
    /// lexer has not encountered [`EOF_CHAR`].
    ///
    /// # Panics
    /// This function may panic if and only if `predicate` can panic.
    pub(super) fn eat_while<F>(&mut self, predicate: F)
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
    pub(super) fn eat_while_advanced<F, E>(&mut self, mut predicate: F) -> Result<(), E>
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
        let start_column = self.current_column();
        let start_line = self.current_line();

        let Some(first_char) = self.bump() else {
            return Token {
                kind: TokenKind::Eof,
                line: start_line,
                column: start_column,
            };
        };

        let token_kind = 'kind: {
            match first_char {
                '.' => TokenKind::Dot,
                '=' => TokenKind::Equals,
                '[' => TokenKind::LeftSquareBracket,
                ']' => TokenKind::RightSquareBracket,
                '{' => TokenKind::LeftCurlyBracket,
                '}' => TokenKind::RightCurlyBracket,
                '#' => {
                    self.eat_until_line_end();
                    TokenKind::Comment
                }
                ',' => TokenKind::Comma,
                '"' => {
                    self.reset_token();
                    if self.nth(1) == '"' && self.nth(2) == '"' {
                        MultiLineBasicStringIterator::parse(self)
                            .map_or(TokenKind::LexingError, TokenKind::MultiLineBasicString)
                    } else {
                        BasicStringIterator::parse(self)
                            .map_or(TokenKind::LexingError, TokenKind::BasicString)
                    }
                }
                '\'' => {
                    self.flush_token();
                    if self.first() == '\'' && self.second() == '\'' {
                        let res = self.eat_while_advanced(|s, ch| {
                            match s.second() {
                                '\u{0}'..='\u{8}' | '\u{A}'..='\u{1F}' => return Err(()),
                                _ => {}
                            }

                            Ok(ch != '\'' && s.second() == '\'' && s.nth(2) == '\'')
                        });

                        if res.is_err() {
                            break 'kind TokenKind::LexingError;
                        }

                        let token = TokenKind::MultiLineLiteralString(StringIterator::new(
                            self.token_str().chars(),
                        ));

                        self.bump();

                        token
                    } else {
                        let res = self.eat_while_advanced(|s, ch| {
                            match s.second() {
                                '\u{0}'..='\u{8}' | '\u{A}'..='\u{1F}' => return Err(()),
                                _ => {}
                            }

                            Ok(ch != '\'')
                        });

                        if res.is_err() {
                            break 'kind TokenKind::LexingError;
                        }

                        let token =
                            TokenKind::LiteralString(StringIterator::new(self.token_str().chars()));

                        self.bump();

                        token
                    }
                }
                c if c.is_ascii_alphanumeric() || c == '_' || c == '-' => {
                    self.eat_while(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-');

                    TokenKind::BareString(StringIterator::new(self.token_str().chars()))
                }
                '\u{9}' | '\u{20}' => {
                    self.eat_while(is_whitespace);
                    TokenKind::Whitespace
                }
                '\n' => TokenKind::Newline,
                '\r' => match self.bump() {
                    Some('\n') => TokenKind::Newline,
                    _ => TokenKind::LexingError,
                },
                _ => TokenKind::LexingError,
            }
        };

        self.flush_token();

        let token = Token {
            kind: token_kind,
            line: start_line,
            column: start_column,
        };

        core::mem::replace(&mut self.next, token)
    }

    /// Returns the next [`Token`] in the token stream.
    pub fn peek(&self) -> Token<'str> {
        self.next.clone()
    }

    /// Advances the token stream if the next [`Token`] in the token stream
    /// is equals to `token`.
    ///
    /// Returns the peeked next [`Token`] otherwise.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "`token` is generally used for trivially creatable items"
    )]
    pub fn consume(&mut self, token_kind: TokenKind) -> Result<(), Token<'str>> {
        let peek = self.peek();
        if peek.kind == token_kind {
            let _ = self.next();
            Ok(())
        } else {
            Err(peek)
        }
    }

    /// Skips all TOML whitespace and resets the token position.
    ///
    /// TOML whitespace is defined as tabs (U+0009) and spaces (U+0020).
    pub fn skip_whitespace(&mut self) {
        while self.peek().kind == TokenKind::Whitespace {
            let _ = self.next();
        }
    }

    /// Skips until the next non whitespace or newline character is found.
    ///
    /// TOML whitespace is defined as tabs (U+0009) and spaces (U+0020).
    /// TOML newlines are defined as either U+000A or the sequence U+000D, U+000A.
    #[allow(dead_code)]
    fn skip_whitespace_newline(&mut self) {
        while self.peek().kind == TokenKind::Whitespace || self.peek().kind == TokenKind::Newline {
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
        while self.peek().kind == TokenKind::Whitespace
            || self.peek().kind == TokenKind::Newline
            || self.peek().kind == TokenKind::Comment
        {
            let _ = self.next();
        }
    }

    /// Skips until the new line characters.
    ///
    /// TOML newlines are defined as either U+000A or the sequence U+000D, U+000A.
    fn eat_until_line_end(&mut self) {
        let _ = self.eat_while_advanced::<_, core::convert::Infallible>(|s, _| {
            Ok(!is_newline(s.untokenized_str()))
        });
    }
}

/// A basic unit in TOML.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token<'str> {
    /// The kind of the [`Token`].
    pub kind: TokenKind<'str>,
    /// The line it starts on.
    pub line: usize,
    /// The column it starts on.
    pub column: usize,
}

/// All possible token types produced by lexing a toml file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind<'str> {
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
    LexingError,
    /// A token signalling the end of the toml file.
    Eof,
}
