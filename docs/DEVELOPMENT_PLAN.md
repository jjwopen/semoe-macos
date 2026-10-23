# 세모이(세벌식 모아치기 e) macOS 입력기 개발 계획서

- 작성일: 2026-10-21 (개정: Rust 엔진 + Swift 껍데기 구조로 변경)
- 원본 자판 저장소: `../Semo-e_keyboard` (Sinseiki/Semo-e_keyboard, CC BY-SA 4.0)
- 기준 문서: `세벌식 모아치기 e 개발 규격.pdf`, `세모이 (한 번에 모아치기).xml`, `세모이 (이어치기 & 중종성 모아치기).xml`

---

## 1. 목표

Windows 날개셋 입력기 설정(XML)으로만 구현되어 있는 세모이 자판을 macOS 네이티브 입력기(Input Method)로 구현한다.

- **입력 엔진은 Rust**로 작성해 플랫폼과 무관하게 만들고, **macOS 연동(InputMethodKit, UI, 번들)은 Swift**로 얇게 감싼다.
- 엔진은 이후 Linux(ibus·fcitx5), Windows(TSF) 입력기에서도 그대로 재사용할 수 있게 설계한다. 다만 이번 계획의 범위는 macOS다.

| 우선순위 | 범위 | 규격상 분류 |
| --- | --- | --- |
| P0 | 아랫글쇠 + 특수 시프트(모음·종성 시프트) 배열, 현대 한글 결합 규칙(순서 무관) | 필수 |
| P0 | 입력 방식 3종: **한 번에 모아치기** / **중·종성 모아치기** / **이어치기** 전환 | 권장 |
| P0 | 조합 종료 타이머(기본 40 ms, 사용자 설정 20~100 ms) | 한 번에 모아치기 필수 요소 |
| P1 | 약어 사전(약 1,360개, 옛한글 음절 → 단어 치환) | 권장 |
| P1 | 윗글쇠 기호, 스페이스 모아치기(`강 ` 한 타 입력) | 권장 / 가능 |
| P2 | 기호 확장 배열(Shift+J → Shift+K/L/; 1~3단계), 기호 조합(UserCompo) | 가능 |
| P2 | Caps Lock 순아래(윗글쇠 고정), 옛한글 모드(원본은 Scroll Lock) | 가능 |
| P3 | 후보 변환(UserCandi 약 370개), Colemak/Dvorak 물리 배열 대응 | 가능 |

비목표: 모바일 9열 배열, 직결식 자판, 두벌식 혼용 자판, macOS 외 플랫폼 껍데기(엔진 구조만 열어 둔다).

---

## 2. 세모이 자판 분석

### 2.1 배열 (QWERTY 위치 기준, `KeyTable` 소문자 값)

```
        q    w    e    r    t  │ y    u    i    o    p
아래:  종ㅅ 종ㅂ 종ㄹ  ㅓ   ㅕ  │초ㅁ 초ㄴ 초ㄷ 초ㅂ   .
        a    s    d    f    g  │ h    j    k    l    ;
아래:  종ㅇ 종ㄴ  ㅣ   ㅏ   ㅡ  │초ㅎ 초ㅇ 초ㄱ 초ㅈ 종ㅆ(종성시프트)
        z    x    c    v    b  │ n    m    ,    .    /
아래:  종ㅁ 종ㄱ  ㅔ   ㅗ   ㅜ  │초ㅅ 초ㄹ   ,  모음시프트  /
```

- **왼손 = 종성(바깥) + 중성(안쪽), 오른손 = 초성** 구조. 한 손 안에 한 역할이 몰려 있어 모아치기 시 세 벌이 손가락 충돌 없이 동시에 눌린다.
- 기본 낱자: 초성 10(ㄱㄴㄷㄹㅁㅂㅅㅇㅈㅎ), 중성 8 + 모음시프트, 종성 7 + 종성시프트(ㅆ).
- **일반 Shift로 입력하는 한글 윗글쇠가 없다.** Shift+문자는 모두 기호(←, ↓, ♡, ☆ 등)이며, 이 덕분에 한글 조합과 Shift가 분리되어 한 번에 모아치기가 가능하다.
- `p` = 마침표, `P` = 세미콜론, `;` = 종성시프트, `.` = 모음시프트 (원래 자리 기호가 이동함에 유의).

### 2.2 특수 시프트 (결합 규칙으로 구현)

모음시프트는 날개셋에서 **가상 낱자 500**(단독으로는 ㅗ로 출력)으로 구현되어 있고, `v`의 실제 ㅗ와 구별된다.

| 모음시프트 + | ㅓ | ㅕ | ㅔ | ㅗ | ㅜ | ㅡ | ㅏ | ㅣ |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 결과 | ㅝ | ㅒ | ㅖ | ㅛ | ㅠ | **ㅑ** | ㅘ | ㅚ |

- ㅑ는 `.`+`g`(가상 500 + ㅡ)로만 나온다. `v`(ㅗ)+ㅡ는 옛한글(ᅩ+ᅳ 계열)로 가므로 **ㅗ와 가상 ㅗ는 반드시 별개 낱자**여야 한다.

| 종성시프트(ㅆ) + | ㄱ | ㄴ | ㄹ | ㅁ | ㅂ | ㅅ | ㅇ |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 결과 | ㅋ | ㅎ | ㅈ | ㄷ | ㅍ | ㅊ | ㅀ |

### 2.3 현대 한글 결합 규칙 (순서 무관, 규격 <부록> "수용 필수")

