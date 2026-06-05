//! Windows shebang support for non-native executable files.
//!
//! Shebangs are not specified by POSIX. On Unix-like systems they are handled
//! by the kernel as part of `execve`, commonly using this form:
//!
//! ```text
//! #!interpreter [optional-arg]
//! ```
//!
//! The optional argument is passed to the interpreter as one argument. It is
//! not generally split on whitespace. The resulting invocation is equivalent
//! to:
//!
//! ```text
//! interpreter [optional-arg] script-path original-args...
//! ```
//!
//! Windows `CreateProcess` does not interpret shebangs, so shell performs that
//! step before spawning a non-native file. Files ending in `.com`, `.exe`,
//! `.bat`, or `.cmd` remain native Windows programs and bypass this module.
//!
//! `/usr/bin/env command` is treated as a portability request and resolves
//! `command` through the Windows `PATH` and `PATHEXT`. The non-POSIX `env -S`
//! extension is also supported for scripts that intentionally require
//! multiple interpreter arguments. Without `-S`, the complete optional
//! argument remains one command name, matching common Unix shebang behavior.
//!
//! Parsing is intentionally limited to the first line and rejects lines over
//! [`MAX_LINE_LENGTH`] bytes. A shebang must begin with `#!` at byte zero;
//! UTF-8 BOM-prefixed files are not treated as shebang scripts.

use anyhow::{Result, bail};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_LINE_LENGTH: usize = 4096;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Invocation {
    pub program: String,
    pub args: Vec<String>,
}

pub(crate) fn is_candidate(path: &Path) -> bool {
    path.is_file() && !has_windows_native_extension(path)
}

pub(crate) fn script_path(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().replace('/', "\\"))
}

fn has_windows_native_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "com" | "exe" | "bat" | "cmd"
            )
        })
}

pub(crate) fn read(path: &Path) -> Result<Option<Invocation>> {
    let mut bytes = [0; MAX_LINE_LENGTH + 1];
    let length = File::open(path)?.read(&mut bytes)?;
    let bytes = &bytes[..length];
    let line_end = match bytes.iter().position(|byte| *byte == b'\n') {
        Some(line_end) => line_end,
        None if length > MAX_LINE_LENGTH => bail!("shebang line exceeds {MAX_LINE_LENGTH} bytes"),
        None => bytes.len(),
    };
    let Ok(line) = std::str::from_utf8(&bytes[..line_end]) else {
        return Ok(None);
    };
    parse(line)
}

fn parse(line: &str) -> Result<Option<Invocation>> {
    let command = match line.strip_prefix("#!") {
        Some(command) => command.trim_matches([' ', '\t', '\r']),
        None => return Ok(None),
    };
    if command.is_empty() {
        return Ok(None);
    }

    let split = command.find([' ', '\t']).unwrap_or(command.len());
    let interpreter = &command[..split];
    let optional_arg = command[split..].trim_matches([' ', '\t']);

    if is_env(interpreter) {
        return parse_env(optional_arg).map(Some);
    }

    let args = if optional_arg.is_empty() {
        Vec::new()
    } else {
        vec![optional_arg.to_string()]
    };
    Ok(Some(Invocation {
        program: interpreter.to_string(),
        args,
    }))
}

fn is_env(interpreter: &str) -> bool {
    Path::new(interpreter)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("env"))
}

fn parse_env(argument: &str) -> Result<Invocation> {
    if argument.is_empty() {
        bail!("env shebang is missing a command");
    }

    if let Some(command) = argument.strip_prefix("-S") {
        let mut words = split_env_s(command.trim_start())?;
        if words.is_empty() {
            bail!("env -S shebang is missing a command");
        }
        return Ok(Invocation {
            program: words.remove(0),
            args: words,
        });
    }

    Ok(Invocation {
        program: argument.to_string(),
        args: Vec::new(),
    })
}

fn split_env_s(input: &str) -> Result<Vec<String>> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut chars = input.chars().peekable();
    let mut quote = None;
    let mut started = false;

    while let Some(ch) = chars.next() {
        match (quote, ch) {
            (None, '\'' | '"') => {
                quote = Some(ch);
                started = true;
            }
            (Some(current), ch) if ch == current => quote = None,
            (None, '\\') | (Some('"'), '\\') => {
                let Some(next) = chars.next() else {
                    bail!("env -S shebang ends with an escape");
                };
                word.push(next);
                started = true;
            }
            (None, ch) if ch.is_ascii_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            _ => {
                word.push(ch);
                started = true;
            }
        }
    }

    if quote.is_some() {
        bail!("env -S shebang has an unterminated quote");
    }
    if started {
        words.push(word);
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_interpreter_and_one_optional_argument() {
        assert_eq!(
            parse("#!/usr/bin/node --no-warnings --trace-warnings\r").unwrap(),
            Some(Invocation {
                program: "/usr/bin/node".into(),
                args: vec!["--no-warnings --trace-warnings".into()],
            })
        );
    }

    #[test]
    fn parses_env_command() {
        assert_eq!(
            parse("#!/usr/bin/env node").unwrap(),
            Some(Invocation {
                program: "node".into(),
                args: Vec::new(),
            })
        );
    }

    #[test]
    fn keeps_unsplit_env_argument_as_one_command_name() {
        assert_eq!(
            parse("#!/usr/bin/env node --no-warnings").unwrap(),
            Some(Invocation {
                program: "node --no-warnings".into(),
                args: Vec::new(),
            })
        );
    }

    #[test]
    fn parses_env_split_string() {
        assert_eq!(
            parse("#!/usr/bin/env -S node --eval 'console.log(\"hello world\")'").unwrap(),
            Some(Invocation {
                program: "node".into(),
                args: vec!["--eval".into(), "console.log(\"hello world\")".into(),],
            })
        );
    }

    #[test]
    fn rejects_invalid_env_split_string() {
        assert!(parse("#!/usr/bin/env -S node 'unterminated").is_err());
    }

    #[test]
    fn ignores_non_shebang_lines() {
        assert_eq!(parse("\u{feff}#!/usr/bin/env node").unwrap(), None);
        assert_eq!(parse("echo hello").unwrap(), None);
    }

    #[test]
    fn keeps_windows_native_programs_direct() {
        assert!(has_windows_native_extension(Path::new("tool.exe")));
        assert!(has_windows_native_extension(Path::new("tool.CMD")));
        assert!(has_windows_native_extension(Path::new("tool.bat")));
        assert!(!has_windows_native_extension(Path::new("tool.ts")));
        assert!(!has_windows_native_extension(Path::new("tool")));
    }
}
