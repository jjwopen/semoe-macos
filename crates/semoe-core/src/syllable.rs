//! 조합 중인 음절: 초·중·종 슬롯과 낱자 단위 되돌리기.
//!
//! 슬롯마다 들어온 낱자를 순서대로 모아 두고, 결합 결과는 그때그때 계산한다. 결합 규칙은 둘씩 합치는
//! 꼴이라 셋 이상이면 합치는 순서에 따라 성공 여부가 갈린다(예: 모음시프트+ㅣ+ㅓ는 ㅓ+모음시프트=ㅝ,
//! ㅝ+ㅣ=ㅞ 순서로만 된다). 모아치기는 누르는 순서와 무관해야 하므로, 들어온 순서로 먼저 합쳐 보고
//! 안 되면 다른 순서를 찾는다. 어떤 순서로도 다 합쳐지지 않을 때만 결합 실패로 본다.
//!
//! 순서에 따라 서로 다른 결과로 합쳐지는 묶음도 있다(종성 ㄹ+ㄴ+ㅆ → ㄵ 또는 ㅀ 등, Phase 2에서 4묶음 확인).
//! 한 번에 모아치기는 누르는 순서가 손가락 타이밍의 우연이므로 [`Syllable::whole`]은 낱자를 코드 순으로
//! 정렬해 합쳐 같은 chord가 언제나 같은 결과를 내게 한다. 이어치기([`Syllable::add`])는 누른 순서를 따른다.

use crate::data::{Slot, Unit};
use crate::hangul;
use crate::keyboard::Keyboard;

