{
  pkgs ? import <nixpkgs> { },
}:
let
  # Keep this image's compilers on LLVM 22 so rustc plugin LTO matches clang.
  # The full dev tool set lives in nix/shell.nix (`nix develop`).
  llvmPkgs = pkgs.llvmPackages_22;
  llvmLib = llvmPkgs.libllvm.lib or llvmPkgs.libllvm;
  binutilsZstd = pkgs.callPackage ../nix/binutils-zstd.nix { };
  glibcTests = pkgs.callPackage ../nix/glibc-tests.nix { };
in
pkgs.mkShell {
  nativeBuildInputs = [
    (pkgs.writeShellApplication {
      name = "gcc";
      text = ''${pkgs.lib.getExe pkgs.gcc} "$@" -B${binutilsZstd}/bin -B${pkgs.binutils-unwrapped-all-targets}/bin '';
    })
    (pkgs.writeShellApplication {
      name = "g++";
      text = ''${pkgs.lib.getExe' pkgs.gcc "g++"} "$@" -B${binutilsZstd}/bin -B${pkgs.binutils-unwrapped-all-targets}/bin '';
    })
    binutilsZstd
    pkgs.binutils-unwrapped-all-targets
    pkgs.cargo-chef
    (pkgs.writeShellApplication {
      name = "clang";
      text = ''${pkgs.lib.getExe llvmPkgs.clang} "$@" -B${llvmLib}/lib -B${binutilsZstd}/bin -B${pkgs.binutils-unwrapped-all-targets}/bin '';
    })
    (pkgs.writeShellApplication {
      name = "clang++";
      text = ''${pkgs.lib.getExe' llvmPkgs.clang "clang++"} "$@" -B${llvmLib}/lib -B${binutilsZstd}/bin -B${pkgs.binutils-unwrapped-all-targets}/bin '';
    })
    llvmPkgs.clang-tools
    llvmPkgs.lld
    llvmPkgs.llvm.dev
    pkgs.glibc.out
    pkgs.glibc.static
    pkgs.rustup
    pkgs.mold
    pkgs.mimalloc
    pkgs.pkg-config
    pkgs.hyperfine
    pkgs.gdb
    pkgs.elfutils
  ]
  ++ glibcTests.packages;

  ELYLD_MOLD_TESTS = "1";

  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
    pkgs.stdenv.cc.cc.lib
    pkgs.mimalloc
  ];

  inherit (glibcTests) shellHook;
}
