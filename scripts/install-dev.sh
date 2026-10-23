#!/bin/bash
# 개발용 설치: 빌드 → ~/Library/Input Methods에 복사 → 입력 소스 등록·활성화.
# 처음 설치 후 입력 소스가 목록에 안 보이면 로그아웃했다가 다시 로그인한다.
set -euo pipefail
cd "$(dirname "$0")/.."

BUNDLE_ID=io.github.jjwopen.inputmethod.Semoe
MODE_ID=$BUNDLE_ID.Chord
DEST="$HOME/Library/Input Methods/SemoeIME.app"

[[ "${SKIP_BUILD:-}" == 1 ]] || scripts/build-macos.sh

pkill -x SemoeIME 2>/dev/null || true
mkdir -p "$HOME/Library/Input Methods"
rm -rf "$DEST"
cp -R build/SemoeIME.app "$DEST"

swift scripts/tis.swift register "$DEST"
# 모드가 있는 입력기는 상위 소스(번들 ID)와 모드를 모두 켜야 선택할 수 있다.
{ swift scripts/tis.swift enable "$BUNDLE_ID" && swift scripts/tis.swift enable "$MODE_ID"; } || echo "활성화 실패: 시스템 설정 → 키보드 → 입력 소스에서 '세모이'를 직접 추가하세요."
swift scripts/tis.swift status

# pkill 후에는 시스템이 입력기를 바로 다시 띄우지 않아, 이미 열린 문서에서는 영문이 그대로 들어간다.
open "$DEST"
[[ -f "$HOME/.semoe-dev/signing.keychain-db" ]] || \
    echo "참고: ad-hoc 서명은 빌드마다 서명이 바뀌어 '입력 모니터링' 권한이 풀린다. scripts/setup-dev-signing.sh를 한 번 실행하면 유지된다."