- 초성: 된소리 = 자음 + ㅇ (ㄲㄸㅃㅆㅉ), 거센소리 = 자음 + ㅎ (ㅊ=ㅈ+ㅎ, ㅋ=ㄱ+ㅎ, ㅌ=ㄷ+ㅎ, ㅍ=ㅂ+ㅎ)
- 중성: ㅐ=ㅏ+ㅣ, ㅢ=ㅡ+ㅣ, ㅛ=ㅓ+ㅏ(보조), ㅟ=ㅔ+ㅣ(보조), 그 외 ㅘ/ㅙ/ㅝ/ㅞ 등은 구성 모음끼리
- 종성: ㄲ=ㄱ+ㅇ, ㅆ=ㅅ+ㅇ, 겹받침 11개(ㄳㄵㄶㄺㄻㄼㄽㄾㄿㅀㅄ)
  - 갈마들이: ㄵ=ㄴ+ㄹ, ㄶ=ㄴ+ㅇ, ㄾ=ㅇ+ㅁ, ㄿ=ㅇ+ㅂ
  - 보조: ㄺ=ㅁ+ㄱ, ㄱ=ㄹ+ㅇ, ㅌ=ㄴ+ㅁ, ㄲ=ㄽ+ㅆ, ㄳ=ㄱ+ㅁ+ㅆ, ㄻ=ㅄ+ㅆ, ㅌ=ㄼ+ㅆ
- XML `UnitMixTable`에는 **439개 규칙**(초성 161 / 중성 132 / 종성 147)이 `(a, b) → to` 형태로 양방향 모두 들어 있다. 상당수는 약어용 옛한글 조합이다.

### 2.4 입력 방식별 오토마타 (XML 분석)

| 방식 | 파일 | 오토마타 | 키뗌 사용 | 타이머 |
| --- | --- | --- | --- | --- |
| 한 번에 모아치기 | `세모이 (한 번에 모아치기).xml` | `A\|B\|C ? 1 : 0` (어떤 낱자든 들어오면 조합 유지) | O (`KeyProgramTable`) | 40 ms (`AUTOMATIC_TIMER`) |
| 중·종성 모아치기 / 이어치기 | `세모이 (이어치기 & 중종성 모아치기).xml` | `0: A?1 : B\|C?2`, `1: A?1 : B\|C?2`, `2: B\|C?2 : 0` | X | 없음 |

**한 번에 모아치기 동작 (날개셋 구현 해석)**

1. 키 누름: 동시 눌림 수 `v` 갱신, 낱자를 해당 벌(초/중/종) 슬롯에 넣거나 기존 낱자와 결합.
2. 키 뗌: `v < 1`(모두 뗌)이면 상태 0 → 음절 확정. 하나라도 눌려 있으면 상태 1 유지(조합 계속).
3. 타이머: 마지막 입력 후 40 ms가 지나면 키가 눌려 있어도 강제 확정 (기호 확장 상태 `T>=100`이면 비활성).
4. 스페이스: 조합 중에 눌리면 플래그(`s=2`)만 세우고, 확정 시점에 공백을 덧붙인다 → `ㄱ ㅏ ㅇ Space` 동시 입력이 `강 `이 된다.

**중·종성 모아치기 / 이어치기**: 일반 세벌식 오토마타. 초성 뒤에 중·종성이 오면 같은 음절, 중·종성 뒤에 초성이 오면 새 음절. 같은 벌 안에서는 순서 무관 결합.

### 2.5 약어 시스템

- 결합 규칙으로 **현대 한글에 없는 옛한글 음절**(예: 초성 ㄱ + 채움 중성 + 종성 ㄱ = `ᄀᅠᆨ`)을 만들고, `HanSubstTable`(1,363개)에서 찾은 단어를 **조합 중 표시(marked text)** 로 보여 준다.
- 조합이 끝나야 확정되므로 중간 약어를 거쳐도 안전하다.
  - 예: 초ㄱ+종ㄱ → `국가` 표시 → 종ㅇ 추가(종성 ㄲ) → `공격` → Backspace → 다시 `국가`.
- 사전에 없으면 옛한글 음절을 그대로 출력한다. 일부 약어는 끝에 공백을 포함한다(`기는 `, `하는 `).
- 기호 약어: `? `=n+m(ㅅ+ㄹ), `, `=u+i(ㄴ+ㄷ), `. `=i+o(ㄷ+ㅂ), `! `=y+u(ㅁ+ㄴ). 초성끼리 결합한 뒤 사전으로 치환한다. 원본 XML 설명문에는 `?`와 `!`가 서로 바뀌어 적혀 있다(실제 표 기준, Phase 1에서 확인).
- 옛한글 모드에서는 약어를 꺼야 한다.
- **추가 약어(원본 사전에 없는 것)**: 원본 XML과 별도로 `data/abbreviations-extra.json`에 두고, `semoe-datagen`이 원본 사전과 합친다. 원본 항목과 키가 겹치면 변환 시 오류로 처리한다.
  - `ojz`(초ㅂ+초ㅇ+종ㅁ = `ᄈᅠᆷ`) → **뺨**. 정상 입력에는 다섯 키(`o j . g z`)를 동시에 눌러야 해서, 맥북 내장 키보드에서는 동시 입력 한계로 `뺘ㅁ`이 된다(Phase 0). `ᄈᅠᆷ`은 원본 사전 1,363개에 없는 빈 자리다.

### 2.6 그 밖의 XML 요소

