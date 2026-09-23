//#AbstractConfig:default
//#RequiresLinkerPlugin:true
//#Compiler:gcc
//#LinkerDriver:gcc
//#CompArgs:-flto -O1 -fPIC -ffunction-sections -fno-asynchronous-unwind-tables -fno-unwind-tables
//#LinkArgs:-flto -nostdlib -shared -Wl,-z,now,--gc-sections,--version-script=./lto-got-ref.map
//#ReferenceLinkers:bfd,mold
//#RunEnabled:false
//#DiffIgnore:.dynamic.DT_RELA
//#DiffIgnore:.dynamic.DT_RELAENT
//#DiffIgnore:.dynamic.DT_RUNPATH
//#DiffIgnore:file-header.entry
//#ExpectDynSym:keep
//#NoDynSym:hidden_target
//#SkipArch:loongarch64
//#SkipArch:ppc64le
//#Config:shared:default

// `keep` is exported, so its section is a GC root. The pointer initializer
// references `hidden_target`, which a version script localizes after LTO has
// already marked it for export. That reference must still load hidden_target's
// section.

__attribute__((section(".text.hidden_target"), noinline, used)) void hidden_target(void) {}

void (*keep)(void) = hidden_target;
