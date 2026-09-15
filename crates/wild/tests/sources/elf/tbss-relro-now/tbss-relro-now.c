// Nix stdenv links shared libraries with `-z now -z relro --compress-debug-sections`
// and 64-byte-aligned `.tbss` (libsodium's `stream` TLS buffer). Compressing debug
// info used to realign `.tbss` in the file without updating PT_DYNAMIC, so glibc
// BIND_NOW parsed zeros and SIGSEGV'd in `elf_dynamic_do_Rela`.

//#Config:default
//#SkipArch: ppc64le
//#Object:runtime.c
//#Shared:tbss-relro-now-lib.c
//#CompSoArgs:-g -fPIC
//#LinkSoArgs:-z now -z relro --no-rosegment --compress-debug-sections=zlib
//#LinkArgs:-z now -z relro
//#Mode:dynamic
//#RunEnabled:true
//#RequiresGlibc:true
//#DiffIgnore:.dynamic.DT_NEEDED
//#DiffIgnore:.dynamic.DT_RELA
//#DiffIgnore:.dynamic.DT_RELAENT
//#DiffIgnore:section.debug_*
//#DiffIgnore:section.got
//#DiffMatchAny:true

#include "../common/runtime.h"

int get_ready(void);

void _start(void) {
  runtime_init();
  if (get_ready() != 2) {
    exit_syscall(20);
  }
  exit_syscall(42);
}
