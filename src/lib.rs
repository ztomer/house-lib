//! The private shared library of the house tools (LIB.1; owner rulings
//! OW.20/OW.22/OW.22b: "new private library for common tasks. keep GOH QA
//! only"). Each module was a tracked copy in ztools, `sys_updater` and
//! `app_updates`; the README records where each came from and how they had
//! drifted.
//!
//! - [`selfheal`] -- probe, install with a stated reason, re-probe, degrade.
//! - [`rows`] -- the dependency rows several tools declare (brew, python3,
//!   the Camoufox package and browser), with their pins and the fetch script.
//! - [`platform`] -- the house platform policy: one accept set, refused at
//!   build time by `build.rs`, checked at run time, and in shell as
//!   `platform.sh`.
//! - [`expand`] -- the one reader of a leading `~`.
//! - [`spawn`] -- the one place a child process is built, and what it may do
//!   with the terminal.
//! - [`which`] -- where an executable is, with the search path as a seam.
//!
//! What this crate is NOT: QA. Gates, checkers and the TUI stay in
//! `gates_of_heck` (OW.85: goh-tui stays there).

pub mod expand;
pub mod platform;
pub mod rows;
pub mod selfheal;
pub mod spawn;
pub mod which;
