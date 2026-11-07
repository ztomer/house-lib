use super::*;

use rustix::process::{Pid, getpgid, getpgrp, getsid};
use std::path::Path;

/// The variable that tells a re-run of this test binary which role it plays.
const ROLE: &str = "HOUSE_LIB_SPAWN_ROLE";

#[test]
fn only_the_detach_argument_makes_a_process_the_trampoline() {
    let os = |s: &str| OsString::from(s);
    let raw = [os("__detach"), os("/bin/echo"), os("echo"), os("hi")];
    assert_eq!(
        trampoline_args(&raw),
        Some(&[os("/bin/echo"), os("echo"), os("hi")][..])
    );
    assert_eq!(trampoline_args(&[os("--deps")]), None);
    assert_eq!(trampoline_args(&[]), None);
}

#[test]
fn the_trampoline_refuses_a_short_argv_with_its_usage() {
    assert!(detach_exec(&[]).starts_with("usage: __detach"));
    assert!(detach_exec(&[OsString::from("/bin/true")]).starts_with("usage:"));
}

/// The process group of a child built under `terminal`.
fn group_of(terminal: Terminal) -> Pid {
    let mut child = command("sleep", terminal)
        .unwrap()
        .arg("5")
        .stdin(Stdio::null())
        .spawn()
        .unwrap();
    let group = getpgid(Some(Pid::from_child(&child))).unwrap();
    child.kill().unwrap();
    child.wait().unwrap();
    group
}

#[test]
fn a_ticket_holder_runs_in_a_group_of_its_own() {
    assert_ne!(group_of(Terminal::OwnGroup), getpgrp());
}

#[test]
fn a_handover_shares_our_group() {
    assert_eq!(group_of(Terminal::Handover), getpgrp());
}

#[test]
fn detached_without_a_trampoline_still_leaves_our_group() {
    // This suite never installs one in-process: a caller without the
    // trampoline gets the own-group isolation.
    assert!(detacher().is_none());
    assert_ne!(group_of(Terminal::Detached), getpgrp());
}

#[test]
fn a_ticket_holder_and_a_handover_keep_our_session() {
    // sudo keys its ticket on the terminal AND the session: a ticket holder
    // in a new session would find no ticket.
    for terminal in [Terminal::OwnGroup, Terminal::Handover] {
        let mut child = command("sleep", terminal)
            .unwrap()
            .arg("5")
            .stdin(Stdio::null())
            .spawn()
            .unwrap();
        let theirs = getsid(Some(Pid::from_child(&child))).unwrap();
        child.kill().unwrap();
        child.wait().unwrap();
        assert_eq!(theirs, getsid(None).unwrap(), "{terminal:?}");
    }
}

/// A detached child's stdin is the null device even when the tool's own stdin
/// carries input: run in a fresh process whose stdin is a pipe holding a
/// line, the child must not read it. (`Command::output` would null stdin by
/// itself, so the role spawns and waits instead.)
#[test]
fn a_detached_child_reads_the_null_device_unless_told_otherwise() {
    let text = run_role("stdin", b"leaked\n");
    assert!(
        text.contains("role stdin done"),
        "the role returned early: {text}"
    );
}

#[test]
fn stdin_role() {
    if std::env::var(ROLE).as_deref() != Ok("stdin") {
        return;
    }
    let out = command("cat", Terminal::Detached)
        .unwrap()
        .arg("-")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "");
    println!("role stdin done");
}

/// This test binary re-run for the one test `<role>_role`, with `input` on
/// its stdin; its stdout, after asserting it ran and passed.
fn run_role(role: &str, input: &[u8]) -> String {
    use std::io::Write as _;
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            &format!("spawn::tests::{role}_role"),
            "--nocapture",
        ])
        .env(ROLE, role)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    let out = child.wait_with_output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "{text}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains("1 passed"), "the role did not run: {text}");
    text
}

#[test]
fn a_missing_program_without_a_trampoline_fails_at_spawn_as_std_would() {
    let err = command("definitely-not-a-real-binary-xyz", Terminal::Detached)
        .unwrap()
        .spawn()
        .unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}

/// The trampoline in a process of its own: `setsid` takes the process out of
/// its session, and the installed trampoline is process-wide.
#[test]
fn the_trampoline_and_the_detached_command_hold_in_a_fresh_process() {
    let text = run_role("trampoline", b"");
    assert!(
        text.contains("role trampoline done"),
        "the role returned early: {text}"
    );
}

#[test]
fn trampoline_role() {
    if std::env::var(ROLE).as_deref() != Ok("trampoline") {
        return;
    }
    install_detacher(PathBuf::from("/the/trampoline"));
    install_detacher(PathBuf::from("/another"));
    assert_eq!(
        detacher(),
        Some(&PathBuf::from("/the/trampoline")),
        "first wins"
    );
    // A program that is not there is refused before anything runs.
    let err = command("definitely-not-a-real-binary-xyz", Terminal::Detached).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    // A real one runs THROUGH the trampoline: `<exe> __detach <path> <name>`.
    let built = command("sh", Terminal::Detached).unwrap();
    assert_eq!(built.get_program(), "/the/trampoline");
    let args: Vec<_> = built.get_args().collect();
    assert_eq!(args[0], DETACH_ARG);
    assert!(Path::new(args[1]).is_absolute() && args[1].to_string_lossy().ends_with("/sh"));
    assert_eq!(args[2], "sh");
    // The other modes never go through it.
    assert_eq!(
        command("sh", Terminal::OwnGroup).unwrap().get_program(),
        "sh"
    );
    assert_eq!(
        command("sh", Terminal::Handover).unwrap().get_program(),
        "sh"
    );
    // setsid succeeds in a fresh process and the program it cannot exec is
    // named; a session leader's second setsid is refused, and named too.
    let missing = [
        OsString::from("/definitely/not/a/program"),
        OsString::from("argv0-name"),
    ];
    let first = detach_exec(&missing);
    assert!(first.starts_with("cannot run argv0-name:"), "{first}");
    let second = detach_exec(&missing);
    assert!(
        second.starts_with("cannot leave the terminal's session for argv0-name:"),
        "{second}"
    );
    // `enter`: a trampoline request is answered as one...
    let request = [
        OsString::from(DETACH_ARG),
        OsString::from("/definitely/not/a/program"),
        OsString::from("asked-for"),
    ];
    let reason = enter_with(&request, None).expect("a trampoline request is answered");
    assert!(reason.contains("asked-for"), "{reason}");
    // ...anything else starts a run, installing nothing without an exe...
    assert_eq!(enter_with(&[OsString::from("--deps")], None), None);
    // ...and the real entry leaves the installed trampoline standing.
    assert_eq!(enter(), None);
    assert_eq!(detacher(), Some(&PathBuf::from("/the/trampoline")));
    println!("role trampoline done");
}
