use elyld_args::wasm::WasmArgs;
use elyld_error::bail;
use elyld_fs::fs::FileSystem;
use elyld_platform::Args as _;

pub(crate) fn link_for_arch<'data, F: FileSystem>(
    linker: &'data crate::Linker<F>,
    args: &'data WasmArgs,
) -> elyld_error::error::Result<crate::LinkerOutput<'data>> {
    if !(cfg!(feature = "wasm") || args.experimental_platforms()) {
        bail!("Wasm support is still experimental. Rebuild with `--features wasm` to enable it.");
    }

    linker.link_for_arch::<elyld_wasm::Wasm, elyld_wasm::WasmWasm32>(args)
}
