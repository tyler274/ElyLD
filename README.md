# ElyLD

<p align="center">
  <img src="images/elyld.png" alt="ElyLD logo — Elysia from Honkai Impact 3rd" width="280">
</p>

ElyLD is a GNU-ld-compatible linker aimed at fast iterative development. It is a fork of
[Wild](https://github.com/wild-linker/wild), named after Elysia from Honkai Impact 3rd.

`--incremental` can patch an existing output when inputs change (GC and GCC LTO still fall back to a
full padded link; LLVM ThinLTO restages plugin objects and can update in place). It is also used as
a drop-in ELF linker for Linux kernels (`vmlinux`), glibc DSOs, and as a NixOS stdenv linker.

## Installation

### Build from git

```sh
cargo install --locked --bin elyld --git https://github.com/tyler274/ElyLD.git elyld
```

`release`, `opt`, and `dist` profiles use ThinLTO. CI (`profile.ci`) and `dev` do not.

To bootstrap (stage1 with the system linker, then relink ElyLD with itself):

```sh
./scripts/bootstrap.sh --profile release
./scripts/bootstrap.sh --profile opt
```

GNU dist releases run a PGO build trained on the full `integration_tests` suite (still self-hosted):

```sh
./scripts/pgo-build.sh
```

Needs clang (rustc linker driver) and the rustup `llvm-tools-preview` component.

### Nix

To use this tree from Nix, see [the nix documentation](./nix/nix.md).

## Using as your default linker

Being a drop-in replacement, ElyLD can be used similarly to other linkers by being invoked by GCC or
Clang. Meaning you have several options:

* Clang's exclusive option `--ld-path=elyld`
* GCC 16.1+ and Clang's option `-fuse-ld=elyld` (note that Clang requires `ld.elyld` binary/symlink)
* Generally supported `-B <path>`, where `<path>` is the directory containing `ld` that points to
  `elyld`

Below are examples of integrating ElyLD with various build systems.

### Rust (Cargo)

You can use one of the options mentioned above in `~/.cargo/config.toml`:

```toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-Clink-arg=--ld-path=elyld"]
```

Or:

```toml
[target.x86_64-unknown-linux-gnu]
# linker = "clang" # Uncomment this line if your GCC is older than version 16.
rustflags = ["-Clink-arg=-fuse-ld=elyld"]
```

### CMake

CMake 4.4 or later has `CMAKE_LINKER_TYPE=WILD` for upstream Wild (`ld.wild`). For ElyLD, pass
`--ld-path=elyld` / `-fuse-ld=elyld`, or use the generic `-B` instructions below.

### C/C++ (autotools, meson, old CMake etc.)

Usually setting `LDFLAGS` is enough, but there are projects that implement their own solutions:

```sh
export LDFLAGS="${LDFLAGS} -fuse-ld=elyld"
```

Or (especially useful for older GCC versions), create a symlink `ld` pointing to `elyld` and pass the
directory to GCC:

```sh
ln -s /usr/bin/elyld /tmp/ld

export CFLAGS="${CFLAGS} -B/tmp"
export CXXFLAGS="${CXXFLAGS} -B/tmp"
export LDFLAGS="${LDFLAGS} -B/tmp"
```

Then configure the project (you might need to remove the configuration cache first) and run your
usual build steps.

Due to the complexity of these build systems, you might want to verify that ElyLD was used to link a
binary with [readelf](#how-can-i-verify-that-elyld-was-used-to-link-a-binary).

### Illumos specific Cargo configuration:

```toml
[target.x86_64-unknown-illumos]
# Absolute path to clang - on OmniOS this is likely something like /opt/ooce/bin/clang.
linker = "/usr/bin/clang"

rustflags = [
    # Will silently delegate to GNU ld or Sun ld unless the absolute path to ElyLD is provided.
    "-Clink-arg=-fuse-ld=/absolute/path/to/elyld"
]
```

## Using ElyLD in CI

If you'd like to use ElyLD as your linker for Rust code in CI, see
[wild-action](https://github.com/wild-linker/action).

## Q&A

### Why another linker?

Mold is already very fast, however it doesn't do incremental linking and the author has stated that
they don't intend to. ElyLD implements `--incremental` so repeated links can patch an existing
output instead of rewriting it from scratch.

### What's working?

The following platforms / architectures are currently supported:

* x86-64 on Linux
* ARM64 on Linux
* RISC-V (riscv64gc) on Linux
* LoongArch64 on Linux (initial support)
* PPC64LE on Linux (initial support)

Experimental Mach-O (AArch64) and Wasm32 are available behind cargo features (`macho`, `wasm`).

The following is working with the caveat that there may be bugs:

* Output to statically linked, non-relocatable binaries
* Output to statically linked, position-independent binaries (static-PIE)
* Output to dynamically linked binaries
* Output to shared objects (.so files)
* Rust proc-macros, when linked with ElyLD work
* Most of the top downloaded crates on crates.io have been tested with ElyLD and pass their tests
* Debug info (`--gdb-index`)
* GNU jobserver support
* `--incremental` / `ELYLD_INCREMENTAL=1` — see [Incremental linking](#incremental-linking)
* Linux kernel `vmlinux` (x86_64 vs GNU ld; Clang ThinLTO vs LLD) — see [Linux kernel](#linux-kernel)
* glibc `ld.so` / `libc.so` / `lib%.so` relink — see [Glibc](#glibc)
* GNU ld compatible `--version` (`GNU ld (ElyLD) 2.44`) so glibc `configure` and the kernel's
  `scripts/ld-version.sh` accept ElyLD
* Linker-plugin LTO (GNU Gold API: `liblto_plugin.so`, `LLVMgold.so`, rustc `-Clinker-plugin-lto`)
* GNU `--wrap`, including wrap after LTO
* Linker scripts used by the kernel and glibc. See the [linker script support matrix](LINKER_SCRIPT_SUPPORT.md)
* Nix stdenv linker via `-B` / `ld.elyld` (see [nix/nix.md](nix/nix.md))

### What isn't yet supported?

Here are some of the larger remaining gaps:

* Incremental links with GC or GCC LTO (WPA) still fall back to a full padded link
* Mach-O and Wasm are experimental (not on by default)
* Windows / COFF
* Remaining linker-script gaps listed in [LINKER_SCRIPT_SUPPORT.md](LINKER_SCRIPT_SUPPORT.md)

### Incremental linking

Pass `--incremental` or set `ELYLD_INCREMENTAL=1`. The first link writes a `{output}.incr` state
directory and pads output sections so a later link can patch in place. Unchanged objects skip
payloads. Custom linker scripts skip section padding so kernel `ASSERT`s keep their sizes; unchanged
inputs can still skip payloads.

GC (`--gc-sections`) and GCC LTO (WPA) fall back to a full padded link. LLVM ThinLTO plugin objects
are copied to `{output}.incr/plugin/{NNNN}.o` so a later link can skip unchanged payloads. You can
also pass `--plugin-opt=cache-dir=PATH` for LLVM's own cache; ElyLD does not inject that option
because GCC rejects unknown plugin opts. Integration tests cover GCC, Clang (including `-flto=thin`),
and rustc at several `-O` levels, plus unchanged relinks of x86_64 `vmlinux` and glibc DSOs. See
[DESIGN.md](DESIGN.md) for the atom table and skip-update model.

### Linux kernel

ElyLD links x86_64 `vmlinux` with the kernel's `vmlinux.lds` (`--no-gc-sections`, `--orphan-handling=error`).
Key symbols (`_stext`, `_etext`, `__init_begin`, `_end`, …) are checked against GNU ld. Clang ThinLTO
`vmlinux` is checked against LLD. `--incremental` on the same objects is a padded link (not compared
to GNU addresses); a dirty `init/version-timestamp.o` is expected to drop skip_payloads. ThinLTO
incremental restages LLVM plugin objects and expects an in-place update.

Point `ELYLD_LINUX_TREE` at a tree that already has `vmlinux.o` and GNU `vmlinux.unstripped` (pack
with `scripts/pack-vmlinux-objects.sh`), then:

```sh
cargo test -p elyld --test integration_tests -- vmlinux
```

ThinLTO objects: `ELYLD_LINUX_LTO_TREE` and `scripts/pack-vmlinux-lto-objects.sh`. Arch-specific
kernel-like scripts (x86_64, aarch64, riscv64, loongarch64, ppc64le) are covered by the integration
suite. Feature status is in [LINKER_SCRIPT_SUPPORT.md](LINKER_SCRIPT_SUPPORT.md#linux-kernel-requirements).

### Glibc

ElyLD relinks GNU-built glibc objects: `ld.so`, `libc.so`, and `lib%.so` PIC archives (`libm`,
`libresolv`, stubs, …). `--incremental` on those DSOs skips unchanged object payloads and still
rewrites dynamic reloc tables.

`nix develop` sets `ELYLD_GLIBC_TREE` / `ELYLD_GLIBC_BUILD` and provides `wild-build-glibc`. Then:

```sh
cargo test -p elyld --test integration_tests -- glibc
```

`wild-glibc-check` runs a glibc `make test` subset against the ElyLD-linked DSOs. Details:
[LINKER_SCRIPT_SUPPORT.md](LINKER_SCRIPT_SUPPORT.md#glibc-libcso--ldso--libso) and
[nix/nix.md](nix/nix.md).

### How can I verify that ElyLD was used to link a binary?

Install `readelf` (available from binutils package), then run:

```sh
readelf --string-dump .comment my-executable
```

Look for a line like:

```
Linker: ElyLD 1.0.0 (compatible with GNU linkers)
```

You can probably also get away with `strings` (also available from binutils package):

```sh
strings my-executable | grep 'Linker:'
```

### Where did the name come from?

The name is after Elysia from Honkai Impact 3rd. Linkers traditionally end in "ld" (GNU ld, gold,
lld, mold); ElyLD keeps that suffix. It is a fork of [Wild](https://github.com/wild-linker/wild).

## Benchmarks

The goal of ElyLD is to be very fast via incremental linking, and as fast as we can be for
non-incremental linking and for the initial link when incremental linking is enabled.

All benchmarks are run with output to a tmpfs. See [BENCHMARKING.md](BENCHMARKING.md) for details on
running benchmarks.

We run benchmarks on a few different systems:

* [Ryzen 9 9955HX (16 core, 32 thread)](benchmarks/ryzen-9955hx.md)
* [2020 era Intel-based laptop with 4 cores and 8 threads](benchmarks/lemp9.md)
* [Raspberry Pi 5](benchmarks/raspberrypi.md)

Here's a few highlights.

### Ryzen 9955HX (16 core, 32 thread)

First, we link the Chrome web browser (or technically, Chromium).

![Benchmark of linking chrome-crel](benchmarks/images/ryzen-9955hx/chrome-crel-time.svg)

Memory consumption when linking Chromium:

![Benchmark of linking chrome-crel](benchmarks/images/ryzen-9955hx/chrome-crel-memory.svg)

librustc-driver is the shared object where most of the code in the Rust compiler lives. This
benchmark shows the time to link it.

![Benchmark of linking librustc-driver](benchmarks/images/ryzen-9955hx/librustc-driver-time.svg)

For something much smaller, this is the time to link the linker itself. This also shows a few
different versions, so you can see how the link time has been tracking over releases.

![Benchmark of linking wild](benchmarks/images/ryzen-9955hx/wild-time.svg)

### Raspberry Pi 5

Here's linking rust-analyzer on a Raspberry Pi 5.

![Time to link rust-analyzer-no-debug](benchmarks/images/raspberrypi/rust-analyzer-no-debug-time.svg)

## Linking Rust code

The following is a `cargo test` command-line that can be used to build and test a crate using ElyLD.
This has been run successfully on a few popular crates (e.g. ripgrep, serde, tokio, rand, bitflags).
It assumes that the `elyld` binary is on your path. It also depends on the Clang compiler being
installed, since GCC doesn't allow using an arbitrary linker.

```sh
RUSTFLAGS="-Clinker=clang -Clink-args=--ld-path=elyld" cargo test
```

Alternatively, with `ld.elyld` symlink pointing at `elyld`:
```sh
RUSTFLAGS="-Clinker=clang -Clink-args=-fuse-ld=elyld" cargo test
```

## Contributing

For more information on contributing to ElyLD see [CONTRIBUTING.md](CONTRIBUTING.md).

For a high-level overview of ElyLD's design, see [DESIGN.md](DESIGN.md).

## Chat server

We have a Zulip server for ElyLD-related chat. You can join
[here](https://wild.zulipchat.com/join/bbopdeg6howwjpaiyowngyde/).

## Further reading

Many of the posts on [David's blog](https://davidlattimore.github.io/) are about various aspects of
ElyLD.

## Sponsorship

If you'd like to [sponsor this work](https://github.com/sponsors/davidlattimore), that would be very
much appreciated. The more sponsorship I get the longer I can continue to work on this project full
time.

# Code of Conduct

This project adheres to the [Rust code of
conduct](https://rust-lang.org/policies/code-of-conduct/). If you have any moderation concerns or
queries, please email wild-mod@googlegroups.com.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT)
at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
ElyLD by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
