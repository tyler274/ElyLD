// Weak `__wrap_foo` lives in a separate LTO TU from the caller, matching
// Nix's `--wrap=__assert_fail` (weak LTO `__wrap___assert_fail`). After LTO,
// wrap must resolve to the codegen object, not the disabled IR input.

//#AbstractConfig:default
//#RequiresLinkerPlugin:true
//#Object:runtime.c
//#Object:wrap-lto-weak-def.c:-fno-lto
//#Object:wrap-lto-weak-wrap.c
//#ReferenceLinkers:
//#CompArgs:-flto
//#LinkArgs:-flto -nostdlib -z now -Wl,-wrap,foo
//#ExpectSym:__wrap_foo section=".text"

//#Config:gcc:default
//#LinkerDriver:gcc

//#Config:clang:default
//#Compiler:clang
//#LinkerDriver:clang
//#ReferenceLinkers:lld

#include "../common/runtime.h"

int foo(void);

void _start(void) {
  runtime_init();
  if (foo() != 42) {
    exit_syscall(100);
  }
  exit_syscall(42);
}
