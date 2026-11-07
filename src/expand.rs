//! The one reader for a leading `~`.
//!
//! WHY THIS FILE EXISTS. Three readers read this fact -- one in each update
//! app, plus a second one in `app_updates` -- and they gave three answers:
//!
//! | reader | `~` | `~/x` | `~user` |
//! |---|---|---|---|
//! | `app_updates` core | the home directory | `<home>/x` | left as written |
//! | `app_updates` cli | left as written | `<home>/x` | left as written |
//! | `sys_updater` | the home directory | `<home>/x` | **`<home>user`** |
//!
//! The third answer is a path that does not exist: `~ztomer/x` became
//! `<home>ztomer/x`. One fact, three readers, three behaviours -- and one of
//! them wrong.
//!
//! There is now ONE behaviour, and this file is its only reader:
//!
//! | input | answer |
//! |---|---|
//! | `~` | the home directory |
//! | `~/x` | `<home>/x` |
//! | `~user` | REFUSED, with the reason |
//! | anything else | the path, unchanged |
//!
//! `~user` is refused rather than resolved. This tool never resolves another
//! account's home directory: the only answer it could give is a path that does
//! not exist, and a path that does not exist is not a path.
//!
//! A `~` that is not LEADING is an ordinary character. `a/~/b`, `x~y` and
//! `~/x~y` are returned exactly as they came in.
//!
//! WHERE THIS FILE LIVES. It was tracked, byte-identical, in both update
//! apps, because the house had no cross-repo crate mechanism. It is now the
//! shared library's (LIB.1), and the behaviour table above is the contract.

use std::path::{Path, PathBuf};

/// A leading `~` that names ANOTHER account's home directory.
///
/// This tool never resolves another account's home: the only answer it could
/// give is a path that does not exist (`~ztomer/x` read as `<home>ztomer/x`).
/// So the path is refused, and the refusal names the account that was asked
/// for.
#[derive(thiserror::Error, Debug)]
#[error("refusing to expand `~{user}` in {path:?}: only `~` and `~/` name this account's home")]
pub struct OtherUsersHome {
    /// The account named after the `~`.
    pub user: String,
    /// The path as it was written, so the reason names what was refused.
    pub path: String,
}

/// Expand a leading `~` or `~/...` against `home`.
///
/// * `~` is the home directory;
/// * `~/x` is under it;
/// * `~user` is REFUSED -- see [`OtherUsersHome`];
/// * every other path is returned unchanged, so a `~` that is not leading
///   (`a/~/b`, `x~y`) is an ordinary character.
///
/// `home` is `None` when the account has no home directory. There is then
/// nothing to expand to, and the path is returned unchanged.
///
/// # Errors
/// If the path names another account (`~user`): this tool refuses rather
/// than resolve a home directory it does not own.
pub fn expand_home(home: Option<PathBuf>, p: &Path) -> Result<PathBuf, OtherUsersHome> {
    let text = p.to_string_lossy();
    if text == "~" {
        return Ok(home.unwrap_or_else(|| p.to_path_buf()));
    }
    if let Some(rest) = text.strip_prefix("~/") {
        // `home.join(rest)` would DROP the home when `rest` starts with a
        // separator (`~//x` would become `/x`), so the separators are trimmed
        // first: `<home>/x` is the answer, and `<home>//x` names the same
        // path.
        let trimmed = rest.trim_start_matches('/');
        return Ok(home.map_or_else(|| p.to_path_buf(), |h| h.join(trimmed)));
    }
    if let Some(rest) = text.strip_prefix('~') {
        // `~user` names ANOTHER account. It is REFUSED, not resolved: the old
        // behaviour turned `~ztomer/x` into `<home>ztomer/x`, a path that
        // does not exist.
        let user = rest.split('/').next().unwrap_or(rest).to_owned();
        return Err(OtherUsersHome {
            user,
            path: text.into_owned(),
        });
    }
    Ok(p.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> PathBuf {
        PathBuf::from("/h")
    }

    #[test]
    fn a_bare_tilde_is_the_home_directory() {
        assert_eq!(
            expand_home(Some(home()), Path::new("~")).unwrap(),
            PathBuf::from("/h")
        );
    }

    #[test]
    fn a_tilde_slash_is_under_the_home_directory() {
        assert_eq!(
            expand_home(Some(home()), Path::new("~/x/y")).unwrap(),
            PathBuf::from("/h/x/y")
        );
        // A doubled separator is the same path, not the root.
        assert_eq!(
            expand_home(Some(home()), Path::new("~//x")).unwrap(),
            PathBuf::from("/h/x")
        );
    }

    #[test]
    fn another_account_is_refused_with_a_reason() {
        let err = expand_home(Some(home()), Path::new("~ztomer/x")).unwrap_err();
        assert_eq!(err.user, "ztomer");
        assert!(err.to_string().contains("ztomer"));
        // A bare `~name` is the same refusal.
        let err = expand_home(Some(home()), Path::new("~ztomer")).unwrap_err();
        assert_eq!(err.user, "ztomer");
    }

    #[test]
    fn a_tilde_that_is_not_leading_is_an_ordinary_character() {
        for path in ["a/~/b", "x~y", "/abs/~/x", "./~"] {
            assert_eq!(
                expand_home(Some(home()), Path::new(path)).unwrap(),
                PathBuf::from(path),
                "{path} must come back unchanged"
            );
        }
    }

    #[test]
    fn a_tilde_in_a_file_name_is_untouched() {
        assert_eq!(
            expand_home(Some(home()), Path::new("~/note~.txt")).unwrap(),
            PathBuf::from("/h/note~.txt")
        );
    }

    #[test]
    fn no_home_directory_leaves_the_path_as_written() {
        assert_eq!(
            expand_home(None, Path::new("~")).unwrap(),
            PathBuf::from("~")
        );
        assert_eq!(
            expand_home(None, Path::new("~/x")).unwrap(),
            PathBuf::from("~/x")
        );
    }
}
