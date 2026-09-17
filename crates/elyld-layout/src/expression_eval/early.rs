use super::{ResolvedLocationCounter, SymbolValue};
use crate as layout;
use crate::grouping::Group;
use crate::layout_rules::SectionKind;
use crate::output_section_id::{OutputSectionId, OutputSections};
use crate::output_section_part_map::OutputSectionPartMap;
use crate::parsing::{SymbolLoc, SymbolPlacement};
use crate::symbol::UnversionedSymbolName;
use crate::symbol_db::{SymbolDb, SymbolId};
use crate::{
    EnginePlatform, FileLayoutState, GroupState, InputSectionPositions, MemoryRegion,
    OutputRecordLayout,
};
use hashbrown::{HashMap, HashSet};
use std::cell::OnceCell;
use elyld_error::bail;
use elyld_error::error::Result;
use elyld_platform::output_section_map::OutputSectionMap;
use elyld_scripts::linker_script::Expression;

/// End VMA of `section_id`, including secondary contributions that have not yet been
/// merged into the primary. Needed for `_etext = .` / `text_size = _etext - _stext` while
/// later sections (`.orc_lookup`) are still being laid out.
pub fn section_mem_end<'data, P: EnginePlatform>(
    section_id: OutputSectionId,
    section_layouts: &OutputSectionMap<OutputRecordLayout>,
    output_sections: &OutputSections<'data, P>,
) -> u64 {
    let primary = output_sections.primary_output_section(section_id);
    let primary_layout = section_layouts.get(primary);
    let mut end = primary_layout.mem_offset + primary_layout.mem_size;
    for (id, info) in output_sections.ids_with_info() {
        if matches!(info.kind, SectionKind::Secondary(p) if p == primary) {
            let layout = section_layouts.get(id);
            end = end.max(layout.mem_offset + layout.mem_size);
        }
    }
    end
}

pub fn evaluate_early_expression<'data, P: EnginePlatform>(
    expr: &Expression<'data>,
    loc: &SymbolLoc,
    memory_regions: &HashMap<&[u8], MemoryRegion>,
    section_layouts: &OutputSectionMap<OutputRecordLayout>,
    resolved_lc: &[ResolvedLocationCounter],
    laid_out_mem_offsets: &OutputSectionPartMap<Option<u64>>,
    group_states: &[GroupState<'data, P>],
    sizes: &OutputSectionPartMap<u64>,
    output_sections: &OutputSections<'data, P>,
    symbol_db: &SymbolDb<'data, P>,
    sizeof_headers: u64,
    section_positions: &OnceCell<InputSectionPositions>,
    visited_nodes: &mut HashSet<SymbolId>,
    const_script_symbols: &HashMap<&[u8], u64>,
    visiting: &mut Vec<Vec<u8>>,
) -> Result<u64> {
    crate::expression_eval::evaluate_expression(
        expr,
        loc,
        None,
        section_layouts,
        output_sections,
        memory_regions,
        symbol_db,
        sizeof_headers,
        resolved_lc,
        laid_out_mem_offsets,
        &mut |name| {
            if let Some(&value) = const_script_symbols.get(name) {
                return Ok(SymbolValue::Absolute(value));
            }

            if visiting.iter().any(|seen| seen.as_slice() == name) {
                return Ok(SymbolValue::Absolute(0));
            }

            let Some(symbol_id) =
                symbol_db.get_unversioned(&UnversionedSymbolName::prehashed(name))
            else {
                bail!(
                    "undefined symbol '{}' in linker script expression",
                    String::from_utf8_lossy(name)
                );
            };

            let canonical_id = symbol_db.definition(symbol_id);
            let file_id = symbol_db.file_id_for_symbol(canonical_id);
            let file = group_states
                .get(file_id.group())
                .and_then(|group| group.files.get(file_id.file()));
            match file {
                Some(FileLayoutState::Object(obj)) => {
                    let value = layout::resolve_early_object_symbol(
                        canonical_id,
                        obj,
                        section_positions.get_or_init(|| {
                            // Start from current part VMAs so input-section alignment is
                            // applied to the output address, matching GNU ld and the
                            // post-layout symbol assignment pass.
                            layout::compute_input_section_positions(
                                group_states,
                                laid_out_mem_offsets.map(|_, vma| vma.unwrap_or(0)),
                                symbol_db,
                                output_sections,
                            )
                        }),
                        symbol_db,
                    )?;
                    // Those positions are already output VMAs when the part has been laid
                    // out; `PartRelative` would add the part start a second time.
                    Ok(match value {
                        SymbolValue::PartRelative { part_id, offset }
                            if laid_out_mem_offsets.get(part_id).is_some() =>
                        {
                            SymbolValue::Absolute(offset)
                        }
                        other => other,
                    })
                }
                Some(FileLayoutState::LinkerScript(ls))
                    if let Group::LinkerScripts(scripts) = &symbol_db.groups[file_id.group()] =>
                {
                    let script = &scripts[file_id.file()];
                    let symbol_offset = ls.symbol_id_range.id_to_offset(canonical_id);

                    let def_info = &script.parsed.symbol_defs[symbol_offset];
                    visiting.push(name.to_vec());
                    let result = evaluate_early_expression_internal_symbol(
                        memory_regions,
                        section_layouts,
                        resolved_lc,
                        laid_out_mem_offsets,
                        group_states,
                        sizes,
                        output_sections,
                        symbol_db,
                        sizeof_headers,
                        section_positions,
                        visited_nodes,
                        const_script_symbols,
                        visiting,
                        canonical_id,
                        def_info,
                    );
                    visiting.pop();
                    result
                }
                _ => Ok(SymbolValue::Absolute(layout::layout_time_symbol_value(
                    name,
                    symbol_db,
                    section_layouts,
                    output_sections,
                    memory_regions,
                    loc,
                    sizeof_headers,
                    resolved_lc,
                    const_script_symbols,
                    visiting,
                )?)),
            }
        },
    )
}

