use crate::commands::BuiltinResult;
use anyhow::Result;
use std::io::Write;

pub(crate) fn run(argv: &[String]) -> Result<BuiltinResult> {
    let mut stdout = Vec::new();
    writeln!(stdout, "{}", argv.join(" "))?;
    Ok(BuiltinResult::stdout(0, stdout))
}
