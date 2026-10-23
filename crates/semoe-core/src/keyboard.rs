//! 내장 자판 데이터(`data/*.json`)를 조회하기 쉬운 꼴로 들고 있는다.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::data::{
    Abbreviation, ComposeRule, Cond, KeyAction, KeyEntry, KeyRule, MixRule, Slot, VirtualUnit,
};

#[derive(Debug)]
pub struct Keyboard {
    keys: HashMap<char, Vec<KeyRule>>,
    mix: HashMap<(Slot, char, char), char>,
    /// 가상 낱자 → 단독으로 보일 낱자
    shows_as: HashMap<char, char>,
    abbreviations: HashMap<String, String>,
    /// 기호 조합 (상태, 문자) → 규칙
    compose: HashMap<(u32, char), ComposeRule>,
}

/// 키 규칙의 조건을 판단할 상태.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyState {
    /// 기호 확장 단계 `T` (0 = 평소)
    pub stage: u32,
    pub caps_lock: bool,
    pub scroll_lock: bool,
}

impl KeyState {
    fn holds(&self, c: &Cond) -> bool {
        match *c {
            Cond::Stage { is } => self.stage == is,
            Cond::CapsLock => self.caps_lock,
            Cond::ScrollLock => self.scroll_lock,
            // 날개셋 내부 플래그 E·F는 기호 확장 단계 명령(`T==100&&!E&&!F`)에만 쓰인다.
            // 뜻을 확인하지 못했으므로 꺼진 것으로 본다(단계 명령이 늘 동작한다).
            Cond::Flag { value, .. } => !value,
        }
    }
}

#[derive(Deserialize)]
struct LayoutFile {
    keys: Vec<KeyEntry>,
    virtual_units: Vec<VirtualUnit>,
}

impl Keyboard {
    /// 바이너리에 내장한 세모이 데이터. 처음 부를 때 한 번 파싱한다.
    pub fn builtin() -> &'static Keyboard {
        static KB: OnceLock<Keyboard> = OnceLock::new();
        KB.get_or_init(|| {
            Keyboard::from_json(
                include_str!("../../../data/layout.json"),
                include_str!("../../../data/unit-mix.json"),
                include_str!("../../../data/abbreviations.json"),
                include_str!("../../../data/compose.json"),
            )
            .expect("내장 자판 데이터가 깨짐")
        })
    }

    pub fn from_json(
        layout: &str,
        mix: &str,
        abbreviations: &str,
        compose: &str,
    ) -> Result<Keyboard, serde_json::Error> {
        let layout: LayoutFile = serde_json::from_str(layout)?;
        let mix: Vec<MixRule> = serde_json::from_str(mix)?;
        let abbreviations: Vec<Abbreviation> = serde_json::from_str(abbreviations)?;
        let compose: Vec<ComposeRule> = serde_json::from_str(compose)?;
        Ok(Keyboard {
            keys: layout.keys.into_iter().map(|k| (k.ch, k.rules)).collect(),
            mix: mix
                .into_iter()
                .map(|m| ((m.slot, m.a, m.b), m.to))
                .collect(),
            shows_as: layout
                .virtual_units
                .iter()
                .map(|v| (v.unit.ch, v.shows_as))
                .collect(),
            abbreviations: abbreviations.into_iter().map(|a| (a.from, a.to)).collect(),
            compose: compose.into_iter().map(|c| ((c.state, c.key), c)).collect(),
        })
    }

    /// QWERTY 문자 `ch`(Shift 반영)에 해당하는 키의 동작과, 함께 바꿀 기호 확장 단계.
    pub fn action(&self, ch: char, state: &KeyState) -> Option<(&KeyAction, Option<u32>)> {
        self.keys
            .get(&ch)?
            .iter()
            .find(|r| r.when.iter().all(|c| state.holds(c)))
            .map(|r| (&r.action, r.set_stage))
    }

    /// 같은 벌의 두 낱자를 결합한다. 원본에는 한 방향만 적힌 규칙이 있어(Phase 1),
    /// 적힌 순서를 먼저 보고 없으면 반대 순서를 본다.
    pub fn mix(&self, slot: Slot, a: char, b: char) -> Option<char> {
        self.mix
            .get(&(slot, a, b))
            .or_else(|| self.mix.get(&(slot, b, a)))
            .copied()
    }

    /// 가상 낱자(모음시프트 등)를 실제 낱자로. 가상 낱자가 아니면 그대로.
    pub fn shown(&self, c: char) -> char {
        self.shows_as.get(&c).copied().unwrap_or(c)
    }

    /// 기호 조합 상태 `state`에서 문자 `ch`가 들어왔을 때의 규칙.
    pub fn compose(&self, state: u32, ch: char) -> Option<&ComposeRule> {
        self.compose.get(&(state, ch))
    }

    /// 첫가끝 문자열(약어 키 꼴)에 해당하는 약어.
    pub fn abbreviation(&self, key: &str) -> Option<&str> {
        self.abbreviations.get(key).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Unit;

    #[test]
    fn builtin_data_lookups() {
        let kb = Keyboard::builtin();
        let base = KeyState::default();
        assert_eq!(
            kb.action('k', &base),
            Some((
                &KeyAction::Unit {
                    unit: Unit {
                        slot: Slot::Cho,
                        ch: 'ᄀ'
                    }
                },
                None
            ))
        );
        assert_eq!(
            kb.action('p', &base),
            Some((&KeyAction::Char { ch: '.' }, None))
        );
        assert_eq!(
            kb.action('P', &base),
            Some((&KeyAction::Char { ch: ';' }, None))
        );
        // 한 방향만 적힌 규칙도 양쪽 순서로 결합된다.
        assert_eq!(kb.mix(Slot::Jung, 'ᅦ', 'ᅬ'), Some('ퟀ'));
        assert_eq!(kb.mix(Slot::Jung, 'ᅬ', 'ᅦ'), Some('ퟀ'));
        assert_eq!(kb.abbreviation("ᄀᅠᆨ"), Some("국가"));
        assert_eq!(kb.abbreviation("ᄈᅠᆷ"), Some("뺨"));
    }

    #[test]
    fn caps_lock_selects_shifted_symbol() {
        let kb = Keyboard::builtin();
        let caps = KeyState {
            caps_lock: true,
            ..Default::default()
        };
        assert_eq!(
            kb.action('a', &caps),
            Some((&KeyAction::Char { ch: '←' }, None))
        );
    }
}
