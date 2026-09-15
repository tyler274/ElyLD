//! Opt-in userspace package links captured with `ELYLD_SAVE_BASE`.
//!
//! Each `ELYLD_*_LINK` is a save-dir containing `run-with` (see BENCHMARKING.md),
//! or the `run-with` script itself. Unset variables skip. These links are too
//! large for the 10-minute CI matrix; run them locally after capturing a link.

use crate::{Filter, elyld_path};
use libtest_mimic::Trial;
use libelyld::bail;
use libelyld::error::{Context as _, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

const PACKAGES: &[(&str, &str)] = &[
    ("elf/package/python", "ELYLD_PYTHON_LINK"),
    ("elf/package/rustc", "ELYLD_RUSTC_LINK"),
    ("elf/package/gcc", "ELYLD_GCC_LINK"),
    ("elf/package/llvm", "ELYLD_LLVM_LINK"),
    ("elf/package/firefox", "ELYLD_FIREFOX_LINK"),
    ("elf/package/blender", "ELYLD_BLENDER_LINK"),
    ("elf/package/chrome", "ELYLD_CHROME_LINK"),
];

pub(super) fn collect_tests(tests: &mut Vec<Trial>, filter: &Filter) {
    for (name, var) in PACKAGES {
        if filter.excludes(name) {
            continue;
        }
        let name = *name;
        let var = *var;
        tests.push(Trial::ignorable_test(name, move || {
            run_package_link(name, var).map_err(|e| libtest_mimic::Failed::from(e.to_string()))
        }));
    }
}

fn run_package_link(name: &str, var: &str) -> Result<libtest_mimic::Completion> {
    let Some(dir) = std::env::var_os(var).map(PathBuf::from) else {
        return Ok(libtest_mimic::Completion::ignored_with(format!(
            "{var} is unset"
        )));
    };
    let Some(run_with) = run_with_path(&dir) else {
        bail!(
            "{var} is set to `{}` but has no run-with script (capture a link with ELYLD_SAVE_BASE)",
            dir.display()
        );
    };
    let status = Command::new(&run_with)
        .arg(elyld_path())
        .status()
        .with_context(|| format!("Failed to spawn {} for {name}", run_with.display()))?;
    if !status.success() {
        bail!(
            "{name}: `{}` with ElyLD failed ({status})",
            run_with.display()
        );
    }
    Ok(libtest_mimic::Completion::Completed)
}

fn run_with_path(dir: &Path) -> Option<PathBuf> {
    let nested = dir.join("run-with");
    if nested.is_file() {
        return Some(nested);
    }
    if dir.is_file() {
        return Some(dir.to_path_buf());
    }
    None
}
