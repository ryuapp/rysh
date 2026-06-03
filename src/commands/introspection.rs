use crate::commands::{BuiltinResult, is_builtin};
use crate::path::display_path;
use crate::runtime::Shell;

pub(crate) fn command(shell: &Shell, argv: &[String]) -> BuiltinResult {
    match argv.first().map(String::as_str) {
        Some("-v") => command_v(shell, &argv[1..]),
        Some(option) if option.starts_with('-') => BuiltinResult::stderr(
            2,
            format!("command: unsupported option: {option}\n").into_bytes(),
        ),
        Some(_) => BuiltinResult::stderr(
            2,
            b"command: only command -v is currently supported\n".to_vec(),
        ),
        None => BuiltinResult::success(),
    }
}

pub(crate) fn type_(shell: &Shell, argv: &[String]) -> BuiltinResult {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut status = 0;

    for name in argv {
        if is_builtin(name) {
            stdout.extend_from_slice(format!("{name} is a shell builtin\n").as_bytes());
        } else if let Some(path) = shell.resolve_program(name) {
            stdout.extend_from_slice(format!("{name} is {}\n", display_path(&path)).as_bytes());
        } else {
            status = 1;
            stderr.extend_from_slice(format!("type: {name}: not found\n").as_bytes());
        }
    }

    BuiltinResult {
        status,
        stdout,
        stderr,
    }
}

fn command_v(shell: &Shell, names: &[String]) -> BuiltinResult {
    let mut stdout = Vec::new();
    let mut status = 0;

    for name in names {
        if is_builtin(name) {
            stdout.extend_from_slice(format!("{name}\n").as_bytes());
        } else if let Some(path) = shell.resolve_program(name) {
            stdout.extend_from_slice(format!("{}\n", display_path(&path)).as_bytes());
        } else {
            status = 1;
        }
    }

    BuiltinResult {
        status,
        stdout,
        stderr: Vec::new(),
    }
}