| 요소 | 개수 | 용도 |
| --- | --- | --- |
| `KeyTable` (0x21~0x7E) | 94 | 키 → 낱자/기호. 조건식(`T==101` 기호확장 단계, `P` Caps Lock, `N&2` Scroll Lock) 포함 |
| `VirtualUnit` | 1 | 중성 500 → ㅗ (모음시프트) |
| `UserCompoTable` | 178 | 기호 연타 조합 상태기계 (`;`, `→`, `☆` 등에서 시작) |
| `UserCandiTable` | 370 | 후보 변환 (`가가` → `가가호호`) |
| `Bksp` | 4 | 조합 중 Backspace는 낱자 단위(`ByUnitStep`) |

낱자 표기는 날개셋 고유 이름(`G_`=ㄱ, `Q_`=ㅇ, `EO`=ㅓ, `SS`=ㅆ, `RH`=ㅀ, 옛한글 `BT`, `QP` …)이므로 **유니코드 첫가끝 자모와의 대응표**가 필요하다. 교차 검증용으로 openwnn-korean의 `LayoutMoachigiSebul.java`(`COMB_SEBUL_SEMOE`, `createSemoeAbbreviations`)가 유니코드 기반 구현을 갖고 있다.

---

## 3. macOS 구현 시 핵심 쟁점

### 3.1 키뗌(keyUp) 수신 — 최대 위험 요소

한 번에 모아치기는 "모든 키가 떼어졌는가"를 알아야 한다. InputMethodKit(`IMKInputController`)은 `recognizedEvents(_:)`로 keyUp 마스크를 요청할 수 있으나, **앱/환경에 따라 keyUp이 안정적으로 전달되는지 검증되지 않았다.** Phase 0에서 반드시 확인한다. (언어 선택과 무관한 문제다.)

| 방안 | 장점 | 단점 |
| --- | --- | --- |
| A. IMK keyUp (`recognizedEvents`에 `.keyUp` 포함) | 권한 불필요, 표준 경로 | 전달 보장 여부 불확실 |
| B. 입력기 프로세스 내 `CGEventTap`(listenOnly)로 키 상태만 관측, 문자 처리는 IMK | 물리 키 상태를 정확히 앎 | "입력 모니터링" 권한 필요, 두 이벤트 경로 동기화 필요 |
| C. 타이머 전용 모아치기 (keyDown 간격만으로 chord 경계 판단) | 구현 단순, 권한 불필요 | 키를 누른 채 다음 키를 늦게 치는 패턴 미지원, 원본과 체감 차이 |

~~권장: A를 기본으로, 실패하는 환경을 위해 C를 폴백으로 둔다.~~ **Phase 0 결과 IMK는 keyUp을 전혀 넘기지 않고, keyDown도 시각이 부정확하고 늦으므로 B를 기본, C를 폴백으로 한다** ([PHASE0_REPORT.md](PHASE0_REPORT.md)). 탭이 keyDown·keyUp을 물리 순서대로 기록하고 IMK keyDown과 짝을 맞춘다. 엔진은 순서가 맞춰진 이벤트만 받는다. 엔진은 세 방안 모두를 지원하도록 "키뗌 정보 신뢰 여부"를 옵션으로 받는다.

### 3.2 그 밖의 macOS 차이

- **Scroll Lock 없음** → 옛한글/기호 확장 모드는 메뉴 막대 메뉴 + 단축키로 전환.
- **한/영 전환**: 입력기 내부 영문 모드 없이 **시스템 입력 소스 전환**에 맡긴다. 지원하는 방법은 두 가지다.
  1. **Caps Lock (기본)**: 시스템 설정 → 키보드 → 텍스트 입력 → 입력 소스의 "Caps Lock 키로 ABC 입력 소스 전환" 옵션을 그대로 쓴다. 이 경우 Caps Lock은 한/영 전환 전용이 되므로 순아래(윗글쇠 고정) 기능과 겹친다.
  2. **오른쪽 Command**: `hidutil`로 오른쪽 Command(HID `0x7000000E7`)를 F18(HID `0x70000006D`)로 바꾸고, F18을 "이전 입력 소스 선택" 단축키로 지정한다. 재부팅 후에도 유지되도록 LaunchAgent(`~/Library/LaunchAgents/com.user.KeyRemapping.plist`)로 등록한다. 오른쪽 Command는 원래 Command 기능을 잃는다.

     ```xml
     <?xml version="1.0" encoding="UTF-8"?>
     <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
     <plist version="1.0">
     <dict>
         <key>Label</key>
         <string>com.user.KeyRemapping</string>
         <key>ProgramArguments</key>
         <array>
             <string>/usr/bin/hidutil</string>
             <string>property</string>
             <string>--set</string>
             <string>{"UserKeyMapping":[{"HIDKeyboardModifierMappingSrc":0x7000000E7,"HIDKeyboardModifierMappingDst":0x70000006D}]}</string>
         </array>
         <key>RunAtLoad</key>
         <true/>
     </dict>
     </plist>
     ```

     적용: `launchctl load ~/Library/LaunchAgents/com.user.KeyRemapping.plist` (즉시 적용만 하려면 `hidutil` 명령을 직접 실행). 해제: `hidutil property --set '{"UserKeyMapping":[]}'` 후 plist 삭제.
