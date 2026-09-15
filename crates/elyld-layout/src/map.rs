//! Writes a GNU-ld-style link map for `-Map=FILE` / `-M` / `--print-map`.
//!
//! The format is the "Linker script and memory map" section: output sections with VMA and size,
//! input sections that contributed to them, and non-local symbols defined in those sections.

use crate::output_section_id::{OrderEvent, OutputSectionId};
use crate::resolution::SectionSlot;
use crate::{EnginePlatform, FileLayout, Layout};
use hashbrown::HashMap;
use object::SectionIndex;
use std::io::Write;
use std::path::Path;
use elyld_args::InputRef;
use elyld_error::error::{Context as _, Result};
use elyld_platform::{Args as _, ObjectFile, Symbol as _};

pub fn maybe_write_map<'data, P: EnginePlatform>(layout: &Layout<'data, P>) -> Result {
    let print_stdout = layout.args().print_map();
    let map_path = layout.args().map_file();
    match (print_stdout, map_path) {
        (false, None) => Ok(()),
        (true, Some(path)) => {
            let mut buf = Vec::new();
            write_map(layout, &mut buf)?;
            std::io::stdout()
                .write_all(&buf)
                .context("Failed to write link map to stdout")?;
            write_map_file(path, &buf)
        }
        (true, None) => {
            let stdout = std::io::stdout();
            write_map(layout, stdout.lock()).context("Failed to write link map to stdout")
        }
        (false, Some(path)) => {
            let file = std::fs::File::create(path)
                .with_context(|| format!("Failed to create link map `{}`", path.display()))?;
            write_map(layout, std::io::BufWriter::new(file))
                .with_context(|| format!("Failed to write link map `{}`", path.display()))
        }
    }
}

fn write_map_file(path: &Path, buf: &[u8]) -> Result {
    std::fs::write(path, buf)
        .with_context(|| format!("Failed to write link map `{}`", path.display()))
}

struct InputContribution {
    vma: u64,
    size: u64,
    name: String,
    file: String,
    symbols: Vec<(u64, String)>,
}

fn write_map<'data, P: EnginePlatform>(layout: &Layout<'data, P>, mut out: impl Write) -> Result {
    writeln!(out, "Linker script and memory map")?;
    writeln!(out)?;

    for group in &layout.group_layouts {
        for file in &group.files {
            match file {
                FileLayout::Object(obj) => {
                    writeln!(out, "LOAD {}", map_input_name(&obj.input))?;
                }
                FileLayout::Dynamic(dyn_obj) => {
                    writeln!(out, "LOAD {}", map_input_name(&dyn_obj.input))?;
                }
                _ => {}
            }
        }
    }
    writeln!(out)?;

    let mut by_output: HashMap<OutputSectionId, Vec<InputContribution>> = HashMap::new();

    for group in &layout.group_layouts {
        for file in &group.files {
            let FileLayout::Object(obj) = file else {
                continue;
            };
            let mut symbols_by_section: Vec<Vec<(u64, String)>> =
                vec![Vec::new(); obj.sections.len()];
            for (symbol_id, resolution) in layout.resolutions_in_range(obj.symbol_id_range) {
                let Some(resolution) = resolution else {
                    continue;
                };
                if !layout.symbol_db.is_canonical(symbol_id) {
                    continue;
                }
                let local = symbol_id.to_input(obj.symbol_id_range);
                let Ok(sym) = obj.object.symbol(local) else {
                    continue;
                };
                if sym.is_undefined() || sym.is_local() || !sym.has_name() {
                    continue;
                }
                let Ok(Some(section_index)) = obj.object.symbol_section(sym, local) else {
                    continue;
                };
                if section_index.0 >= symbols_by_section.len() {
                    continue;
                }
                let name = layout.symbol_db.symbol_name_for_display(symbol_id);
                symbols_by_section[section_index.0].push((resolution.raw_value, name.to_string()));
            }

            for (sec_idx, slot) in obj.sections.iter().enumerate() {
                let Some(size) = loaded_section_size(slot) else {
                    continue;
                };
                let Some(vma) = section_vma(obj, sec_idx, slot) else {
                    continue;
                };
                let section_index = SectionIndex(sec_idx);
                let part_id =
                    obj.section_part_id(section_index, &layout.symbol_db.section_part_ids);
                let output_id = layout
                    .output_sections
                    .primary_output_section(part_id.output_section_id::<P>());
                let Ok(name) = obj.object.section_name(section_index) else {
                    continue;
                };
                let mut symbols = std::mem::take(&mut symbols_by_section[sec_idx]);
                symbols.sort_by_key(|(sym_vma, _)| *sym_vma);
                by_output
                    .entry(output_id)
                    .or_default()
                    .push(InputContribution {
                        vma,
                        size,
                        name: String::from_utf8_lossy(name).into_owned(),
                        file: map_input_name(&obj.input),
                        symbols,
                    });
            }
        }
    }

    for contributions in by_output.values_mut() {
        contributions.sort_by_key(|c| c.vma);
    }

    for event in &layout.output_order {
        let OrderEvent::Section(section_id) = event else {
            continue;
        };
        let primary = layout.output_sections.primary_output_section(section_id);
        if primary != section_id {
            continue;
        }
        let record = layout.merged_section_layouts.get(primary);
        let contributions = by_output.get(&primary).map(Vec::as_slice).unwrap_or(&[]);
        if record.mem_size == 0 && contributions.is_empty() {
            continue;
        }
        let name = output_section_name(&layout.output_sections, primary);
        if name.is_empty() && contributions.is_empty() {
            continue;
        }
        write_named_record(&mut out, &name, record.mem_offset, record.mem_size)?;
        for contrib in contributions {
            write_input_section(&mut out, contrib)?;
            for (sym_vma, sym_name) in &contrib.symbols {
                writeln!(out, "{:16}{sym_vma:#018x}                {sym_name}", "")?;
            }
        }
    }

    Ok(())
}