fn evaluate_early_expression_internal_symbol<'data, P: EnginePlatform>(
    memory_regions: &HashMap<&[u8], MemoryRegion>,
    section_layouts: &OutputSectionMap<OutputRecordLayout>,
    resolved_lc: &[ResolvedLocationCounter],
    laid_out_mem_offsets: &OutputSectionPartMap<Option<u64>>,
    group_states: &[GroupState<'data, P>],
    sizes: &OutputSectionPartMap<u64>,
    output_sections: &OutputSections<'data, P>,
    symbol_db: &SymbolDb<'data, P>,
    sizeof_headers: u64,
    section_positions: &OnceCell<InputSectionPositions>,
    visited_nodes: &mut HashSet<SymbolId>,
    const_script_symbols: &HashMap<&[u8], u64>,
    visiting: &mut Vec<Vec<u8>>,
    canonical_id: SymbolId,
    def_info: &crate::parsing::InternalSymDefInfo<'data, P>,
) -> Result<SymbolValue> {
    match &def_info.placement {
        SymbolPlacement::Redirect(redirect) => {
            if !visited_nodes.insert(canonical_id) {
                return Ok(SymbolValue::Absolute(0));
            }
            let value = evaluate_early_expression(
                &redirect.expression,
                &redirect.loc,
                memory_regions,
                section_layouts,
                resolved_lc,
                laid_out_mem_offsets,
                group_states,
                sizes,
                output_sections,
                symbol_db,
                sizeof_headers,
                section_positions,
                visited_nodes,
                const_script_symbols,
                visiting,
            );
            visited_nodes.remove(&canonical_id);
            let value = value?;
            let symbol_section = redirect
                .loc
                .relative_section_id()
                .map(|id| output_sections.primary_output_section(id));
            if let Some(symbol_section) = symbol_section {
                Ok(SymbolValue::SectionRelative {
                    section_id: symbol_section,
                    address: value,
                })
            } else {
                Ok(SymbolValue::Absolute(value))
            }
        }
        SymbolPlacement::SectionStart(section_id) => Ok(SymbolValue::SectionRelative {
            section_id: *section_id,
            address: 0,
        }),
        SymbolPlacement::SectionEnd(section_id) => Ok(SymbolValue::SectionRelative {
            section_id: *section_id,
            address: section_layouts.get(*section_id).mem_size,
        }),
        _ => {
            bail!("Unsupported symbol type");
        }
    }
}
