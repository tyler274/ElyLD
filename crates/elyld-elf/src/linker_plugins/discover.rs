use std::path::PathBuf;
use elyld_error::error::Result;

pub(super) fn discover_llvm_gold_plugin() -> Result<PathBuf> {
    let rustc_llvm = rustc_llvm_version();
    let mut candidates = Vec::new();

    if let Some(sysroot) = command_stdout_trim("rustc", &["--print", "sysroot"]) {
        candidates.push(PathBuf::from(sysroot).join("lib/LLVMgold.so"));
    }
    if let Some(libdir) = command_stdout_trim("llvm-config", &["--libdir"]) {
        candidates.push(PathBuf::from(libdir).join("LLVMgold.so"));
    }

    if let Ok(entries) = std::fs::read_dir("/usr/lib") {
        let mut llvm_dirs: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("llvm-"))
            })
            .collect();
        llvm_dirs.sort();
        llvm_dirs.reverse();
        for dir in llvm_dirs {
            candidates.push(dir.join("lib/LLVMgold.so"));
        }
    }

    if let Some(version) = rustc_llvm
        && let Some(matched) = candidates
            .iter()
            .find(|p| p.to_string_lossy().contains(&format!("llvm-{version}")) && p.is_file())
    {
        return Ok(matched.clone());
    }

    candidates.into_iter().find(|p| p.is_file()).ok_or_else(|| {
        elyld_error::error!(
            "Input file contains LLVM-IR, but linker plugin was not supplied and LLVMgold.so could not be found"
        )
    })
}

pub(super) fn discover_gcc_lto_plugin() -> Result<PathBuf> {
    let mut compilers = Vec::new();
    if let Ok(cc) = std::env::var("CC")
        && let Some(first) = cc.split_whitespace().next()
        && !first.is_empty()
    {
        compilers.push(first.to_owned());
    }
    for name in ["gcc", "cc", "g++"] {
        if !compilers.iter().any(|c| c == name) {
            compilers.push(name.to_owned());
        }
    }

    for compiler in &compilers {
        if let Some(path) = gcc_plugin_from_compiler(compiler) {
            return Ok(path);
        }
    }
    elyld_error::bail!(
        "Input file contains GCC-IR, but linker plugin was not supplied and liblto_plugin.so could not be found"
    )
}

fn gcc_plugin_from_compiler(compiler: &str) -> Option<PathBuf> {
    if let Some(path) = command_stdout_trim(compiler, &["-print-file-name=liblto_plugin.so"]) {
        let p = PathBuf::from(&path);
        if p.is_file() {
            return Some(p);
        }
    }
    // Nix/wrapped gcc often reports the bare name for `-print-file-name=liblto_plugin.so`
    // even when the plugin sits next to `cc1` / `lto-wrapper` in libexec.
    for probe in ["cc1", "lto-wrapper"] {
        let arg = format!("-print-file-name={probe}");
        if let Some(path) = command_stdout_trim(compiler, &[&arg]) {
            let p = PathBuf::from(path);
            if p.is_file() {
                let plugin = p.parent()?.join("liblto_plugin.so");
                if plugin.is_file() {
                    return Some(plugin);
                }
            }
        }
    }
    None
}

fn rustc_llvm_version() -> Option<u32> {
    let verbose = command_stdout_trim("rustc", &["--version", "--verbose"])?;
    for line in verbose.lines() {
        if let Some(rest) = line.strip_prefix("LLVM version: ")
            && let Some(major) = rest.split('.').next()
            && let Ok(v) = major.parse()
        {
            return Some(v);
        }
    }
    None
}

fn command_stdout_trim(program: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}
