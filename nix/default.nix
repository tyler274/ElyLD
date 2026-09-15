{
  lib,
  craneLib,
  versionCheckHook,
  rustc,
  callPackage,
  path,
  lld,
  clang-tools,
  taplo,
  binutils-unwrapped-all-targets,
  glibc,
  stdenv,
}:
assert lib.assertMsg (lib.versionAtLeast rustc.version "1.97.1")
  "ElyLD requires at least Rust 1.97.1, this instance of nixpkgs has Rust ${rustc.version}";

let
  cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);

  fs = lib.fileset;

  # Only track files checked into git when `.git` is present (local
  # checkouts). Flake inputs from GitHub have no `.git`, so `gitTracked`
  # fails there; the fetched tree is already the committed snapshot.
  files = fs.difference (
    if builtins.pathExists ../.git then
      fs.gitTracked ../.
    else
      fs.fromSource (lib.cleanSource ../.)
  ) (
    fs.unions [
      ../.gitignore
      ../flake.lock
      ../docker
      ../test-config.toml.sample
      ../test-config-ci.toml
      ../.dockerignore
      ../cackle.toml
      ../rustfmt.toml
      ../LICENSE-MIT
      ../LICENSE-APACHE
      (fs.fileFilter (file: file.hasExt "md") ../.)
      (fs.fileFilter (file: file.hasExt "nix") ../.)
    ]
  );

  commonArgs = {
    pname = "elyld";
    inherit (cargoToml.workspace.package) version;

    strictDeps = true;
    src = fs.toSource {
      root = ../.;
      fileset = files;
    };
  };

  inherit (callPackage ./wrappers.nix { }) gccWrapper gppWrapper clangWrapper;
in
craneLib.buildPackage (
  commonArgs
  // {
    cargoArtifacts = craneLib.buildDepsOnly commonArgs;
    cargoBuildCommand = "cargo build --profile release -p elyld";

    # Do the check in the separate derivation so it can be done
    # in parallel in the dev profile
    doCheck = false;
    nativeCheckInputs = [
      lld
      clangWrapper
      clang-tools
      taplo
      binutils-unwrapped-all-targets
      gccWrapper
      gppWrapper
    ];
    checkInputs = [
      glibc.out
      glibc.static
    ];

    env.LD_LIBRARY_PATH = lib.makeLibraryPath [
      stdenv.cc.cc.lib
    ];

    # Do the install check instead just as a smoke-tests that Wild
    # built correctly.
    doInstallCheck = true;
    nativeInstallCheckInputs = [ versionCheckHook ];
    versionCheckProgramArg = "--version";

    meta = {
      changelog = "https://github.com/tyler274/ElyLD/blob/${commonArgs.version}/CHANGELOG.md";
      description = "A very fast linker for Linux";
      homepage = "https://github.com/tyler274/ElyLD";
      license = [
        lib.licenses.asl20 # or
        lib.licenses.mit
      ];
      mainProgram = "elyld";
      platforms = lib.platforms.linux;
    };
  }
)
