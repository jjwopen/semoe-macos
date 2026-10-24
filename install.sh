#!/bin/bash
# 세모이(세벌식 모아치기 e) macOS 입력기 설치
#
#   curl -fsSL https://raw.githubusercontent.com/jjwopen/semoe-macos/main/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/jjwopen/semoe-macos/main/install.sh | bash -s -- --right-cmd
#
# 옵션
#   --version vX.Y.Z  특정 릴리스 설치 (기본: 최신)
#   --right-cmd       오른쪽 Command를 F18로 바꾸는 LaunchAgent도 설치 (한/영 전환용)
#   --zip <파일>       내려받지 않고 이 zip으로 설치 (시험용)
#
# 이 입력기는 Apple 공증을 받지 않았다. curl로 받은 파일은 격리 속성이 붙지 않아 Gatekeeper 경고 없이
# 실행된다. 대신 받은 파일의 SHA-256과 서명 인증서가 이 프로젝트의 것인지 확인한 뒤에만 설치한다.
set -euo pipefail

REPO="jjwopen/semoe-macos"
# 릴리스 서명 인증서(Semoe Release Signing)의 SHA-1. codesign의 "certificate leaf" 값과 같다.
EXPECTED_CERT_SHA1=6567a14962fdeadb31c85ade1e0437b334ff1f8f
BUNDLE_ID="io.github.jjwopen.inputmethod.Semoe"
# 시험용: SEMOE_INSTALL_DIR로 설치 위치를 바꾸고, SEMOE_SKIP_REGISTER=1이면 등록·실행을 건너뛴다.
DEST_DIR="${SEMOE_INSTALL_DIR:-$HOME/Library/Input Methods}"
APP="$DEST_DIR/SemoeIME.app"
AGENT_LABEL="io.github.jjwopen.semoe.right-command"
AGENT="$HOME/Library/LaunchAgents/$AGENT_LABEL.plist"
# 오른쪽 Command(HID 0x7000000E7) → F18(0x70000006D)
RIGHT_CMD_MAPPING='{"UserKeyMapping":[{"HIDKeyboardModifierMappingSrc":0x7000000E7,"HIDKeyboardModifierMappingDst":0x70000006D}]}'

version=latest
zip=""
right_cmd=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --version) version="$2"; shift 2 ;;
        --zip) zip="$2"; shift 2 ;;
        --right-cmd) right_cmd=true; shift ;;
        -h|--help) sed -n '2,13p' "$0" 2>/dev/null || true; exit 0 ;;
        *) echo "알 수 없는 옵션: $1" >&2; exit 2 ;;
    esac
done

say() { printf '\033[1m%s\033[0m\n' "$*"; }
fail() { printf '설치 실패: %s\n' "$*" >&2; exit 1; }

major=$(sw_vers -productVersion | cut -d. -f1)
[[ "$major" -ge 13 ]] || fail "macOS 13 이상이 필요합니다 (지금 $(sw_vers -productVersion))"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

if [[ -z "$zip" ]]; then
    if [[ "$version" == latest ]]; then
        base="https://github.com/$REPO/releases/latest/download"
    else
        base="https://github.com/$REPO/releases/download/$version"
    fi
    say "내려받는 중: $base/SemoeIME.zip"
    curl -fL --retry 3 --progress-bar -o "$tmp/SemoeIME.zip" "$base/SemoeIME.zip" || fail "내려받지 못했습니다"
    curl -fsL --retry 3 -o "$tmp/SemoeIME.zip.sha256" "$base/SemoeIME.zip.sha256" || fail "체크섬을 내려받지 못했습니다"
    (cd "$tmp" && shasum -a 256 -c SemoeIME.zip.sha256 >/dev/null) || fail "체크섬이 맞지 않습니다"
else
    cp "$zip" "$tmp/SemoeIME.zip"
fi

ditto -x -k "$tmp/SemoeIME.zip" "$tmp/unzipped"
new="$tmp/unzipped/SemoeIME.app"
[[ -d "$new" ]] || fail "zip 안에 SemoeIME.app이 없습니다"

