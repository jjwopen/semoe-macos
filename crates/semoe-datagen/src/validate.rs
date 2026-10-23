//! 변환 결과 검증과 추가 약어 처리.

use std::collections::{BTreeMap, BTreeSet};

use semoe_core::data::{
    AbbrevSource, Abbreviation, JUNG_FILLER, KeyAction, Layout, MixRule, Slot, VirtualUnit,
    virtual_unit_char,
};
use serde_json::Value;

use crate::convert::Converted;
use crate::units::{SLOTS, UnitNames, in_block, slot_key};

/// 키에서 나오는 모든 낱자(모든 조건·단계 포함)에서 출발해 결합 규칙을 반복 적용했을 때
/// 벌마다 만들 수 있는 낱자 집합.
pub fn reachable(layout: &Layout, mix: &[MixRule]) -> BTreeMap<Slot, BTreeSet<char>> {
    let mut reach: BTreeMap<Slot, BTreeSet<char>> =
        SLOTS.iter().map(|&s| (s, BTreeSet::new())).collect();
    for k in &layout.keys {
        for r in &k.rules {
            if let KeyAction::Unit { unit } = r.action {
                reach.get_mut(&unit.slot).unwrap().insert(unit.ch);
            }
        }
    }
    loop {
        let mut changed = false;
        for m in mix {
            let set = reach.get_mut(&m.slot).unwrap();
            if set.contains(&m.a) && set.contains(&m.b) && set.insert(m.to) {
                changed = true;
            }
        }
        if !changed {
            return reach;
        }
    }
}

/// 같은 벌의 두 낱자를 결합한다. 원본은 대부분 두 순서를 모두 적어 두었지만 한 방향만 적힌 규칙도
/// 있다(예: ㅚ+ㅔ). 모아치기는 누르는 순서와 무관해야 하므로 적힌 순서를 먼저, 없으면 반대 순서를 본다.
pub fn mix_pair(mix: &[MixRule], slot: Slot, a: char, b: char) -> Option<char> {
    let find = |x: char, y: char| {
        mix.iter()
            .find(|m| m.slot == slot && m.a == x && m.b == y)
            .map(|m| m.to)
    };
    find(a, b).or_else(|| find(b, a))
}

/// 가상 낱자를 단독으로 보일 때의 낱자로 바꾼다.
fn shown(c: char, virtuals: &[VirtualUnit]) -> char {
    virtuals
        .iter()
        .find(|v| v.unit.ch == c)
        .map_or(c, |v| v.shows_as)
}

/// 현대 한글 완성형으로 합쳐지는 첫가끝 조합인가 (약어 키로 쓰면 일반 입력을 가린다).
fn is_modern_syllable(chars: &[char]) -> bool {
    let cp = |i: usize| chars.get(i).map(|&c| c as u32);
    let cho = cp(0).is_some_and(|c| (0x1100..=0x1112).contains(&c));
    let jung = cp(1).is_some_and(|c| (0x1161..=0x1175).contains(&c));
    let jong = cp(2).is_none_or(|c| (0x11A8..=0x11C2).contains(&c));
    cho && jung && jong && chars.len() <= 3
}

pub struct Report {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn check(c: &Converted, names: &UnitNames) -> Report {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // 자판: ASCII 0x21~0x7E가 빠짐없이 한 번씩
    let chars: Vec<u32> = c.layout.keys.iter().map(|k| k.ch as u32).collect();
    if chars != (0x21..=0x7E).collect::<Vec<_>>() {
        errors.push("KeyTable이 0x21~0x7E를 차례로 한 번씩 덮지 않음".into());
    }

    // 결합 규칙: 같은 (벌, a, b)가 두 번 나오지 않음
    let mut pairs = BTreeMap::new();
    for m in &c.mix {
        if let Some(prev) = pairs.insert((m.slot, m.a, m.b), m.to)
            && prev != m.to
        {
            errors.push(format!(
                "결합 규칙 충돌: {:?} {}+{} → {} / {}",
                m.slot, m.a, m.b, prev, m.to
            ));
        }
    }

