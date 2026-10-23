#!/bin/bash
# semoe-ffi를 arm64 + x86_64로 빌드해 유니버설 staticlib을 만든다.
# 결과: target/universal/release/libsemoe_ffi.a
set -euo pipefail
cd "$(dirname "$0")/.."

export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-13.0}"
TARGETS=(aarch64-apple-darwin x86_64-apple-darwin)

for t in "${TARGETS[@]}"; do
    cargo build --release -p semoe-ffi --target "$t"
done

out=target/universal/release
mkdir -p "$out"
lipo -create $(printf 'target/%s/release/libsemoe_ffi.a ' "${TARGETS[@]}") -output "$out/libsemoe_ffi.a"
lipo -info "$out/libsemoe_ffi.a"
