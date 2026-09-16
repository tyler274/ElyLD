#!/usr/bin/env bash
# Two-stage self-host, then PGO: instrument, train on full integration_tests plus
# an ElyLD-linked opt rebuild, merge, and emit a profile-use dist binary.
#
#   ./scripts/pgo-build.sh
#   ./scripts/pgo-build.sh --target x86_64-unknown-linux-gnu
#   ./scripts/pgo-build.sh --no-pgo --profile release   # self-host only
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

profile=dist
target=""
do_pgo=1
extra=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile)
      profile="$2"
      shift 2
      ;;
    --target)
      target="$2"
      shift 2
      ;;
    --no-pgo)
      do_pgo=0
      shift
      ;;
    --)
      shift
      extra+=("$@")
      break
      ;;
    *)
      extra+=("$1")
      shift
      ;;
  esac
done

if [[ "$do_pgo" -eq 0 ]]; then
  bootstrap_args=(--profile "$profile")
  if [[ -n "$target" ]]; then
    bootstrap_args+=(--target "$target")
  fi
  exec "$root/scripts/bootstrap.sh" "${bootstrap_args[@]}" "${extra[@]}"
fi

if ! command -v clang >/dev/null 2>&1; then
  echo "pgo-build.sh needs clang on PATH (rustc uses it as the linker driver)" >&2
  exit 1
fi

if command -v rustup >/dev/null 2>&1; then
  rustup component add llvm-tools-preview >/dev/null
fi

sysroot="$(rustc --print sysroot)"
llvm_profdata="$(find "$sysroot" -name llvm-profdata -type f | head -n 1)"
if [[ -z "$llvm_profdata" ]]; then
  echo "llvm-profdata not found under ${sysroot}; install rustup component llvm-tools-preview" >&2
  exit 1
fi

host="$(rustc -vV | awk '/^host:/{print $2}')"
target="${target:-$host}"
triple_env="$(printf '%s' "$target" | tr '[:lower:]-' '[:upper:]_')"
profdir="$(mktemp -d "${TMPDIR:-/tmp}/elyld-pgo.XXXXXX")"
merged="$profdir/merged.profdata"
trap 'rm -rf "$profdir"' EXIT

echo "==> stage1: cargo build --release -p elyld (system linker)"
cargo build --release -p elyld "${extra[@]}"

stage1="$root/target/release/elyld"
if [[ ! -x "$stage1" ]]; then
  stage1="$root/target/${host}/release/elyld"
fi
"$stage1" --version

export "CARGO_TARGET_${triple_env}_LINKER=clang"
base_rustflags="${RUSTFLAGS:-} -Clink-arg=--ld-path=${stage1}"
pgo_gen="${base_rustflags} -Cprofile-generate=${profdir}"
export LLVM_PROFILE_FILE="${profdir}/elyld-%p-%m.profraw"

echo "==> PGO generate: cargo build --profile ${profile} --target ${target} (linked by ElyLD)"
RUSTFLAGS="$pgo_gen" cargo build --profile "$profile" -p elyld --target "$target" "${extra[@]}"

echo "==> PGO train: cargo test -p elyld --test integration_tests"
# Keep generate rustflags so cargo does not rebuild an uninstrumented linker.
RUSTFLAGS="$pgo_gen" cargo test -p elyld --test integration_tests --profile "$profile" --target "$target"

echo "==> PGO train: self-link --profile opt with the instrumented linker"
gen_elyld="$root/target/${target}/${profile}/elyld"
if [[ ! -x "$gen_elyld" ]]; then
  gen_elyld="$root/target/${profile}/elyld"
fi
RUSTFLAGS="${pgo_gen} -Clink-arg=--ld-path=${gen_elyld}" \
  cargo build --profile opt -p elyld --target "$target" "${extra[@]}"

shopt -s nullglob
raws=("$profdir"/*.profraw)
if (( ${#raws[@]} == 0 )); then
  echo "no .profraw files written to ${profdir}" >&2
  exit 1
fi
echo "==> llvm-profdata merge (${#raws[@]} profiles)"
"$llvm_profdata" merge -o "$merged" "${raws[@]}"

echo "==> PGO use: cargo build --profile ${profile} --target ${target}"
RUSTFLAGS="${base_rustflags} -Cprofile-use=${merged} -Cllvm-args=-pgo-warn-missing-function" \
  cargo build --profile "$profile" -p elyld --target "$target" "${extra[@]}"

final="$root/target/${target}/${profile}/elyld"
if [[ ! -x "$final" ]]; then
  final="$root/target/${profile}/elyld"
fi
"$final" --version
if command -v readelf >/dev/null 2>&1; then
  readelf -p .comment "$final" | grep -E "Linker: ElyLD" || echo "warning: ${final} .comment does not mention ElyLD" >&2
fi
echo "PGO dist binary: ${final}"