- **Caps Lock 순아래**: 기본 꺼짐. 켤 때 Caps Lock이 한/영 전환에 쓰이고 있으면 오른쪽 Command 방식으로 바꾸라고 안내한다.
- **F18 등 기능 키**: 입력기는 F18(`kVK_F18`)을 소비하지 않고 통과시켜야 시스템 단축키가 동작한다. 조합 중이면 먼저 확정한다(입력 소스 전환 시 `deactivateServer`에서도 `semoe_flush`).
- **키 자동 반복**: `event.isARepeat`인 keyDown은 모아치기 중 무시.
- **물리 키 기준 처리**: `characters`가 아닌 `keyCode`로 키를 식별해야 시스템 영문 배열(Dvorak 등)과 무관하게 동작한다. Swift 껍데기가 macOS 가상 키 코드를 엔진의 플랫폼 중립 키(`SemoeKey`)로 바꾼다.
- **Cmd/Ctrl/Option 조합**: 처리하지 않고 통과시키되, 조합 중이면 먼저 확정.
- **Marked text 호환성**: 터미널, Electron, JetBrains IDE, MS Office 등에서 조합 표시·확정 동작이 다르다. 호환성 매트릭스로 관리.
- **옛한글 표시**: 첫가끝 자모 조합 출력이므로 글꼴에 따라 깨져 보일 수 있다(약어 미등록 조합에서만 노출).
- **고스트 키**: 3키 이상 동시 입력이 안 되는 키보드는 하드웨어 한계 → 문서로 안내.

---

## 4. 아키텍처

### 4.1 전체 구조

```
┌──────────────────────── SemoeIME.app (Swift, macOS 전용) ────────────────────────┐
│ IMKServer ── SemoeInputController (IMKInputController)                            │
│               ├─ KeyMapper     : NSEvent.keyCode/flags → SemoeKey, 수정키 상태      │
│               ├─ EngineHandle  : Rust 엔진 래퍼 (생성/해제, 호출, 문자열 변환)       │
│               ├─ ClientBridge  : setMarkedText / insertText / 확정 처리             │
│               ├─ CommitTimer   : DispatchSourceTimer (엔진이 알려 준 마감 시각에 발화)│
│               └─ Preferences   : 입력 방식, 타이머 ms, 약어 on/off, 확장 모드 (UI)   │
└───────────────────────────────────────┬──────────────────────────────────────────┘
                                        │ C ABI (cbindgen 헤더 + Swift module map)
┌──────────────── crates/semoe-ffi (Rust, staticlib) ────────────────────────────────┐
│ extern "C" 함수 몇 개, 불투명 핸들, catch_unwind로 panic 차단                        │
└───────────────────────────────────────┬──────────────────────────────────────────┘
┌──────────────── crates/semoe-core (Rust, 플랫폼 중립) ─────────────────────────────┐
│ layout      : SemoeKey + 수정키 + 모드 → Unit(초·중·종/기호), 레이어 테이블          │
│ mixer       : (slot, a, b) → to  (순서 무관, 다단 결합)                             │
│ syllable    : 초·중·종 슬롯 + 낱자 단위 undo 스택(Backspace)                         │
│ automaton   : Sequential(이어치기·중종성) / Chord(한 번에) 전략                       │
│ chord       : 눌린 키 집합, 마지막 입력 시각, 확정 마감 시각 계산                    │
│ abbrev      : 첫가끝 문자열 → 치환어 (marked text 표시값 계산)                       │
│ symbol      : UserCompo 상태기계, 기호 확장 단계(T=100~103)                          │
│ engine      : 위 요소를 묶어 Output{consumed, commit, marked, deadline} 반환        │
└───────────────────────────────────────┬──────────────────────────────────────────┘
                                        │ include_str! (빌드 시 내장)
┌──────────────── data/*.json (생성물, CC BY-SA 4.0) ◀── crates/semoe-datagen ───────┐
│ 날개셋 XML → layout.json / unit-mix.json / abbreviations.json / compose.json        │
│ 날개셋 낱자 이름 → 유니코드 첫가끝 자모 매핑표(unit-names.json, 수작업 + 검증)        │
└────────────────────────────────────────────────────────────────────────────────────┘
```

설계 원칙

- **원본 XML을 단일 진실 원천(source of truth)으로 유지**하고, 런타임 데이터는 `semoe-datagen`으로 생성한다. 원본 저장소가 갱신되면 재생성만 하면 된다. 생성된 JSON은 커밋해 변경 내역을 diff로 검토한다.
- 날개셋 조건식(`T==101 ? … : P ? … : H3|_S`)은 범용 인터프리터를 만들지 않고, 변환기에서 **구조화된 레이어 테이블**(기본/Shift/CapsLock/확장모드/기호확장 1~3단계)로 풀어 둔다. 풀 수 없는 식은 변환 시 실패로 처리해 누락을 막는다.
- **`semoe-core`는 순수 로직이다.** OS API, 스레드, 실제 타이머를 쓰지 않는다. 시각은 호출자가 넘겨 주고, 타이머가 필요하면 "언제 다시 불러 달라"는 마감 시각만 돌려준다. 덕분에 타이밍이 얽힌 시나리오도 `cargo test`로 결정적으로 재현된다.
- **Swift 껍데기는 얇게 유지한다.** 입력 판단 로직이 Swift 쪽으로 새어 나오면 다른 플랫폼 이식 시 다시 짜야 하므로, Swift에는 OS 연동과 UI만 둔다.

### 4.2 FFI 경계

경계는 작고 단순하게 유지한다. 함수 수가 적어 UniFFI 같은 바인딩 생성기 대신 **`cbindgen` + 손으로 쓴 C ABI**를 쓴다(의존성·생성 코드 최소화, 동작이 투명함).

