// C++ template instantiations are COMDAT. Linker-plugin LTO must merge them
// across TUs (mold lto-comdat.sh).

// C++ template instantiations are COMDAT. Linker-plugin LTO must merge them
// across TUs (mold lto-comdat.sh).

//#AbstractConfig:default
//#Object:lto-comdat-2.cc
//#SkipArch:ppc64le

//#Config:gcc:default
//#CompArgs:-flto -O2
//#LinkerDriver:g++
//#LinkArgs:-flto -O2 -Wl,-z,now
//#ReferenceLinkers:bfd

//#Config:clang:default
//#Compiler:clang
//#CompArgs:-flto -O2
//#LinkerDriver:clang++
//#LinkArgs:-flto -O2 -Wl,-z,now
//#ReferenceLinkers:lld,mold
// GNU ld aligns `.gnu.version_r` to 8; lld and mold use 4. crtbeginS/crti
// GOT and PLT addresses differ across all three linkers.
//#DiffIgnore:section.gnu.version_r.alignment
//#DiffIgnore:rel.R_X86_64_REX_GOTPCRELX*
//#DiffIgnore:rel.R_X86_64_PLT32*

//#Config:clang-thin:default
//#Compiler:clang
//#CompArgs:-flto=thin -O2
//#LinkerDriver:clang++
//#LinkArgs:-flto=thin -O2 -Wl,-z,now
//#ReferenceLinkers:lld
//#DiffIgnore:section.gnu.version_r.alignment

template <typename T>
inline T add(T a, T b) {
  return a + b;
}

int from_other();

int main() {
  int x = add(20, 22);
  int y = from_other();
  if (x != 42 || y != 42) {
    return 1;
  }
  return 42;
}
