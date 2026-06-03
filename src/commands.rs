use crate::runtime::Shell;
use anyhow::Result;
use std::collections::HashMap;

mod cd;
mod echo;
mod environment;
mod exit;
mod pwd;
mod source;
mod status;

#[derive(Debug)]
pub(crate) struct BuiltinResult {
    pub status: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
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
        }
    }

    fn stdout(status: i32, stdout: Vec<u8>) -> Self {
        Self {
            status,
            stdout,
            stderr: Vec::new(),
        }
    }

    fn stderr(status: i32, stderr: Vec<u8>) -> Self {
        Self {
            status,
            stdout: Vec::new(),
            stderr,
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
        "pwd" => pwd::run()?,
        "exit" => exit::run(argv),
        "export" => environment::export(shell, argv, env_overlay),
        "unset" => environment::unset(shell, argv),
        "set" => environment::set(shell)?,
        "true" => status::true_(),
        "false" => status::false_(),
        "echo" => echo::run(argv)?,
        "." | "source" => source::run(shell, argv)?,
        _ => return Ok(None),
    };

    Ok(Some(result))
}
