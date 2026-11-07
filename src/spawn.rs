//! The ONE place a child process is built, and what it may do with the
//! terminal.
//!
//! THE DEFECT THIS CLOSES (`sys_updater` U.1, 2026-10-31; `app_updates` 3.5).
//! A child in the tool's own process group and session, on its controlling
//! terminal, that starts an interactive shell (`zsh -i`) moves that shell into
//! a group of its own and makes it the terminal's foreground; the next read of
//! the terminal by ANY member of the tool's group sends SIGTTIN to the whole
//! group, the tool with it ("suspended (tty input)"); the login shell takes
//! the terminal back, the interactive shell's exit hands it to the stopped
//! group, and the login shell's own read fails with EIO and it exits, closing
//! the window. Each consumer reproduces it under a pty in its own
//! `tty_isolation` test.
//!
//! THE RULE. Every child names, here, what it may do with the terminal:
//!
//! - [`Terminal::Detached`]: a new SESSION with no controlling terminal, and
//!   the null device as stdin until the caller gives it something else. It
//!   cannot read the terminal, be stopped by it, or take its foreground.
//! - [`Terminal::OwnGroup`]: the tool's session (sudo keys its ticket on the
//!   terminal AND the session, `timestamp_type=tty`) in a process group of its
//!   own: a read of the terminal stops THAT group, never the tool's.
//! - [`Terminal::Handover`]: the tool's own group -- the terminal is handed
//!   over on purpose (a `sudo -v` prompt, `tmux attach`, a self-update re-exec).
//!
//! HOW A DETACHED CHILD GETS ITS SESSION. `setsid` must run in the child
//! between fork and exec, which std offers only behind `unsafe` (`pre_exec`)
//! or an unstable API, and this crate forbids `unsafe`. So the binary is its
//! own trampoline: the child is `<exe> __detach <program> <argv0> <args...>`,
//! which calls `setsid` (safe in rustix: the trampoline is a fresh process,
//! never a group leader) and execs the program in place -- same pid, pipes,
//! environment and working directory. A binary's `main` calls [`enter`]
//! first; a caller that never installs a trampoline (a unit test) gets
//! [`Terminal::OwnGroup`]'s isolation for a detached child instead.
//!
//! WHICH PORT THIS IS. Two ports of the rule existed: `sys_updater`'s
//! `spawn.rs` (10cbd86) and `app_updates`' `core::spawn` (d5caf15). This is
//! `sys_updater`'s, because its three named modes are the SUPERSET: its
//! sudo-ticket and handover steps cannot be expressed in `app_updates`'
//! detached-only port, while `app_updates`' one mode is [`Terminal::Detached`]
//! here. From `app_updates`' port it takes the two things `sys_updater`'s
//! lacked: [`enter`] (the trampoline-or-install decision every `main` makes,
//! testable as [`enter_with`]) and the null stdin for a detached child -- a
//! child that has left the terminal's session but still holds the terminal
//! as fd 0 can read keystrokes meant for the tool, with no SIGTTIN to stop it.

use std::ffi::OsString;
use std::os::unix::process::CommandExt as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// The argument that turns the binary into the detaching trampoline.
pub const DETACH_ARG: &str = "__detach";

/// What a child may do with the controlling terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terminal {
    /// A new session, no controlling terminal, stdin the null device.
    Detached,
    /// The tool's session (the sudo ticket), a process group of its own.
    OwnGroup,
    /// The tool's own group: the terminal is handed over on purpose.
    Handover,
}

static DETACHER: OnceLock<PathBuf> = OnceLock::new();

/// Make `exe` (a binary whose `main` answers [`DETACH_ARG`] through [`enter`])
/// the trampoline for every detached child. The first call wins.
pub fn install_detacher(exe: PathBuf) {
    let _ = DETACHER.set(exe);
}

/// The trampoline in use, if one is installed.
#[must_use]
pub fn detacher() -> Option<&'static PathBuf> {
    DETACHER.get()
}

/// `Some(<program> <argv0> <args...>)` when `raw` (the process's arguments
/// without the program name) asks this process to be the trampoline.
#[must_use]
pub fn trampoline_args(raw: &[OsString]) -> Option<&[OsString]> {
    match raw.split_first() {
        Some((first, rest)) if first == DETACH_ARG => Some(rest),
        _ => None,
    }
}

/// The first thing a binary's `main` does: be the trampoline if asked to,
/// else install itself as the trampoline for every detached child.
///
/// `Some(reason)`: this process was only a trampoline and it failed; the
/// caller prints the reason and exits non-zero without running anything else.
/// `None`: the ordinary start of a run.
#[must_use]
pub fn enter() -> Option<String> {
    let raw: Vec<OsString> = std::env::args_os().skip(1).collect();
    enter_with(&raw, std::env::current_exe().ok())
}

/// [`enter`] for given arguments and executable, so every arm is a test.
#[must_use]
pub fn enter_with(raw: &[OsString], exe: Option<PathBuf>) -> Option<String> {
    if let Some(rest) = trampoline_args(raw) {
        return Some(detach_exec(rest));
    }
    if let Some(exe) = exe {
        install_detacher(exe);
    }
    None
}

/// A [`Command`] for `program` with the terminal arrangement `terminal`
/// names. Callers add arguments, stdio, environment and a working directory
/// exactly as they would to `Command::new(program)`.
///
/// # Errors
///
/// With a trampoline installed, a detached `program` that is not an
/// executable on `PATH` (or, with a `/`, at that path) is refused here with
/// the error the spawn itself would have given: past the trampoline a
/// missing program is only an exit code, and callers read "could not start"
/// differently from "ran and failed".
pub fn command(program: &str, terminal: Terminal) -> std::io::Result<Command> {
    match terminal {
        Terminal::Handover => Ok(Command::new(program)),
        Terminal::OwnGroup => Ok(own_group(program)),
        Terminal::Detached => {
            let mut command = match DETACHER.get() {
                None => own_group(program),
                Some(exe) => {
                    let resolved = crate::which::find_on_path(program).ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("{program}: not an executable on PATH"),
                        )
                    })?;
                    let mut through = Command::new(exe);
                    through.arg(DETACH_ARG).arg(resolved).arg(program);
                    through
                }
            };
            command.stdin(Stdio::null());
            Ok(command)
        }
    }
}

fn own_group(program: &str) -> Command {
    let mut command = Command::new(program);
    command.process_group(0);
    command
}

/// The trampoline: `args` is `<program> <argv0> <args...>` (what follows
/// [`DETACH_ARG`]). Leave the terminal's session, then become `program`.
///
/// Returns only on failure, with the reason; the caller prints it and exits
/// non-zero. It never runs the program on the terminal it was asked to leave.
#[must_use]
pub fn detach_exec(args: &[OsString]) -> String {
    let [program, argv0, rest @ ..] = args else {
        return format!("usage: {DETACH_ARG} <program> <argv0> [args...]");
    };
    if let Err(e) = rustix::process::setsid() {
        return format!(
            "cannot leave the terminal's session for {}: {e}",
            argv0.to_string_lossy()
        );
    }
    let error = Command::new(program).arg0(argv0).args(rest).exec();
    format!("cannot run {}: {error}", argv0.to_string_lossy())
}

#[cfg(test)]
#[path = "spawn_tests.rs"]
mod tests;
