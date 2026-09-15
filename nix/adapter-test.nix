# Smoke tests derived from nixpkgs `wild-unwrapped/adapterTest.nix`,
# checking ElyLD's `.comment` identity instead of Wild's.
{
  lib,
  stdenv,
  gccStdenv,
  clangStdenv,
  buildPackages,
  runCommandCC,
  elyld-unwrapped,
  useElyldLinker,
  hello,
}:
let
  helloTest =
    name: helloElyld:
    let
      command = "$READELF -p .comment ${lib.getExe helloElyld}";
      emulator = stdenv.hostPlatform.emulator buildPackages;
    in
    runCommandCC "elyld-${name}-test" { passthru = { inherit helloElyld; }; } ''
      echo "Testing running the 'hello' binary which should be linked with ElyLD" >&2
      ${emulator} ${lib.getExe helloElyld}

      echo "Checking for ElyLD in the '.comment' section" >&2
      if output=$(${command} 2>&1); then
        if grep -Fw -- "ElyLD" - <<< "$output"; then
          touch $out
        else
          echo "No mention of 'ElyLD' detected in the '.comment' section" >&2
          echo "The command was:" >&2
          echo "${command}" >&2
          echo "The output was:" >&2
          echo "$output" >&2
          exit 1
        fi
      else
        echo -n "${command}" >&2
        echo " returned a non-zero exit code." >&2
        echo "$output" >&2
        exit 1
      fi
    '';
in
{
  adapterGcc = helloTest "adapter-gcc" (
    hello.override (_: {
      stdenv = useElyldLinker gccStdenv;
    })
  );

  adapter-llvm = helloTest "adapter-llvm" (
    hello.override (_: {
      stdenv = useElyldLinker clangStdenv;
    })
  );

  inherit elyld-unwrapped;
}
