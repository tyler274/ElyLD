//! i386-specific relocation definitions.
//!
//! i386 uses REL-style relocations: the addend is stored in the bytes at the relocation
//! offset rather than in the relocation entry itself. The layout engine reads the addend
//! from the section contents, so the kinds here match the x86-64 kinds where the formula
//! is the same.

use crate::elf::{AllowedRange, RelocationKind, RelocationKindInfo, RelocationSize, Sign};

type RelocSizeAndRange = (RelocationSize, AllowedRange);

const RELOC_4_BYTE_SIGNED: RelocSizeAndRange = (
    RelocationSize::ByteSize(4),
    AllowedRange::from_byte_size(4, Sign::Signed),
);
const RELOC_4_BYTE_UNSIGNED: RelocSizeAndRange = (
    RelocationSize::ByteSize(4),
    AllowedRange::from_byte_size(4, Sign::Unsigned),
);
const RELOC_2_BYTE_SIGNED: RelocSizeAndRange = (
    RelocationSize::ByteSize(2),
    AllowedRange::from_byte_size(2, Sign::Signed),
);
const RELOC_2_BYTE_UNSIGNED: RelocSizeAndRange = (
    RelocationSize::ByteSize(2),
    AllowedRange::from_byte_size(2, Sign::Unsigned),
);
const RELOC_1_BYTE_SIGNED: RelocSizeAndRange = (
    RelocationSize::ByteSize(1),
    AllowedRange::from_byte_size(1, Sign::Signed),
);
const RELOC_1_BYTE_UNSIGNED: RelocSizeAndRange = (
    RelocationSize::ByteSize(1),
    AllowedRange::from_byte_size(1, Sign::Unsigned),
);
const RELOC_NONE: RelocSizeAndRange = (RelocationSize::ByteSize(0), AllowedRange::no_check());

/// Returns the supplied i386 relocation as RelocationKindInfo. Returns `None` if the r_type
/// isn't recognised.
#[must_use]
#[inline(always)]
pub const fn relocation_from_raw(
    r_type: object::elf::RelocationType,
) -> Option<RelocationKindInfo> {
    let (kind, (size, range)) = match r_type {
        object::elf::R_386_NONE => (RelocationKind::None, RELOC_NONE),
        object::elf::R_386_32 => (RelocationKind::Absolute, RELOC_4_BYTE_UNSIGNED),
        object::elf::R_386_PC32 => (RelocationKind::Relative, RELOC_4_BYTE_SIGNED),
        object::elf::R_386_GOT32 => (RelocationKind::GotRelGotBase, RELOC_4_BYTE_SIGNED),
        object::elf::R_386_GOT32X => (RelocationKind::GotRelGotBase, RELOC_4_BYTE_SIGNED),
        object::elf::R_386_PLT32 => (RelocationKind::PltRelative, RELOC_4_BYTE_SIGNED),
        object::elf::R_386_GOTOFF => (RelocationKind::SymRelGotBase, RELOC_4_BYTE_SIGNED),
        object::elf::R_386_GOTPC => (RelocationKind::Relative, RELOC_4_BYTE_SIGNED),
        object::elf::R_386_16 => (RelocationKind::Absolute, RELOC_2_BYTE_UNSIGNED),
        object::elf::R_386_PC16 => (RelocationKind::Relative, RELOC_2_BYTE_SIGNED),
        object::elf::R_386_8 => (RelocationKind::Absolute, RELOC_1_BYTE_UNSIGNED),
        object::elf::R_386_PC8 => (RelocationKind::Relative, RELOC_1_BYTE_SIGNED),
        object::elf::R_386_32PLT => (RelocationKind::PltRelative, RELOC_4_BYTE_SIGNED),
        object::elf::R_386_TLS_TPOFF => (RelocationKind::TpOff, RELOC_4_BYTE_SIGNED),
        object::elf::R_386_TLS_GD => (RelocationKind::TlsGd, RELOC_4_BYTE_SIGNED),
        object::elf::R_386_TLS_LDM => (RelocationKind::TlsLd, RELOC_4_BYTE_SIGNED),
        object::elf::R_386_TLS_LDO_32 => (RelocationKind::DtpOff, RELOC_4_BYTE_SIGNED),
        object::elf::R_386_TLS_IE => (RelocationKind::GotTpOff, RELOC_4_BYTE_SIGNED),
        object::elf::R_386_TLS_GOTIE => (RelocationKind::GotTpOffGotBase, RELOC_4_BYTE_SIGNED),
        object::elf::R_386_TLS_LE => (RelocationKind::TpOff, RELOC_4_BYTE_SIGNED),
        object::elf::R_386_TLS_GD_32 => (RelocationKind::TlsGd, RELOC_4_BYTE_SIGNED),
        object::elf::R_386_TLS_GD_CALL => (RelocationKind::PltRelative, RELOC_4_BYTE_SIGNED),
        object::elf::R_386_TLS_GOTDESC => (RelocationKind::TlsDescGotBase, RELOC_4_BYTE_SIGNED),
        object::elf::R_386_TLS_DESC_CALL => (RelocationKind::TlsDescCall, RELOC_NONE),
        object::elf::R_386_TLS_DESC => (RelocationKind::TlsDesc, RELOC_4_BYTE_UNSIGNED),
        object::elf::R_386_SIZE32 => (RelocationKind::AbsoluteSet, RELOC_4_BYTE_UNSIGNED),
        _ => return None,
    };

    Some(RelocationKindInfo {
        kind,
        size,
        mask: None,
        range,
        alignment: 1,
        bias: 0,
        thunkable: false,
    })
}
