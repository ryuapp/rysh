use crate::commands::BuiltinResult;
use crate::path::shell_path;
use crate::runtime::{RunOptions, Shell};
use anyhow::{Context, Result};

pub(crate) fn run(shell: &mut Shell, argv: &[String]) -> Result<BuiltinResult> {
    let path = argv.first().context(".: missing script path")?;
    let source = std::fs::read_to_string(shell_path(path))
        .with_context(|| format!("failed to read script {}", path))?;

    Ok(BuiltinResult::status(
        shell.run_script(&source, RunOptions::default())?,
    ))
}
