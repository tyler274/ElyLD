//! Build DuckDB with ElyLD and run the unittest suite NixOS uses as `installCheck`.
//!
//! Cyrene's `duckdb` installCheck died with SIGSEGV inside `./test/unittest` after
//! `duckdb --version` had already succeeded. stdout is discarded on x86_64, so the
//! failing test name never reached the build log. This test keeps that log.
//!
//! `nix develop` unpacks nixpkgs `duckdb.src` and sets `ELYLD_DUCKDB_TREE`.
//! Otherwise point that variable at a checkout (the Nix failure was v1.5.5,
//! revision d8cdaa33fd). Ignored when the variable is unset so CI stays lean.

use crate::{Filter, build_dir, elyld_b_dir, elyld_path};
use libelyld::error::{Context as _, Result};
use libelyld::{bail, ensure};
use libtest_mimic::Trial;
use object::{Object as _, ObjectSection as _};
use std::fs::{self, File};
use std::io::{Read as _, Seek as _, SeekFrom};
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;

const TREE_VAR: &str = "ELYLD_DUCKDB_TREE";
const BUILD_VAR: &str = "ELYLD_DUCKDB_BUILD";
const DESCRIBE_VAR: &str = "ELYLD_DUCKDB_DESCRIBE";
const TEST_NAME: &str = "elf/x86_64/duckdb";

/// Same filters as nixpkgs `duckdb` `installCheckPhase` on non-aarch64, non-Darwin.
const EXCLUDES: &[&str] = &[
    "[s3]",
    "Test closing database during long running query",
    "Test using a remote optimizer pass in case thats important to someone",
    "test/common/test_cast_hugeint.test",
    "test/sql/copy/csv/test_csv_remote.test",
    "test/sql/copy/parquet/test_parquet_remote.test",
    "test/sql/copy/parquet/test_parquet_remote_foreign_files.test",
    "test/sql/storage/compression/chimp/chimp_read.test",
    "test/sql/storage/compression/chimp/chimp_read_float.test",
    "test/sql/storage/compression/patas/patas_compression_ratio.test_coverage",
    "test/sql/storage/compression/patas/patas_read.test",
    "test/sql/json/read_json_objects.test",
    "test/sql/json/read_json.test",
    "test/sql/json/table/read_json_objects.test",
    "test/sql/json/table/read_json.test",
    "test/sql/copy/parquet/parquet_5968.test",
    "test/fuzzer/pedro/buffer_manager_out_of_memory.test",
    "test/sql/storage/compression/bitpacking/bitpacking_size_calculation.test",
    "test/sql/copy/parquet/delta_byte_array_length_mismatch.test",
    "test/sql/function/timestamp/test_icu_strptime.test",
    "test/sql/timezone/test_icu_timezone.test",
    "test/sql/copy/parquet/snowflake_lineitem.test",
    "test/sql/copy/parquet/test_parquet_force_download.test",
    "test/sql/copy/parquet/delta_byte_array_multiple_pages.test",
    "test/sql/copy/csv/test_csv_httpfs_prepared.test",
    "test/sql/copy/csv/test_csv_httpfs.test",
    "test/sql/settings/test_disabled_file_system_httpfs.test",
    "test/sql/copy/csv/parallel/test_parallel_csv.test",
    "test/sql/copy/csv/parallel/csv_parallel_httpfs.test",
    "test/common/test_cast_struct.test",
    "test/sql/copy/parquet/parquet_glob.test",
    "[!hide]",
    "test/sql/copy/csv/auto/test_csv_auto.test",
    "test/sql/table_function/read_text_and_blob.test",
    "test/sql/copy/parquet/batched_write/batch_memory_usage.test",
    "test/sql/copy/csv/recursive_query_csv.test",
    "test/sql/copy/csv/test_mixed_lines.test",
    "test/parquet/parquet_long_string_stats.test",
    "test/sql/attach/attach_remote.test",
    "test/sql/copy/csv/test_sniff_httpfs.test",
    "test/sql/httpfs/internal_issue_2490.test",
    "test/sql/copy/file_size_bytes.test",
    "test/issues/general/test_17757.test",
];

