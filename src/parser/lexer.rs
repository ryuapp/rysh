use super::ParseError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Word(String),
    Semi,
    AndIf,
    OrIf,
    Pipe,
    RedirectIn,
    RedirectOut,
    RedirectAppend,
    RedirectErrOut,
    RedirectErrAppend,
    RedirectErrToOut,
}

pub fn lex(input: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let mut chars = input.char_indices().peekable();

    while let Some((idx, ch)) = chars.peek().copied() {
        if ch == '#' && tokens.last().is_none_or(is_command_boundary) {
            break;
        }
        if ch == '\r' {
            chars.next();
            continue;
        }
        if ch == '\n' {
            tokens.push(Token::Semi);
            chars.next();
            continue;
        }
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        let rest = &input[idx..];
        if rest.starts_with("2>&1") {
            tokens.push(Token::RedirectErrToOut);
            for _ in 0..4 {
                chars.next();
            }
            continue;
        }
        if rest.starts_with("2>>") {
            tokens.push(Token::RedirectErrAppend);
            for _ in 0..3 {
                chars.next();
            }
            continue;
        }
        if rest.starts_with("2>") {
            tokens.push(Token::RedirectErrOut);
            for _ in 0..2 {
                chars.next();
            }
            continue;
        }
        if rest.starts_with("&&") {
            tokens.push(Token::AndIf);
            chars.next();
            chars.next();
            continue;
        }
        if rest.starts_with("||") {
            tokens.push(Token::OrIf);
            chars.next();
            chars.next();
            continue;
        }
        if rest.starts_with(">>") {
            tokens.push(Token::RedirectAppend);
            chars.next();
            chars.next();
            continue;
        }

        match ch {
            ';' => {
                tokens.push(Token::Semi);
                chars.next();
            }
            '|' => {
                tokens.push(Token::Pipe);
                chars.next();
            }
            '<' => {
                tokens.push(Token::RedirectIn);
                chars.next();
            }
            '>' => {
                tokens.push(Token::RedirectOut);
                chars.next();
            }
            _ => tokens.push(Token::Word(read_word(input, &mut chars)?)),
        }
    }

    Ok(tokens)
}

fn is_command_boundary(token: &Token) -> bool {
    matches!(
        token,
        Token::Semi | Token::AndIf | Token::OrIf | Token::Pipe
    )
}

fn read_word(
    input: &str,
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Result<String, ParseError> {
    let start = chars.peek().map(|(idx, _)| *idx).unwrap_or(input.len());
    let mut end = start;
    let mut single = false;
    let mut double = false;
    let mut escaped = false;
    let mut command_sub_depth = 0usize;

    while let Some((idx, ch)) = chars.peek().copied() {
        if escaped {
            escaped = false;
            end = idx + ch.len_utf8();
            chars.next();
            continue;
        }

        if command_sub_depth > 0 {
            match ch {
                '\\' if !single => {
                    escaped = true;
                }
                '\'' if !double => single = !single,
                '"' if !single => double = !double,
                '(' if !single => command_sub_depth += 1,
                ')' if !single => command_sub_depth -= 1,
                _ => {}
            }
            end = idx + ch.len_utf8();
            chars.next();
            continue;
        }

        match ch {
            '\\' if !single => {
                escaped = true;
                end = idx + ch.len_utf8();
                chars.next();
            }
            '\'' if !double => {
                single = !single;
                end = idx + ch.len_utf8();
                chars.next();
            }
            '$' if !single => {
                end = idx + ch.len_utf8();
                chars.next();
                if matches!(chars.peek(), Some((_, '('))) {
                    let (idx, ch) = chars.next().expect("peeked char exists");
                    end = idx + ch.len_utf8();
                    command_sub_depth = 1;
                }
            }
            '"' if !single => {
                double = !double;
                end = idx + ch.len_utf8();
                chars.next();
            }
            ';' | '|' | '<' | '>' | '&' if !single && !double => break,
            ch if ch.is_whitespace() && !single && !double => break,
            _ => {
                end = idx + ch.len_utf8();
                chars.next();
            }
        }
    }

    if single || double || escaped || command_sub_depth > 0 {
        return Err(ParseError::UnterminatedQuote);
    }

    Ok(input[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_control_and_redirects() {
        assert_eq!(
            lex("echo hi > out && cat < out 2>&1").unwrap(),
            vec![
                Token::Word("echo".into()),
                Token::Word("hi".into()),
                Token::RedirectOut,
                Token::Word("out".into()),
                Token::AndIf,
                Token::Word("cat".into()),
                Token::RedirectIn,
                Token::Word("out".into()),
                Token::RedirectErrToOut,
            ]
        );
    }

    #[test]
    fn preserves_quoted_words() {
        assert_eq!(
            lex("echo 'a b' \"c d\"").unwrap(),
            vec![
                Token::Word("echo".into()),
                Token::Word("'a b'".into()),
                Token::Word("\"c d\"".into()),
            ]
        );
    }

    #[test]
    fn keeps_command_substitution_as_word() {
        assert_eq!(
            lex("echo $(echo a | cat)").unwrap(),
            vec![
                Token::Word("echo".into()),
                Token::Word("$(echo a | cat)".into()),
            ]
        );
    }

    #[test]
    fn treats_newline_as_separator() {
        assert_eq!(
            lex("echo a\r\necho b").unwrap(),
            vec![
                Token::Word("echo".into()),
                Token::Word("a".into()),
                Token::Semi,
                Token::Word("echo".into()),
                Token::Word("b".into()),
            ]
        );
    }
}
