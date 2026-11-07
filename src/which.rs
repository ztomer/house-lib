//! Where an executable is: the lookup a spawn makes, with the search path as a
//! value (the seam), so no test writes the process environment -- it is
//! shared by every test thread.
//!
//! From `sys_updater`'s `deps.rs`, which `app_updates`' `spawn::find_on_path`
//! duplicated without the seam.

use std::ffi::OsStr;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

/// `cmd` resolved against `search_path`, a `PATH`-style list (`None`: no
/// `PATH` at all). A `cmd` with a `/` is a path, checked directly and never
/// looked up. Only a regular file with an execute bit counts.
#[must_use]
pub fn find_in(cmd: &str, search_path: Option<&OsStr>) -> Option<PathBuf> {
    if cmd.contains('/') {
        let path = PathBuf::from(cmd);
        return is_executable(&path).then_some(path);
    }
    std::env::split_paths(search_path?)
        .map(|dir| dir.join(cmd))
        .find(|p| is_executable(p))
}

/// [`find_in`] over the process `PATH`.
#[must_use]
pub fn find_on_path(cmd: &str) -> Option<PathBuf> {
    find_in(cmd, std::env::var_os("PATH").as_deref())
}

fn is_executable(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("fakebrew");
        std::fs::write(&bin, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::write(dir.path().join("notexec"), "x").unwrap();
        dir
    }

    #[test]
    fn an_executable_is_found_past_an_entry_that_does_not_exist() {
        let dir = fixture();
        let sp =
            std::env::join_paths([Path::new("/nonexistent-house-lib-dir"), dir.path()]).unwrap();
        assert_eq!(
            find_in("fakebrew", Some(&sp)),
            Some(dir.path().join("fakebrew"))
        );
        assert_eq!(
            find_in("definitely-not-a-real-command-xyz", Some(&sp)),
            None
        );
    }

    #[test]
    fn the_search_path_given_is_the_only_one_consulted() {
        let dir = fixture();
        assert_eq!(find_in("sh", Some(dir.path().as_os_str())), None);
        assert_eq!(find_in("fakebrew", None), None);
        assert!(find_on_path("sh").is_some_and(|p| p.is_absolute()));
    }

    #[test]
    fn a_path_is_checked_where_it_is_and_must_be_an_executable_file() {
        let dir = fixture();
        let exe = dir.path().join("fakebrew");
        assert_eq!(find_in(exe.to_str().unwrap(), None), Some(exe.clone()));
        assert_eq!(find_on_path(exe.to_str().unwrap()), Some(exe));
        let plain = dir.path().join("notexec");
        assert_eq!(find_in(plain.to_str().unwrap(), None), None);
        assert_eq!(find_in("notexec", Some(dir.path().as_os_str())), None);
        assert_eq!(find_in(dir.path().to_str().unwrap(), None), None);
    }
}
