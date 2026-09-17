//! Build and run ianlancetaylor/libbacktrace's test suite with ElyLD as `ld`.
//!
//! Cyrene's checkPhase failed on `ctestzstd` / `ctestzstd_alloc` ("missing file
//! name or function name") because libbacktrace ships its own RFC 8878 decoder
//! that rejects windowed zstd frames. This test compiles that suite with `-B`
//! pointing at ElyLD so `ctestzstd` is actually linked by us.
//!
//! `nix develop` unpacks nixpkgs `libbacktrace.src` and sets
//! `ELYLD_LIBBACKTRACE_TREE`. Otherwise set that variable to a checkout.
//! Ignored when the variable is unset so the 10-minute CI matrix stays lean.

use crate::{Filter, build_dir, elyld_b_dir, elyld_path};
use libelyld::error::{Context as _, Result};
use libelyld::{bail, ensure};
use libtest_mimic::Trial;
use object::{Object as _, ObjectSection as _};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const TREE_VAR: &str = "ELYLD_LIBBACKTRACE_TREE";
const BUILD_VAR: &str = "ELYLD_LIBBACKTRACE_BUILD";
const TEST_NAME: &str = "elf/x86_64/libbacktrace";

pub(super) fn collect_tests(tests: &mut Vec<Trial>, filter: &Filter) {
    if filter.excludes(TEST_NAME) {
        return;
    }
    tests.push(Trial::ignorable_test(TEST_NAME, || {
        run_libbacktrace().map_err(|e| libtest_mimic::Failed::from(e.to_string()))
    }));
}

fn run_libbacktrace() -> Result<libtest_mimic::Completion> {
    let Some(tree) = std::env::var_os(TREE_VAR).map(PathBuf::from) else {
        return Ok(libtest_mimic::Completion::ignored_with(format!(
            "{TREE_VAR} is unset"
        )));
    };
    if !tree.join("elf.c").is_file() {
        bail!(
            "{TREE_VAR}={} is not a libbacktrace source tree",
            tree.display()
        );
    }

    let build = std::env::var_os(BUILD_VAR)
        .map(PathBuf::from)
        .unwrap_or_else(|| build_dir().join("libbacktrace"));
    fs::create_dir_all(&build).with_context(|| format!("Failed to create {}", build.display()))?;

    let src = ensure_writable_tree(&tree, &build)?;
    configure_and_check(&src, &build)?;
    Ok(libtest_mimic::Completion::Completed)
}

fn ensure_writable_tree(tree: &Path, build: &Path) -> Result<PathBuf> {
    let dest = build.join("src");
    if dest.join("elf.c").is_file() {
        let _ = Command::new("chmod")
            .args(["-R", "u+w"])
            .arg(&dest)
            .status();
        return Ok(dest);
    }
    fs::create_dir_all(&dest).with_context(|| format!("Failed to create {}", dest.display()))?;
    let status = Command::new("cp")
        .arg("-a")
        .arg(format!("{}/.", tree.display()))
        .arg(&dest)
        .status()
        .context("Failed to spawn cp for libbacktrace source")?;
    if !status.success() {
        bail!(
            "cp -a {} -> {} failed ({status})",
            tree.display(),
            dest.display()
        );
    }
    let status = Command::new("chmod")
        .args(["-R", "u+w"])
        .arg(&dest)
        .status()
        .context("Failed to spawn chmod for libbacktrace source")?;
    if !status.success() {
        bail!("chmod -R u+w {} failed ({status})", dest.display());
    }
    if !dest.join("configure").is_file() {
        let status = Command::new("autoreconf")
            .args(["-fi"])
            .current_dir(&dest)
            .status()
            .context("Failed to spawn autoreconf")?;
        if !status.success() {
            bail!("autoreconf -fi in {} failed ({status})", dest.display());
        }
    }
    Ok(dest)
}

