use std::path::Path;

pub trait PathHelper {
    /// Check if the file extension matches the given string
    /// (case insensitive).
    fn extension_eq(&self, ext: &str) -> bool;

    /// Check if the file extension matches any of the given strings
    /// (case insensitive).
    fn extension_eqs(&self, exts: &[&str]) -> bool {
        exts.iter().any(|ext| self.extension_eq(ext))
    }
}

impl PathHelper for Path {
    fn extension_eq(&self, ext: &str) -> bool {
        self.extension()
            .and_then(|e| e.to_str())
            .map_or(false, |e| e.eq_ignore_ascii_case(ext))
    }
}
