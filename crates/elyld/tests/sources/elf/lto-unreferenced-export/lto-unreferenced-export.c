//#AbstractConfig:default
//#RequiresLinkerPlugin:true
//#Compiler:gcc
//#LinkerDriver:gcc
//#CompArgs:-flto -O2 -fPIC -fno-asynchronous-unwind-tables -fno-unwind-tables
//#RunEnabled:false
//#DiffIgnore:section.got
//#DiffIgnore:.dynamic.DT_RELA
//#DiffIgnore:.dynamic.DT_RELAENT
//#DiffIgnore:file-header.entry
//#SkipArch:loongarch64
//#SkipArch:ppc64le

// Nothing in the link references these symbols. Under --gc-sections a shared
// object must still keep a default-visibility definition so it can be exported.
// Unwind tables are off so .eh_frame cannot retain .text on their behalf.

//#Config:version-script:default
//#LinkArgs:-flto -nostdlib -shared -Wl,-z,now,--gc-sections,--version-script=./lto-unreferenced-export.map
// GNU ld and mold apply the version script to LTO symbols. LLD rejects the
// script with "symbol not defined" because it assigns versions before LTO
// codegen, so it is not a reference for this config.
//#ReferenceLinkers:bfd,mold
//#ExpectDynSym:PyInit_mod
//#NoDynSym:_AsyncioDebug
//#NoSym:_AsyncioDebug

//#Config:export-all:default
//#LinkArgs:-flto -nostdlib -shared -Wl,-z,now,--gc-sections
// GNU ld and mold export both unreferenced default-visibility definitions.
// LLD 21 does not: its LTO internalises them, so dynsym lacks PyInit_mod and
// _AsyncioDebug. ElyLD follows GNU ld and mold.
//#ReferenceLinkers:bfd,mold
//#ExpectDynSym:PyInit_mod
//#ExpectDynSym:_AsyncioDebug

__attribute__((section(".AsyncioDebug"))) int _AsyncioDebug = 1;

void PyInit_mod(void) {}