    // 두 순서의 결과가 다른 규칙: 엔진은 적힌 순서를 먼저 보므로 누르는 순서에 따라 결과가 달라질 수 있다.
    for m in &c.mix {
        if let Some(&rev) = pairs.get(&(m.slot, m.b, m.a))
            && rev != m.to
            && m.a < m.b
        {
            warnings.push(format!(
                "결합 규칙이 순서에 따라 다름: {:?} {}+{} → {}, {}+{} → {}",
                m.slot, m.a, m.b, m.to, m.b, m.a, rev
            ));
        }
    }

    // 쓰이지 않는 이름
    let used: BTreeSet<(Slot, char)> = c
        .mix
        .iter()
        .flat_map(|m| [(m.slot, m.a), (m.slot, m.b), (m.slot, m.to)])
        .chain(c.layout.keys.iter().flat_map(|k| {
            k.rules.iter().filter_map(|r| match r.action {
                KeyAction::Unit { unit } => Some((unit.slot, unit.ch)),
                _ => None,
            })
        }))
        .collect();
    for (&slot, map) in &names.by_slot {
        for (name, &ch) in map {
            if !used.contains(&(slot, ch)) {
                warnings.push(format!(
                    "unit-names.json {}:{name}가 어디에도 쓰이지 않음",
                    slot_key(slot)
                ));
            }
        }
    }

    // 약어
    let reach = reachable(&c.layout, &c.mix);
    let image = |slot: Slot| -> BTreeSet<char> {
        reach[&slot]
            .iter()
            .map(|&ch| shown(ch, &c.layout.virtual_units))
            .collect()
    };
    let (cho, jung, jong) = (image(Slot::Cho), image(Slot::Jung), image(Slot::Jong));
    let mut seen = BTreeMap::new();
    for a in &c.abbreviations {
        let cs: Vec<char> = a.from.chars().collect();
        let ok_shape = (2..=3).contains(&cs.len())
            && in_block(Slot::Cho, cs[0])
            && (cs[1] == JUNG_FILLER || in_block(Slot::Jung, cs[1]))
            && cs.get(2).is_none_or(|&j| in_block(Slot::Jong, j));
        if !ok_shape {
            errors.push(format!(
                "약어 {:?} → {:?}: 초성+중성(채움)+종성? 꼴이 아님",
                a.from, a.to
            ));
            continue;
        }
        let unreachable = !cho.contains(&cs[0])
            || (cs[1] != JUNG_FILLER && !jung.contains(&cs[1]))
            || cs.get(2).is_some_and(|j| !jong.contains(j));
        if unreachable {
            errors.push(format!(
                "약어 {:?} → {:?}: 키와 결합 규칙으로 만들 수 없음",
                a.from, a.to
            ));
        }
        if is_modern_syllable(&cs) {
            errors.push(format!(
                "약어 {:?} → {:?}: 현대 한글 음절이라 일반 입력을 가림",
                a.from, a.to
            ));
        }
        if let Some(prev) = seen.insert(a.from.clone(), a) {
            errors.push(format!(
                "약어 키 중복 {:?}: {:?}({:?}) / {:?}({:?})",
                a.from, prev.to, prev.source, a.to, a.source
            ));
        }
    }

    // 기호 조합: 가리키는 상태가 실제로 있음
    let states: BTreeSet<u32> = c.compose.iter().map(|r| r.state).collect();
    for r in &c.compose {
        if r.next.is_some_and(|n| !states.contains(&n)) {
            errors.push(format!(
                "기호 조합 상태 {}에서 {:?} → 없는 상태 {:?}",
                r.state, r.key, r.next
            ));
        }
        if r.next.is_none() == r.result.is_none() {
            errors.push(format!(
                "기호 조합 상태 {} {:?}: newstate와 result 중 하나만 있어야 함",
                r.state, r.key
            ));
        }
    }

