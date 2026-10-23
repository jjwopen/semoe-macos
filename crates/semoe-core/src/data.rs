//! 자판 데이터 형식. `semoe-datagen`이 날개셋 XML에서 만들고(`data/*.json`), 엔진이 읽는다.
//!
//! 낱자는 유니코드 첫가끝 자모(U+1100 영역과 확장 A·B)로 나타낸다. 날개셋의 가상 낱자(모음시프트 등)는
//! 사용 영역 문자 `U+F000 + id`로 나타낸다(`virtual_unit_char`).

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Slot {
    Cho,
    Jung,
    Jong,
}

/// 가상 낱자 id(날개셋 `VirtualUnit`의 번호)를 데이터 안의 문자로 바꾼다.
pub fn virtual_unit_char(id: u32) -> Option<char> {
    (id < 0x1000).then(|| char::from_u32(0xF000 + id)).flatten()
}

/// 초성 채움 문자. 중성만 있는 음절을 나타낼 때 쓴다.
pub const CHO_FILLER: char = '\u{115F}';
/// 중성 채움 문자. 초성·종성만 있는 음절(주로 약어)을 나타낼 때 쓴다.
pub const JUNG_FILLER: char = '\u{1160}';

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Unit {
    pub slot: Slot,
    pub ch: char,
}

/// 키 조건. 날개셋 조건식의 원자 하나에 대응한다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "if", rename_all = "snake_case")]
pub enum Cond {
    /// `T==n`: 기호 확장 단계. 0 = 평소, 100 = 기호 배열 준비(Shift+J 뒤), 101~103 = 1~3단계.
    Stage { is: u32 },
    /// `P`: Caps Lock (세모이에서는 Shift Lock 역할)
    CapsLock,
    /// `N&2`: Scroll Lock. 원본에서 옛한글·특수기호 확장 모드. macOS에서는 메뉴로 대신한다.
    ScrollLock,
    /// `E`, `F`: 날개셋 내부 플래그. `!E&&!F` 꼴로만 쓰인다.
    Flag { name: char, value: bool },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "do", rename_all = "snake_case")]
pub enum KeyAction {
    /// 아무것도 하지 않는다 (`0`).
    Nothing,
    /// 문자를 낸다.
    Char { ch: char },
    /// 낱자를 조합에 넣는다.
    Unit { unit: Unit },
    /// 날개셋 명령 `C<group>|<n>`. 세모이에서는 `C1|100~103`(기호 확장 단계 지정)만 쓰인다.
    Command { group: u32, n: u32 },
}

/// 위에서부터 차례로 보고 `when`이 모두 맞는 첫 규칙을 쓴다. 마지막 규칙은 조건이 없다.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyRule {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub when: Vec<Cond>,
    #[serde(flatten)]
    pub action: KeyAction,
    /// 동작과 함께 기호 확장 단계를 바꾼다 (`(T=n, …)`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_stage: Option<u32>,
}

/// 날개셋 `KeyTable` 한 항목. `ch`는 QWERTY에서 그 키(와 Shift)로 나오는 ASCII 문자다.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyEntry {
    pub ch: char,
    pub rules: Vec<KeyRule>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualUnit {
    pub unit: Unit,
    /// 단독으로 확정될 때 보이는 낱자.
    pub shows_as: char,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layout {
    pub keys: Vec<KeyEntry>,
    pub virtual_units: Vec<VirtualUnit>,
}

/// 같은 벌의 낱자 `a` 뒤에 `b`가 오면 `to`가 된다. 원본처럼 두 순서가 각각 들어 있다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MixRule {
    pub slot: Slot,
    pub a: char,
    pub b: char,
    pub to: char,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AbbrevSource {
    /// 원본 XML `HanSubstTable`
    Original,
    /// `data/src/abbreviations-extra.json` (원본에 없는 추가 약어)
    Extra,
}

/// 조합 중인 음절(첫가끝 자모 문자열)이 `from`이면 `to`로 바꿔 보이고 확정한다.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Abbreviation {
    pub from: String,
    pub to: String,
    pub source: AbbrevSource,
}

/// 기호 연타 조합(날개셋 `UserCompoTable`). 상태 `state`에서 문자 `key`가 들어오면,
/// `next`가 있으면 그 상태로 가며 `interim`을 보이고, 없으면 `result`로 확정한다.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComposeRule {
    pub state: u32,
    pub key: char,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interim: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

/// 후보 변환(날개셋 `UserCandiTable`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateEntry {
    pub key: String,
    pub candidates: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // 생성물(`data/*.json`)이 이 타입들로 읽히는지 확인한다. 엔진에 내장하는 일은 Phase 2에서 한다.
    #[test]
    fn generated_data_deserializes() {
        #[derive(Deserialize)]
        struct LayoutFile {
            keys: Vec<KeyEntry>,
            virtual_units: Vec<VirtualUnit>,
        }
        let layout: LayoutFile =
            serde_json::from_str(include_str!("../../../data/layout.json")).unwrap();
        assert_eq!(layout.keys.len(), 94);
        assert_eq!(
            layout.virtual_units[0].unit.ch,
            virtual_unit_char(500).unwrap()
        );
        assert_eq!(layout.virtual_units[0].shows_as, 'ᅩ');

        let mix: Vec<MixRule> =
            serde_json::from_str(include_str!("../../../data/unit-mix.json")).unwrap();
        assert!(mix.contains(&MixRule {
            slot: Slot::Jung,
            a: 'ᅬ',
            b: 'ᅡ',
            to: 'ᅫ'
        }));

        let abbr: Vec<Abbreviation> =
            serde_json::from_str(include_str!("../../../data/abbreviations.json")).unwrap();
        assert!(abbr.iter().any(|a| a.from == "ᄀᅠᆨ" && a.to == "국가"));
        assert!(
            abbr.iter()
                .any(|a| a.from == "ᄈᅠᆷ" && a.to == "뺨" && a.source == AbbrevSource::Extra)
        );

        let compose: Vec<ComposeRule> =
            serde_json::from_str(include_str!("../../../data/compose.json")).unwrap();
        assert_eq!(compose.len(), 178);
        let candidates: Vec<CandidateEntry> =
            serde_json::from_str(include_str!("../../../data/candidates.json")).unwrap();
        assert_eq!(candidates[0].candidates, ["가가호호"]);
    }
}
