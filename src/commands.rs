use crate::runtime::Shell;
use anyhow::Result;
use std::collections::HashMap;

mod cd;
mod echo;
mod environment;
mod exit;
mod introspection;
mod pwd;
mod source;
mod status;

#[derive(Debug)]
pub(crate) struct BuiltinResult {
    pub status: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit: bool,
}

impl BuiltinResult {
    fn success() -> Self {
        Self::status(0)
    }

    fn status(status: i32) -> Self {
        Self {
            status,
            stdout: Vec::new(),
            stderr: Vec::new(),
            exit: false,
        }
    }

    fn stdout(status: i32, stdout: Vec<u8>) -> Self {
        Self {
            status,
            stdout,
            stderr: Vec::new(),
            exit: false,
        }
    }

    fn stderr(status: i32, stderr: Vec<u8>) -> Self {
        Self {
            status,
            stdout: Vec::new(),
            stderr,
            exit: false,
        }
    }

    fn exit(status: i32) -> Self {
        Self {
            status,
            stdout: Vec::new(),
            stderr: Vec::new(),
            exit: true,
        }
    }
}

pub(crate) fn run(
    shell: &mut Shell,
    name: &str,
    argv: &[String],
    env_overlay: &HashMap<String, String>,
) -> Result<Option<BuiltinResult>> {
    let result = match name {
        "cd" => cd::run(shell, argv)?,
        "command" => introspection::command(shell, argv),
        "pwd" => pwd::run()?,
        "exit" => exit::run(argv),
        "export" => environment::export(shell, argv, env_overlay),
        "unset" => environment::unset(shell, argv),
        "set" => environment::set(shell)?,
        "type" => introspection::type_(shell, argv),
        "true" => status::true_(),
        "false" => status::false_(),
        "echo" => echo::run(argv)?,
        "." | "source" => source::run(shell, argv)?,
        _ => return Ok(None),
    };

    Ok(Some(result))
}

pub(crate) fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "." | "cd"
            | "command"
            | "echo"
            | "exit"
            | "export"
            | "false"
            | "pwd"
            | "set"
            | "source"
            | "true"
            | "type"
            | "unset"
    )
}
