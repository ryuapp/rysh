#[cfg(not(windows))]
mod fallback;
#[cfg(windows)]
mod windows;

#[derive(Debug, PartialEq, Eq)]
pub enum LineRead {
    Line(String),
    Interrupted,
    Eof,
}

#[cfg(not(windows))]
pub use fallback::Terminal;
#[cfg(windows)]
pub use windows::Terminal;
