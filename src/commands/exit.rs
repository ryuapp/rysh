use crate::commands::BuiltinResult;

pub(crate) fn run(argv: &[String]) -> BuiltinResult {
    let code = argv
        .first()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    BuiltinResult::exit(code)
}
