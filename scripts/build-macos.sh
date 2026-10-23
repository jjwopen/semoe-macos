#!/bin/bash
# 세모이 입력기 앱 번들(build/SemoeIME.app)을 만든다. Xcode 없이 Command Line Tools만으로 빌드한다.
#   SIGN_IDENTITY  codesign 인증서. 기본은 scripts/setup-dev-signing.sh로 만든 개발용 인증서,
#                  그것도 없으면 "-" (ad-hoc: 빌드마다 입력 모니터링 권한이 풀린다)
set -euo pipefail
cd "$(dirname "$0")/.."

MIN_MACOS=13.0
DEV_KEYCHAIN="$HOME/.semoe-dev/signing.keychain-db"
SIGN_ARGS=()
if [[ -z "${SIGN_IDENTITY:-}" && -f "$DEV_KEYCHAIN" ]]; then
    security unlock-keychain -p semoe-dev "$DEV_KEYCHAIN"
    SIGN_IDENTITY="Semoe Dev Signing"
    SIGN_ARGS=(--keychain "$DEV_KEYCHAIN")
fi
SIGN_IDENTITY="${SIGN_IDENTITY:--}"
SRC=macos/SemoeIME
APP=build/SemoeIME.app
OBJ=build/obj

MACOSX_DEPLOYMENT_TARGET=$MIN_MACOS scripts/build-rust-universal.sh

rm -rf "$APP" "$OBJ"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$OBJ"

for arch in arm64 x86_64; do
    swiftc -O -swift-version 5 \
        -target "$arch-apple-macos$MIN_MACOS" \
        -I crates/semoe-ffi/include \
        -L "target/universal/release" -lsemoe_ffi \
        -framework AppKit -framework InputMethodKit \
        -o "$OBJ/SemoeIME-$arch" \
        "$SRC"/*.swift
done
lipo -create "$OBJ"/SemoeIME-arm64 "$OBJ"/SemoeIME-x86_64 -output "$APP/Contents/MacOS/SemoeIME"

cp "$SRC/Info.plist" "$APP/Contents/Info.plist"
cp "$SRC"/Resources/* "$APP/Contents/Resources/"
printf 'APPL????' > "$APP/Contents/PkgInfo"

codesign --force --sign "$SIGN_IDENTITY" ${SIGN_ARGS[@]+"${SIGN_ARGS[@]}"} --timestamp=none "$APP"
codesign --verify --verbose=2 "$APP"
lipo -info "$APP/Contents/MacOS/SemoeIME"
