use crate::commands::BuiltinResult;
use anyhow::Result;
use std::env;

pub(crate) fn run() -> Result<BuiltinResult> {
    Ok(BuiltinResult::stdout(
        0,
        format!("{}\n", env::current_dir()?.display()).into_bytes(),
    ))
}
