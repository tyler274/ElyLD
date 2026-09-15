// systemd-minimal-libs links `libsystemd-shared` with `--gc-sections` and
// `__start_`/`__stop_` on GNU_RETAIN custom sections that do not start with `.`
// (SYSTEMD_BUS_ERROR_MAP). The start/stop GC map must be sized after those
// custom output-section IDs are assigned.

//#Config:default
//#SkipArch: ppc64le
//#CompArgs:-fPIC
//#LinkArgs:-shared -z now -z relro --warn-common --no-undefined --version-script=./start-stop-gc-shared.sym
//#RunEnabled:false
//#Mode:dynamic
//#DiffIgnore:.dynamic.DT_RELA
//#DiffIgnore:.dynamic.DT_RELAENT
//#DiffIgnore:.dynamic.DT_NEEDED
//#ExpectDynSym:live_start_stop
//#ExpectSection:SYSTEMD_BUS_ERROR_MAP
//#ExpectSection:.note.dlopen
//#ExpectSym:__start_SYSTEMD_BUS_ERROR_MAP
//#ExpectSym:__stop_SYSTEMD_BUS_ERROR_MAP

#include <stdint.h>

extern const int __start_SYSTEMD_BUS_ERROR_MAP[];
extern const int __stop_SYSTEMD_BUS_ERROR_MAP[];

__attribute__((retain, used, section("SYSTEMD_BUS_ERROR_MAP"), aligned(8)))
static const int bus_error_map[] = {1, 2, 3};

__attribute__((retain, used, section("SYSTEMD_CMDLINE_OPTIONS"), aligned(8)))
static const int cmdline_options[] = {4};

__attribute__((used, section(".note.dlopen"), aligned(4))) static const uint32_t
    note_dlopen[3] = {4, 0, 0};

__attribute__((constructor(101))) static void ctor_101(void) {}
__attribute__((constructor(200))) static void ctor_200(void) {}
__attribute__((constructor(65535))) static void ctor_65535(void) {}

int live_start_stop(void) {
  int sum = 0;
  for (const int* p = __start_SYSTEMD_BUS_ERROR_MAP; p < __stop_SYSTEMD_BUS_ERROR_MAP;
       p++) {
    sum += *p;
  }
  return sum + cmdline_options[0] + (int)note_dlopen[0];
}
