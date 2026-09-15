// Nix links libutil with `-flto=auto` and `-Wl,--wrap=__assert_fail`. The
// wrap is a weak C++ `extern "C"` symbol in a separate LTO object that
// includes headers using `assert` and calls other LTO C++ (nix::panic).
// Relocs in the LTO output still name `__assert_fail` and must resolve to
// `__wrap___assert_fail` from codegen, not the disabled IR input.

//#AbstractConfig:default
//#RequiresLinkerPlugin:true
//#SkipArch:ppc64le
//#Object:wrap-lto-assert-wrap.cc
//#Object:wrap-lto-assert-panic.cc
//#Object:wrap-lto-assert-more.c
//#ReferenceLinkers:
//#CompArgs:-flto=auto
//#LinkArgs:-flto=auto -Wl,--wrap=__assert_fail -Wl,--no-undefined,-z,now
//#ExpectSym:__wrap___assert_fail section=".text"
//#DiffIgnore:section.rodata
//#DiffIgnore:section.got
//#DiffIgnore:eh_frame
//#DiffIgnore:section.data

//#Config:gcc:default
//#LinkerDriver:g++

#include "wrap-lto-assert.h"

int main(void) {
  wrap_lto_assert_more(1);
  wrap_lto_assert_from_header();
  if (wrap_lto_assert_value() != 1) {
    return 1;
  }
  __assert_fail("wrapped", __FILE__, 1, __func__);
  return 1;
}
