// GNU `-init`/`-fini` (libnuma) set DT_INIT/DT_FINI to named symbols.
// `--warn-common` and `-Map` (busybox) are accepted and ignored.

//#AbstractConfig:default
//#CompArgs:-fPIC
//#Mode:dynamic
//#RunEnabled:false
//#ReferenceLinkers:bfd,lld,mold
//#DiffIgnore:.dynamic.DT_FLAGS_1.NOW
//#DiffIgnore:segment.LOAD.RX.alignment

//#Config:named:default
//#LinkArgs:-shared -z now --gc-sections -init my_init -fini my_fini
//#ExpectDynamic:DT_INIT
//#ExpectDynamic:DT_FINI

//#Config:equals:default
//#LinkArgs:-shared -z now --gc-sections --init=my_init --fini=my_fini
//#ExpectDynamic:DT_INIT
//#ExpectDynamic:DT_FINI

//#Config:ignored-flags:default
//#LinkArgs:-shared -z now --warn-common -Map /dev/null
//#DiffEnabled:false

void __attribute__((visibility("hidden"))) my_init(void) {}
void __attribute__((visibility("hidden"))) my_fini(void) {}

int foo(void) { return 1; }
