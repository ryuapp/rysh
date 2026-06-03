use crate::commands::BuiltinResult;
use crate::path::shell_path;
use crate::runtime::Shell;
use anyhow::{Context, Result};
use std::env;

pub(crate) fn run(shell: &Shell, argv: &[String]) -> Result<BuiltinResult> {
    let target = argv
        .first()
        .cloned()
        .or_else(|| shell.vars.get("HOME").cloned())
        .or_else(|| shell.vars.get("USERPROFILE").cloned())
        .context("cd: missing destination and HOME/USERPROFILE is unset")?;

    match env::set_current_dir(shell_path(&target)) {
        Ok(()) => Ok(BuiltinResult::success()),
        Err(err) => Ok(BuiltinResult::stderr(
            1,
            format!("cd: {err}\n").into_bytes(),
        )),
    }
}
