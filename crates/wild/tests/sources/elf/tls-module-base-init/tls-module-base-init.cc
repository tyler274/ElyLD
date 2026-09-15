// GCC `__tls_init` for `thread_local` objects with constructors uses
// TLSDESC(`_TLS_MODULE_BASE_`) plus DTPOFF. On executables the module base
// must sit at the alignment-rounded TLS end (the thread pointer), not the
// last `.tbss` byte. A 64-byte-aligned TLS object makes that padding visible
// (onetbb/doctest SIGSEGV in `__tls_init.part.0`).

//#AbstractConfig:default
//#LinkArgs:-Wl,-z,now
//#CompArgs:-fPIE
//#DiffIgnore:section.rodata
//#DiffIgnore:section.data
//#DiffIgnore:version._ZSt21ios_base_library_initv

//#Config:gcc:default
//#SkipArch: ppc64le
//#LinkerDriver:g++

#include <sstream>

thread_local char aligned_tls[64] __attribute__((aligned(64)));
thread_local std::ostringstream oss;

int main() {
  aligned_tls[0] = 1;
  oss << "ok";
  if (oss.str() != "ok" || aligned_tls[0] != 1) {
    return 1;
  }
  return 42;
}