/// 한 슬롯에 모을 수 있는 낱자 수의 상한. 순서 탐색이 n!이라 막아 둔다(실제 세모이 조합은 3개 이하).
const MAX_UNITS: usize = 6;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Cell {
    units: Vec<char>,
    value: Option<char>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Syllable {
    cells: [Cell; 3],
    /// 낱자를 넣은 슬롯의 순서. Backspace로 한 낱자씩 되돌린다 (날개셋 `ByUnitStep`).
    history: Vec<Slot>,
}

fn index(slot: Slot) -> usize {
    match slot {
        Slot::Cho => 0,
        Slot::Jung => 1,
        Slot::Jong => 2,
    }
}

/// 낱자들을 결합 규칙으로 하나로 합친다. 들어온 순서를 먼저 시도하고, 안 되면 다른 순서를 찾는다.
fn combine(kb: &Keyboard, slot: Slot, units: &[char]) -> Option<char> {
    fn fold(kb: &Keyboard, slot: Slot, units: &[char], order: &[usize]) -> Option<char> {
        let mut it = order.iter().map(|&i| units[i]);
        let first = it.next()?;
        it.try_fold(first, |acc, c| kb.mix(slot, acc, c))
    }
    fn search(
        kb: &Keyboard,
        slot: Slot,
        units: &[char],
        order: &mut Vec<usize>,
        used: &mut [bool],
    ) -> Option<char> {
        if order.len() == units.len() {
            return fold(kb, slot, units, order);
        }
        for i in 0..units.len() {
            if !used[i] {
                used[i] = true;
                order.push(i);
                let r = search(kb, slot, units, order, used);
                order.pop();
                used[i] = false;
                if r.is_some() {
                    return r;
                }
            }
        }
        None
    }
    // search는 사전순으로 순서를 보므로 첫 후보가 들어온 순서(0, 1, 2, …)다.
    search(
        kb,
        slot,
        units,
        &mut Vec::with_capacity(units.len()),
        &mut vec![false; units.len()],
    )
}

impl Syllable {
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    pub fn has(&self, slot: Slot) -> bool {
        self.cells[index(slot)].value.is_some()
    }

    /// 낱자를 넣는다. 그 슬롯의 낱자들이 어떤 순서로도 하나로 합쳐지지 않으면 `false`를 돌려주고
    /// 아무것도 바꾸지 않는다(호출자가 현재 음절을 확정하고 새 음절에 넣는다).
    pub fn add(&mut self, kb: &Keyboard, unit: Unit) -> bool {
        let cell = &mut self.cells[index(unit.slot)];
        if cell.units.len() >= MAX_UNITS {
            return false;
        }
        cell.units.push(unit.ch);
        match combine(kb, unit.slot, &cell.units) {
            Some(v) => {
                cell.value = Some(v);
                self.history.push(unit.slot);
                true
            }
            None => {
                cell.units.pop();
                false
            }
        }
    }

    /// 낱자들을 모두 한 음절에 넣었을 때 슬롯마다 하나로 합쳐지면 그 음절. 중간 상태는 따지지 않는다
    /// (예: ㅣ, ㅓ는 둘만으로는 안 합쳐지지만 모음시프트까지 들어오면 ㅞ가 된다).
    pub fn whole(kb: &Keyboard, units: &[Unit]) -> Option<Syllable> {
        let mut s = Syllable::default();
        for u in units {
            let cell = &mut s.cells[index(u.slot)];
            if cell.units.len() >= MAX_UNITS {
                return None;
            }
            cell.units.push(u.ch);
            s.history.push(u.slot);
        }
        for slot in [Slot::Cho, Slot::Jung, Slot::Jong] {
            let cell = &mut s.cells[index(slot)];
            if !cell.units.is_empty() {
                let mut sorted = cell.units.clone();
                sorted.sort_unstable();
                cell.value = Some(combine(kb, slot, &sorted)?);
            }
        }
        Some(s)
    }

    /// 한 번에 모아치기로 함께 들어온 낱자들을 음절로 나눈다. 모두 한 음절로 합쳐지면 한 음절,
    /// 아니면 들어온 순서대로 넣다가 합쳐지지 않는 낱자에서 새 음절을 연다.
    pub fn segments(kb: &Keyboard, units: &[Unit]) -> Vec<Syllable> {
        if units.is_empty() {
            return Vec::new();
        }
        if let Some(s) = Self::whole(kb, units) {
            return vec![s];
        }
        let mut out = vec![Syllable::default()];
        for &u in units {
            if !out.last_mut().unwrap().add(kb, u) {
                let mut s = Syllable::default();
                s.add(kb, u);
                out.push(s);
            }
        }
        out
    }

    /// 마지막 낱자를 되돌린다. 되돌릴 것이 없으면 `false`.
    pub fn undo(&mut self, kb: &Keyboard) -> bool {
        let Some(slot) = self.history.pop() else {
            return false;
        };
        let cell = &mut self.cells[index(slot)];
        cell.units.pop();
        // 남은 낱자들은 넣을 때 이미 합쳐졌으므로 다시 합쳐진다.
        cell.value = combine(kb, slot, &cell.units);
        true
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    fn shown(&self, kb: &Keyboard) -> (Option<char>, Option<char>, Option<char>) {
        let [l, v, t] = &self.cells;
        (l.value, v.value.map(|c| kb.shown(c)), t.value)
    }

    /// 약어 사전을 찾을 때 쓰는 첫가끝 문자열 (초성 + 중성 또는 채움 + 종성?).
    pub fn key(&self, kb: &Keyboard) -> String {
        let (l, v, t) = self.shown(kb);
        hangul::conjoining(l, v, t)
    }

    /// 화면에 보일 문자열. `abbreviations`가 켜져 있고 사전에 있으면 약어로 바꿔 보인다.
    ///
    /// `old_hangul`이 꺼져 있으면 옛한글 자모가 남은 음절(약어가 아닌 `ㅁ`+`ㄷ` → `ꥰ` 등)은 낱자로 나눠
    /// 보인다(`ㅁㄷ`). 옛한글 겹자모는 약어 키로 많이 쓰여 결합 자체는 막지 않는다. 그대로 내면 채움 문자와
    /// 함께 첫가끝 꼴이 되어, 옛한글을 그리지 못하는 앱에서는 보이지 않거나 `?`가 된다.
    pub fn render(&self, kb: &Keyboard, abbreviations: bool, old_hangul: bool) -> String {
        if self.is_empty() {
            return String::new();
        }
        if let Some(a) = self.abbreviation(kb, abbreviations) {
            return a.to_string();
        }
        let (l, v, t) = self.shown(kb);
        if !old_hangul && !hangul::is_modern(l, v, t) && self.history.len() > 1 {
            return self.split(kb, abbreviations);
        }
        hangul::display(l, v, t)
    }

    fn abbreviation<'a>(&self, kb: &'a Keyboard, abbreviations: bool) -> Option<&'a str> {
        (abbreviations && self.has(Slot::Cho))
            .then(|| kb.abbreviation(&self.key(kb)))
            .flatten()
    }

    /// 현대 한글로 보이거나 약어인가
    fn plain(&self, kb: &Keyboard, abbreviations: bool) -> bool {
        let (l, v, t) = self.shown(kb);
        hangul::is_modern(l, v, t) || self.abbreviation(kb, abbreviations).is_some()
    }

    /// 넣은 순서대로의 낱자
    fn units(&self) -> Vec<Unit> {
        let mut next = [0; 3];
        self.history
            .iter()
            .map(|&slot| {
                let i = index(slot);
                next[i] += 1;
                Unit {
                    slot,
                    ch: self.cells[i].units[next[i] - 1],
                }
            })
            .collect()
    }

    /// 낱자를 넣은 순서대로 다시 넣으며, 옛한글이 되는 낱자에서 새 음절을 연다 (`ㅁ ㄷ ㅏ` → `ㅁ다`).
    fn split(&self, kb: &Keyboard, abbreviations: bool) -> String {
        let mut out = String::new();
        let mut cur = Syllable::default();
        for u in self.units() {
            let mut next = cur.clone();
            if next.add(kb, u) && next.plain(kb, abbreviations) {
                cur = next;
            } else {
                // 낱자 하나뿐인 음절은 더 나누지 않으므로 되부름이 끝난다.
                out += &cur.render(kb, abbreviations, false);
                cur = Syllable::default();
                cur.add(kb, u);
            }
        }
        out + &cur.render(kb, abbreviations, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::virtual_unit_char;

    fn u(slot: Slot, ch: char) -> Unit {
        Unit { slot, ch }
    }

    #[test]
    fn abbreviation_follows_undo() {
        let kb = Keyboard::builtin();
        let mut s = Syllable::default();
        assert!(s.add(kb, u(Slot::Cho, 'ᄀ')));
        assert!(s.add(kb, u(Slot::Jong, 'ᆨ')));
        assert_eq!(s.render(kb, true, false), "국가");
        assert!(s.add(kb, u(Slot::Jong, 'ᆼ')));
        assert_eq!(s.render(kb, true, false), "공격");
        assert!(s.undo(kb));
        assert_eq!(s.render(kb, true, false), "국가");
        assert_eq!(s.render(kb, false, false), "ㄱㄱ");
        assert_eq!(s.key(kb), "ᄀᅠᆨ"); // 약어 사전 키는 그대로 첫가끝 꼴
    }

    #[test]
    fn mix_failure_leaves_syllable_unchanged() {
        let kb = Keyboard::builtin();
        let mut s = Syllable::default();
        s.add(kb, u(Slot::Jung, 'ᅡ'));
        let before = s.clone();
        assert!(!s.add(kb, u(Slot::Jung, 'ᅢ')));
        assert_eq!(s, before);
    }

    #[test]
    fn three_units_combine_in_any_order() {
        // 모음시프트 + ㅣ + ㅓ = ㅞ: ㅓ+모음시프트=ㅝ, ㅝ+ㅣ=ㅞ 순서로만 규칙이 있다.
        let kb = Keyboard::builtin();
        let vs = virtual_unit_char(500).unwrap();
        for order in [[vs, 'ᅵ', 'ᅥ'], ['ᅵ', vs, 'ᅥ'], ['ᅵ', 'ᅥ', vs], ['ᅥ', 'ᅵ', vs]]
        {
            let units: Vec<_> = order.iter().map(|&c| u(Slot::Jung, c)).collect();
            let s = Syllable::whole(kb, &units).unwrap_or_else(|| panic!("{order:?}"));
            assert_eq!(s.render(kb, false, false), "ㅞ", "{order:?}");
        }
        // 한 낱자씩 넣으면 ㅣ, ㅓ에서 막힌다 (이어치기는 이 방식)
        let mut s = Syllable::default();
        assert!(s.add(kb, u(Slot::Jung, 'ᅵ')));
        assert!(!s.add(kb, u(Slot::Jung, 'ᅥ')));
    }
}
