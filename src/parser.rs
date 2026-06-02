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

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum ParseError {
    #[error("unterminated quote")]
    UnterminatedQuote,
    #[error("expected command after operator")]
    ExpectedCommand,
    #[error("expected redirect target")]
    ExpectedRedirectTarget,
    #[error("unexpected token: {0}")]
    UnexpectedToken(String),
}
