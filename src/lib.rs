mod commands;
mod parser;
mod path;
mod runtime;
#[cfg(windows)]
mod shebang;

pub use runtime::{RunOptions, Shell};

#[doc(hidden)]
pub fn path_for_cli(path: &str) -> std::path::PathBuf {
    self::path::shell_path(path)
}
