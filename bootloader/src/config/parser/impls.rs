//! Definition of [`ParseFromLexer`] and various implementations of it, along with
//! their error types.

use core::{error::Error, fmt::Display};

use digest::sha512::Digest;
use log::LevelFilter;

use super::{
    lexer::{Lexer, Token},
    strings::{MultiplexedStringIterator, StringIterator},
};

/// `ParseFromLexer` is a trait representing the ability to
/// parse `Self` from a lexer.
pub(super) trait ParseFromLexer<'config>: Sized {
    /// Returned when any errors occur during lexing.
    type Error: Error;

    /// Parses an instance of `Self` from the current state of `lexer`.
    fn parse(lexer: &mut Lexer<'config>) -> Result<Self, Self::Error>;
}

impl<'config> ParseFromLexer<'config> for bool {
    type Error = ParseBoolError<'config>;

    fn parse(lexer: &mut Lexer<'config>) -> Result<Self, Self::Error> {
        match lexer.next() {
            Token::BareString(value) if <StringIterator as PartialEq<str>>::eq(&value, "true") => {
                Ok(true)
            }
            Token::BareString(value) if <StringIterator as PartialEq<str>>::eq(&value, "false") => {
                Ok(false)
            }
            token => Err(ParseBoolError { token }),
        }
    }
}

/// The error returned when a [`bool`] fails to parse from the lexer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ParseBoolError<'config> {
    /// The token that caused the parse to fail.
    token: Token<'config>,
}

impl Display for ParseBoolError<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "expected `true` or `false`, got {:?}", self.token)
    }
}

impl Error for ParseBoolError<'_> {}

impl<'config> ParseFromLexer<'config> for LevelFilter {
    type Error = ParseFilterError<'config>;

    fn parse(
        lexer: &mut Lexer<'config>,
    ) -> Result<Self, <LevelFilter as ParseFromLexer<'config>>::Error> {
        let string = MultiplexedStringIterator::parse(lexer)?;

        let comparator: &dyn PartialEq<str> = &string;

        if comparator.eq("off") {
            Ok(LevelFilter::Off)
        } else if comparator.eq("error") {
            Ok(LevelFilter::Error)
        } else if comparator.eq("warn") {
            Ok(LevelFilter::Warn)
        } else if comparator.eq("info") {
            Ok(LevelFilter::Info)
        } else if comparator.eq("debug") {
            Ok(LevelFilter::Debug)
        } else if comparator.eq("trace") {
            Ok(LevelFilter::Trace)
        } else {
            Err(ParseFilterError::InvalidString(string))
        }
    }
}

/// The error returned when a [`LevelFilter`] fails to parse from the lexer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ParseFilterError<'config> {
    /// An error occurred while parsing a [`MultiplexedStringIterator`].
    StringError(ParseMultiplexedStringError<'config>),
    /// The string was not a valid value.
    InvalidString(MultiplexedStringIterator<'config>),
}

impl<'config> From<ParseMultiplexedStringError<'config>> for ParseFilterError<'config> {
    fn from(value: ParseMultiplexedStringError<'config>) -> Self {
        Self::StringError(value)
    }
}

impl Display for ParseFilterError<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParseFilterError::StringError(err) => {
                write!(f, "an error occurred parsing the string: {err}")
            }
            ParseFilterError::InvalidString(str) => {
                write!(f, "expected \"off\", \"error\", \"warn\", \"info\", \"debug\", or \"trace\", got {str:?}")
            }
        }
    }
}

impl Error for ParseFilterError<'_> {}

impl<'config> ParseFromLexer<'config> for MultiplexedStringIterator<'config> {
    type Error = ParseMultiplexedStringError<'config>;

    fn parse(lexer: &mut Lexer<'config>) -> Result<Self, Self::Error> {
        let token = lexer.next();

        let string = match token {
            Token::BasicString(value) => MultiplexedStringIterator::Basic(value),
            Token::MultiLineBasicString(value) => MultiplexedStringIterator::MultiLineBasic(value),
            Token::LiteralString(value) | Token::MultiLineLiteralString(value) => {
                MultiplexedStringIterator::Simple(value)
            }
            token => {
                return Err(ParseMultiplexedStringError::InvalidValueType { token });
            }
        };

        Ok(string)
    }
}

/// The error returned when a [`MultiplexedStringIterator`] fails to parse from the lexer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ParseMultiplexedStringError<'config> {
    /// An unexpected token was encountered.
    InvalidValueType {
        /// The token that caused the parse to fail.
        token: Token<'config>,
    },
}

impl Display for ParseMultiplexedStringError<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParseMultiplexedStringError::InvalidValueType { token } => {
                write!(f, "expected a string, got {token:?}")
            }
        }
    }
}

impl Error for ParseMultiplexedStringError<'_> {}

impl<'config> ParseFromLexer<'config> for Digest {
    type Error = ParseDigestError<'config>;

    fn parse(lexer: &mut Lexer<'config>) -> Result<Self, Self::Error> {
        let hex_string = MultiplexedStringIterator::parse(lexer)?;

        let digest = Digest::from_chars(hex_string.clone())
            .ok_or(ParseDigestError::InvalidDigestFormat(hex_string))?;

        Ok(digest)
    }
}

/// The error returned when a [`Digest`] fails to parse from the lexer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ParseDigestError<'config> {
    /// An error occurred while parsing a [`MultiplexedStringIterator`].
    StringError(ParseMultiplexedStringError<'config>),
    /// The string was not a valid hex encoding of a SHA-512 hash.
    InvalidDigestFormat(MultiplexedStringIterator<'config>),
}

impl<'config> From<ParseMultiplexedStringError<'config>> for ParseDigestError<'config> {
    fn from(value: ParseMultiplexedStringError<'config>) -> Self {
        Self::StringError(value)
    }
}

impl Display for ParseDigestError<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParseDigestError::StringError(err) => {
                write!(f, "an error occurred parsing the string: {err}")
            }
            ParseDigestError::InvalidDigestFormat(str) => {
                write!(f, "expected a hex string, got {str:?}")
            }
        }
    }
}

impl Error for ParseDigestError<'_> {}
