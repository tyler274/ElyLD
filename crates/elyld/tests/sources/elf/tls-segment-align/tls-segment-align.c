// A DSO's TLS block is aligned to the strictest TLS section. .tdata is often
// only 1- or 4-byte aligned while .tbss holds a 64-byte thread_local. The
// symbol's st_value is its offset from the start of that block, so the block
// start has to carry the 64-byte alignment. Otherwise dlopen adds a misaligned
// offset and an aligned store faults.

//#AbstractConfig:default
//#SkipArch: ppc64le
//#CompArgs:-fPIC -ftls-model=global-dynamic
//#LinkArgs:-shared -z now
//#RunDynSym:entry
//#ExpectSym:tls_big type=tls,alignment=64

//#Config:shared:default

#include <stdalign.h>

__thread char tls_anchor = 1;
__thread alignas(64) char tls_big[56];

int entry(void) {
  unsigned long addr = (unsigned long)&tls_big;
  if (addr % 64 != 0) {
    return 1;
  }
  tls_big[0] = 7;
  if (tls_big[0] != 7 || tls_anchor != 1) {
    return 2;
  }
  return 42;
}