fn configure_and_check(src: &Path, build: &Path) -> Result {
    let ld_prefix = elyld_b_dir();
    let stamp = build.join(".elyld-libbacktrace-ld");
    let ld_path = elyld_path().display().to_string();
    let cflags = compiler_flags();
    let ldflags = format!(
        "{} -B{} {}",
        std::env::var("LDFLAGS").unwrap_or_default(),
        ld_prefix.display(),
        pkg_config_libs(&["zlib", "libzstd", "liblzma"])
    );
    let stamp_payload = format!("{ld_path}\n{cflags}\n{ldflags}\n");
    let need_configure = !build.join("Makefile").is_file()
        || fs::read_to_string(&stamp).unwrap_or_default() != stamp_payload;

    if need_configure {
        let mut configure = Command::new(src.join("configure"));
        configure.current_dir(build);
        configure.env("CC", "gcc");
        configure.env("CFLAGS", &cflags);
        configure.env("CPPFLAGS", &cflags);
        configure.env("LDFLAGS", &ldflags);
        configure.env_remove("LD");
        let status = configure
            .status()
            .with_context(|| format!("Failed to spawn {}", src.join("configure").display()))?;
        if !status.success() {
            bail!("libbacktrace configure failed ({status})");
        }
        fs::write(&stamp, stamp_payload)
            .with_context(|| format!("Failed to write {}", stamp.display()))?;
    }

    let status = Command::new("make")
        .args(["-C", &build.display().to_string(), "-j", &nproc(), "check"])
        .env("CC", "gcc")
        .env("CFLAGS", &cflags)
        .env("CPPFLAGS", &cflags)
        .env("LDFLAGS", &ldflags)
        .env_remove("LD")
        .status()
        .context("Failed to spawn make check for libbacktrace")?;
    if !status.success() {
        bail!("libbacktrace make check failed ({status})");
    }

    for binary in ["ctestzstd", "ctestzstd_alloc"] {
        let path = build.join(binary);
        ensure!(
            path.is_file(),
            "libbacktrace did not build `{binary}` (HAVE_COMPRESSED_DEBUG_ZSTD missing?); \
             ElyLD must accept --compress-debug-sections=zstd"
        );
        let linked_with = readelf_linker_comment(&path)?;
        if !linked_with.is_empty() {
            eprintln!("{binary} .comment: {linked_with}");
        }
        let status = Command::new(&path)
            .current_dir(build)
            .status()
            .with_context(|| format!("Failed to spawn {}", path.display()))?;
        if !status.success() {
            bail!("{binary} failed ({status})");
        }
    }
    Ok(())
}

fn nproc() -> String {
    std::thread::available_parallelism()
        .map(|n| n.get().to_string())
        .unwrap_or_else(|_| "1".to_owned())
}

fn compiler_flags() -> String {
    format!(
        "{} -g -O2 {}",
        std::env::var("CFLAGS").unwrap_or_default(),
        pkg_config_cflags(&["zlib", "libzstd", "liblzma"])
    )
}

fn pkg_config_cflags(pkgs: &[&str]) -> String {
    pkg_config(pkgs, "--cflags")
}

fn pkg_config_libs(pkgs: &[&str]) -> String {
    pkg_config(pkgs, "--libs")
}

fn pkg_config(pkgs: &[&str], kind: &str) -> String {
    let mut out = String::new();
    for pkg in pkgs {
        let Ok(output) = Command::new("pkg-config").args([kind, pkg]).output() else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        out.push(' ');
        out.push_str(String::from_utf8_lossy(&output.stdout).trim());
    }
    out
}

fn readelf_linker_comment(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let Ok(obj) = object::File::parse(bytes.as_slice()) else {
        return Ok(String::new());
    };
    let Some(section) = obj.section_by_name(".comment") else {
        return Ok(String::new());
    };
    let Ok(data) = section.data() else {
        return Ok(String::new());
    };
    Ok(String::from_utf8_lossy(data)
        .split('\0')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("; "))
}
