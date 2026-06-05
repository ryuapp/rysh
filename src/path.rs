use std::path::PathBuf;

pub(crate) fn shell_path(path: &str) -> PathBuf {
    #[cfg(windows)]
    if let Some(path) = slash_drive_path(path) {
        return path;
    }

    PathBuf::from(path)
}

pub(crate) fn is_explicit_path(path: &str) -> bool {
    let path = path.replace('\\', "/");
    path.contains('/') || windows_drive_prefix(&path)
}

#[cfg(windows)]
fn slash_drive_path(path: &str) -> Option<PathBuf> {
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

#[cfg(windows)]
fn windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

#[cfg(not(windows))]
fn windows_drive_prefix(_path: &str) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn converts_slash_drive_root() {
        assert_eq!(shell_path("/c"), PathBuf::from("C:\\"));
    }

    #[cfg(windows)]
    #[test]
    fn converts_slash_drive_path() {
        assert_eq!(
            shell_path("/c/Users/test"),
            PathBuf::from("C:\\Users\\test")
        );
    }

    #[cfg(windows)]
    #[test]
    fn keeps_windows_drive_path() {
        assert_eq!(shell_path("C:/Users/test"), PathBuf::from("C:/Users/test"));
    }

    #[test]
    fn detects_explicit_paths() {
        #[cfg(windows)]
        assert!(is_explicit_path("C:/Windows"));
        assert!(is_explicit_path("/c/Windows"));
        assert!(is_explicit_path("./script"));
        assert!(!is_explicit_path("command"));
    }
}
