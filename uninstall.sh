#!/bin/bash
# 세모이 입력기 제거
#
#   curl -fsSL https://raw.githubusercontent.com/jjwopen/semoe-macos/main/uninstall.sh | bash
#   … | bash -s -- --purge     설정과 로그까지 지움
set -euo pipefail

BUNDLE_ID="io.github.jjwopen.inputmethod.Semoe"
APP="$HOME/Library/Input Methods/SemoeIME.app"
AGENT_LABEL="io.github.jjwopen.semoe.right-command"
AGENT="$HOME/Library/LaunchAgents/$AGENT_LABEL.plist"

purge=false
[[ "${1:-}" == --purge ]] && purge=true

if [[ -x "$APP/Contents/MacOS/SemoeIME" ]]; then
    "$APP/Contents/MacOS/SemoeIME" --unregister || true
fi
pkill -x SemoeIME 2>/dev/null || true
rm -rf "$APP"
echo "입력기를 지웠습니다: $APP"

if [[ -f "$AGENT" ]]; then
    launchctl bootout "gui/$(id -u)" "$AGENT" 2>/dev/null || true
    rm -f "$AGENT"
    # 세모이가 넣은 매핑일 때만 되돌린다.
    if [[ "$(hidutil property --get UserKeyMapping 2>/dev/null)" == *30064771303* ]]; then
        hidutil property --set '{"UserKeyMapping":[]}' >/dev/null
    fi
    echo "오른쪽 Command 설정을 지웠습니다."
fi

if $purge; then
    defaults delete "$BUNDLE_ID" 2>/dev/null || true
    rm -rf "$HOME/Library/Logs/SemoeIME"
    echo "설정과 로그를 지웠습니다."
fi

cat <<EOF

남은 정리 (필요하면):
 - 시스템 설정 → 키보드 → 입력 소스에 세모이가 남아 있으면 − 로 지우기
 - 시스템 설정 → 개인정보 보호 및 보안 → 입력 모니터링에서 SemoeIME 지우기
EOF
