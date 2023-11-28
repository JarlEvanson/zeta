//! Definition of [`ParseFromLexer`] and various implementations of it, along with
//! their error types.

use core::{error::Error, fmt::Display};

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
        let token = lexer.next();

        let string = match token {
            Token::BasicString(value) => MultiplexedStringIterator::Basic(value),
            Token::MultiLineBasicString(value) => MultiplexedStringIterator::MultiLineBasic(value),
            Token::LiteralString(value) | Token::MultiLineLiteralString(value) => {
                MultiplexedStringIterator::Simple(value)
            }
            token => {
                return Err(ParseFilterError::InvalidValueType { token });
            }
        };

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
    /// An unexpected token was encountered.
    InvalidValueType {
        /// The token that caused the parse to fail.
        token: Token<'config>,
    },
    /// The string was not a valid value.
    InvalidString(MultiplexedStringIterator<'config>),
}

impl Display for ParseFilterError<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParseFilterError::InvalidValueType { token } => {
                write!(f, "expected a string, got {token:?}")
            }
            ParseFilterError::InvalidString(str) => {
                write!(f, "expected \"off\", \"error\", \"warn\", \"info\", \"debug\", or \"trace\", got {str}")
            }
        }
    }
}

impl Error for ParseFilterError<'_> {}
