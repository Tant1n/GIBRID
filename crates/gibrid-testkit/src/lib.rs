//! Test helpers. Missing fixtures must be SKIPPED, never reported as PASS.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

pub fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/fixtures")
        .join(name)
}

pub fn skip_if_missing(path: &Path) -> bool {
    if path.exists() {
        false
    } else {
        eprintln!("SKIPPED: missing fixture {}", path.display());
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_game_fixture_is_skipped_not_passed() {
        let path = fixture_path("from-installed-game.mca");
        assert!(skip_if_missing(&path));
        assert!(!path.exists());
    }
}
