# 세모이 macOS 입력기

[세모이(세벌식 모아치기 e)](https://github.com/Sinseiki/Semo-e_keyboard) 자판의 macOS 입력기입니다. Windows 날개셋 입력기 설정으로만 쓸 수 있던 세모이를 macOS에서 쓸 수 있게 했습니다.

- **입력 방식**: 한 번에 모아치기(한 글자를 한 타에), 이어치기·중종성 모아치기
- **약어**: 원본 약어 1,363개, 기호 조합(※ △ → `가? ` 등)
- **기호**: 윗글쇠 기호, 기호 확장(Shift+J → Shift+K·L·; → 기호)
- **옛한글 모드**: 숫자열 옛한글, Shift+Y·U 방점
- **조합 종료 타이머**: 20~100 ms

macOS 13 이상, Apple Silicon과 Intel을 모두 지원합니다.

## 설치

터미널에서 실행합니다.

```sh
curl -fsSL https://raw.githubusercontent.com/jjwopen/semoe-macos/main/install.sh | bash
```

오른쪽 Command로 한/영을 전환하려면 `--right-cmd`를 붙입니다(아래 "한/영 전환" 참고).

```sh
curl -fsSL https://raw.githubusercontent.com/jjwopen/semoe-macos/main/install.sh | bash -s -- --right-cmd
```

설치 스크립트가 하는 일:

1. 최신 릴리스의 `SemoeIME.zip`을 내려받고 SHA-256을 확인합니다.
2. 앱이 이 프로젝트의 릴리스 인증서로 서명되었는지 확인합니다(아래 "보안" 참고).
3. `~/Library/Input Methods/SemoeIME.app`에 설치하고 입력 소스로 등록합니다.

### 처음 설치한 뒤 한 번만

1. **입력 소스 추가**: 시스템 설정 → 키보드 → 텍스트 입력 → 입력 소스 "편집…" → `+` → 한국어 → **세모이**. 목록에 없으면 로그아웃했다가 다시 로그인하세요.
2. **입력 모니터링 권한**: 세모이를 고른 뒤 메뉴 막대의 입력기 메뉴 → 환경설정… → "입력 모니터링 권한 요청…". 또는 시스템 설정 → 개인정보 보호 및 보안 → 입력 모니터링에서 SemoeIME을 켭니다.
   - 모아치기는 "모든 키를 뗐는지"를 알아야 하는데, macOS 입력기 API는 키를 뗀 이벤트를 넘겨 주지 않습니다. 그래서 키보드 이벤트를 읽는 권한이 필요합니다.
   - 권한이 없어도 입력은 되지만, 키를 뗄 때가 아니라 마지막 키 입력 뒤 40 ms 타이머로만 확정합니다.
   - 키 입력 내용은 저장하거나 밖으로 보내지 않습니다. 개발용 이벤트 로그는 기본으로 꺼져 있고, 켜도 키 코드만 `~/Library/Logs/SemoeIME/`에 남습니다.

### 직접 내려받아 설치하기

[Releases](https://github.com/jjwopen/semoe-macos/releases)에서 브라우저로 zip을 받으면 macOS가 격리 속성을 붙입니다. 이 경우 압축을 풀고 다음을 실행하세요.

```sh
xattr -dr com.apple.quarantine SemoeIME.app
mv SemoeIME.app ~/Library/Input\ Methods/
~/Library/Input\ Methods/SemoeIME.app/Contents/MacOS/SemoeIME --register
open ~/Library/Input\ Methods/SemoeIME.app
```

### 업데이트 · 제거

- **업데이트**: 설치 명령을 다시 실행합니다. 같은 인증서로 서명하므로 입력 모니터링 권한은 유지됩니다.
- **제거**: 아래 명령을 실행합니다. `bash -s -- --purge`를 붙이면 설정과 로그까지 지웁니다.

  ```sh
  curl -fsSL https://raw.githubusercontent.com/jjwopen/semoe-macos/main/uninstall.sh | bash
  ```

## 쓰는 법

입력기 메뉴(메뉴 막대)에서 입력 방식, 약어, 기호 조합, 옛한글 모드를 바꿀 수 있습니다. 환경설정 창에서는 조합 종료 타이머와 Caps Lock 순아래를 설정합니다.

| 기능 | 키 |
| --- | --- |
| 기호 확장 | Shift+J 다음 Shift+K(1단계) · Shift+L(2단계) · Shift+;(3단계), 그다음 기호 키. Esc는 취소 |
| 기호 조합 | 기호 두 개를 이어서. 예: Shift+K(※) Shift+F(△) → `가? ` |
| 스페이스 모아치기 | 음절과 Space를 함께 누르면 뒤에 공백이 붙습니다 (`강 `) |
| 옛한글 방점 | 옛한글 모드에서 Shift+Y(〮), Shift+U(〯) |

자판 배열과 결합 규칙, 약어 목록은 [원본 저장소](https://github.com/Sinseiki/Semo-e_keyboard)의 배열도와 설명서를 참고하세요.

### 한/영 전환

입력기 안에 영문 모드가 따로 없습니다. macOS 입력 소스 전환을 씁니다.

- **Caps Lock**: 시스템 설정 → 키보드 → 텍스트 입력의 "Caps Lock 키로 ABC 입력 소스 전환". 이 경우 Caps Lock 순아래 기능과 겹칩니다.
- **오른쪽 Command**: `install.sh --right-cmd`로 오른쪽 Command를 F18로 바꾼 뒤, 시스템 설정 → 키보드 → 키보드 단축키 → 입력 소스 → "이전 입력 소스 선택"을 오른쪽 Command(F18)로 지정합니다. 오른쪽 Command는 원래 Command 기능을 잃습니다. 이미 다른 키 매핑(`hidutil`)이 있으면 덮어쓰지 않고 건너뜁니다.

### 알아 둘 점

- **키보드 동시 입력 한계**: 맥북 내장 키보드처럼 동시 입력 수가 적은 키보드는, 4~5키를 함께 누르면 일부 키가 늦게 들어와 음절이 쪼개질 수 있습니다(예: `뺨` → `뺘ㅁ`). `뺨`은 추가 약어 `o j z`(초ㅂ+초ㅇ+종ㅁ)로도 칠 수 있습니다.
- **조합 종료 타이머**: 키를 누른 채로 40 ms(기본)가 지나면 확정합니다. 너무 빨리 끊기면 환경설정에서 늘리세요.
- 앱별 확인 결과와 알려진 문제는 [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md)에 있습니다.

## 보안

이 입력기는 **Apple 공증을 받지 않았습니다**(Apple Developer ID 없음). 대신 다음을 지킵니다.

- 모든 릴리스는 GitHub Actions에서 이 저장소의 코드로 빌드하고, 같은 자체 서명 인증서 "Semoe Release Signing"으로 서명합니다.
- 인증서 SHA-1은 `6567a14962fdeadb31c85ade1e0437b334ff1f8f`입니다. `install.sh`는 이 값이 아니면 설치하지 않습니다. 직접 확인하려면 다음을 실행하세요.

  ```sh
  codesign -dr - ~/Library/Input\ Methods/SemoeIME.app
  # designated => identifier "io.github.jjwopen.inputmethod.Semoe" and certificate leaf = H"6567a149…"
  ```

## 소스에서 빌드

Xcode 없이 Command Line Tools, Rust(stable), cbindgen으로 빌드합니다.

```sh
xcode-select --install
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo install cbindgen --locked
git clone https://github.com/Sinseiki/Semo-e_keyboard ../Semo-e_keyboard   # 데이터 다시 만들 때만
scripts/setup-dev-signing.sh   # 선택: 개발용 인증서 (빌드할 때마다 권한이 풀리지 않게)
scripts/install-dev.sh         # 빌드 → ~/Library/Input Methods에 설치
cargo test
```

구조와 설계는 [docs/DEVELOPMENT_PLAN.md](docs/DEVELOPMENT_PLAN.md)를 보세요. 요약하면 입력 엔진(`crates/semoe-core`)은 플랫폼과 무관한 Rust이고, macOS 입력기(`macos/SemoeIME`)는 이를 감싼 얇은 Swift 껍데기입니다. 자판 데이터(`data/`)는 원본 날개셋 XML에서 `semoe-datagen`으로 만듭니다.

## 라이선스와 출처

- **코드**: [MIT 라이선스](LICENSE)
- **자판 데이터**: 세모이 자판(배열, 결합 규칙, 약어)은 **신세기** 님이 만들었습니다([Sinseiki/Semo-e_keyboard](https://github.com/Sinseiki/Semo-e_keyboard)). 이 저장소의 `data/`는 그 자판 데이터를 변환한 것이며, MIT가 아니라 **CC BY-SA 4.0**을 따릅니다.
- 배포하는 입력기 바이너리에는 이 자판 데이터가 들어 있습니다. 그래서 바이너리는 MIT와 CC BY-SA 4.0을 함께 따릅니다.
