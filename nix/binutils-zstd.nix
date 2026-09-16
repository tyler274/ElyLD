{
  binutils-unwrapped,
  zstd,
  pkg-config,
}:
# Native ld.bfd with --compress-debug-sections=zstd. nixpkgs binutils is
# built --without-zstd (and Cyrene's profile bfd is the same), so the
# compressed-debug zstd integration tests skip on RequiresLinkerFlags.
# Native-only: do not overlay this onto binutils-unwrapped-all-targets.
binutils-unwrapped.overrideAttrs (old: {
  buildInputs = (old.buildInputs or [ ]) ++ [ zstd ];
  nativeBuildInputs = (old.nativeBuildInputs or [ ]) ++ [ pkg-config ];
  configureFlags = (old.configureFlags or [ ]) ++ [ "--with-zstd" ];
})
