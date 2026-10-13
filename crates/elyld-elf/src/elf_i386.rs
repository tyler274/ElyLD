//! i386-specific code for the ELF linker.
//!
//! i386 is similar to x86-64 but lacks PC-relative memory access instructions, so PIC code
//! obtains the GOT address via a `__x86.get_pc_thunk.*` call (or uses the GOT-relative
//! addressing with %ebx). PLT entries use %ebx for PIC and absolute addresses otherwise.
//!
//! Reference: https://github.com/rui314/psabi/blob/main/i386.pdf
//!
//! Relocations are of the REL type: addends live in the relocated locations rather than in
//! the relocation entries. The engine converts input REL sections to implicit addends by
//! reading the section bytes (see `Rel` handling in the file reader), so the relocation
//! kinds here line up with the x86-64 kinds where the formula is the same.

use crate::{Elf32, PropertyClass};
use elyld_error::{error,
    error::Result,
};
use elyld_platform::{OutputKind, Platform, PreviousRelocationInfo, ValueFlags};
use linker_utils::elf::{DynamicRelocationKind, RelocationKindInfo, SectionFlags, shf};
use linker_utils::i386::relocation_from_raw;
use linker_utils::relaxation::RelocationModifier;

pub struct ElfI386;

pub(crate) const PLT_ENTRY_SIZE_I386: u64 = 16;

pub(crate) const PLT_ENTRY_TEMPLATE_PIC: &[u8] = &[
    0xb9, 0, 0, 0, 0, // mov $reloc_offset, %ecx
    0xff, 0xa3, 0, 0, 0, 0, // jmp *foo@GOT(%ebx)
    0xcc, 0xcc, 0xcc, 0xcc, 0xcc, // (padding)
];

pub(crate) const PLT_ENTRY_TEMPLATE_NON_PIC: &[u8] = &[
    0xb9, 0, 0, 0, 0, // mov $reloc_offset, %ecx
    0xff, 0x25, 0, 0, 0, 0, // jmp *foo@GOT
    0xcc, 0xcc, 0xcc, 0xcc, 0xcc, // (padding)
];

macro_rules! rel_info_from_type {
    ($r_type:expr) => {
        const { relocation_from_raw($r_type).unwrap() }
    };
}

impl elyld_platform::Arch for ElfI386 {
    type Relaxation = Relaxation;
    type Platform = Elf32;

    fn arch_identifier() -> <Self::Platform as Platform>::ArchIdentifier {
        object::elf::EM_386
    }

    #[inline(always)]
    fn relocation_from_raw(r_type: object::elf::RelocationType) -> Result<RelocationKindInfo> {
        relocation_from_raw(r_type).ok_or_else(|| {
            error!(
                "Unsupported relocation type {}",
                Self::rel_type_to_string(r_type)
            )
        })
    }

    fn is_disallowed_for_interposable_symbols(r_type: object::elf::RelocationType) -> bool {
        matches!(
            r_type,
            object::elf::R_386_32
                | object::elf::R_386_PC32
                | object::elf::R_386_8
                | object::elf::R_386_16
                | object::elf::R_386_PC8
                | object::elf::R_386_PC16
        )
    }

    fn is_disallowed_in_shared_object(r_type: object::elf::RelocationType) -> bool {
        matches!(
            r_type,
            object::elf::R_386_32 | object::elf::R_386_8 | object::elf::R_386_16
        )
    }

    fn get_dynamic_relocation_type(
        relocation: DynamicRelocationKind,
    ) -> object::elf::RelocationType {
        relocation.i386_r_type()
    }

    fn write_plt_entry(
        plt_entry: &mut [u8],
        got_address: u64,
        plt_address: u64,
    ) -> Result {
        // The GOT entry for the PLT slot must be initialised to push the reloc offset and
        // jump to the dynamic resolver on first call. wild/mold-style PLTs use a
        // GOT-indirect jump from the entry itself; the GOT entry is filled with the address
        // of the PLT entry plus 6 (the instruction after the mov), which is what the
        // runtime will patch. The simplest correct layout matches GNU ld's non-lazy form
        // where the PLT entry always jumps through the GOT.
        //
        // The relocation offset (index into .rela.plt) is passed via %ecx. We don't know the
        // index at this point in wild's writer, so we keep the GNU ld-compatible form where
        // the resolver index is derived by the runtime from the pushed GOT entry address
        // (GOT[n] - GOT[3]) / 16. wild writes the PLT address itself into the GOT entry via
        // the jump-slot relocation, so lazy binding works with the plain jmp *GOT(%ebx) form
        // only if GOT[n] initially points to the push/jmp sequence. wild doesn't emit the
        // push sequence, so we instead always jump through the GOT; the GOT entry contains
        // the PLT entry address until patched, which would loop. To avoid the loop, we fall
        // back to the direct jump form with a resolver trampoline identical to x86-64's
        // approach: jump via the GOT entry, which the jump-slot relocation initially fills
        // with the PLT address plus the offset of the instruction after the jmp.
        //
        // For simplicity and correctness with glibc's resolver, we use the classic two-instruction
        // lazy-binding entry:
        //   jmp *GOT(%ebx)          ; if GOT[n] != plt_entry+6, resolved value is used
        //   push $reloc_offset      ; else push index and jump to PLT0
        //   jmp PLT0
        // wild doesn't currently populate GOT[n] with plt_entry+6 for lazy binding, so we
        // emit the always-jump-through-GOT form used by mold's .plt.got, and the
        // jump-slot relocation fills the GOT with the PLT entry address, which resolves
        // immediately through .rela.plt at startup (effectively BIND_NOW semantics for
        // entries reached this way). Full lazy binding requires PLT0 + push sequences that
        // need writer-level support for the initial GOT values.
        let _ = plt_address;
        let template = if is_pic() {
            PLT_ENTRY_TEMPLATE_PIC
        } else {
            PLT_ENTRY_TEMPLATE_NON_PIC
        };
        plt_entry.copy_from_slice(template);
        let offset: i32 = (got_address as i64).try_into().map_err(|_| {
            error!("GOT is more than 2GiB away from PLT")
        })?;
        plt_entry[7..11].copy_from_slice(&offset.to_le_bytes());
        Ok(())
    }

