# Overlay derived from nixpkgs' `wild` wrap (`wrapBintoolsWith` +
# `useWildLinker` in pkgs/stdenv/adapters.nix), independent of the
# nixpkgs `wild` / `wild-unwrapped` packages.
#
# `elyld-unwrapped` is the Crane-built binary. `elyld` is that binary
# behind nix's ld-wrapper (`elyld`, `ld.elyld`, `ld`) so `-L` becomes
# DT_RUNPATH. `elyld-ld` is the `-B` prefix GCC 15 collect2 needs
# (`-fuse-ld=elyld` is GCC 16+). `useElyldLinker` swaps a stdenv's
# bintools the same way `useWildLinker` does.
final: prev:
let
  inherit (prev) lib;

  # Crane comes from this tree's lockfile so overlay consumers do not
  # need to thread a flake input.
  craneNodes = (builtins.fromJSON (builtins.readFile ./flake.lock)).nodes.crane.locked;
  craneSrc = prev.fetchFromGitHub {
    inherit (craneNodes) owner repo rev;
    hash = craneNodes.narHash;
  };
  craneLib = import craneSrc { pkgs = prev; };

  # `prev.callPackage` so a later overlay that injects ElyLD into
  # `stdenv` cannot rebuild ElyLD with itself (stdenv → elyld → rustc → stdenv).
  elyldUnwrapped = prev.callPackage ./nix { inherit craneLib; };

  ldWrapper = "${prev.path}/pkgs/build-support/bintools-wrapper/ld-wrapper.sh";
  targetPrefix = prev.stdenv.cc.bintools.targetPrefix;
  elyldExe = lib.getExe elyldUnwrapped;

  elyldWrapped = prev.wrapBintoolsWith {
    bintools = elyldUnwrapped;
    extraBuildCommands = ''
      wrap elyld ${ldWrapper} ${elyldExe}
      wrap ld.elyld ${ldWrapper} ${elyldExe}
      wrap ${targetPrefix}ld.elyld ${ldWrapper} ${elyldExe}
      wrap ${targetPrefix}ld ${ldWrapper} ${elyldExe}
    '';
  };

  # `bin/` has unwrapped `elyld` / `ld.elyld` for PATH lookups. `ld` stays
  # out of `bin/` so PATH's nix ld-wrapper is not shadowed. `ld-prefix/ld`
  # is wrapped ElyLD for GCC `-B`.
  elyldLd = prev.runCommand "elyld-ld" { } ''
    mkdir -p $out/bin $out/ld-prefix
    ln -s ${elyldExe} $out/bin/elyld
    ln -s ${elyldExe} $out/bin/ld.elyld
    ln -s ${elyldWrapped}/bin/${targetPrefix}ld $out/ld-prefix/ld
    ln -s ${elyldWrapped}/bin/${targetPrefix}ld.elyld $out/ld-prefix/ld.elyld
  '';

  useElyldLinker =
    stdenv:
    if !stdenv.targetPlatform.isLinux then
      throw "ElyLD only supports building Linux ELF files from Linux hosts."
    else
      stdenv.override (old: {
        allowedRequisites = null;
        cc = old.cc.override {
          bintools = old.cc.bintools.override {
            extraBuildCommands = ''
              ln -fs ${elyldWrapped}/bin/* "$out/bin"
            '';
          };
        };
      });
in
{
  elyld-unwrapped = elyldUnwrapped;
  elyld = elyldWrapped;
  elyld-ld = elyldLd;
  inherit useElyldLinker;
}
