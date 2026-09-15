// GNU ld copies undefined DSO symbols into .symtab as well as .dynsym.
// `nm -an` then prints blank-address `U` lines; `cut -f3` turns those into
// empty records. nixpkgs blas-3 greps `^$symbol_$`, which bash expands as
// `^$`, so the check only passes when those empty lines exist.

//#Config:default
//#SkipArch: ppc64le
//#Object:runtime.c
//#Shared:undefined-dso-symtab-lib.c
//#Mode:dynamic
//#LinkArgs:-z now
//#RunEnabled:true
//#ExpectSym:from_dso section="UND"
//#ExpectDynSym:from_dso
//#DiffIgnore:.dynamic.DT_NEEDED
//#DiffIgnore:.dynamic.DT_RELA
//#DiffIgnore:.dynamic.DT_RELAENT
//#DiffIgnore:section.got
//#DiffMatchAny:true

#include "../common/runtime.h"

int from_dso(void);

void _start(void) {
  runtime_init();
  if (from_dso() != 7) {
    exit_syscall(20);
  }
  exit_syscall(42);
}
