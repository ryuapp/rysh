mod lexer;
mod syntax;

pub(crate) use syntax::{Command, ListItem, Pipeline, RedirectKind, parse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Word {
    pub raw: String,
}

impl Word {
    pub fn new(raw: impl Into<String>) -> Self {
        Self { raw: raw.into() }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ParseError {
    UnterminatedQuote,
    ExpectedCommand,
    ExpectedRedirectTarget,
    UnexpectedToken(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnterminatedQuote => write!(f, "unterminated quote"),
            Self::ExpectedCommand => write!(f, "expected command after operator"),
            Self::ExpectedRedirectTarget => write!(f, "expected redirect target"),
            Self::UnexpectedToken(token) => write!(f, "unexpected token: {token}"),
        }
    }
}

impl std::error::Error for ParseError {}
