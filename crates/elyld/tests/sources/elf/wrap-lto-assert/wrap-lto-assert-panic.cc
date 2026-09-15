#include "wrap-lto-assert.h"

#include <stdlib.h>
#include <string>

void wrap_lto_assert_panic() {
  std::string why = "wrapped";
  (void)why;
  _Exit(42);
}
