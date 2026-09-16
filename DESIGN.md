# Design

This document provides a high level overview of ElyLD's design. The intent is to not go into too much
detail, otherwise we increase the risk that it'll get out-of-sync with the code. For full details,
see comments in the code and the code itself.

## Phases

The linker runs several phases. Each phase borrows data immutably from the previous phases. The high
level phases are:

* `args.rs`: Parse command-line arguments.
* `input_data.rs`: Open input files with mmap and split archives into their separate objects.
* `string_merging.rs`: Strings in string-merge sections are deduplicated.
* `symbol_db.rs`: Build a hashmap from symbol names to symbol IDs.
* `resolution.rs`: Resolve all undefined symbols and in the process decide which archived objects
  will be processed.
* `layout/`:
  * Traverse graph of relocations, in the process, determining which input sections are needed and
    how much space is needed in the various linker-generated sections such as the GOT (global offset
    table), symbol tables, dynamic relocation tables etc.
  * Allocate addresses for sections, symbols, program segments etc.
* `elf_writer/`: Copy input sections to the output file, applying relocations as we go. Write
  linker-generated sections.

For a more detailed look at the phases of the linker, run with the `--time` flag.

## Incremental linking

`--incremental` keeps dense `SymbolId` / `FileId` indexes for the GC graph (those are rebuilt every
link). Cross-run identity is a generational atom table in `incremental/`: unchanged inputs reuse a
handle, a replaced path reuses a slot with a new generation, and reverse-reloc lists plus
resolutions are keyed by `(atom, local symbol)` so a neighboring file cannot reshuffle IDs. Skip
updates merge reverse-reloc lists, replacing sites in rewritten objects and keeping sites in skipped
ones. GC and GCC LTO (WPA) still fall back to a full padded link. LLVM ThinLTO plugin objects are
restaged into `{output}.incr/plugin/` so skip_payloads can match stable paths. Custom linker scripts
skip section padding so kernel `ASSERT`s keep their sizes; unchanged inputs can still skip payloads.

Integration tests cover GCC, Clang, and rustc at `-O0`/`-O1`/`-O2`/`-O3`/`-Os` (GCC LTO still
falls back; Clang ThinLTO restages plugin objects) and an unchanged `--incremental` relink of
x86_64 `vmlinux` (`ELYLD_LINUX_TREE`). Rust `--emit=obj`
keeps a stable `.o` path; rustc save-dir tests allow fallback because codegen-unit hashes change
with `--cfg wild_inc`. Unchanged `--incremental` relinks of glibc `ld.so` / `libc.so` / `libm.so`
(`ELYLD_GLIBC_TREE`) skip object payloads; skip updates still rewrite dynamic reloc tables so
`.rela.dyn` / `.relr.dyn` stay in lockstep with layout. Input `.interp` objects (glibc `interp.os`)
are laid out like other allocated sections.

## Threading

The linker makes extensive use of multiple threads. The thread pool is owned by the rayon library.
Where possible, we use functions like rayon's `par_iter` to process collections in parallel. Failing
that, we use `par_bridge` which allows the main thread to create work to send out to the thread
pool. In a couple of cases however, we have graph algorithms that don't fit neatly into rayon's
model. In those cases, we spawn one rayon scoped task per thread and then do job control ourselves.

There are various phases within the linker that are single threaded. This is fine, so long as those
phases run quickly enough.

## Linker-plugin LTO

Wild implements the GNU Gold plugin API (`liblto_plugin.so`, `LLVMgold.so`, rustc
`-Clinker-plugin-lto`). GCC ≥ 14 is required (GetSymbols V3). Mixed Clang IR + GCC driver (and the
reverse) is expected to fail. `-mllvm` is an alias for `--plugin-opt` so Clang's `-Wl,-mllvm,...` is
not parsed as `-m` emulation.

Compatibility tests live under `linker-plugin-lto`, `lto-comdat` (C++ template COMDAT),
`rust-integration` (`-Clinker-plugin-lto` and rustc `-C lto` / `-C lto=thin`), wrap/export-dynamic,
and the mold external suite. Mold skips that Wild does not copy: no-plugin as a hard error (Wild
auto-discovers), duplicate IR as a hard error (Wild deduplicates), and `lto-archive4` (asm-invisible
symbols). Incremental links with GCC LTO still fall back to a full padded link. LLVM ThinLTO
restages plugin objects; `--plugin-opt=cache-dir=` is accepted for LLVM's cache and is not
auto-injected (GCC may error on unknown plugin options).

## Testing

Most testing is done by `integration_tests.rs`. This compiles various programs that are written in
C, C++, Rust and assembly. It then links them with our reference linkers — GNU ld, and for general
ELF cases also LLD and Mold (`ReferenceLinkers:bfd,lld,mold`). It links them with ElyLD and compares
the resulting binaries using our own custom diff tool, `linker-diff`. Provided that succeeds, it
then executes all the linked programs and checks that they give the correct answer.

A four-way diff (GNU ld, LLD, Mold, Wild) is the default for tests that list all three reference
linkers. Tests that omit `ReferenceLinkers` still use GNU ld only. Set `ELYLD_FOUR_WAY=1` or
`default_reference_linkers = ["bfd", "lld", "mold"]` in the test config to opt unpinned tests into
the same four-way. Linker-script tests pin `ReferenceLinkers:bfd`; GNU ld is the script oracle.

