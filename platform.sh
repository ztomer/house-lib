# shellcheck shell=bash
# platform.sh -- the house platform policy, shell form. SOURCE it; it runs nothing.
#
#   . platform.sh
#   house_platform_check "${MYTOOL_OS:-$(uname -s)}" "${MYTOOL_ARCH:-$(uname -m)}" || exit 1
#
# house_platform_check OS ARCH prints nothing and returns 0 for a supported pair,
# else prints the reason on stderr (naming the rule it hit) and returns 1. The
# caller owns the seam (its own OS/ARCH override variables) and the exit.
#
# This is the SAME decision as src/platform_policy.rs, held to it over every
# spelling by src/platform_tests.rs: change one and that test is red until the
# other agrees. 64-bit only; macOS is Apple silicon ONLY; Linux keeps x86_64 and
# aarch64. Every other pair is a hard failure, never a warn-and-build fallback.

HOUSE_PLATFORM_SUPPORTED="macOS/arm64, Linux/x86_64, Linux/aarch64"

house_platform_check() {
    local os="$1" arch="$2" os_c arch_c
    case "$os" in
        macOS) os_c=Darwin ;;
        *) os_c="$os" ;;
    esac
    case "$arch" in
        aarch64) arch_c=arm64 ;;
        amd64) arch_c=x86_64 ;;
        *) arch_c="$arch" ;;
    esac
    if [ "$os_c" = Darwin ] && [ "$arch_c" = x86_64 ]; then
        printf '%s\n' "unsupported platform $os/$arch: macOS is Apple silicon (arm64) ONLY, Intel Macs are dropped; Linux x86_64 remains supported. Supported: $HOUSE_PLATFORM_SUPPORTED." >&2
        return 1
    fi
    case "$arch_c" in
        arm64 | x86_64) ;;
        *)
            printf '%s\n' "unsupported platform $os/$arch: 64-bit only (arm64, aarch64 or x86_64), 32-bit is never supported. Supported: $HOUSE_PLATFORM_SUPPORTED." >&2
            return 1
            ;;
    esac
    case "$os_c" in
        Darwin | Linux) ;;
        *)
            printf '%s\n' "unsupported platform $os/$arch: the OS is neither macOS nor Linux. Supported: $HOUSE_PLATFORM_SUPPORTED." >&2
            return 1
            ;;
    esac
    return 0
}
