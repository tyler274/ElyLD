# Nix

ElyLD ships a Nix flake, overlay, and derivation independent of nixpkgs'
`wild` package. The wrap and stdenv adapter are derived from Wild's
(`wrapBintoolsWith` + `useWildLinker`); names and `.comment` identity are ElyLD.

Unstable Nixpkgs is required until NixOS 25.11 is branched.

## NixOS

Add the flake and enable the module. That overlays `elyld` / `elyld-ld` and
injects ElyLD into `stdenv` / `clangStdenv` via GCC `-B` (GCC 15 has no
`-fuse-ld=elyld`). Packages can opt out with `dontUseElyldLinker = true`.

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    elyld.url = "github:tyler274/ElyLD";
  };

  outputs =
    { nixpkgs, elyld, ... }:
    {
      nixosConfigurations.hostname = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [
          elyld.nixosModules.default
          {
            programs.elyld.enable = true;
          }
        ];
      };
    };
}
```

This is the same inject path Cyrene uses. Hosts that set `gcc.arch` should keep
a nested nixpkgs overlay (see Cyrene) instead of `stdenv.override`, which this
module already avoids.

## Overlay (packages and shells)

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    elyld.url = "github:tyler274/ElyLD";
  };

  outputs =
    {
      self,
      nixpkgs,
      elyld,
    }:
    let
      pkgs = import nixpkgs {
        system = "x86_64-linux";
        overlays = [ elyld.overlays.default ];
      };
      elyldStdenv = pkgs.useElyldLinker pkgs.stdenv;
    in
    {
      packages.x86_64-linux.default = pkgs.callPackage ./package.nix { stdenv = elyldStdenv; };

      devShells.x86_64-linux.default = pkgs.mkShell.override { stdenv = elyldStdenv; } {
        inputsFrom = [ self.packages.x86_64-linux.default ];
        packages = [ pkgs.rust-analyzer ];
      };
    };
}
```

Without flakes (npins shown, but any solution can be used):

Add the dependencies to lockfile with npins: `$ npins add github tyler274 ElyLD -b main`

```nix
let
  sources = import ./npins;
  pkgs = import sources.nixpkgs {
    overlays = [
      (import sources.elyld)
    ];
  };
  elyldStdenv = pkgs.useElyldLinker pkgs.stdenv;
in
{
  # C Package
  package = pkgs.callPackage ./package.nix { stdenv = elyldStdenv; };
}
```
If building a Rust package with `rustPlatform.buildRustPackage`, a little more
setup is required. This applies to Flake-based packages, or other solutions.

```nix
let
  # First steps are the same as above. Create a Nixpkgs instance
  # with ElyLD.
  pkgs = import nixpkgs {
    system = "x86_64-linux";
    overlays = [
      (import elyld)
    ];
  };

  # Create a stdenv that uses ElyLD as its linker
  elyldStdenv = pkgs.useElyldLinker pkgs.stdenv;

  # Next a custom rustPlatform is required.
  #
  # This uses Nixpkgs rustc and cargo, but uses
  # the stdenv that has ElyLD.
  elyldRustPlatform = pkgs.makeRustPlatform {
    inherit (pkgs) rustc cargo;
    stdenv = elyldStdenv;
  };
in
# Then create whatever cool package you are building
callPackage ./package.nix { rustPlatform = elyldRustPlatform; }
```

## Development shell

`nix develop` (or `nix-shell nix/shell.nix`) is the local environment for
building, testing, debugging, and benchmarking Wild. It wraps **LLVM 22** clang /
LLVMgold / lld / lldb so they match rustup nightly's LLVM, plus GCC with the
LTO plugin search path, mold, glibc (static + source for relink tests), and:

* `mimalloc` + `pkg-config` for `--features mimalloc-dynamic` (default builds use mimalloc-rs)
* `gdb`, `lldb`, `elfutils`, `valgrind`, `strace` for inspecting links
* `hyperfine` and `samply` (see [BENCHMARKING.md](../BENCHMARKING.md))
* `bc`, `pahole`, `rsync`, `openssl`, `ncurses` for optional kernel rebuilds
* `zstd` for `scripts/pack-vmlinux-objects.sh` / `pack-vmlinux-lto-objects.sh`
* `cmake` / `ninja` for opt-in userspace package trees

Kani is not in nixpkgs. Install it with `cargo install --locked kani-verifier &&
cargo kani setup`, then `./scripts/kani.sh`. Firefox, Blender, Chrome, and kernel
source trees are **not** pulled into the shell; point `ELYLD_*_TREE` /
`ELYLD_*_LINK` at local checkouts.

## Glibc relink tests

`nix develop` (or `nix-shell nix/shell.nix`) puts glibc's build tools on `PATH` (`python3`, `bison`,
`gawk`, …), unpacks the nixpkgs glibc source, and sets:

* `ELYLD_GLIBC_TREE` — that source (override to use another checkout)
* `ELYLD_GLIBC_BUILD` — `$PWD/target/glibc-gnu`
* `ELYLD_GLIBC_HEADERS` — nixpkgs `linuxHeaders`

Glibc `configure` rejects Wild, so the GNU oracle is built first:

```sh
wild-build-glibc
cargo test -p elyld --test integration_tests -- glibc
```

`wild-build-glibc` uses unwrapped GCC 15 (the Nix gcc wrapper injects `_FORTIFY_SOURCE=3`) and GNU
ld. Pass `--force` to reconfigure. The shell sets `LIBRARY_PATH` so glibc tests can link `-lgcc_s`.
A from-scratch glibc build is not part of `nix flake check`. After relink:

```sh
wild-glibc-check
```

That swaps Wild-linked `libc.so` / `ld.so` (and `libm.so` / `libresolv.so` / stubs when the relink
tests produced them) into `$ELYLD_GLIBC_BUILD`, runs a `make test` subset (TLS, IFUNC, RELR, ctors,
malloc, libm, nptl), and restores the GNU oracles. `ELYLD_GLIBC_FULL_CHECK=1` runs `make check`
instead. Single tests:
`make -C "$ELYLD_GLIBC_BUILD" test t=elf/tst-tls1`.