fn output_section_name<P: EnginePlatform>(
    output_sections: &crate::output_section_id::OutputSections<'_, P>,
    section_id: OutputSectionId,
) -> String {
    output_sections
        .name(section_id)
        .map(|name| name.to_string())
        .unwrap_or_else(|| format!("section-{}", section_id.as_usize()))
}

fn write_named_record(out: &mut impl Write, name: &str, vma: u64, size: u64) -> Result {
    if name.len() > 15 {
        writeln!(out, "{name}")?;
        writeln!(out, "{:16}{vma:#018x} {size:#10x}", "")?;
    } else {
        writeln!(out, "{name:<16}{vma:#018x} {size:#10x}")?;
    }
    Ok(())
}

fn write_input_section(out: &mut impl Write, contrib: &InputContribution) -> Result {
    let name = format!(" {}", contrib.name);
    if name.len() > 16 {
        writeln!(out, "{name}")?;
        writeln!(
            out,
            "{:16}{:#018x} {:#10x} {}",
            "", contrib.vma, contrib.size, contrib.file
        )?;
    } else {
        writeln!(
            out,
            "{name:<16}{:#018x} {:#10x} {}",
            contrib.vma, contrib.size, contrib.file
        )?;
    }
    Ok(())
}

fn loaded_section_size(slot: &SectionSlot) -> Option<u64> {
    match slot {
        SectionSlot::Loaded(sec) | SectionSlot::LoadedDebugInfo(sec) => Some(sec.size),
        SectionSlot::Sorted(sorted) => Some(sorted.section.size),
        SectionSlot::PartialLinkSingleton(singleton) => Some(singleton.section.size),
        _ => None,
    }
}

fn section_vma<P: EnginePlatform>(
    obj: &crate::ObjectLayout<P>,
    sec_idx: usize,
    slot: &SectionSlot,
) -> Option<u64> {
    if let SectionSlot::Sorted(sorted) = slot {
        return Some(sorted.address);
    }
    obj.section_resolutions.get(sec_idx)?.address()
}

fn map_input_name(input: &InputRef<'_>) -> String {
    if let Some(entry) = &input.entry {
        format!(
            "{}({})",
            input.file.filename.display(),
            String::from_utf8_lossy(entry.identifier.as_slice())
        )
    } else {
        input.file.filename.display().to_string()
    }
}
