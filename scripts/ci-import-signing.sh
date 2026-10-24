#!/bin/bash
# CI 전용: 저장소 비밀값의 릴리스 서명 인증서를 임시 키체인에 넣는다.
#   입력 환경변수  P12 (base64), P12_PASSWORD
#   출력          $GITHUB_ENV에 SIGN_KEYCHAIN, SIGN_IDENTITY(인증서 SHA-1)
# 키체인을 검색 목록에 넣지 않으면 러너의 codesign이 개인 키를 찾지 못한다
# ("The specified item could not be found in the keychain", v0.1.0 첫 시도).
set -euo pipefail

: "${P12:?SEMOE_SIGNING_P12 비밀값이 없습니다}"
: "${P12_PASSWORD:?SEMOE_SIGNING_P12_PASSWORD 비밀값이 없습니다}"

keychain="$RUNNER_TEMP/release.keychain-db"
kpass=$(openssl rand -hex 16)
p12="$RUNNER_TEMP/release.p12"
# 비밀값을 파일에서 등록했으면 끝에 줄바꿈이 붙어 있을 수 있다.
password=$(printf '%s' "$P12_PASSWORD" | tr -d '\r\n')

printf '%s' "$P12" | base64 --decode > "$p12"
security create-keychain -p "$kpass" "$keychain"
security set-keychain-settings "$keychain"
security unlock-keychain -p "$kpass" "$keychain"
security import "$p12" -k "$keychain" -P "$password" -T /usr/bin/codesign
security set-key-partition-list -S apple-tool:,apple: -s -k "$kpass" "$keychain" >/dev/null
rm -f "$p12"

# 검색 목록 앞에 넣는다 (기존 목록은 유지).
existing=$(security list-keychains -d user | tr -d '"' | xargs)
# shellcheck disable=SC2086
security list-keychains -d user -s "$keychain" $existing

identity=$(security find-identity -p codesigning "$keychain" | sed -n 's/^ *1) \([0-9A-F]\{40\}\) .*/\1/p')
[[ -n "$identity" ]] || { security find-identity -p codesigning "$keychain"; echo "서명 인증서를 찾지 못함" >&2; exit 1; }
echo "서명 인증서: $identity"
{
    echo "SIGN_KEYCHAIN=$keychain"
    echo "SIGN_IDENTITY=$identity"
} >> "$GITHUB_ENV"