Kernel `vmlinux` is GNU-only. Set `ELYLD_LINUX_TREE` to an x86_64 tree that already has `vmlinux.o`
and GNU `vmlinux.unstripped`, then `cargo test -p elyld --test integration_tests -- vmlinux`.
Pack objects with `scripts/pack-vmlinux-objects.sh`. CI job `vmlinux` runs when the repository
variable `ELYLD_LINUX_OBJECTS_URL` points at that tarball (a from-scratch kernel build will not fit
the 10-minute timeout). `vmlinux-incremental` links the same objects with `--incremental` and checks
an unchanged second link records `incremental-update`. `vmlinux-incremental-dirty` flips a byte in
one extra object (mtime is 1s granularity) and checks that skip_payloads drops. Clang ThinLTO uses
`ELYLD_LINUX_LTO_TREE` / `scripts/pack-vmlinux-lto-objects.sh` with an LLD-linked oracle; CI job
`vmlinux-lto` is gated on `ELYLD_LINUX_LTO_OBJECTS_URL`. Incremental ThinLTO restages plugin
objects (`vmlinux-lto-incremental`). Follow-up: a small userspace / initramfs also linked
with ElyLD.

Glibc's `libc.so` link uses GNU ld's default shared script (`DATA_SEGMENT_*`, `CONSTANT`,
`ONLY_IF_*`). ElyLD can parse and link that script (see `linker-script-gnu-default`). `nix develop`
unpacks nixpkgs glibc, sets `ELYLD_GLIBC_TREE` / `ELYLD_GLIBC_BUILD` / `ELYLD_GLIBC_HEADERS`, and
provides `wild-build-glibc` (GNU ld + GCC 15). ElyLD's `--version` first line is GNU ld compatible
so glibc `configure` and the kernel's `scripts/ld-version.sh` accept it; the GNU oracle is still
linked with GNU ld so the relink tests have something to diff. Then
`cargo test -p elyld --test integration_tests -- glibc`. Override the env vars to use another
tree. `wild-glibc-check` installs those Wild-linked `libc.so` / `ld.so` / `libm.so` (and other
`lib%.so` relinks when present) into the GNU build and runs a `make test` subset (TLS, IFUNC,
RELR, ctors, malloc, libm, nptl), then restores the GNU oracles. `ELYLD_GLIBC_FULL_CHECK=1` runs
`make check`. `glibc-*-incremental` tests an unchanged `--incremental` relink of `ld.so` / `libc.so`
/ `libm.so`; `glibc-*-incremental-dirty` flips a byte in `csu/abi-note.o`. Capture a package link
with `ELYLD_SAVE_BASE` and set `ELYLD_PYTHON_LINK` / `ELYLD_RUSTC_LINK` / `ELYLD_GCC_LINK` /
`ELYLD_LLVM_LINK` / `ELYLD_FIREFOX_LINK` / `ELYLD_BLENDER_LINK` / `ELYLD_CHROME_LINK` to that save-dir.

Kani proofs live in `elyld-util` (alignment, GNU LMA, skip-payload, plugin/GC fallback, atom
generations) so they do not compile `elyld-layout`. `./scripts/kani.sh` no-ops without `cargo-kani`;
CI job `kani` uses the official GitHub action.

`--features mimalloc` (on by default) statically embeds mimalloc-rs as the process allocator.
`--features mimalloc-dynamic` links `libmimalloc.so` and requires
`--no-default-features --features fork,plugins,zstd,mimalloc-dynamic`. The two are mutually
exclusive with each other and with `dhat`.

## Modularity (Mold and LLD)

Wild stays one `libelyld` crate. The notes below are about *module* boundaries, not new workspace
crates.

Mold is an ELF-first C++ linker. A `Context` holds all state. `src/passes.cc` runs named passes
(resolve, GC, create output sections, LTO, copy). Output is a list of `Chunk` subclasses
(`OutputEhdr`, `OutputSection`, synthetic sections) each with `copy_buf`. Architecture files
(`arch-x86-64.cc` and similar) stay thin. Mach-O is a separate tree, not a shared abstraction.

LLD splits by object format first: `lld/ELF`, `lld/COFF`, `lld/MachO`, `lld/wasm`, plus `lld/Common`.
Each format has its own driver. ELF work lives in `Writer.cpp`, `OutputSections.cpp`,
`InputFiles.cpp`, `Relocations.cpp`, `SyntheticSections.cpp`, and `LinkerScript.cpp` (parse,
evaluate, and orphan insertion together). Arch code sits in `lld/ELF/Arch/`.

Wild already matches the useful parts of both without a god `ctx` or extra crates:

* Format modules (`elf/`, `wasm/`, `macho/`) plus a `Platform` trait (static dispatch) — LLD's
  format split, Mold's lack of virtual `Platform`.
* Phase modules (`args`, `input_data`, `resolution`, `layout/`, `elf_writer/`) — Mold's passes, but
  each phase borrows the previous instead of mutating one context.
* `linker_script/parse` is separate from `layout/script.rs` and output-order in `elf/abi.rs`. LLD
  keeps script parse and orphan placement in one file; keep them apart here.
* `OutputSectionId` / `PartId` plus `elf_writer` section writers play the role of Mold's `Chunk`.
* Arch files (`elf_x86_64.rs`, …) stay thin, like both.

Do not name child modules `layout`, `platform`, or `object` (they shadow parent modules). Keep
`crate::elf::*` / `crate::wasm::*` re-exports. Further splits should stay inside those trees.