    Report { errors, warnings }
}

/// QWERTY 아랫글쇠(조건 없는 규칙)로 `keys`를 한 번에 눌렀을 때 만들어지는 음절. 모든 순서에서
/// 같은 결과가 나와야 한다.
pub fn chord_syllable(layout: &Layout, mix: &[MixRule], keys: &str) -> Result<String, String> {
    let units: Vec<_> = keys
        .chars()
        .map(|k| {
            let entry = layout
                .keys
                .iter()
                .find(|e| e.ch == k)
                .ok_or(format!("{k:?} 키가 없음"))?;
            match entry
                .rules
                .iter()
                .find(|r| r.when.is_empty())
                .map(|r| &r.action)
            {
                Some(KeyAction::Unit { unit }) => Ok(*unit),
                other => Err(format!("{k:?} 키의 아랫글쇠가 낱자가 아님: {other:?}")),
            }
        })
        .collect::<Result<_, String>>()?;

    let fold = |order: &[usize]| -> Result<String, String> {
        let mut slots: BTreeMap<Slot, char> = BTreeMap::new();
        for &i in order {
            let u = units[i];
            let next = match slots.get(&u.slot) {
                None => u.ch,
                Some(&prev) => mix_pair(mix, u.slot, prev, u.ch)
                    .ok_or(format!("{} + {} 결합 규칙 없음", prev, u.ch))?,
            };
            slots.insert(u.slot, next);
        }
        let cho = slots.get(&Slot::Cho).ok_or("초성이 없음")?;
        let jung = slots
            .get(&Slot::Jung)
            .map_or(JUNG_FILLER, |&c| shown(c, &layout.virtual_units));
        Ok([Some(*cho), Some(jung), slots.get(&Slot::Jong).copied()]
            .into_iter()
            .flatten()
            .collect())
    };

    let mut results = BTreeSet::new();
    for order in permutations(units.len()) {
        results.insert(fold(&order).map_err(|e| format!("{keys:?} 순서 {order:?}: {e}"))?);
    }
    match results.len() {
        1 => Ok(results.pop_first().unwrap()),
        _ => Err(format!(
            "{keys:?}: 누르는 순서에 따라 결과가 다름 {results:?}"
        )),
    }
}

fn permutations(n: usize) -> Vec<Vec<usize>> {
    if n == 0 {
        return vec![vec![]];
    }
    let mut out = Vec::new();
    for p in permutations(n - 1) {
        for i in 0..=p.len() {
            let mut q = p.clone();
            q.insert(i, n - 1);
            out.push(q);
        }
    }
    out
}

/// `data/src/abbreviations-extra.json`: `[{ "keys": "ojz", "to": "뺨", "note": "…" }]`
pub fn extra_abbreviations(
    text: &str,
    layout: &Layout,
    mix: &[MixRule],
) -> Result<Vec<Abbreviation>, String> {
    let v: Value =
        serde_json::from_str(text).map_err(|e| format!("abbreviations-extra.json: {e}"))?;
    let arr = v
        .as_array()
        .ok_or("abbreviations-extra.json: 배열이 아님")?;
    arr.iter()
        .map(|e| {
            let keys = e["keys"].as_str().ok_or("keys 없음")?;
            let to = e["to"].as_str().ok_or("to 없음")?;
            let from = chord_syllable(layout, mix, keys)?;
            if let Some(expected) = e["from"].as_str()
                && expected != from
            {
                return Err(format!(
                    "{keys:?}: from {expected:?}라고 적혀 있지만 실제로는 {from:?}"
                ));
            }
            Ok(Abbreviation {
                from,
                to: to.to_string(),
                source: AbbrevSource::Extra,
            })
        })
        .collect()
}

/// 가상 낱자가 데이터 문자로 맞게 바뀌었는지 (모음시프트 = 500).
pub fn vowel_shift(layout: &Layout) -> Option<char> {
    layout
        .virtual_units
        .iter()
        .map(|v| v.unit.ch)
        .find(|&c| Some(c) == virtual_unit_char(500))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permutations_are_complete() {
        assert_eq!(permutations(3).len(), 6);
        assert_eq!(
            permutations(3).into_iter().collect::<BTreeSet<_>>().len(),
            6
        );
    }

    #[test]
    fn modern_syllable_detection() {
        assert!(is_modern_syllable(&['ᄀ', 'ᅡ', 'ᆫ']));
        assert!(is_modern_syllable(&['ᄀ', 'ᅡ']));
        assert!(!is_modern_syllable(&['ᄀ', JUNG_FILLER, 'ᆨ']));
        assert!(!is_modern_syllable(&['ᅙ', 'ᅳ']));
    }
}
