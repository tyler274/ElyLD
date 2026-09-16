// Keep LLVM PGO / coverage and GCC gcov section names under --gc-sections
// and --orphan-handling=error. These names are otherwise Custom orphans.

//#Config:default
//#Object:runtime.c
//#LinkArgs:-nostdlib -znow --gc-sections --orphan-handling=error
//#DiffEnabled:false
//#RunEnabled:false
//#ReferenceLinkers:
//#Contains:__llvm_prf_cnts
//#Contains:__llvm_covmap
//#Contains:__gcov_ctr

#include "../common/runtime.h"

__attribute__((section("__llvm_prf_cnts"), used)) unsigned char llvm_prf_cnts[8];
__attribute__((section("__llvm_covmap"), used)) unsigned char llvm_covmap[8];
__attribute__((section("__gcov_ctr"), used)) unsigned char gcov_ctr[8];

void _start(void) {
  runtime_init();
  exit_syscall(42);
}