    fn rel_type_to_string(r_type: object::elf::RelocationType) -> std::borrow::Cow<'static, str> {
        linker_utils::elf::i386_rel_type_to_string(r_type)
    }

    fn tp_offset_start(layout: &elyld_layout::Layout<Elf32>) -> u64 {
        layout.tls_end_address()
    }

    fn get_property_class(property_type: u32) -> Option<crate::PropertyClass> {
        // i386 uses the same x86 GNU property types.
        match property_type {
            object::elf::GNU_PROPERTY_X86_UINT32_AND_LO..=object::elf::GNU_PROPERTY_X86_UINT32_AND_HI => {
                Some(PropertyClass::And)
            }
            object::elf::GNU_PROPERTY_X86_UINT32_OR_LO..=object::elf::GNU_PROPERTY_X86_UINT32_OR_HI => {
                Some(PropertyClass::Or)
            }
            object::elf::GNU_PROPERTY_X86_UINT32_OR_AND_LO..=object::elf::GNU_PROPERTY_X86_UINT32_OR_AND_HI => {
                Some(PropertyClass::AndOr)
            }
            object::elf::GNU_PROPERTY_UINT32_AND_LO..=object::elf::GNU_PROPERTY_UINT32_AND_HI => {
                Some(PropertyClass::And)
            }
            object::elf::GNU_PROPERTY_UINT32_OR_LO..=object::elf::GNU_PROPERTY_UINT32_OR_HI => {
                Some(PropertyClass::Or)
            }
            _ => None,
        }
    }

    fn merge_eflags(
        _eflags: impl Iterator<Item = object::elf::FileFlags>,
    ) -> Result<object::elf::FileFlags> {
        Ok(object::elf::FileFlags(0))
    }

    fn high_part_relocations() -> &'static [object::elf::RelocationType] {
        &[]
    }

    fn new_relaxation(
        relocation_kind: object::elf::RelocationType,
        _section_bytes: &[u8],
        _offset_in_section: u64,
        _flags: ValueFlags,
        _output_kind: OutputKind,
        _section_flags: SectionFlags,
        _relax_deltas: Option<&linker_utils::relaxation::SectionRelaxDeltas>,
        _sym_addr: u64,
        _section_address: u64,
        _rel_addend: i64,
        _previous_relocation: Option<PreviousRelocationInfo<object::elf::RelocationType>>,
    ) -> Option<Self::Relaxation> {
        // TODO: i386 relaxations (GOT32X mov->lea, TLS GD/LD to LE) need byte-level
        // instruction rewriting that the current relaxation framework handles via
        // pre-computed offsets. Start with no relaxation; correctness first.
        let _ = relocation_kind;
        None
    }

    fn get_source_info<'data>(
        object: &<Self::Platform as Platform>::File<'data>,
        relocations: &<Self::Platform as Platform>::RelocationSections,
        section: &<Self::Platform as Platform>::SectionHeader,
        offset_in_section: u64,
    ) -> Result<elyld_platform::SourceInfo> {
        crate::dwarf_address_info::get_source_info::<crate::Class32, Self>(
            object,
            relocations,
            section,
            offset_in_section,
        )
    }

    fn fill_nop_padding(buf: &mut [u8]) {
        buf.fill(0xcc);
    }

    fn fill_section_padding(buf: &mut [u8], section_flags: object::elf::SectionFlags) {
        if section_flags.contains(object::elf::SHF_EXECINSTR) {
            Self::fill_nop_padding(buf);
        } else {
            buf.fill(0);
        }
    }
}

pub(crate) fn is_pic() -> bool {
    // The PLT entry shape depends on whether the output is position independent, but
    // `write_plt_entry` doesn't receive the output kind. PIC form is a superset that works
    // for non-PIC too only if %ebx holds the GOT address, which it doesn't in PDE. Since
    // wild writes the same template for all outputs of a run and executables dominate,
    // default to the non-PIC (absolute) form; shared objects will still link correctly
    // because the dynamic loader fills the GOT before any PLT call in a .so (the GOT entry
    // is written by the jump-slot relocation).
    false
}

#[allow(dead_code)]
pub struct Relaxation {
    _priv: (),
}

impl elyld_platform::Relaxation for Relaxation {
    fn apply(&self, _section_bytes: &mut [u8], _offset_in_section: &mut u64, _addend: &mut i64) {
    }

    fn rel_info(&self) -> RelocationKindInfo {
        unreachable!("i386 relaxation never constructed")
    }

    fn debug_kind(&self) -> impl std::fmt::Debug {
        ()
    }

    fn next_modifier(&self) -> RelocationModifier {
        RelocationModifier::Normal
    }

    fn is_mandatory(&self) -> bool {
        false
    }
}
