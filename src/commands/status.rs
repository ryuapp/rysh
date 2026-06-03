use crate::commands::BuiltinResult;

pub(crate) fn true_() -> BuiltinResult {
    BuiltinResult::success()
}

pub(crate) fn false_() -> BuiltinResult {
    BuiltinResult::status(1)
}
