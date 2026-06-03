use crate::commands::BuiltinResult;
use crate::path::display_path;
use anyhow::Result;
use std::env;

pub(crate) fn run() -> Result<BuiltinResult> {
    Ok(BuiltinResult::stdout(
        0,
        format!("{}\n", display_path(&env::current_dir()?)).into_bytes(),
    ))
}
