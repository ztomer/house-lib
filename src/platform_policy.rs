//! The house platform policy, as a PURE function of two names.
//!
//! This file is compiled twice: as `house_lib::platform::policy`, and by
//! `build.rs`, which refuses to build the library (and so every tool that
//! depends on it) for a target outside the accept set. It therefore uses
//! nothing but `std`, reads no environment, and every item in it is used by
//! the build script -- a helper only the library needed would be dead code
//! there, and the gate builds at `-D warnings`.
//!
//! The policy (`target-platform-policy`): 64-bit only; macOS is Apple silicon
//! ONLY; Linux keeps `x86_64` AND `aarch64`. "Drop Intel" is macOS-specific.
//! Every other pair is a hard failure with the rule it hit, never a warning.

/// The accept set, in the spellings an operator recognises.
pub const SUPPORTED: &str = "macOS/arm64, Linux/x86_64, Linux/aarch64";

/// The `uname -s` spelling, with the marketing name mapped in.
fn canonical_os(os: &str) -> &str {
    match os {
        "macOS" => "Darwin",
        other => other,
    }
}

/// The one name per silicon: macOS says `arm64` and Linux `aarch64`; Linux
/// may say `amd64` for `x86_64`.
fn canonical_arch(arch: &str) -> &str {
    match arch {
        "aarch64" => "arm64",
        "amd64" => "x86_64",
        other => other,
    }
}

/// `Ok` for a supported `os`/`arch` (`uname -s`/`uname -m` spellings, or the
/// aliases above), else the reason, naming the rule it hit and the accept set.
///
/// Three rules, checked in order on the canonical names: macOS Intel; an arch
/// that is not 64-bit; an OS that is neither Darwin nor Linux.
///
/// # Errors
///
/// The pair is outside the accept set; the message says which rule refused it.
pub fn check(os: &str, arch: &str) -> Result<(), String> {
    let (os_c, arch_c) = (canonical_os(os), canonical_arch(arch));
    if os_c == "Darwin" && arch_c == "x86_64" {
        return Err(format!(
            "unsupported platform {os}/{arch}: macOS is Apple silicon (arm64) ONLY, Intel Macs are dropped; Linux x86_64 remains supported. Supported: {SUPPORTED}."
        ));
    }
    if !matches!(arch_c, "arm64" | "x86_64") {
        return Err(format!(
            "unsupported platform {os}/{arch}: 64-bit only (arm64, aarch64 or x86_64), 32-bit is never supported. Supported: {SUPPORTED}."
        ));
    }
    if !matches!(os_c, "Darwin" | "Linux") {
        return Err(format!(
            "unsupported platform {os}/{arch}: the OS is neither macOS nor Linux. Supported: {SUPPORTED}."
        ));
    }
    Ok(())
}

/// Cargo's target names as the `uname` spellings [`check`] reads.
///
/// `CARGO_CFG_TARGET_OS`/`CARGO_CFG_TARGET_ARCH` say `macos`, `linux`,
/// `aarch64`, `x86_64`, `x86`, `arm`, ...; a name this does not know is passed
/// through, and [`check`] refuses it by the rule it breaks.
#[must_use]
pub fn from_target_cfg(target_os: &str, target_arch: &str) -> (String, String) {
    let os = match target_os {
        "macos" => "Darwin",
        "linux" => "Linux",
        other => other,
    };
    let arch = match target_arch {
        "aarch64" => "arm64",
        other => other,
    };
    (os.to_owned(), arch.to_owned())
}
