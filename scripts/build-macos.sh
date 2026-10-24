#!/bin/bash
# 세모이 입력기 앱 번들(build/SemoeIME.app)을 만든다. Xcode 없이 Command Line Tools만으로 빌드한다.
#   SIGN_IDENTITY  codesign 인증서. 기본은 scripts/setup-dev-signing.sh로 만든 개발용 인증서,
#                  그것도 없으면 "-" (ad-hoc: 빌드마다 입력 모니터링 권한이 풀린다)
#   SIGN_KEYCHAIN  SIGN_IDENTITY를 찾을 키체인 (릴리스 CI에서 임시 키체인을 쓸 때)
#   SEMOE_VERSION  앱 버전 (CFBundleShortVersionString, 예: 0.1.0). 없으면 Info.plist 그대로
#   SEMOE_BUILD    빌드 번호 (CFBundleVersion)
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
if [[ -n "${SIGN_KEYCHAIN:-}" ]]; then
    SIGN_ARGS=(--keychain "$SIGN_KEYCHAIN")
fi
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
[[ -z "${SEMOE_VERSION:-}" ]] || plutil -replace CFBundleShortVersionString -string "$SEMOE_VERSION" "$APP/Contents/Info.plist"
[[ -z "${SEMOE_BUILD:-}" ]] || plutil -replace CFBundleVersion -string "$SEMOE_BUILD" "$APP/Contents/Info.plist"
cp -R "$SRC"/Resources/ "$APP/Contents/Resources/"
# 라이선스 고지: 코드(MIT)와, 바이너리에 내장한 자판 데이터(CC BY-SA 4.0)의 출처
cp LICENSE "$APP/Contents/Resources/LICENSE.txt"
cp data/README.md "$APP/Contents/Resources/DATA-NOTICE.md"
printf 'APPL????' > "$APP/Contents/PkgInfo"

codesign --force --sign "$SIGN_IDENTITY" ${SIGN_ARGS[@]+"${SIGN_ARGS[@]}"} --timestamp=none "$APP"
codesign --verify --verbose=2 "$APP"
lipo -info "$APP/Contents/MacOS/SemoeIME"
