//! The platform policy at BUILD time: the library refuses to compile for a
//! target outside the house accept set, so no tool that depends on it can ship
//! a binary for a platform nobody tested (`target-platform-policy`: a hard
//! failure, never an arch fallback that warns and builds anyway).
//!
//! The decision is `src/platform_policy.rs`'s, the same file the library
//! exports, so the build gate and the runtime check cannot disagree. Its seam
//! for tests is the pure `from_target_cfg` + `check` pair, driven in both
//! directions by `src/platform_tests.rs`; this file only reads cargo's names.

#[path = "src/platform_policy.rs"]
mod policy;

fn main() {
    println!("cargo::rerun-if-changed=src/platform_policy.rs");
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let (os, arch) = policy::from_target_cfg(&os, &arch);
    if let Err(reason) = policy::check(&os, &arch) {
        println!("cargo::error=house-lib refuses this target: {reason}");
    }
    // Named so a reader of the build log sees the accept set was consulted.
    println!("cargo::rustc-env=HOUSE_LIB_SUPPORTED={}", policy::SUPPORTED);
}
