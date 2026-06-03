use crate::commands::BuiltinResult;
use crate::runtime::Shell;
use anyhow::Result;
use std::collections::HashMap;

pub(crate) fn export(
    shell: &mut Shell,
    argv: &[String],
    env_overlay: &HashMap<String, String>,
) -> BuiltinResult {
    for arg in argv {
        if let Some((name, value)) = arg.split_once('=') {
            shell.vars.insert(name.to_string(), value.to_string());
        } else if let Some(value) = env_overlay.get(arg).cloned() {
            shell.vars.insert(arg.to_string(), value);
        }
    }

    BuiltinResult::success()
}

pub(crate) fn unset(shell: &mut Shell, argv: &[String]) -> BuiltinResult {
    for arg in argv {
        shell.vars.remove(arg);
    }

    BuiltinResult::success()
}

pub(crate) fn set(shell: &Shell) -> Result<BuiltinResult> {
    let mut pairs: Vec<_> = shell.vars.iter().collect();
    pairs.sort_by(|a, b| a.0.cmp(b.0));

    let mut stdout = Vec::new();
    for (key, value) in pairs {
        stdout.extend_from_slice(format!("{key}={value}\n").as_bytes());
    }

    Ok(BuiltinResult::stdout(0, stdout))
}
