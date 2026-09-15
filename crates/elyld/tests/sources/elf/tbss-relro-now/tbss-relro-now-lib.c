// 64-byte aligned NOBITS TLS, matching libsodium's `stream` buffer. Keep `.text`
// size 1-mod-16 so the RW LOAD start is not 64-aligned and compression cannot
// "accidentally" skip the `.tbss` file-offset bump.

__thread unsigned char tls_zero[64] __attribute__((aligned(64)));
static int ready;

__attribute__((constructor)) static void ctor(void) {
  tls_zero[0] = 1;
  ready = 1;
}

__attribute__((noinline, used)) void pad_text(void) {
  asm volatile(".byte 0x90");
}

int get_ready(void) {
  pad_text();
  return ready + tls_zero[0];
}