```c
typedef struct SemoeEngine SemoeEngine;            // 불투명 핸들

typedef enum { SEMOE_KEY_DOWN, SEMOE_KEY_UP } SemoeKeyAction;

typedef struct {
    uint16_t key;          // SemoeKey (플랫폼 중립 물리 키)
    SemoeKeyAction action;
    uint32_t modifiers;    // SHIFT | CAPS | CTRL | ALT | CMD
    bool     is_repeat;
    uint64_t time_ms;      // 단조 증가 시각
} SemoeKeyEvent;

typedef struct {
    bool        consumed;      // false면 앱으로 그대로 전달
    const char *commit;        // 확정 문자열(UTF-8), 없으면 NULL
    const char *marked;        // 조합 중 표시 문자열, 빈 문자열이면 조합 없음
    int64_t     deadline_ms;   // 이 시각에 semoe_on_timer 호출 필요, -1이면 타이머 해제
} SemoeOutput;

SemoeEngine *semoe_engine_new(void);
void         semoe_engine_free(SemoeEngine *);
SemoeOutput  semoe_handle_key(SemoeEngine *, SemoeKeyEvent);
SemoeOutput  semoe_on_timer(SemoeEngine *, uint64_t now_ms);
SemoeOutput  semoe_flush(SemoeEngine *);            // 포커스 이동 등 강제 확정
void         semoe_reset(SemoeEngine *);
bool         semoe_set_option(SemoeEngine *, const char *key, const char *value);
```

- **메모리**: `SemoeOutput`의 문자열은 엔진이 소유하고 다음 호출 전까지만 유효하다. Swift는 받은 즉시 `String`으로 복사한다. 해제 함수를 따로 두지 않아 누수 위험을 없앤다.
- **스레드**: IMK 콜백은 메인 스레드에서 오므로 엔진은 `Send`만 보장하고 내부 잠금은 두지 않는다. 컨트롤러(클라이언트 세션)마다 엔진 인스턴스를 하나씩 만든다.
- **panic 안전**: 모든 `extern "C"` 함수는 `catch_unwind`로 감싸고, panic 시 `consumed = false`를 돌려 키를 앱에 넘긴다. 입력기가 죽어 타자가 막히는 것보다 원래 키가 들어가는 편이 낫다. 릴리스 빌드는 `panic = "unwind"` 유지.
- **데이터 로딩**: JSON은 `include_str!`로 바이너리에 내장하고 엔진 생성 시 한 번 파싱한다. 1,300여 항목이라 비용이 무시할 만하다. 사용자 약어 추가 기능이 생기면 `semoe_load_user_dict(path)`를 추가한다.

### 4.3 핵심 알고리즘 (한 번에 모아치기, semoe-core)

```
handle_key(Down, key, t):
  if repeat: return consumed
  if composing && t >= deadline: flush()          # 타이머 이벤트가 늦게 온 경우 보정
  pressed.insert(key)
  unit = layout.lookup(key, modifiers, mode)
  if unit is Space && composing: pending_space = true
  elif unit is 낱자: syllable.add(unit)          # 같은 슬롯이면 mixer로 결합, 실패 시 flush 후 새 음절
  else: flush(); 기호 처리
  deadline = t + timeout                          # 기호 확장 중이면 없음
  return Output{ marked: abbrev[syllable.jamo] ?? syllable.render(), deadline }

handle_key(Up, key, t):
  pressed.remove(key)
  if pressed.is_empty(): return flush()

on_timer(now):
  if composing && now >= deadline: return flush()

flush():
  text = abbrev[syllable.jamo] ?? syllable.render()
  commit(text + (pending_space ? " " : "")); reset()
```

- 같은 슬롯에 결합 불가능한 낱자가 오면(예: 초성 ㄱ 다음 초성 ㄹ에 규칙 없음) 앞 음절을 확정하고 새 음절을 시작한다. 정확한 동작은 날개셋과 대조 테스트로 확정한다.
- Swift 쪽은 `deadline_ms`가 오면 `DispatchSourceTimer`를 다시 걸고, 발화 시 `semoe_on_timer`를 불러 나온 `commit`을 `client.insertText`로 넣는다.
- 키뗌을 믿을 수 없는 환경(3.1 방안 C)에서는 `semoe_set_option("keyup", "off")`로 Up 이벤트를 무시하고 마감 시각만으로 확정한다.

---

## 5. 개발 단계

### Phase 0 — 기술 검증 스파이크 (1.5주)

- [x] 최소 IMK 입력기(Swift) 빌드·설치·등록 절차 정리 (번들 ID, `Info.plist`의 `InputMethodConnectionName`, `tsInputModeListKey`) — `scripts/install-dev.sh`
- [x] keyUp 수신 검증: 10개 앱 모두 **IMK keyUp 0건** → 방안 A 폐기
- [x] 이벤트 타임스탬프 정밀도, 40 ms 타이머 지연 측정: 이벤트 지연 p50 < 0.5 ms, 타이머 발화 지연 p50 ≈ 1 ms (최악 19 ms)
- [x] **Rust ↔ Swift 연결 검증**: 더미 `semoe_handle_key`를 staticlib로 빌드해 입력기에 링크, 유니버설 바이너리(arm64 + x86_64) 생성, 코드 서명까지 통과 확인 (ad-hoc 서명)
- [x] 결과에 따라 3.1의 A/B/C 방안 확정 → **B(탭으로 keyDown·keyUp 물리 스트림) 기본 + C 폴백**, 자동 타건 테스트 통과

진행 상황과 남은 수동 검증 절차는 [`PHASE0_REPORT.md`](PHASE0_REPORT.md) 참고.

