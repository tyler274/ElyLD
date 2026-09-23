use super::output_section_id::OutputSectionId;
use super::{Platform, SectionAttributes as _};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrphanClass {
    Exec,
    Ro,
    Data,
    Bss,
    Tdata,
    Tbss,
    NonAlloc,
}

#[derive(Default, Clone)]
pub struct CustomSectionIds {
    pub ro: Vec<OutputSectionId>,
    pub exec: Vec<OutputSectionId>,
    pub data: Vec<OutputSectionId>,
    pub bss: Vec<OutputSectionId>,
    pub nonalloc: Vec<OutputSectionId>,
    pub tdata: Vec<OutputSectionId>,
    pub tbss: Vec<OutputSectionId>,
    /// When a replacing `-T` script is present, place unnamed (orphan) output
    /// sections after the last section with the same flags, matching GNU ld.
    /// `INSERT` fragments and GROUP/OUTPUT_FORMAT scripts (e.g. `libc.so`)
    /// splice into or keep the default layout and do not set this.
    pub place_after_similar: bool,
    /// Replacing script, no `PHDRS` and no `MEMORY`: contiguous allocatable
    /// sections share one `PT_LOAD` until a script address assignment.
    pub pack_script_loads: bool,
    /// Script-mentioned custom sections emitted immediately after the previous
    /// builtin named in `SECTIONS`. Without this, those sections are grouped
    /// with orphans (e.g. RO customs before `.text`) and GNU `AT>` LMA
    /// continuation never sees them.
    pub script_followers: Vec<(OutputSectionId, OutputSectionId)>,
}

impl OrphanClass {
    /// GNU places orphans after the last section with similar flags, not between
    /// `.data` and `.bss` (both writable ALLOC) or `.tdata` and `.tbss`.
    pub fn similar_to(self, other: OrphanClass) -> bool {
        use OrphanClass::*;
        match (self, other) {
            (Data | Bss, Data | Bss) => true,
            (Tdata | Tbss, Tdata | Tbss) => true,
            (a, b) => a == b,
        }
    }
}

impl CustomSectionIds {
    pub fn class_of<P: Platform>(attr: &P::SectionAttributes) -> OrphanClass {
        if attr.is_executable() {
            OrphanClass::Exec
        } else if attr.is_tls() {
            if attr.is_no_bits() {
                OrphanClass::Tbss
            } else {
                OrphanClass::Tdata
            }
        } else if !attr.is_writable() {
            if attr.is_alloc() {
                OrphanClass::Ro
            } else {
                OrphanClass::NonAlloc
            }
        } else if attr.is_no_bits() {
            OrphanClass::Bss
        } else {
            OrphanClass::Data
        }
    }

    pub fn take_class(&mut self, class: OrphanClass) -> Vec<OutputSectionId> {
        match class {
            OrphanClass::Exec => core::mem::take(&mut self.exec),
            OrphanClass::Ro => core::mem::take(&mut self.ro),
            OrphanClass::Data => core::mem::take(&mut self.data),
            OrphanClass::Bss => core::mem::take(&mut self.bss),
            OrphanClass::Tdata => core::mem::take(&mut self.tdata),
            OrphanClass::Tbss => core::mem::take(&mut self.tbss),
            OrphanClass::NonAlloc => core::mem::take(&mut self.nonalloc),
        }
    }

    /// Flush every orphan class that is similar to `class`, keeping PROGBITS
    /// before NOBITS so `.data` orphans still precede `.bss` orphans.
    pub fn take_similar(&mut self, class: OrphanClass) -> Vec<OutputSectionId> {
        use OrphanClass::*;
        match class {
            Data | Bss => {
                let mut ids = core::mem::take(&mut self.data);
                ids.append(&mut self.bss);
                ids
            }
            Tdata | Tbss => {
                let mut ids = core::mem::take(&mut self.tdata);
                ids.append(&mut self.tbss);
                ids
            }
            other => self.take_class(other),
        }
    }
}
