{
  lib,
  stdenvNoCC,
  libbacktrace,
  zstd,
  zlib,
  xz,
  autoconf,
  automake,
  libtool,
  pkg-config,
}:
let
  # Read-only upstream source from the same nixpkgs as the rest of the shell.
  # The integration test copies this into a writable build dir.
  src = stdenvNoCC.mkDerivation {
    name = "libbacktrace-${libbacktrace.version}-src";
    inherit (libbacktrace) src;
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
  pkgConfigPath = lib.concatStringsSep ":" [
    (lib.makeSearchPath "lib/pkgconfig" [
      zstd.dev
      xz.dev
    ])
    (lib.makeSearchPath "share/pkgconfig" [ zlib.dev ])
  ];
in
{
  inherit src;
  packages = [
    zstd
    zlib
    xz
    autoconf
    automake
    libtool
    pkg-config
  ];
  shellHook = ''
    if [ -z "''${ELYLD_LIBBACKTRACE_TREE:-}" ]; then
      export ELYLD_LIBBACKTRACE_TREE="${src}"
    fi
    if [ -z "''${ELYLD_LIBBACKTRACE_BUILD:-}" ]; then
      export ELYLD_LIBBACKTRACE_BUILD="$PWD/target/libbacktrace"
    fi
    export PKG_CONFIG_PATH="${pkgConfigPath}''${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
    export CFLAGS="-I${zlib.dev}/include -I${zstd.dev}/include -I${xz.dev}/include ''${CFLAGS-}"
    export LDFLAGS="-L${zlib.out}/lib -L${zstd.out}/lib -L${xz.out}/lib ''${LDFLAGS-}"
    export LD_LIBRARY_PATH="${zlib.out}/lib:${zstd.out}/lib:${xz.out}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
  '';
}
