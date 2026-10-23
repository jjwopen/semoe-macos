#!/bin/bash
# 개발용 자체 서명 코드 서명 인증서를 별도 키체인(~/.semoe-dev/signing.keychain-db)에 만든다.
# ad-hoc 서명은 빌드마다 서명이 바뀌어 "입력 모니터링" 권한이 풀리지만, 같은 인증서로 서명하면 유지된다.
# 로그인 키체인은 건드리지 않는다. 지우려면 ~/.semoe-dev 를 삭제한다.
set -euo pipefail

DIR="$HOME/.semoe-dev"
KEYCHAIN="$DIR/signing.keychain-db"
PASS=semoe-dev   # 이 키체인에는 개발용 인증서만 들어 있다.
NAME="Semoe Dev Signing"

if [[ -f "$KEYCHAIN" ]]; then
    echo "이미 있음: $KEYCHAIN"
    exit 0
fi
mkdir -p "$DIR"
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
openssl req -x509 -newkey rsa:2048 -nodes -days 3650 -config "$tmp/cert.cnf" \
    -keyout "$tmp/key.pem" -out "$tmp/cert.pem" 2>/dev/null
# macOS security는 OpenSSL 3의 기본 PKCS12 암호화를 못 읽으므로 -legacy를 쓴다(LibreSSL에는 없는 옵션).
legacy=$(openssl pkcs12 -help 2>&1 | grep -q -- -legacy && echo -legacy || true)
openssl pkcs12 -export $legacy -inkey "$tmp/key.pem" -in "$tmp/cert.pem" -out "$tmp/id.p12" -passout pass:$PASS

security create-keychain -p "$PASS" "$KEYCHAIN"
security set-keychain-settings "$KEYCHAIN"   # 자동 잠금 끄기
security unlock-keychain -p "$PASS" "$KEYCHAIN"
security import "$tmp/id.p12" -k "$KEYCHAIN" -P "$PASS" -T /usr/bin/codesign >/dev/null
security set-key-partition-list -S apple-tool:,apple: -s -k "$PASS" "$KEYCHAIN" >/dev/null
echo "만듦: $KEYCHAIN ($NAME)"