# 서명 확인: 손상되지 않았고, 이 프로젝트의 릴리스 인증서로 서명되었는지
codesign --verify --strict "$new" 2>/dev/null || fail "서명이 손상되었습니다"
leaf=$(codesign -dr - "$new" 2>&1 | sed -n 's/.*certificate leaf = H"\([0-9a-f]*\)".*/\1/p')
[[ "$leaf" == "$EXPECTED_CERT_SHA1" ]] || fail "서명 인증서가 다릅니다 (기대 $EXPECTED_CERT_SHA1, 실제 ${leaf:-없음})"
installed_version=$("$new/Contents/MacOS/SemoeIME" --version 2>/dev/null || echo "?")

say "설치 중: 세모이 $installed_version → $APP"
[[ "${SEMOE_SKIP_REGISTER:-}" == 1 ]] || pkill -x SemoeIME 2>/dev/null || true
mkdir -p "$DEST_DIR"
rm -rf "$APP"
ditto "$new" "$APP"
xattr -dr com.apple.quarantine "$APP" 2>/dev/null || true

if [[ "${SEMOE_SKIP_REGISTER:-}" != 1 ]]; then
    if ! "$APP/Contents/MacOS/SemoeIME" --register; then
        echo "입력 소스 등록에 실패했습니다. 로그아웃했다가 다시 로그인한 뒤 이 스크립트를 다시 실행해 보세요." >&2
    fi
    # 입력기를 다시 띄운다. 띄우지 않으면 이미 열려 있던 앱에서 한동안 영문이 그대로 들어간다.
    open "$APP"
fi

if $right_cmd; then
    # hidutil은 장치마다 매핑을 보여 준다. 30064771303 = 0x7000000E7(오른쪽 Command)
    current=$(hidutil property --get UserKeyMapping 2>/dev/null || true)
    if grep -q 'MappingSrc = 30064771303' <<<"$current" && grep -q 'MappingDst = 30064771181' <<<"$current"; then
        say "오른쪽 Command → F18 매핑이 이미 있어 건너뜁니다."
    elif grep -q HIDKeyboardModifierMapping <<<"$current"; then
        echo "이미 다른 키 매핑(hidutil)이 있어 오른쪽 Command 설정을 건너뜁니다. 덮어쓰지 않으려는 조치입니다." >&2
    else
        mkdir -p "$(dirname "$AGENT")"
        cat > "$AGENT" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>$AGENT_LABEL</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/bin/hidutil</string>
        <string>property</string>
        <string>--set</string>
        <string>$RIGHT_CMD_MAPPING</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
PLIST
        launchctl bootout "gui/$(id -u)" "$AGENT" 2>/dev/null || true
        launchctl bootstrap "gui/$(id -u)" "$AGENT"
        hidutil property --set "$RIGHT_CMD_MAPPING" >/dev/null
        say "오른쪽 Command를 F18로 바꿨습니다."
    fi
fi

cat <<EOF

설치했습니다. 처음 설치했다면 다음을 한 번씩 해 주세요.

 1. 시스템 설정 → 키보드 → 텍스트 입력 → 입력 소스 "편집…" → + → 한국어 → "세모이" 추가
    (목록에 없으면 로그아웃했다가 다시 로그인)
 2. 메뉴 막대에서 세모이를 고른 뒤, 입력기 메뉴 → 환경설정… → "입력 모니터링 권한 요청…"
    (시스템 설정 → 개인정보 보호 및 보안 → 입력 모니터링에서 SemoeIME을 켜도 됩니다.
     이 권한이 있어야 키를 모두 뗄 때 바로 확정됩니다. 없으면 40 ms 타이머로만 확정합니다.)
EOF
if $right_cmd; then
    cat <<EOF
 3. 시스템 설정 → 키보드 → 키보드 단축키 → 입력 소스 → "이전 입력 소스 선택"을 누르고
    오른쪽 Command를 눌러 F18로 지정
EOF
fi
echo
echo "업데이트는 같은 명령을 다시 실행하면 됩니다. 제거: curl -fsSL https://raw.githubusercontent.com/$REPO/main/uninstall.sh | bash"
