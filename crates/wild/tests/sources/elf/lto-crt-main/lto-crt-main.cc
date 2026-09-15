// nix-store-tests: LTO `main` plus CRT (`Scrt1.o` GOTPCRELX) and often
// libgtest_main.so which also exports `main`. The IR `main` must stay
// prevailing so the CRT reloc is not left pointing at a disabled LTO input.

//#AbstractConfig:default
//#RequiresLinkerPlugin:true
//#SkipArch:ppc64le
//#CompArgs:-flto=auto
//#LinkArgs:-flto=auto -Wl,--as-needed,--no-undefined,-z,now
//#ReferenceLinkers:
//#DiffIgnore:section.rodata
//#DiffIgnore:section.got
//#DiffIgnore:eh_frame
//#DiffIgnore:section.data

//#Config:gcc:default
//#LinkerDriver:g++

//#Config:gcc-gtest-main:default
//#Shared:lto-crt-main-so.c
//#LinkerDriver:g++

int main() { return 42; }
