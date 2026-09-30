//! A `file://` URI's path, as this platform spells a path.
//!
//! A URI writes a Windows path with a slash before the drive
//! (`file://host/C:/Users/x`); everything that reads a cwd there — psmux's
//! `pane_current_path` among them — writes `C:\Users\x`.

#[cfg(not(windows))]
pub(crate) fn native(path: String) -> String {
    path
}

#[cfg(windows)]
pub(crate) fn native(path: String) -> String {
    drive_path(&path).unwrap_or(path)
}

/// `/C:/Users/x` as `C:\Users\x`, or `None` for a path with no drive.
#[cfg(any(windows, test))]
pub(crate) fn drive_path(path: &str) -> Option<String> {
    let rest = path.strip_prefix('/')?;
    let mut chars = rest.chars();
    let drive = chars.next().filter(char::is_ascii_alphabetic)?;
    if chars.next() != Some(':') {
        return None;
    }
    let after = &rest[2..];
    if !(after.is_empty() || after.starts_with('/')) {
        return None;
    }
    let after = if after.is_empty() { "/" } else { after };
    Some(format!("{drive}:{}", after.replace('/', "\\")))
}

#[cfg(test)]
mod tests {
    use super::drive_path;

    #[test]
    fn a_drive_loses_the_slash_a_uri_puts_before_it() {
        assert_eq!(
            drive_path("/C:/Users/Jo Ann/src").as_deref(),
            Some(r"C:\Users\Jo Ann\src")
        );
        assert_eq!(drive_path("/d:").as_deref(), Some(r"d:\"));
        assert_eq!(drive_path("/C:/").as_deref(), Some(r"C:\"));
    }

    #[test]
    fn a_path_with_no_drive_is_left_alone() {
        for path in ["/home/jo", "/C", "/Cx/y", "/1:/y", "C:/y", "/C:x"] {
            assert_eq!(drive_path(path), None, "{path}");
        }
    }
}
