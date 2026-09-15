// Nix's wrap-assert-fail.cc: weak, extern "C", [[noreturn]], in a C++ LTO
// TU that includes headers using assert and calls other LTO C++ code.

#include "wrap-lto-assert.h"

#include <stdlib.h>
#include <string>

extern "C" [[noreturn]] void __attribute__((weak))
__wrap___assert_fail(const char *assertion, const char *file, unsigned line,
                     const char *function) {
  (void)assertion;
  (void)file;
  (void)line;
  (void)function;
  // Pull a C++ LTO definition into this TU the way Nix pulls nix::panic.
  wrap_lto_assert_panic();
}

void wrap_lto_assert_from_header() { assert(1); }