pub(super) fn collect_tests(tests: &mut Vec<Trial>, filter: &Filter) {
    if filter.excludes(TEST_NAME) {
        return;
    }
    tests.push(Trial::ignorable_test(TEST_NAME, || {
        run_duckdb().map_err(|e| libtest_mimic::Failed::from(e.to_string()))
    }));
}

fn run_duckdb() -> Result<libtest_mimic::Completion> {
    if !cfg!(all(target_arch = "x86_64", target_os = "linux")) {
        return Ok(libtest_mimic::Completion::ignored_with(
            "duckdb installCheck coverage is the x86_64 Linux failure".to_owned(),
        ));
    }
    let Some(tree) = std::env::var_os(TREE_VAR).map(PathBuf::from) else {
        return Ok(libtest_mimic::Completion::ignored_with(format!(
            "{TREE_VAR} is unset"
        )));
    };
    if !tree.join("CMakeLists.txt").is_file() {
        bail!(
            "{TREE_VAR}={} is not a DuckDB source tree",
            tree.display()
        );
    }

    let root = std::env::var_os(BUILD_VAR)
        .map(PathBuf::from)
        .unwrap_or_else(|| build_dir().join("duckdb"));
    fs::create_dir_all(&root).with_context(|| format!("Failed to create {}", root.display()))?;

    let src = ensure_writable_tree(&tree, &root)?;
    let build = root.join("build");
    fs::create_dir_all(&build).with_context(|| format!("Failed to create {}", build.display()))?;
    configure_and_build(&src, &build)?;
    run_unittest(&build)?;
    Ok(libtest_mimic::Completion::Completed)
}

fn ensure_writable_tree(tree: &Path, root: &Path) -> Result<PathBuf> {
    let dest = root.join("src");
    if dest.join("CMakeLists.txt").is_file() {
        return Ok(dest);
    }
    fs::create_dir_all(&dest).with_context(|| format!("Failed to create {}", dest.display()))?;
    let status = Command::new("cp")
        .arg("-a")
        .arg(format!("{}/.", tree.display()))
        .arg(&dest)
        .status()
        .context("Failed to spawn cp for DuckDB source")?;
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
        .context("Failed to spawn chmod for DuckDB source")?;
    if !status.success() {
        bail!("chmod -R u+w {} failed ({status})", dest.display());
    }
    Ok(dest)
}

fn configure_and_build(src: &Path, build: &Path) -> Result {
    let ld_prefix = elyld_b_dir();
    let ld_path = elyld_path().display().to_string();
    let meta = fs::metadata(elyld_path())
        .with_context(|| format!("Failed to stat {}", elyld_path().display()))?;
    let describe = std::env::var(DESCRIBE_VAR).unwrap_or_default();
    let b_flag = format!("-B{}", ld_prefix.display());
    let stamp_payload = format!(
        "{ld_path}\n{:?}\n{}\n{describe}\n{b_flag}\n",
        meta.modified().ok(),
        meta.len()
    );
    let stamp = build.join(".elyld-duckdb-ld");
    let need_configure = !build.join("build.ninja").is_file()
        || fs::read_to_string(&stamp).unwrap_or_default() != stamp_payload;

    if need_configure && build.join("build.ninja").is_file() {
        drop_linked_outputs(build)?;
    }

    if need_configure {
        let mut configure = Command::new("cmake");
        configure.current_dir(build);
        configure.arg("-G").arg("Ninja");
        configure.arg("-S").arg(src);
        configure.arg("-B").arg(build);
        configure.arg("-DCMAKE_BUILD_TYPE=Release");
        configure.args([
            "-DBUILD_UNITTESTS=ON",
            "-DDUCKDB_EXTENSION_CONFIGS=.github/config/in_tree_extensions.cmake",
        ]);
        if !describe.is_empty() {
            configure.arg(format!("-DOVERRIDE_GIT_DESCRIBE={describe}"));
        }
        configure.arg(format!("-DCMAKE_EXE_LINKER_FLAGS={b_flag}"));
        configure.arg(format!("-DCMAKE_SHARED_LINKER_FLAGS={b_flag}"));
        configure.arg(format!("-DCMAKE_MODULE_LINKER_FLAGS={b_flag}"));
        configure.env_remove("NIX_CFLAGS_LINK");
        configure.env("CC", "gcc");
        configure.env("CXX", "g++");
        let status = configure.status().context("Failed to spawn cmake")?;
        if !status.success() {
            bail!("DuckDB cmake failed ({status})");
        }
        fs::write(&stamp, stamp_payload)
            .with_context(|| format!("Failed to write {}", stamp.display()))?;
    }

    let jobs = std::thread::available_parallelism()
        .map(|n| n.get().to_string())
        .unwrap_or_else(|_| "1".to_owned());
    let status = Command::new("ninja")
        .args(["-C", &build.display().to_string(), "-j", &jobs])
        .env_remove("NIX_CFLAGS_LINK")
        .status()
        .context("Failed to spawn ninja")?;
    if !status.success() {
        bail!("DuckDB ninja failed ({status})");
    }

    let unittest = build.join("test/unittest");
    ensure!(
        unittest.is_file(),
        "DuckDB did not build {}",
        unittest.display()
    );
    let comment = readelf_linker_comment(&unittest)?;
    ensure!(
        comment.contains("ElyLD"),
        "unittest was not linked by ElyLD (.comment: {comment})"
    );
    Ok(())
}