**완료 기준**: keyUp 방식 결정 문서, Rust 엔진을 거쳐 "ㄱ+ㅏ+ㄴ 동시 입력 → 간"이 실제 앱에 입력되는 프로토타입

### Phase 1 — 데이터 변환기 `semoe-datagen` (1~1.5주) ✅

- [x] `quick-xml` 기반 파서(`KeyTable`, `UnitMixTable`, `VirtualUnitTable`, `HanSubstTable`, `UserCompoTable`, `UserCandiTable`)
- [x] 날개셋 낱자 이름 ↔ 유니코드 첫가끝 자모 매핑표 작성 (`data/src/unit-names.json`, 초성 76·중성 49·종성 52)
  - 날개셋 이름은 유니코드 자모 이름의 줄임말이다(`ND`=NIEUN-TIKEUT, `BQ`=KAPYEOUNPIEUP, `IYE`=I-YE). `scripts/bootstrap-unit-names.py`로 초안을 만들고 검증했다.
  - openwnn-korean과 교차 검증: 현대 자모와 ㆍ·ᆢ·ㆎ만 있어 일부만 확인됐다. 대신 원본 약어 설명서(xlsx)와 1,363개 전부를 대조했다.
  - 치두음·정치음 10개(`Sw`/`wS` 등)는 근거가 없어 미검증으로 표시했다. 약어 사전에는 쓰이지 않는다.
- [x] 조건식 → 레이어 테이블 변환, 해석 불가 식은 오류 (키별 조건부 규칙 목록, 위에서부터 첫 일치)
- [x] 검증: 모든 결합 결과가 매핑표에 존재, 모든 약어 키가 결합 규칙으로 **도달 가능**한지 탐색 검사 (추가 약어 포함)
  - 벌별 도달 가능성만으로는 두 이름이 서로 바뀐 오류를 못 잡는다. 그래서 약어 설명서와 전체 경로를 대조하는 검사를 더했다.
- [x] 두 XML의 공통 데이터(KeyTable·UnitMix·HanSubst 동일) 확인 후 하나로 통합
  - 조합 종료 타이머(`Extra/Timer`), 오토마타, 키 프로그램만 입력 방식마다 다르다. 이것들은 `meta.json`에 원본 식으로 기록만 한다.
- [x] `serde` 데이터 타입은 `semoe-core`와 공유(`semoe-core`의 `data` 모듈을 datagen이 의존)
- [x] 추가 약어 `data/src/abbreviations-extra.json` 병합 (원본과 키 충돌 시 오류)
- [x] `scripts/gen-data.sh` (`--check`로 최신 여부 확인, Phase 5 CI에서 사용)

**발견 사항** (Phase 2에 반영)

- 결합 규칙 439개 가운데 24개는 한 방향만 적혀 있다(예: ㅚ+ㅔ → ퟀ). 그래서 엔진은 적힌 순서를 먼저 보고, 없으면 반대 순서를 봐야 한다.
- 두 순서의 결과가 다른 규칙이 1쌍 있다(종성 ㄹ+ㅍ=ㄿ, ㅍ+ㄹ=ㅌ). ㅍ은 단일 키가 아니라서 이 규칙은 실제 타건으로 닿지 않는다. 변환기가 경고로 알린다.
- 원본 문서끼리 어긋나는 곳이 있다. XML 설명문에는 `'? '=y+u`, `'! '=n+m`으로 되어 있지만, 실제 약어 표와 설명서(xlsx)는 반대다(`? `=n+m, `! `=y+u). 정답은 XML 약어 표로 한다.

### Phase 2 — 엔진 `semoe-core` (2~3주) ✅

- [x] layout / mixer / syllable(낱자 단위 Backspace) 구현 — `data/*.json`을 `include_str!`로 내장, 결합은 양방향 조회 (Phase 1 발견 사항). Phase 0의 손으로 옮긴 배열(`layout.rs`)은 삭제
- [x] Sequential 오토마타(이어치기, 중·종성 모아치기) — 원본에서 둘은 같은 오토마타 하나이므로 엔진의 `Mode::Sequential` 하나로 둔다
- [x] Chord 오토마타 + 마감 시각 계산 + 스페이스 모아치기
- [x] abbrev(조합 중 표시, 확정 시 치환, Backspace 시 이전 약어 복원) + 추가 약어(`ojz` → 뺨, 2.5)
- [x] 윗글쇠 기호(Shift), 원래 문자와 같은 키는 앱에 넘김. Caps Lock 순아래·옛한글(Scroll Lock) 자리는 옵션으로만 연결(기본 꺼짐)
- [x] `cargo test` 골든 테스트: 규격·설명서 예제 (`tests/golden.rs`, 15개)
  - `ㄱㅏㄴ` / `ㅏㄱㄴ` / `ㄴㄱㅏ` 동시 → `간`
  - 한 번에 모아치기 4타 → `흙짚맞이`
  - 초ㄱ+종ㄱ → `국가`, +종ㅇ → `공격`, Backspace → `국가`
  - 모음시프트+ㅡ → ㅑ, ㅗ+ㅡ → 옛한글(ㅑ 아님. 이 음절은 약어 `그곳`)
  - ㄶ=ㄴ+ㅇ, ㄾ=ㅇ+ㅁ, ㄿ=ㅇ+ㅂ, 종성시프트 단독 → ㅆ
  - `ㄱ ㅏ ㅇ Space` 동시 → `강 `
  - 초ㅂ+초ㅇ+종ㅁ(`ojz`) → `뺨`
  - 추가: `됐` 여러 순서, 기호 약어, 이어치기
