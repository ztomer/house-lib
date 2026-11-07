# house-lib

The PRIVATE shared Rust library of the house tools (queue item LIB.1; owner rulings
OW.20/OW.22/OW.22b: "new private library for common tasks. keep GOH QA only").
It holds the code ztools, sys_updater and app_updates each carried as a tracked copy.
Gates, checkers and the TUI are QA and stay in `gates_of_heck` (OW.85: goh-tui stays there).

| module | what | came from |
|---|---|---|
| `selfheal` | probe, install with a stated reason, re-probe, degrade | all three, byte-identical |
| `rows` | brew, python3, Camoufox package + browser rows, pins, fetch script | ztools + app_updates `deps.rs` |
| `platform` | the house platform policy (+ `build.rs`, `platform.sh`) | ztools `install.sh`, sys_updater `platform.rs` + `scripts/build.sh`, app_updates `build.sh` |
| `expand` | the one reader of a leading `~` | sys_updater + app_updates, byte-identical |
| `spawn` | the one place a child process is built; terminal modes | sys_updater `spawn.rs` (10cbd86) + app_updates `core::spawn` (d5caf15) |
| `which` | executable lookup with the search path as a seam | sys_updater `deps.rs` |

## Use

```toml
[dependencies]
house-lib = { git = "https://github.com/ztomer/house-lib", tag = "v0.1.0" }
```

A tag, never a branch or a path: a path dependency on a sibling checkout makes a clean
clone unbuildable (the release kit builds from `git archive HEAD`). The repo is private,
so cargo needs credentials wherever it fetches it: locally, git's credential helper
(`gh auth git-credential`, already in `~/.gitconfig`) answers cargo's HTTPS fetch.
CI and any Homebrew build need their own -- see "Open: credentials" below.

## Drift found when the copies were diffed (2026-11-07)

- `selfheal.rs`: identical in all three (217 lines). `selfheal_tests.rs`: identical in
  ztools and sys_updater; app_updates' differs only in rustfmt's `assert!` layout.
- `expand.rs`: byte-identical in sys_updater and app_updates.
- Camoufox rows: the pins agreed (0.5.9 / 158.0.1-beta.41) but the fetch script did NOT:
  ztools' still piped `printf 'y\n'` into the installer, which app_updates removed in
  `de40622` (the pinned build is a stable release and asks nothing; a typed `y` would wave
  through a prerelease pin nobody chose). The library takes app_updates' form, and adds a
  probe: the script exits non-zero, naming the change, when camoufox's PRIVATE
  `pkgman.webdl` stops taking `bar`/`progress_callback` (the private-API debt app_updates'
  ROADMAP 8.1 named).
- Platform policy: the accept set agreed in substance, the spellings did not.
  app_updates' `build.sh` accepted `arm64|aarch64|x86_64` and refused `amd64`;
  sys_updater accepted `amd64` and (in `platform.rs`) `macOS`; ztools' `install.sh`
  accepted `amd64` but not `macOS`. The library accepts every spelling of the three pairs
  (`Darwin|macOS`, `arm64|aarch64`, `x86_64|amd64`) and refuses everything else by the rule
  it hits.
- Spawn: two ports of one rule. **sys_updater's won** as the base: its three named modes
  (`Detached`, `OwnGroup`, `Handover`) are the superset -- sys_updater's sudo-ticket and
  handover steps cannot be expressed in app_updates' detached-only port, while that one
  mode is `Detached` here. From app_updates' port the library takes `enter()` (the
  trampoline-or-install decision every `main` makes, testable as `enter_with`) and the null
  stdin for a detached child, which sys_updater's lacked: a child that has left the
  terminal's session but still holds the terminal as fd 0 can read keystrokes meant for
  the tool, with no SIGTTIN to stop it.

## The platform policy, three forms of one decision

- `src/platform_policy.rs` -- `check(os, arch)`, pure. Tested in both directions over every
  spelling (`src/platform_tests.rs`).
- `build.rs` -- the same file, compiled into the build script: the library refuses to build
  for a target outside the set, so no consumer can either. Measured: `cargo check --target
  wasm32-unknown-unknown` fails with "house-lib refuses this target"; the two Linux musl
  targets build.
