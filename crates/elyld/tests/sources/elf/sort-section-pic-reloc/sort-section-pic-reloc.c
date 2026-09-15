// Dynamic relocs in `--sort-section` input sections are written by the
// epilogue, so their .rela.dyn slots must be allocated there too. Musl's
// libc.so link uses `-shared --sort-section=alignment` plus data pointers.

//#AbstractConfig:default
//#CompArgs:-fPIC
//#RunEnabled:false
//#ReferenceLinkers:bfd,lld,mold
//#DiffIgnore:.dynamic.DT_FLAGS_1.NOW
//#DiffIgnore:segment.LOAD.RX.alignment
//#DiffIgnore:section.got

//#Config:alignment:default
//#LinkArgs:-shared -z now --sort-section=alignment --no-gc-sections

//#Config:alignment-gc:default
//#LinkArgs:-shared -z now --sort-section=alignment --gc-sections

//#Config:alignment-bsymbolic:default
//#LinkArgs:-shared -z now --sort-section=alignment --no-gc-sections -Bsymbolic
//#DiffIgnore:.dynamic.DT_FLAGS.SYMBOLIC
//#DiffIgnore:.dynamic.DT_SYMBOLIC

//#Config:alignment-dynamic-list:default
//#LinkArgs:-shared -z now --sort-section=alignment --gc-sections --hash-style=both --dynamic-list ./sort-section-pic-reloc.def
//#DiffIgnore:.dynamic.DT_FLAGS.SYMBOLIC
//#DiffIgnore:.dynamic.DT_SYMBOLIC
//#DiffIgnore:.dynamic.DT_GNU_HASH
//#DiffIgnore:.dynamic.DT_HASH
//#DiffIgnore:section.hash
//#DiffIgnore:section.gnu.hash

int foo = 1;
int *p = &foo;
int bar = 2;
int *q = &bar;
int baz = 3;
int *r = &baz;
