//! The platform policy at RUN time, and the `OS`/`ARCH` seam that tests it.
//!
//! ONE accept set for the house (`macOS/arm64, Linux/x86_64, Linux/aarch64`),
//! in three forms that share it:
//!
//! - [`policy::check`] -- the decision, a pure function of two names;
//! - `build.rs` -- the same file, refusing to build the library for a target
//!   outside the set (so a consumer cannot either);
//! - `platform.sh` -- the shell form for a consumer's build or install script,
//!   held to [`policy::check`] over the whole matrix by `platform_tests.rs`.
//!
//! Before the library, ztools (`install.sh`), `sys_updater` (`platform.rs`,
//! `scripts/build.sh`) and `app_updates` (`build.sh`) each carried their own,
//! and they had drifted: `app_updates`' shell refused the `amd64` spelling
//! `sys_updater` accepted, and only `sys_updater` knew `macOS` for `Darwin`.

#[path = "platform_policy.rs"]
pub mod policy;

pub use policy::{SUPPORTED, check};

/// The pair to judge: each override when given (the seam), else this host.
///
/// The process environment is shared by every test thread, so tests drive
/// this with explicit values; [`current`] is the env-reading wrapper.
#[must_use]
pub fn current_from(os_override: Option<&str>, arch_override: Option<&str>) -> (String, String) {
    let os = os_override.map_or_else(
        || {
            if cfg!(target_os = "macos") {
                "Darwin"
            } else if cfg!(target_os = "linux") {
                "Linux"
            } else {
                "Unknown"
            }
            .to_owned()
        },
        str::to_owned,
    );
    let arch = arch_override.map_or_else(
        || {
            if cfg!(target_arch = "aarch64") {
                "arm64"
            } else if cfg!(target_arch = "x86_64") {
                "x86_64"
            } else {
                "Unknown"
            }
            .to_owned()
        },
        str::to_owned,
    );
    (os, arch)
}

/// [`current_from`] over the environment variables a tool names for its seam
/// (ztools: `ZTOOLS_OS`/`ZTOOLS_ARCH`; `sys_updater`:
/// `SYS_UPDATER_BUILD_OS`/`SYS_UPDATER_BUILD_ARCH`).
#[must_use]
pub fn current(os_var: &str, arch_var: &str) -> (String, String) {
    current_from(
        std::env::var(os_var).ok().as_deref(),
        std::env::var(arch_var).ok().as_deref(),
    )
}

/// The shell form, `platform.sh`, as text: a consumer writes it out or
/// sources the copy in this crate's checkout (see the README).
pub const SHELL_GATE: &str = include_str!("../platform.sh");

#[cfg(test)]
#[path = "platform_tests.rs"]
mod tests;
