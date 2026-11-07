use super::*;

use std::process::{Command, Stdio};

/// Every spelling of every pair the policy must ACCEPT.
const ACCEPT: &[(&str, &str)] = &[
    ("Darwin", "arm64"),
    ("Darwin", "aarch64"),
    ("macOS", "arm64"),
    ("macOS", "aarch64"),
    ("Linux", "x86_64"),
    ("Linux", "amd64"),
    ("Linux", "aarch64"),
    ("Linux", "arm64"),
];

/// Pairs the policy must REFUSE, each with a word of the rule it hits.
const REJECT: &[(&str, &str, &str)] = &[
    ("Darwin", "x86_64", "Intel"),
    ("macOS", "amd64", "Intel"),
    ("Linux", "i686", "64-bit only"),
    ("Linux", "i386", "64-bit only"),
    ("Linux", "armv7l", "64-bit only"),
    ("Linux", "armhf", "64-bit only"),
    ("Darwin", "i386", "64-bit only"),
    ("Darwin", "armv7l", "64-bit only"),
    ("FreeBSD", "x86_64", "neither macOS nor Linux"),
    ("OpenBSD", "arm64", "neither macOS nor Linux"),
    ("Windows_NT", "x86_64", "neither macOS nor Linux"),
    ("", "", "64-bit only"),
];

#[test]
fn every_supported_spelling_is_accepted() {
    for (os, arch) in ACCEPT {
        assert_eq!(check(os, arch), Ok(()), "{os}/{arch} must pass");
    }
}

#[test]
fn every_unsupported_pair_is_refused_by_the_rule_it_hits() {
    for (os, arch, rule) in REJECT {
        let err = check(os, arch).expect_err("must be refused");
        assert!(err.contains(rule), "{os}/{arch}: {err}");
        assert!(
            err.contains(SUPPORTED),
            "{os}/{arch} names the accept set: {err}"
        );
    }
    // "Drop Intel" is macOS-only: the refusal says Linux x86_64 stays.
    assert!(
        check("Darwin", "x86_64")
            .unwrap_err()
            .contains("Linux x86_64 remains supported")
    );
}

#[test]
fn cargo_target_names_map_onto_the_same_decision_both_ways() {
    // The build script's seam: what cargo calls the target, judged by `check`.
    for (os, arch) in [
        ("macos", "aarch64"),
        ("linux", "x86_64"),
        ("linux", "aarch64"),
    ] {
        let (o, a) = policy::from_target_cfg(os, arch);
        assert_eq!(check(&o, &a), Ok(()), "{os}/{arch} must build");
    }
    for (os, arch) in [
        ("macos", "x86_64"),
        ("linux", "x86"),
        ("linux", "arm"),
        ("windows", "x86_64"),
        ("freebsd", "aarch64"),
        ("linux", "wasm32"),
        ("unknown", "wasm32"),
    ] {
        let (o, a) = policy::from_target_cfg(os, arch);
        assert!(check(&o, &a).is_err(), "{os}/{arch} must not build");
    }
}

#[test]
fn the_seam_overrides_and_the_host_is_a_supported_platform() {
    assert_eq!(
        current_from(Some("Linux"), Some("i686")),
        ("Linux".to_owned(), "i686".to_owned())
    );
    let (os, arch) = current_from(None, None);
    assert_eq!(check(&os, &arch), Ok(()), "host {os}/{arch}");
    let (only_os, host_arch) = current_from(Some("FreeBSD"), None);
    assert_eq!(
        (only_os.as_str(), host_arch.as_str()),
        ("FreeBSD", arch.as_str())
    );
    // Variables nobody sets read as the host.
    assert_eq!(
        current("HOUSE_LIB_TEST_UNSET_OS", "HOUSE_LIB_TEST_UNSET_ARCH"),
        current_from(None, None)
    );
}

/// `house_platform_check os arch` from `platform.sh`, in bash: exit 0 and the
/// stderr it printed.
fn shell_verdict(os: &str, arch: &str) -> (bool, String) {
    let out = Command::new("bash")
        .args([
            "-c",
            &format!("{SHELL_GATE}\nhouse_platform_check \"$1\" \"$2\""),
            "bash",
            os,
            arch,
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).trim().to_owned(),
    )
}

#[test]
fn the_shell_gate_gives_the_rust_verdict_for_every_pair() {
    let mut pairs: Vec<(&str, &str)> = ACCEPT.to_vec();
    pairs.extend(REJECT.iter().map(|(o, a, _)| (*o, *a)));
    for (os, arch) in pairs {
        let (ok, said) = shell_verdict(os, arch);
        match check(os, arch) {
            Ok(()) => assert!(ok && said.is_empty(), "{os}/{arch}: shell said {said}"),
            Err(reason) => {
                assert!(!ok, "{os}/{arch}: shell accepted what Rust refuses");
                assert_eq!(said, reason, "{os}/{arch}: the two reasons differ");
            }
        }
    }
}
