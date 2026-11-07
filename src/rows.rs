//! The dependency rows more than one tool declares, with their pins.
//!
//! ztools and `app_updates` each carried these rows in their own `deps.rs`
//! and moved them in lockstep by hand (ztools `b861af8`/`7c5911c`,
//! `app_updates` `13322f4`/`d68d512`), and they had DRIFTED: ztools' fetch
//! still piped `printf 'y\n'` into the installer, which `app_updates` removed
//! in `de40622` -- the pinned browser is a stable release and asks nothing, and
//! a typed `y` would silently wave through a prerelease pin nobody chose. This
//! file is `app_updates`' form: the fetch types no answer.
//!
//! A consumer keeps the rows as they are, or restates only what differs for it
//! (`needed_for`), with struct update syntax in its own const table:
//!
//! ```
//! use house_lib::{rows, selfheal::Dependency};
//! const TABLE: &[Dependency] = &[
//!     Dependency { needed_for: "installing python3", ..rows::BREW },
//!     rows::PYTHON3,
//!     rows::CAMOUFOX_PACKAGE,
//!     Dependency { needed_for: "the timeline collector", ..rows::CAMOUFOX_BROWSER },
//! ];
//! # assert_eq!(TABLE.len(), 4);
//! ```

use crate::selfheal::{Dependency, Probe};

macro_rules! python_pin {
    () => {
        "3.15"
    };
}
macro_rules! camoufox_pin {
    () => {
        "0.5.9"
    };
}
macro_rules! browser_pin {
    () => {
        "158.0.1-beta.41"
    };
}

/// The Python the Camoufox installer runs under.
pub const PYTHON_PIN: &str = python_pin!();
/// The `camoufox` Python package (it fetches the browser).
pub const CAMOUFOX_PACKAGE_PIN: &str = camoufox_pin!();
/// The Camoufox browser build the tools are verified against. A stable
/// release (`prerelease: false`), so the fetch asks nothing.
pub const CAMOUFOX_BROWSER_PIN: &str = browser_pin!();
/// What `camoufox fetch` is given for [`CAMOUFOX_BROWSER_PIN`].
pub const CAMOUFOX_BROWSER_SPEC: &str = concat!("official/", browser_pin!());
/// The browser row's name; a tool's host answers its `Probe::Located`.
pub const CAMOUFOX_BROWSER_NAME: &str = "camoufox-browser";

/// The first four words of the browser fetch: a shell that runs
/// `camoufox fetch "$1"` with download progress printed every 10%.
///
/// It types no answer (see the module doc). Progress needs camoufox's PRIVATE
/// `pkgman.webdl` (0.5.9 exports no progress hook; the public
/// `python3 -m camoufox fetch` prints none), so the script first checks that
/// `webdl` still takes `bar` and `progress_callback`, and exits non-zero
/// naming the change when it does not -- a loud failure at the pin bump, never
/// a wrapper that silently stops reporting or breaks the fetch.
pub const CAMOUFOX_FETCH_PREFIX: [&str; 4] = [
    "/bin/sh",
    "-c",
    "# camoufox fetch\npython3 -u -c '\nimport inspect,sys\nimport camoufox.pkgman as p\no=getattr(p,\"webdl\",None)\nif o is None or not {\"bar\",\"progress_callback\"}<=set(inspect.signature(o).parameters):sys.exit(\"camoufox.pkgman.webdl changed shape: this fetch wraps that private API for progress; re-check it at the camoufox pin\")\ndef w(u,**k):\n l=[-1]\n def c(d,t):\n  if not t:return\n  p=int(d*100/t)\n  if d==t and l[0]!=100:l[0]=100;print(f\"Camoufox download: 100% ({d/1048576:.1f}/{t/1048576:.1f} MB)\",flush=True)\n  elif p!=l[0] and p>0 and p%10==0:l[0]=p;print(f\"Camoufox download: {p}% ({d/1048576:.1f}/{t/1048576:.1f} MB)\",flush=True)\n return o(u,**{**k,\"bar\":False,\"progress_callback\":c})\np.webdl=w\nfrom camoufox.__main__ import cli\ncli()\n' fetch \"$1\"",
    "sh",
];

/// The whole browser fetch argv for `spec` (e.g. [`CAMOUFOX_BROWSER_SPEC`]).
#[must_use]
pub const fn camoufox_fetch_argv(spec: &str) -> [&str; 5] {
    let [a, b, c, d] = CAMOUFOX_FETCH_PREFIX;
    [a, b, c, d, spec]
}

/// Homebrew. It installs itself interactively, so no tool installs it.
pub const BREW: Dependency = Dependency {
    name: "brew",
    pin: "7.0",
    needed_for: "installing every other row",
    probe: Probe::Command("brew"),
    install: &[],
    requires: &[],
    manual: "Homebrew installs itself interactively; run the installer from https://brew.sh once",
};

/// `python3`, for the Camoufox installer.
pub const PYTHON3: Dependency = Dependency {
    name: "python3",
    pin: PYTHON_PIN,
    needed_for: "the Camoufox installer",
    probe: Probe::Command("python3"),
    install: &["brew", "install", concat!("python@", python_pin!())],
    requires: &["brew"],
    manual: "",
};

/// The `camoufox` Python package, installed at exactly its pin.
pub const CAMOUFOX_PACKAGE: Dependency = Dependency {
    name: "camoufox-python",
    pin: CAMOUFOX_PACKAGE_PIN,
    needed_for: "fetching the Camoufox browser",
    probe: Probe::PythonModule("camoufox"),
    install: &[
        "python3",
        "-m",
        "pip",
        "install",
        "--user",
        "--break-system-packages",
        concat!("camoufox==", camoufox_pin!()),
    ],
    requires: &["python3"],
    manual: "",
};

/// The Camoufox browser, fetched at exactly its pin.
pub const CAMOUFOX_BROWSER: Dependency = Dependency {
    name: CAMOUFOX_BROWSER_NAME,
    pin: CAMOUFOX_BROWSER_PIN,
    needed_for: "the browser-backed paths",
    probe: Probe::Located(CAMOUFOX_BROWSER_NAME),
    install: &camoufox_fetch_argv(CAMOUFOX_BROWSER_SPEC),
    requires: &["camoufox-python"],
    manual: "",
};

#[cfg(test)]
#[path = "rows_tests.rs"]
mod tests;
