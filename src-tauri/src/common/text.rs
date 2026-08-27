//! Text helpers shared by the kernels.
//!
//! The reference Python kernels read and write files in text mode, which
//! normalizes `\r\n` / `\r` to `\n` on read and translates `\n` to the
//! platform line separator on write. These two helpers reproduce that
//! behavior so the Rust kernels produce byte-identical output.

/// Universal-newline read: collapses `\r\n` and lone `\r` into `\n`.
pub fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Text-mode write: expands `\n` to the platform line separator
/// (`\r\n` on Windows, mirroring Python's `os.linesep` translation).
pub fn to_platform_newlines(text: &str) -> String {
    if cfg!(windows) {
        text.replace('\n', "\r\n")
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newline_roundtrip() {
        assert_eq!(normalize_newlines("a\r\nb\rc\nd"), "a\nb\nc\nd");
        if cfg!(windows) {
            assert_eq!(to_platform_newlines("a\nb"), "a\r\nb");
        }
    }
}