- [x] 키 이벤트 로그 재생 하네스: 텍스트 픽스처(`+k 0`, `+f 3`, `-k 25` …)를 읽어 결과 문자열 비교 (`tests/replay.rs`, `tests/fixtures/*.txt`)
- [x] 결합 규칙·순서 무관성에 대한 속성 기반 테스트(`proptest`): 같은 chord의 모든 순열이 같은 결과를 내는지 (`tests/order.rs`. 키 2~3개는 전수, 2~5개는 무작위 2,000건)

**설계 결정과 발견 사항**

- **chord는 낱자 목록 전체로 판정한다.** 한 키씩 결합하면 중간 상태에서 막히는 경우가 있다. 예: ㅣ→ㅓ→모음시프트는 ㅣ+ㅓ 규칙이 없어 쪼개지지만, 셋을 다 보면 ㅓ+모음시프트=ㅝ, ㅝ+ㅣ=ㅞ가 된다. 그래서 chord에 들어온 낱자를 모아 두었다가 표시·확정할 때 판정한다. 한 음절로 합쳐지는 순서가 하나라도 있으면 한 음절, 없을 때만 들어온 순서대로 나눈다.
- **한 음절이 되는지는 누르는 순서와 무관하다.** 아랫글쇠 키 2~4개 chord 20,826개로 확인했고, 그중 15,585개가 한 음절이 된다.
- **두 가지 답이 나오는 종성 묶음이 4개 있다.** `q e a`(ㄳ/ㅈ), `e s ;`(ㄵ/ㅀ), `q e a s`, `w e s ;`는 원본 규칙으로 두 결과가 모두 나온다. chord에서는 낱자를 코드 순으로 정렬해 합쳐 결과를 하나로 고정했고(각각 ㄳ, ㅀ, ㅀ, ㄿ), 이어치기는 누른 순서를 따른다. 날개셋 실제 동작과 대조할 항목이다.
- 키가 눌려 있는 동안 chord는 계속 모인다(날개셋의 `v`). 그래서 `ㄱ+ㄱ=ㄲ`, `ㅏ+ㅏ=ㅑ` 규칙 때문에, 간을 누른 채 가를 겹쳐 누르면 `꺈` 한 음절이 된다.
- 기호 확장 단계 명령(`C1|100~103`, Shift+J 등)은 Phase 4까지 조합만 확정하고 무시한다. 조건의 날개셋 내부 플래그 `E`·`F`는 뜻을 확인하지 못해 꺼진 것으로 본다.
- 실제 앱 확인: 합성 키로 Safari 6회, Chrome 3회 중 8회가 기대값과 일치했다. 재설치 직후 첫 회 1번은 Safari에서 `가ㅇ`이 `강`으로 나왔는데, 엔진은 같은 이벤트로 `가ㅇ`을 낸다(픽스처 `chord-quick-release.txt`). 입력기가 앱에 글자를 넣는 쪽 문제로 보고 Phase 3 호환성 항목으로 넘긴다.

### Phase 3 — FFI 및 macOS 입력기 통합 (2~2.5주)

- [ ] `semoe-ffi`: 4.2의 C ABI 구현, `cbindgen`으로 헤더 생성, Swift module map
- [ ] 빌드 스크립트: `cargo build --release`를 두 타깃(`aarch64-apple-darwin`, `x86_64-apple-darwin`)으로 돌리고 `lipo`로 합침 → XcodeGen `preBuildScripts`에서 호출
- [ ] `SemoeInputController`: keyCode → `SemoeKey` 변환, marked text 갱신, 확정 시점 처리, `CommitTimer`
- [ ] 포커스 이동·마우스 클릭·앱 전환 시 `semoe_flush`, 수정키 조합 통과
- [ ] 입력 방식 메뉴 전환(한 번에 모아치기 / 이어치기·중종성 모아치기, 엔진 `mode` 옵션), 약어 on/off, 설정 저장(`UserDefaults` → `semoe_set_option`), 타이머 ms 슬라이더(20~100, 5 단위)
- [ ] 재설치·앱 활성화 직후 첫 타건 불안정 조사 (Phase 0의 늦은 IMK keyDown, Phase 2의 Safari `가ㅇ`→`강` 1회)
- [ ] 앱 호환성 매트릭스 작성 및 앱별 우회 처리 (우회 로직이 입력 판단이면 엔진 옵션으로 내린다)

### Phase 4 — 확장 기능 (1.5~2주)

대부분 `semoe-core` 작업이고 Swift는 메뉴·설정만 추가한다.

- [ ] 윗글쇠 기호, Caps Lock 순아래 옵션
- [ ] 기호 확장(Shift+J → Shift+K/L/;), 확장 중 일시적 이어치기 전환(규격 권장)
- [ ] UserCompo 기호 조합
- [ ] 옛한글 모드(숫자열 옛한글, 약어 자동 끔, Shift+Y/U 방점)
- [ ] 후보 변환(UserCandi) — 엔진은 후보 목록만 내고 Swift가 `IMKCandidates`로 표시. P3로 미뤄도 됨

### Phase 5 — 배포 (1주)

