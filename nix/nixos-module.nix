# NixOS module derived from the Cyrene inject overlay, independent of
# nixpkgs' `useWildLinker`. GCC 15 has no `-fuse-ld=elyld`, so collect2
# is pointed at `elyld-ld/ld-prefix` with `-B` instead of reconstructing
# gcc via `stdenv.override` (that loop is fatal when `gcc.arch` is set).
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.programs.elyld;
  elyldBflags = elyldLd: " -B${elyldLd}/ld-prefix";

  injectElyld =
    elyldLd: stdenv:
    let
      oldMk = stdenv.mkDerivation;
      addElyld =
        args:
        if args.dontUseElyldLinker or args.dontUseWildLinker or false then
          args
        else
          let
            nbi = args.nativeBuildInputs or [ ];
            already = builtins.elem elyldLd nbi;
            existing = toString (
              (args.env or { }).NIX_CFLAGS_LINK or (args.NIX_CFLAGS_LINK or "")
            );
          in
          (removeAttrs args [ "NIX_CFLAGS_LINK" ])
          // {
            nativeBuildInputs = if already then nbi else nbi ++ [ elyldLd ];
            env = (args.env or { }) // {
              NIX_CFLAGS_LINK = if already then existing else existing + elyldBflags elyldLd;
            };
          };
    in
    stdenv
    // {
      mkDerivation =
        args: oldMk (if lib.isFunction args then (finalAttrs: addElyld (args finalAttrs)) else addElyld args);
    };
in
{
  options.programs.elyld = {
    enable = lib.mkEnableOption "ElyLD as the NixOS stdenv linker";
  };

  config = {
    nixpkgs.overlays = [
      (import ../.)
    ]
    ++ lib.optionals cfg.enable [
      (lib.mkAfter (
        final: prev: {
          stdenv = injectElyld final.elyld-ld prev.stdenv;
          clangStdenv = injectElyld final.elyld-ld prev.clangStdenv;
        }
      ))
    ];

    environment.systemPackages = lib.mkIf cfg.enable [ pkgs.elyld ];
  };
}
