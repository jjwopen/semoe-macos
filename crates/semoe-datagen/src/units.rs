//! 날개셋 낱자 이름 → 유니코드 첫가끝 자모 (`data/src/unit-names.json`).

use std::collections::{BTreeMap, BTreeSet};

use semoe_core::data::{Slot, Unit, virtual_unit_char};
use serde_json::Value;

#[derive(Debug, Default)]
pub struct UnitNames {
    pub by_slot: BTreeMap<Slot, BTreeMap<String, char>>,
    /// 근거가 부족해 추정으로 둔 이름(치두음·정치음 등).
    pub unverified: BTreeSet<String>,
    /// 가상 낱자 id → 슬롯 (`VirtualUnitTable`)
    pub virtuals: BTreeMap<u32, Slot>,
}

pub const SLOTS: [Slot; 3] = [Slot::Cho, Slot::Jung, Slot::Jong];

pub fn slot_key(slot: Slot) -> &'static str {
    match slot {
        Slot::Cho => "cho",
        Slot::Jung => "jung",
        Slot::Jong => "jong",
    }
}

/// 날개셋 `UnitMix`의 `unit` 속성 값
pub fn slot_from_xml(s: &str) -> Result<Slot, String> {
    match s {
        "CHO" => Ok(Slot::Cho),
        "JUNG" => Ok(Slot::Jung),
        "JONG" => Ok(Slot::Jong),
        _ => Err(format!("알 수 없는 벌 {s:?}")),
    }
}

/// 문자가 그 벌의 첫가끝 자모 영역(채움 문자 제외)에 있는가.
pub fn in_block(slot: Slot, c: char) -> bool {
    let cp = c as u32;
    match slot {
        Slot::Cho => (0x1100..=0x115E).contains(&cp) || (0xA960..=0xA97C).contains(&cp),
        Slot::Jung => (0x1161..=0x11A7).contains(&cp) || (0xD7B0..=0xD7C6).contains(&cp),
        Slot::Jong => (0x11A8..=0x11FF).contains(&cp) || (0xD7CB..=0xD7FB).contains(&cp),
    }
}

impl UnitNames {
    pub fn from_json(text: &str) -> Result<Self, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| format!("unit-names.json: {e}"))?;
        let mut names = UnitNames::default();
        for slot in SLOTS {
            let obj = v[slot_key(slot)]
                .as_object()
                .ok_or(format!("unit-names.json에 {} 없음", slot_key(slot)))?;
            let mut map = BTreeMap::new();
            let mut seen = BTreeMap::new();
            for (name, ch) in obj {
                let s = ch.as_str().ok_or(format!("{name}: 문자열이 아님"))?;
                let mut it = s.chars();
                let (Some(c), None) = (it.next(), it.next()) else {
                    return Err(format!("{}:{name} = {s:?}: 한 글자가 아님", slot_key(slot)));
                };
                if !in_block(slot, c) {
                    return Err(format!(
                        "{}:{name} = U+{:04X}: {} 자모 영역이 아님",
                        slot_key(slot),
                        c as u32,
                        slot_key(slot)
                    ));
                }
                if let Some(other) = seen.insert(c, name.clone()) {
                    return Err(format!(
                        "{}: {other}와 {name}이 같은 문자 U+{:04X}",
                        slot_key(slot),
                        c as u32
                    ));
                }
                map.insert(name.clone(), c);
            }
            names.by_slot.insert(slot, map);
        }
        if let Some(arr) = v["unverified"].as_array() {
            names.unverified = arr
                .iter()
                .filter_map(|x| x.as_str().map(str::to_owned))
                .collect();
        }
        Ok(names)
    }

    /// `UnitMix`·`VirtualUnit`에 쓰인 이름 (벌을 알고 있음). 숫자 이름은 가상 낱자다.
    pub fn resolve_in(&self, slot: Slot, name: &str) -> Result<char, String> {
        if let Ok(id) = name.parse::<u32>() {
            return match self.virtuals.get(&id) {
                Some(&s) if s == slot => {
                    virtual_unit_char(id).ok_or(format!("가상 낱자 id {id}가 너무 큼"))
                }
                _ => Err(format!("{} 벌에 가상 낱자 {id}가 없음", slot_key(slot))),
            };
        }
        self.by_slot[&slot].get(name).copied().ok_or_else(|| {
            format!(
                "{} 낱자 이름 {name:?}를 unit-names.json에서 찾지 못함",
                slot_key(slot)
            )
        })
    }

    /// `KeyTable`의 `H<d>|` 뒤에 쓰인 이름. 종성은 앞에 `_`가 붙고(`_Q`, `_SS`), 초성·중성은
    /// 이름이 겹치지 않는다. 숫자는 `가상 낱자 id << 16` 꼴이다(`0x1F40000` = 500).
    pub fn resolve_key(&self, name: &str) -> Result<Unit, String> {
        if let Some(hex) = name.strip_prefix("0x") {
            let n = u32::from_str_radix(hex, 16).map_err(|e| format!("{name}: {e}"))?;
            let id = n >> 16;
            if n & 0xFFFF != 0 {
                return Err(format!("{name}: 가상 낱자 꼴(id << 16)이 아님"));
            }
            let slot = *self
                .virtuals
                .get(&id)
                .ok_or(format!("{name}: 가상 낱자 {id}가 없음"))?;
            return Ok(Unit {
                slot,
                ch: self.resolve_in(slot, &id.to_string())?,
            });
        }
        if let Some(rest) = name.strip_prefix('_') {
            let canonical = if rest.len() == 1 {
                format!("{rest}_")
            } else {
                rest.to_string()
            };
            return Ok(Unit {
                slot: Slot::Jong,
                ch: self.resolve_in(Slot::Jong, &canonical)?,
            });
        }
        let cho = self.by_slot[&Slot::Cho].get(name);
        let jung = self.by_slot[&Slot::Jung].get(name);
        match (cho, jung) {
            (Some(&ch), None) => Ok(Unit {
                slot: Slot::Cho,
                ch,
            }),
            (None, Some(&ch)) => Ok(Unit {
                slot: Slot::Jung,
                ch,
            }),
            (Some(_), Some(_)) => Err(format!("{name}: 초성·중성 어느 쪽인지 모호함")),
            (None, None) => Err(format!(
                "키 낱자 이름 {name:?}를 unit-names.json에서 찾지 못함"
            )),
        }
    }
}
