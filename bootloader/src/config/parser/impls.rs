use core::{error::Error, fmt::Display};

use log::LevelFilter;

use super::{
    lexer::{Lexer, Token},
    strings::{MultiplexedStringIterator, StringIterator},
};

pub(super) trait ParseFromLexer<'config>: Sized {
    type Error: Error;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ParseBoolError<'config> {
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
            Token::LiteralString(value) => MultiplexedStringIterator::Simple(value),
            Token::MultiLineLiteralString(value) => MultiplexedStringIterator::Simple(value),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ParseFilterError<'config> {
    InvalidValueType { token: Token<'config> },
    InvalidString(MultiplexedStringIterator<'config>),
}

impl Display for ParseFilterError<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParseFilterError::InvalidValueType { token } => {
                write!(f, "expected a string, got {:?}", token)
            }
            ParseFilterError::InvalidString(str) => {
                write!(f, "expected \"off\", \"error\", \"warn\", \"info\", \"debug\", or \"trace\", got {}", str)
            }
        }
    }
}

impl Error for ParseFilterError<'_> {}
