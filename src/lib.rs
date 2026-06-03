mod commands;
mod parser;
mod path;
mod runtime;

pub use runtime::{RunOptions, Shell};

#[doc(hidden)]
pub fn display_path_for_cli(path: &std::path::Path) -> String {
    self::path::display_path(path)
}

#[doc(hidden)]
pub fn path_for_cli(path: &str) -> std::path::PathBuf {
    self::path::shell_path(path)
}
