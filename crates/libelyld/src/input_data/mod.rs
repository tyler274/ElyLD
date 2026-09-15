//! Code for figuring out what input files we need to read then mapping them into memory.

pub(crate) mod load;

#[allow(unused_imports)]
pub(crate) use load::*;
pub(crate) use elyld_args::{InputLinkerScript, InputRef};
pub(crate) use elyld_layout::input_data::{AuxiliaryFiles, FileLoader, InputFile, InputPath};
pub(crate) use elyld_scripts::ScriptData;
