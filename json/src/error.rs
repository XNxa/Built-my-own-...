use std::fmt::Debug;

use crate::Token;

pub enum Error {
    UnrecognizedToken(char),
    MustBeginWithBracket,
    MissingClosingBracket,
    MismatchQuote,
    TrailingComma,
    SyntaxError(Token, u32),
    ParsingError,
    InvalidNumber,
    MissingValue,
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnrecognizedToken(c) => writeln!(f, "Error: {c} is an invalid token."),
            Error::MustBeginWithBracket => {
                writeln!(f, "Error: the json object must begin with '{{'.") // {{ to escape
            }
            Error::MissingClosingBracket => {
                writeln!(f, "Error: a closing bracket '}}' is missing.")
            }
            Error::MismatchQuote => writeln!(f, "Error: a closing \" is missing."),
            Error::TrailingComma => {
                writeln!(f, "Error: the object seems to have a trailing comma.")
            }
            Error::InvalidNumber => writeln!(f, "Error: unable to parse number"),
            Error::SyntaxError(tok, l) => {
                writeln!(f, "Error: invalid syntax on token : {tok:?}. [l. {l}]")
            }
            Error::ParsingError => writeln!(f, "Error: parsing error."),
            Error::MissingValue => writeln!(f, "Error: missing value after key definition."),
        }
    }
}
