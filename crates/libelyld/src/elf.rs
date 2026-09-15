use elyld_args::elf::ElfArgs;
use elyld_error::bail;
use elyld_error::error::Result;
use elyld_fs::fs::FileSystem;

pub(crate) fn link_for_arch<'data, F: FileSystem>(
    linker: &'data crate::Linker<F>,
    args: &'data ElfArgs,
) -> Result<crate::LinkerOutput<'data>> {
    match args.architecture() {
        elyld_util::arch::Architecture::X86_64 => {
            linker.link_for_arch::<elyld_elf::Elf64, elyld_elf::ElfX86_64>(args)
        }
        elyld_util::arch::Architecture::AArch64 => {
            linker.link_for_arch::<elyld_elf::Elf64, elyld_elf::ElfAArch64>(args)
        }
        elyld_util::arch::Architecture::RiscV64 => {
            linker.link_for_arch::<elyld_elf::Elf64, elyld_elf::ElfRiscV64>(args)
        }
        elyld_util::arch::Architecture::LoongArch64 => {
            linker.link_for_arch::<elyld_elf::Elf64, elyld_elf::ElfLoongArch64>(args)
        }
        elyld_util::arch::Architecture::Ppc64 => {
            linker.link_for_arch::<elyld_elf::Elf64, elyld_elf::ElfPpc64>(args)
        }
        elyld_util::arch::Architecture::Unsupported => {
            bail!(
                "No default target architecture known for host platform. \
                    Please specify an architecture with -m"
            )
        }
    }
}
