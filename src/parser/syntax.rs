use super::lexer::{Token, lex};
use super::{ParseError, Word};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct List {
    pub items: Vec<(ListItem, Pipeline)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListItem {
    Always,
    AndIf,
    OrIf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pipeline {
    pub commands: Vec<Command>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub assignments: Vec<Assignment>,
    pub args: Vec<Word>,
    pub redirects: Vec<Redirect>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    pub name: String,
    pub value: Word,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redirect {
    pub kind: RedirectKind,
    pub target: Option<Word>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedirectKind {
    Stdin,
    StdoutTruncate,
    StdoutAppend,
    StderrTruncate,
    StderrAppend,
    StderrToStdout,
}

pub fn parse(input: &str) -> Result<List, ParseError> {
    let tokens = lex(input)?;
    let mut parser = Parser { tokens, pos: 0 };
    parser.parse_list()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn parse_list(&mut self) -> Result<List, ParseError> {
        let mut items = Vec::new();
        let mut next_op = ListItem::Always;

        while self.pos < self.tokens.len() {
            if matches!(self.peek(), Some(Token::Semi)) {
                self.pos += 1;
                next_op = ListItem::Always;
                continue;
            }

            let pipeline = self.parse_pipeline()?;
            items.push((next_op, pipeline));

            next_op = match self.peek() {
                Some(Token::Semi) => {
                    self.pos += 1;
                    ListItem::Always
                }
                Some(Token::AndIf) => {
                    self.pos += 1;
                    ListItem::AndIf
                }
                Some(Token::OrIf) => {
                    self.pos += 1;
                    ListItem::OrIf
                }
                None => break,
                Some(token) => return Err(ParseError::UnexpectedToken(format!("{token:?}"))),
            };
        }

        Ok(List { items })
    }

    fn parse_pipeline(&mut self) -> Result<Pipeline, ParseError> {
        let mut commands = vec![self.parse_command()?];

        while matches!(self.peek(), Some(Token::Pipe)) {
            self.pos += 1;
            commands.push(self.parse_command()?);
        }

        Ok(Pipeline { commands })
    }

    fn parse_command(&mut self) -> Result<Command, ParseError> {
        let mut command = Command {
            assignments: Vec::new(),
            args: Vec::new(),
            redirects: Vec::new(),
        };

        loop {
            match self.peek() {
                Some(Token::Word(word)) => {
                    let word = word.clone();
                    self.pos += 1;
                    if command.args.is_empty()
                        && let Some((name, value)) = parse_assignment(&word)
                    {
                        command.assignments.push(Assignment {
                            name: name.to_string(),
                            value: Word::new(value),
                        });
                    } else {
                        command.args.push(Word::new(word));
                    }
                }
                Some(token) if redirect_kind(token).is_some() => {
                    let kind = redirect_kind(token).unwrap();
                    self.pos += 1;
                    let target = if kind == RedirectKind::StderrToStdout {
                        None
                    } else {
                        Some(self.take_word()?)
                    };
                    command.redirects.push(Redirect { kind, target });
                }
                _ => break,
            }
        }

        if command.args.is_empty() && command.assignments.is_empty() && command.redirects.is_empty()
        {
            return Err(ParseError::ExpectedCommand);
        }

        Ok(command)
    }

    fn take_word(&mut self) -> Result<Word, ParseError> {
        match self.peek() {
            Some(Token::Word(word)) => {
                let word = Word::new(word.clone());
                self.pos += 1;
                Ok(word)
            }
            _ => Err(ParseError::ExpectedRedirectTarget),
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }
}

fn redirect_kind(token: &Token) -> Option<RedirectKind> {
    Some(match token {
        Token::RedirectIn => RedirectKind::Stdin,
        Token::RedirectOut => RedirectKind::StdoutTruncate,
        Token::RedirectAppend => RedirectKind::StdoutAppend,
        Token::RedirectErrOut => RedirectKind::StderrTruncate,
        Token::RedirectErrAppend => RedirectKind::StderrAppend,
        Token::RedirectErrToOut => RedirectKind::StderrToStdout,
        _ => return None,
    })
}

fn parse_assignment(word: &str) -> Option<(&str, &str)> {
    let (name, value) = word.split_once('=')?;
    if name.is_empty() {
        return None;
    }
    let mut chars = name.chars();
    let first = chars.next()?;
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return None;
    }
    if chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric()) {
        Some((name, value))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pipeline_and_and_or() {
        let list = parse("FOO=bar echo $FOO | cat && echo ok").unwrap();
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[0].0, ListItem::Always);
        assert_eq!(list.items[0].1.commands.len(), 2);
        assert_eq!(list.items[1].0, ListItem::AndIf);
    }
}