- `platform.sh` -- the shell form for a consumer's build/install script
  (`house_platform_check OS ARCH`). `platform_tests.rs` runs it in bash over the whole matrix
  and requires the Rust verdict AND the same reason text, so the two cannot drift. A
  consumer's script finds the file through cargo, never a sibling path:
  `cargo metadata --format-version 1 | jq -r '.packages[] | select(.name=="house-lib") |
  .manifest_path'`, then sources `platform.sh` beside it. The seam (its own `*_OS`/`*_ARCH`
  overrides) stays the consumer's.

## Gates

`tools/gate.sh --full` (the pre-push hook and CI): goh's `rust_gate.sh` (fmt, clippy
`-D warnings` with pedantic + nursery, both Linux targets, machete, no `#[allow]`/`#[expect]`,
coverage floor 95), `cargo audit` denying warnings/unmaintained/unsound/yanked (also in
`.cargo/audit.toml`; calibrated by adding `atty` and watching it go red), goh structural,
`cargo test`. `unsafe_code` is forbidden.

## Open: credentials (owner decision, blocks the consumer migrations)

A private git dependency needs read credentials wherever cargo fetches it:

1. **CI of every consumer.** The job token of one repo cannot read another private repo. It
   needs a read-only deploy key on this repo plus an Actions secret in each consumer (or a
   fine-grained PAT), and a checkout step that hands it to cargo
   (`CARGO_NET_GIT_FETCH_WITH_CLI=true` + an ssh agent, or an `insteadOf` rewrite). Creating
   the key and the secrets is a credential change the owner makes or approves.
2. **ztools is PUBLIC, and so is its Homebrew tap** (`ztomer/homebrew-tap`, formula builds
   from the tag tarball with `cargo install`). A private dependency makes that formula
   unbuildable for anyone without access to this repo, and Homebrew's build environment
   does not carry the owner's git credential helper either. One of: make this library
   public; ship the tap as a prebuilt bottle/binary; or make ztools and its tap private.

## Part 2 (what the next agent must do)

Status after part 1: the library is tagged `v0.1.0`; ztools' migration is prepared on the
local branch `lib1-house-lib` in ztools (NOT pushed to main; see "Open: credentials"). Once
the owner has answered both questions above, for EACH of ztools, sys_updater, app_updates:

1. Add `house-lib = { git = "https://github.com/ztomer/house-lib", tag = "v0.1.0" }`.
2. Delete the copies and import the library's:
   - `selfheal.rs` + `selfheal_tests.rs` -> `house_lib::selfheal` (all three).
   - `expand.rs` (sys_updater `crates/sys_updater_lib/src/expand.rs`, app_updates
     `crates/core/src/config/expand.rs`) -> `house_lib::expand::{expand_home, OtherUsersHome}`.
   - `platform.rs` (sys_updater) -> `house_lib::platform::{check, current_from, current}`;
     its seam names stay `SYS_UPDATER_BUILD_OS`/`_ARCH`, passed to `current(os_var, arch_var)`.
   - shell gates (`ztools/install.sh`, `sys_updater/scripts/build.sh` +
     `scripts/install.sh`, `app_updates/build.sh`) -> source the library's `platform.sh` as
     above and call `house_platform_check`; keep each repo's seam variables and its
     `test_*_platform_gate.py`, whose expected messages change to the library's text.
   - spawn: sys_updater `spawn.rs` -> `house_lib::spawn` (same API, `Terminal` modes; its
     `__detach` usage text is now generic). app_updates `core::spawn::command(p)` ->
     `house_lib::spawn::command(p, Terminal::Detached)`, `enter()` unchanged;
     `core::spawn::find_on_path` -> `house_lib::which::find_on_path`. Each repo KEEPS its own
     class gate (`spawn_gate_tests.rs` / the scan in `spawn_tests.rs`), now naming
     `house_lib::spawn` as the one place, and its pty `tty_isolation` test.
   - `deps.rs` Camoufox rows (ztools, app_updates) -> `house_lib::rows::{BREW, PYTHON3,
     CAMOUFOX_PACKAGE, CAMOUFOX_BROWSER}` with `..row` struct update for `needed_for`;
     delete the local `CAMOUFOX_FETCH_PREFIX`/pins. app_updates' `deps_tests.rs` fetch tests
     move out (the library carries them). sys_updater has no Camoufox rows.
3. CI: the credential step from "Open: credentials" (1).
4. `tools/gate.sh --full` green, push through the hooks, release with the repo's own
   release script, install, and check the installed version.
5. Bump the tag here (`vX.Y.Z`) for any library change, and move each consumer's tag
   deliberately in its own commit.
