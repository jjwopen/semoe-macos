#!/bin/bash
# 릴리스용 자체 서명 코드 서명 인증서를 만든다 (한 번만).
#
# Apple Developer ID 없이 배포하므로 Gatekeeper 공증은 없다. 대신 모든 릴리스를 같은 인증서로
# 서명해, macOS가 서명 기준으로 기억하는 "입력 모니터링" 권한이 업데이트 뒤에도 유지되게 한다.
#
# 결과: ~/.semoe-release/release.p12 와 그 암호. 이 둘을 GitHub 저장소 비밀값으로 등록한다.
#   SEMOE_SIGNING_P12           release.p12의 base64
#   SEMOE_SIGNING_P12_PASSWORD  암호
# 개인 키가 유출되면 누구든 같은 서명으로 가짜 업데이트를 만들 수 있다. 저장소에 넣지 말고,
# 비밀번호 관리자 등에 따로 백업한다(잃어버리면 새 인증서로 바꿔야 하고 사용자는 권한을 다시 허용해야 한다).
set -euo pipefail

DIR="$HOME/.semoe-release"
NAME="Semoe Release Signing"
if [[ -f "$DIR/release.p12" ]]; then
    echo "이미 있음: $DIR/release.p12"
else
    mkdir -p "$DIR"
    chmod 700 "$DIR"
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    cat > "$tmp/cert.cnf" <<CNF
[req]
distinguished_name = dn
x509_extensions = ext
prompt = no
[dn]
CN = $NAME
[ext]
basicConstraints = critical, CA:false
keyUsage = critical, digitalSignature
extendedKeyUsage = critical, codeSigning
CNF
    openssl req -x509 -newkey rsa:3072 -nodes -days 7300 -config "$tmp/cert.cnf" \
        -keyout "$tmp/key.pem" -out "$DIR/release.cer" 2>/dev/null
    openssl rand -base64 24 > "$DIR/password"
    # macOS security는 OpenSSL 3 기본 PKCS12 암호화를 못 읽으므로 -legacy (LibreSSL에는 없는 옵션)
    legacy=$(openssl pkcs12 -help 2>&1 | grep -q -- -legacy && echo -legacy || true)
    openssl pkcs12 -export $legacy -inkey "$tmp/key.pem" -in "$DIR/release.cer" -name "$NAME" \
        -out "$DIR/release.p12" -passout file:"$DIR/password"
    chmod 600 "$DIR"/*
    echo "만듦: $DIR/release.p12"
fi

sha1=$(openssl x509 -in "$DIR/release.cer" -noout -fingerprint -sha1 | cut -d= -f2 | tr -d : | tr A-F a-f)
echo
echo "인증서 SHA-1 (codesign의 certificate leaf, install.sh에 넣는 값): $sha1"
echo
echo "GitHub 비밀값 등록 (저장소 주인 계정으로):"
echo "  base64 -i $DIR/release.p12 | gh secret set SEMOE_SIGNING_P12 -R jjwopen/semoe-macos"
echo "  gh secret set SEMOE_SIGNING_P12_PASSWORD -R jjwopen/semoe-macos < $DIR/password"
