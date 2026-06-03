use std::path::{Path, PathBuf};

pub(crate) fn shell_path(path: &str) -> PathBuf {
    if let Some(path) = msys_drive_path(path) {
        return path;
    }

    PathBuf::from(path)
}

pub(crate) fn is_explicit_path(path: &str) -> bool {
    let path = path.replace('\\', "/");
    path.contains('/') || has_windows_drive_prefix(&path)
}

pub(crate) fn display_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn msys_drive_path(path: &str) -> Option<PathBuf> {
    let path = path.replace('\\', "/");
    let mut chars = path.chars();

    if chars.next()? != '/' {
        return None;
    }

    let drive = chars.next()?;
    if !drive.is_ascii_alphabetic() {
        return None;
    }

    match chars.next() {
        None => Some(PathBuf::from(format!("{}:\\", drive.to_ascii_uppercase()))),
        Some('/') => {
            let rest: String = chars.collect();
            Some(PathBuf::from(format!(
                "{}:\\{}",
                drive.to_ascii_uppercase(),
                rest.replace('/', "\\")
            )))
        }
        Some(_) => None,
    }
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_msys_drive_root() {
        assert_eq!(shell_path("/c"), PathBuf::from("C:\\"));
    }

    #[test]
    fn converts_msys_drive_path() {
        assert_eq!(
            shell_path("/c/Users/test"),
            PathBuf::from("C:\\Users\\test")
        );
    }

    #[test]
    fn keeps_windows_drive_path() {
        assert_eq!(shell_path("C:/Users/test"), PathBuf::from("C:/Users/test"));
    }

    #[test]
    fn detects_explicit_paths() {
        assert!(is_explicit_path("C:/Windows"));
        assert!(is_explicit_path("/c/Windows"));
        assert!(is_explicit_path("./script"));
        assert!(!is_explicit_path("command"));
    }
}
