#include "wrap-lto-assert.h"

void wrap_lto_assert_more(int v) { assert(v); }

int wrap_lto_assert_value(void) { return 1; }
