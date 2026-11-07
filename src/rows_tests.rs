use super::*;

use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const CHAIN: [Dependency; 4] = [BREW, PYTHON3, CAMOUFOX_PACKAGE, CAMOUFOX_BROWSER];

#[test]
fn every_prerequisite_names_a_row_declared_before_it() {
    for (i, dep) in CHAIN.iter().enumerate() {
        for req in dep.requires {
            let at = CHAIN.iter().position(|d| d.name == *req);
            assert!(at.is_some_and(|a| a < i), "{} needs {req}", dep.name);
        }
        assert!(
            !dep.install.is_empty() || !dep.manual.is_empty(),
            "{}: a row with no install command must say why",
            dep.name
        );
    }
}

#[test]
fn every_installable_row_installs_exactly_its_pin() {
    assert_eq!(PYTHON3.install.last(), Some(&"python@3.15"));
    assert_eq!(CAMOUFOX_PACKAGE.install.last(), Some(&"camoufox==0.5.9"));
    assert_eq!(
        CAMOUFOX_BROWSER.install.last(),
        Some(&"official/158.0.1-beta.41")
    );
    for dep in &CHAIN[1..] {
        assert!(
            dep.install.last().unwrap().ends_with(dep.pin),
            "{}: installs {:?}, pinned {}",
            dep.name,
            dep.install.last(),
            dep.pin
        );
    }
}

#[test]
fn the_fetch_argv_for_any_spec_ends_in_that_spec() {
    let argv = camoufox_fetch_argv("official/prerelease/9.9");
    assert_eq!(argv.last(), Some(&"official/prerelease/9.9"));
    assert_eq!(argv[..4], CAMOUFOX_FETCH_PREFIX);
    assert_eq!(CAMOUFOX_BROWSER.install[..4], CAMOUFOX_FETCH_PREFIX);
}

fn executable(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// The browser row's install, run the way a tool runs it: no terminal, stdin
/// the null device, `PATH` and `PYTHONPATH` as given.
fn run_fetch(path: &str, pythonpath: Option<&Path>) -> Output {
    let (program, args) = CAMOUFOX_BROWSER.install.split_first().unwrap();
    let mut fetch = Command::new(program);
    fetch
        .args(args)
        .env("PATH", path)
        .env_remove("PYTHONPATH")
        .stdin(Stdio::null());
    if let Some(dir) = pythonpath {
        fetch.env("PYTHONPATH", dir);
    }
    fetch.output().unwrap()
}

/// A stand-in `camoufox` package whose `webdl` takes the parameters in
/// `params`, reports progress through the callback when it has one, and
/// whose `cli` prints what it was asked to fetch.
fn fake_camoufox(dir: &Path, params: &str) {
    let pkg = dir.join("camoufox");
    std::fs::create_dir(&pkg).unwrap();
    std::fs::write(pkg.join("__init__.py"), "").unwrap();
    std::fs::write(
        pkg.join("pkgman.py"),
        format!(
            "def webdl({params}):\n    cb = locals().get('progress_callback')\n    if cb:\n        for d in (5, 50, 73, 100):\n            cb(d * 1048576, 100 * 1048576)\n"
        ),
    )
    .unwrap();
    std::fs::write(
        pkg.join("__main__.py"),
        "import sys\nimport camoufox.pkgman as p\ndef cli():\n    p.webdl('https://example.invalid/b.zip')\n    print('fetched', sys.argv[1:])\n",
    )
    .unwrap();
}

/// Why: a `y` piped into the installer answers a prompt the pinned stable
/// build never asks, and would wave through a prerelease pin nobody chose.
/// The drift this library ended: ztools' copy still piped `printf 'y\n'`.
#[test]
fn the_browser_fetch_types_no_answer_into_the_installer() {
    let dir = tempfile::tempdir().unwrap();
    let (heard, ran) = (dir.path().join("heard"), dir.path().join("ran"));
    executable(
        &dir.path().join("python3"),
        &format!(
            "#!/bin/sh\n: > '{}'\nif read answer; then printf '%s' \"$answer\" > '{}'; fi\nexit 0\n",
            ran.display(),
            heard.display()
        ),
    );
    let out = run_fetch(&format!("{}:/usr/bin:/bin", dir.path().display()), None);
    assert!(out.status.success(), "{out:?}");
    assert!(ran.exists(), "the fetch never reached python3");
    assert!(
        !heard.exists(),
        "the fetch typed {:?} into the installer",
        std::fs::read_to_string(&heard).unwrap_or_default()
    );
}

fn real_path() -> String {
    std::env::var("PATH").unwrap_or_default()
}

#[test]
fn the_fetch_prints_progress_every_tenth_and_fetches_the_pinned_spec() {
    let dir = tempfile::tempdir().unwrap();
    fake_camoufox(
        dir.path(),
        "url, desc=None, buffer=None, bar=True, progress_callback=None",
    );
    let out = run_fetch(&real_path(), Some(dir.path()));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{text}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines,
        [
            "Camoufox download: 50% (50.0/100.0 MB)",
            "Camoufox download: 100% (100.0/100.0 MB)",
            "fetched ['fetch', 'official/158.0.1-beta.41']",
        ]
    );
}

#[test]
fn a_webdl_that_changed_shape_fails_the_fetch_loudly() {
    let dir = tempfile::tempdir().unwrap();
    fake_camoufox(dir.path(), "url, desc=None, buffer=None");
    let out = run_fetch(&real_path(), Some(dir.path()));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "a changed private API must not pass");
    assert!(err.contains("webdl changed shape"), "{err}");
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("fetched"),
        "nothing is fetched past the refusal"
    );
}
