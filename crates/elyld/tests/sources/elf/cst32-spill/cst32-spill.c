// More merge classes than hash buckets. The last class is align-1, and
// the spilled .rodata.cst32 has sh_addralign 32. Its address must be
// 32-aligned or an aligned AVX load of the pool faults.
//
//#AbstractConfig:default
//#LinkArgs:-z noexecstack
//#Object:spill.s
//#Object:runtime.c
//#Arch:x86_64
//#DiffEnabled:false
//
//#Config:cst32_spill:default

#include "../common/runtime.h"

extern const unsigned char mask[];
extern const unsigned char p2[];
extern const unsigned char p3[];
extern const unsigned char p4[];
extern const unsigned char p5[];
extern const unsigned char p6[];
extern const unsigned char p7[];
extern const unsigned char p8[];
extern const unsigned char p9[];
extern const unsigned char p10[];
extern const unsigned char p11[];
extern const unsigned char p12[];
extern const unsigned char p13[];
extern const unsigned char p14[];
extern const unsigned char p15[];
extern const unsigned char p16[];
extern const unsigned char p17[];

void _start(void) {
  runtime_init();
  if (((unsigned long)mask & 31) != 0) {
    exit_syscall(101);
  }
  if (mask[0] != 0x20) {
    exit_syscall(102);
  }
  if (p2[0] != 1) exit_syscall(112);
  if (p3[0] != 1) exit_syscall(113);
  if (p4[0] != 1) exit_syscall(114);
  if (p5[0] != 1) exit_syscall(115);
  if (p6[0] != 1) exit_syscall(116);
  if (p7[0] != 1) exit_syscall(117);
  if (p8[0] != 1) exit_syscall(118);
  if (p9[0] != 1) exit_syscall(119);
  if (p10[0] != 1) exit_syscall(120);
  if (p11[0] != 1) exit_syscall(121);
  if (p12[0] != 1) exit_syscall(122);
  if (p13[0] != 1) exit_syscall(123);
  if (p14[0] != 1) exit_syscall(124);
  if (p15[0] != 1) exit_syscall(125);
  if (p16[0] != 1) exit_syscall(126);
  if (p17[0] != 1) exit_syscall(127);
  exit_syscall(42);
}