- [ ] Developer ID 서명, 공증(notarization), `.pkg` 설치기(`~/Library/Input Methods`)
- [ ] 설치/제거/업데이트 안내, 재로그인 필요 여부 문서화
- [ ] GitHub Actions(macOS 러너): `cargo fmt --check` / `cargo clippy` / `cargo test` → datagen 재실행 후 `data/` diff 없음 확인 → 유니버설 staticlib 빌드 → `xcodebuild`
- [ ] 사용자 문서: 배열도, 결합 규칙 링크, 키보드 고스트 키 안내, 타이머 조절법, 한/영 전환 설정(Caps Lock / 오른쪽 Command, 3.2)
- [ ] 오른쪽 Command 전환용 LaunchAgent plist를 `scripts/`에 포함하고 설치·해제 스크립트 제공(선택 설치)

총 예상: **약 10~12주** (1인 기준, Swift 단독 대비 FFI·이중 빌드 구성으로 약 1주 추가, Phase 0 결과에 따라 ±2주)

---

## 6. 저장소 구조 (제안)

```
semoe-macos/
├── Cargo.toml                   # Rust workspace
├── crates/
│   ├── semoe-core/              # 엔진 (플랫폼 중립, OS 의존성 없음)
│   │   ├── src/
│   │   └── tests/fixtures/      # 키 이벤트 재생 픽스처
│   ├── semoe-ffi/               # C ABI, crate-type = ["staticlib"], cbindgen.toml
│   └── semoe-datagen/           # 날개셋 XML → JSON 변환 CLI
├── data/                        # 생성된 JSON (CC BY-SA 4.0), 커밋 대상
│   └── src/                     # 수작업 원본: unit-names.json, abbreviations-extra.json
├── macos/
│   ├── project.yml              # XcodeGen 정의 (Xcode 프로젝트는 생성물로 취급)
│   └── SemoeIME/                # Swift 소스, Info.plist, 아이콘, module.modulemap
├── scripts/
│   ├── build-rust-universal.sh  # 두 타깃 빌드 + lipo + 헤더 생성
│   ├── gen-data.sh              # ../Semo-e_keyboard XML → data/
│   ├── install-dev.sh           # 개발용 설치·재등록
│   └── right-cmd-hangul/        # 오른쪽 Command → F18 LaunchAgent plist + 설치/해제 스크립트
└── docs/
```

- 현재 저장소에는 IntelliJ 모듈(`semoe-macos.iml`)만 있다. RustRover/IntelliJ Rust 플러그인으로 workspace를 열고, Swift 쪽은 Xcode로 편집한다. 둘 다 CLI(`cargo`, `xcodebuild`)만으로 빌드 가능하게 한다.
- 이후 다른 플랫폼을 붙일 때는 `semoe-core`를 별도 저장소로 떼어 내거나 저장소 이름을 바꾸는 것을 검토한다.

---

## 7. 위험 요소와 대응

| 위험 | 영향 | 대응 |
| --- | --- | --- |
| IMK keyUp 미전달 (Phase 0에서 확인됨) | 키뗌 확정 불가 | CGEventTap(입력 모니터링 권한) + 권한 없을 때 타이머 전용 |
| 입력 모니터링 권한 거부·서명 변경 시 권한 해제 | 타이머 전용으로 체감 차이 | 메뉴에 상태 표시·권한 요청, 고정 서명(배포는 Developer ID) |
| 앱별 marked text 동작 차이 | 글자 누락·중복 | 호환성 매트릭스, 앱 번들 ID별 처리 |
| 날개셋 낱자 이름 → 유니코드 매핑 오류 | 약어·옛한글 오작동 | openwnn-korean 교차 검증, 도달 가능성 검사 |
| 날개셋 세부 동작(결합 실패 시 처리 등) 불명확 | 체감 차이 | 실사용자 피드백, Windows 날개셋과 같은 입력 로그 대조 |
| 타이머 최적값 개인차 | 오입력 | 설정 노출(원본도 개인 설정 방식) |
| 키보드 동시 입력 한계(재밍, 맥북 내장 키보드에서 확인) | 4~5키 chord가 쪼개짐 (`뺨` → `뺘ㅁ`) | 자주 쓰는 다키 음절은 추가 약어로 제공. 키가 눌린 동안 타이머를 보류하는 방안은 Phase 0에서 시험했다가 효과가 부족해 롤백 |
| FFI 경계 결함 (잘못된 포인터 수명, panic 전파) | 입력기 크래시 | 경계 최소화, 문자열은 엔진 소유 + 즉시 복사, `catch_unwind`, FFI 계층 테스트 |
| Rust·Xcode 이중 빌드 구성 | 빌드 실패, 아키텍처 누락 | 스크립트 하나로 통합, CI에서 `lipo -info`로 두 아키텍처 확인 |
| 입력 판단 로직이 Swift로 새어 나감 | 다른 플랫폼 이식 시 재작성 | 리뷰 규칙: 입력 결과를 바꾸는 코드는 `semoe-core`에만 둔다 |
| 라이선스 | 배포 제약 | 자판 데이터(파생물)는 CC BY-SA 4.0 유지·출처(ssgi.kr) 표기, 코드 라이선스는 별도 결정 |

---

## 8. 결정이 필요한 사항

1. 코드 라이선스 (데이터는 CC BY-SA 4.0 고정)
2. 최소 지원 macOS 버전 (제안: macOS 13 이상), Rust 최소 버전(MSRV, 제안: 고정하지 않고 stable 추적)
3. ~~입력기 내부 영문 모드 포함 여부~~ → 시스템 입력 소스 전환(Caps Lock 또는 오른쪽 Command→F18)으로 결정 (3.2)
4. 원본 저장소를 git submodule로 연결할지, 데이터만 복사할지
5. 다른 플랫폼(Linux·Windows) 껍데기 착수 시점과 `semoe-core` 저장소 분리 여부