fn drop_linked_outputs(build: &Path) -> Result {
    let mut stack = vec![build.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir)
            .with_context(|| format!("Failed to read {}", dir.display()))?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let ft = entry.file_type()?;
            if ft.is_dir() {
                stack.push(path);
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name == "unittest" || name.contains(".so") {
                fs::remove_file(&path)
                    .with_context(|| format!("Failed to remove {}", path.display()))?;
            }
        }
    }
    Ok(())
}

fn run_unittest(build: &Path) -> Result {
    let runner = build.join("run-unittest.sh");
    let mut body = String::from("#!/bin/sh\nset -u\n");
    body.push_str("if [ \"$(ulimit -n)\" -lt 1024 ]; then ulimit -n 1024; fi\n");
    body.push_str("export HOME=\"$(mktemp -d)\"\n");
    if let Some(lib) = find_libduckdb(build) {
        body.push_str(&format!(
            "export LD_LIBRARY_PATH=\"{}:${{LD_LIBRARY_PATH-}}\"\n",
            lib.display()
        ));
    }
    body.push_str("exec ./test/unittest");
    for exclude in EXCLUDES {
        body.push(' ');
        body.push_str(&shell_single_quote(&format!("exclude:{exclude}")));
    }
    body.push('\n');
    fs::write(&runner, body).with_context(|| format!("Failed to write {}", runner.display()))?;
    let mut perms = fs::metadata(&runner)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&runner, perms)?;

    let log = build.join("unittest.log");
    let status = Command::new("script")
        .args([
            "-q",
            "-e",
            "-c",
            "./run-unittest.sh",
            "unittest.log",
        ])
        .current_dir(build)
        .env_remove("NIX_CFLAGS_LINK")
        .status()
        .context("Failed to spawn script for DuckDB unittest")?;
    if !status.success() {
        bail!(
            "DuckDB unittest failed ({status}). Tail of {}:\n{}",
            log.display(),
            log_tail(&log)
        );
    }
    Ok(())
}

fn find_libduckdb(build: &Path) -> Option<PathBuf> {
    let mut stack = vec![build.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.file_name().and_then(|n| n.to_str()) == Some("libduckdb.so") {
                return path.parent().map(Path::to_path_buf);
            }
        }
    }
    None
}

fn shell_single_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

fn log_tail(path: &Path) -> String {
    let Ok(mut file) = File::open(path) else {
        return "(no log)".to_owned();
    };
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    let start = len.saturating_sub(8 * 1024);
    if file.seek(SeekFrom::Start(start)).is_err() {
        return "(unreadable log)".to_owned();
    }
    let mut buf = String::new();
    if file.read_to_string(&mut buf).is_err() {
        return "(unreadable log)".to_owned();
    }
    buf
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
