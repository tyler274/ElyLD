use elyld_args::macho::MachOArgs;
use elyld_error::error::Result;
use elyld_fs::fs::FileSystem;
use elyld_platform::Args as _;

pub(crate) fn link_for_arch<'data, F: FileSystem>(
    linker: &'data crate::Linker<F>,
    args: &'data MachOArgs,
) -> Result<crate::LinkerOutput<'data>> {
    if !(cfg!(feature = "macho") || args.experimental_platforms()) {
        elyld_error::bail!(
            "Mach-O support is still experimental. Rebuild with `--features macho` to enable it."
        );
    }

    linker.link_for_arch::<elyld_macho::MachO, elyld_macho::MachOAArch64>(args)
}
