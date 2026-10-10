{
  lib,
  stdenvNoCC,
  duckdb,
  cmake,
  ninja,
  python3,
  openssl,
  pkg-config,
}:
let
  # Read-only upstream source from the same nixpkgs as the rest of the shell.
  # The integration test copies this into a writable build dir.
  src = stdenvNoCC.mkDerivation {
    name = "duckdb-${duckdb.version}-src";
    inherit (duckdb) src;
    dontConfigure = true;
    dontBuild = true;
    dontFixup = true;
    preferLocalBuild = true;
    installPhase = ''
      runHook preInstall
      mkdir -p "$out"
      cp -a . "$out/"
      runHook postInstall
    '';
  };
in
{
  inherit src;
  packages = [
    cmake
    ninja
    python3
    openssl
    pkg-config
  ];
  shellHook = ''
    if [ -z "''${ELYLD_DUCKDB_TREE:-}" ]; then
      export ELYLD_DUCKDB_TREE="${src}"
    fi
    if [ -z "''${ELYLD_DUCKDB_BUILD:-}" ]; then
      export ELYLD_DUCKDB_BUILD="$PWD/target/duckdb"
    fi
    if [ -z "''${ELYLD_DUCKDB_DESCRIBE:-}" ]; then
      export ELYLD_DUCKDB_DESCRIBE="v${duckdb.version}"
    fi
    export CMAKE_PREFIX_PATH="${openssl.dev}''${CMAKE_PREFIX_PATH:+:$CMAKE_PREFIX_PATH}"
    export PKG_CONFIG_PATH="${lib.getDev openssl}/lib/pkgconfig''${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
  '';
}
