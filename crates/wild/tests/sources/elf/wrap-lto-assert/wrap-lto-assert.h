#pragma once

#include <assert.h>

#ifdef __cplusplus
extern "C" {
#endif

void wrap_lto_assert_panic(void) __attribute__((noreturn));
void wrap_lto_assert_from_header(void);
void wrap_lto_assert_more(int v);
int wrap_lto_assert_value(void);

#ifdef __cplusplus
}
#endif
