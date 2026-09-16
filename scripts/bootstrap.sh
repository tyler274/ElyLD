#!/usr/bin/env bash
# Build a host ElyLD with the system linker, then rebuild the requested profile
# linked by that binary (clang --ld-path).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

profile=release
target=""
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

if ! command -v clang >/dev/null 2>&1; then
  echo "bootstrap.sh needs clang on PATH (rustc uses it as the linker driver)" >&2
  exit 1
fi

host="$(rustc -vV | awk '/^host:/{print $2}')"
target="${target:-$host}"
triple_env="$(printf '%s' "$target" | tr '[:lower:]-' '[:upper:]_')"

echo "==> stage1: cargo build --release -p elyld (system linker)"
cargo build --release -p elyld "${extra[@]}"

stage1="$root/target/release/elyld"
if [[ ! -x "$stage1" ]]; then
  stage1="$root/target/${host}/release/elyld"
fi
if [[ ! -x "$stage1" ]]; then
  echo "stage1 elyld not found under target/release" >&2
  exit 1
fi
"$stage1" --version

export "CARGO_TARGET_${triple_env}_LINKER=clang"
export RUSTFLAGS="${RUSTFLAGS:-} -Clink-arg=--ld-path=${stage1}"

echo "==> stage2: cargo build --profile ${profile} -p elyld --target ${target} (linked by ElyLD)"
cargo build --profile "$profile" -p elyld --target "$target" "${extra[@]}"

stage2="$root/target/${target}/${profile}/elyld"
if [[ ! -x "$stage2" ]]; then
  stage2="$root/target/${profile}/elyld"
fi
"$stage2" --version
echo "bootstrapped ${stage2}"
