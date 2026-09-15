# ElyLD

![Wild logo - drawing of rusty chain links with vines](/images/wild.png)

ElyLD is a linker with the goal of being very fast for iterative development.

The plan is to eventually make it incremental, however that isn't yet implemented. It is however
already pretty fast even without incremental linking.

## Installation

### Build from git

```sh
cargo install --locked --bin elyld --git https://github.com/tyler274/wild.git elyld
```

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

CMake 4.4 or later supports Wild directly when used with Clang or GCC 16 or later. You can select
Wild as the linker by adding `-DCMAKE_LINKER_TYPE=WILD` to the cmake command-line.

For older versions of cmake, see the generic instructions below.

### C/C++ (autotools, meson, old CMake etc.)

Usually setting `LDFLAGS` is enough, but there are projects that implement their own solutions:

```sh
export LDFLAGS="${LDFLAGS} -fuse-ld=elyld"
```

Or (especially useful for older GCC versions), create a symlink `ld` pointing to `wild` and pass the
directory to GCC:

```sh
ln -s /usr/bin/elyld /tmp/ld

export CFLAGS="${CFLAGS} -B/tmp"
export CXXFLAGS="${CXXFLAGS} -B/tmp"
export LDFLAGS="${LDFLAGS} -B/tmp"
```

Then configure the project (you might need to remove the configuration cache first) and run your
usual build steps.

Due to the complexity of these build systems, you might want to verify that Wild was used to link a
binary with [readelf](#how-can-i-verify-that-wild-was-used-to-link-a-binary).

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

## Using wild in CI

If you'd like to use ElyLD as your linker for Rust code in CI, see
[wild-action](https://github.com/wild-linker/action).

## Q&A

### Why another linker?

Mold is already very fast, however it doesn't do incremental linking and the author has stated that
they don't intend to. Wild doesn't do incremental linking yet, but that is the end-goal. By writing
Wild in Rust, it's hoped that the complexity of incremental linking will be achievable.

### What's working?

The following platforms / architectures are currently supported:

* x86-64 on Linux
* ARM64 on Linux
* RISC-V (riscv64gc) on Linux
* LoongArch64 on Linux (initial support)
* PPC64LE on Linux (initial support)

The following is working with the caveat that there may be bugs:

* Output to statically linked, non-relocatable binaries
* Output to statically linked, position-independent binaries (static-PIE)
* Output to dynamically linked binaries
* Output to shared objects (.so files)
* Rust proc-macros, when linked with ElyLD work
* Most of the top downloaded crates on crates.io have been tested with ElyLD and pass their tests
* Debug info
* GNU jobserver support
* Partial linker script support. See the [linker script support matrix](LINKER_SCRIPT_SUPPORT.md) for details.
* Linker plugin LTO - [known issues](https://github.com/tyler274/wild/issues?q=is%3Aissue%20state%3Aopen%20label%3ALTO)

### What isn't yet supported?

Here are some of the larger things that aren't yet done, roughly sorted by current priority:

* Incremental linking
* More complex linker scripts
* Mach-O support
* Windows support

### How can I verify that Wild was used to link a binary?

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

ElyLD is a fork of [Wild](https://github.com/wild-linker/wild). Linkers traditionally end in "ld"
(GNU ld, gold, lld, mold). ElyLD keeps that suffix.

## Benchmarks

The goal of ElyLD is to eventually be very fast via incremental linking. However, we also want to be
as fast as we can be for non-incremental linking and for the initial link when incremental linking
is enabled.

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

For something much smaller, this is the time to link Wild itself. This also shows a few different
Wild versions, so you can see how the link time has been tracking over releases.

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

The Wild project adheres to the [Rust code of
conduct](https://rust-lang.org/policies/code-of-conduct/). If you have any moderation concerns or
queries, please email wild-mod@googlegroups.com.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT)
at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
Wild by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
