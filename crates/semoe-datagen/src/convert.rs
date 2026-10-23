//! 날개셋 XML 요소 트리 → 세모이 데이터.

use semoe_core::data::{
    AbbrevSource, Abbreviation, CandidateEntry, ComposeRule, KeyEntry, Layout, MixRule, Slot, Unit,
    VirtualUnit,
};

use crate::expr;
use crate::units::{UnitNames, slot_from_xml};
use crate::xml::Element;

/// 두 입력 방식 XML이 똑같이 가지고 있어야 하는 표. 하나로 합쳐 쓴다.
/// (`Extra`의 조합 종료 타이머, `AutomataTable`, `KeyProgramTable`은 방식마다 다르다.)
pub const SHARED_TABLES: [&str; 5] = [
    "KeyTable",
    "UnitMixTable",
    "VirtualUnitTable",
    "HanSubstTable",
    "UserCompoTable",
];

#[derive(Debug)]
pub struct Converted {
    pub layout: Layout,
    pub mix: Vec<MixRule>,
    pub abbreviations: Vec<Abbreviation>,
    pub compose: Vec<ComposeRule>,
    pub candidates: Vec<CandidateEntry>,
}

fn parse_num(s: &str) -> Result<u32, String> {
    match s.strip_prefix("0x") {
        Some(h) => u32::from_str_radix(h, 16),
        None => s.parse(),
    }
    .map_err(|e| format!("숫자 {s:?}: {e}"))
}

fn table<'a>(root: &'a Element, name: &str) -> Result<&'a Element, String> {
    root.find(name).ok_or(format!("<{name}>가 없음"))
}

/// `VirtualUnitTable`을 읽어 `names.virtuals`를 채운다. 다른 표보다 먼저 불러야 한다.
pub fn virtual_units(
    root: &Element,
    names: &mut UnitNames,
) -> Result<Vec<(u32, Slot, String)>, String> {
    let mut out = Vec::new();
    for v in table(root, "VirtualUnitTable")?.children_named("VirtualUnit") {
        let slot = slot_from_xml(v.attr("unit")?)?;
        let id = parse_num(v.attr("from")?)?;
        names.virtuals.insert(id, slot);
        out.push((id, slot, v.attr("to")?.to_string()));
    }
    Ok(out)
}

pub fn convert(
    root: &Element,
    names: &UnitNames,
    virtuals: &[(u32, Slot, String)],
) -> Result<Converted, String> {
    // 자판
    let mut keys = Vec::new();
    for k in table(root, "KeyTable")?.children_named("Key") {
        let at = parse_num(k.attr("at")?)?;
        let ch = char::from_u32(at).ok_or(format!("Key at={at:#x}"))?;
        let rules = expr::key_rules(k.attr("value")?, &|n| names.resolve_key(n))
            .map_err(|e| format!("키 {ch:?}: {e}"))?;
        keys.push(KeyEntry { ch, rules });
    }
    let virtual_units = virtuals
        .iter()
        .map(|(id, slot, to)| {
            Ok(VirtualUnit {
                unit: Unit {
                    slot: *slot,
                    ch: names.resolve_in(*slot, &id.to_string())?,
                },
                shows_as: names.resolve_in(*slot, to)?,
            })
        })
        .collect::<Result<_, String>>()?;

    // 결합 규칙
    let mut mix = Vec::new();
    for m in table(root, "UnitMixTable")?.children_named("UnitMix") {
        let slot = slot_from_xml(m.attr("unit")?)?;
        let r = |a: &str| names.resolve_in(slot, m.attr(a)?);
        mix.push(MixRule {
            slot,
            a: r("a")?,
            b: r("b")?,
            to: r("to")?,
        });
    }

    // 약어
    let abbreviations = table(root, "HanSubstTable")?
        .children_named("HanSubst")
        .map(|h| {
            Ok(Abbreviation {
                from: h.attr("from")?.to_string(),
                to: h.attr("to")?.to_string(),
                source: AbbrevSource::Original,
            })
        })
        .collect::<Result<_, String>>()?;

    // 기호 조합
    let mut compose = Vec::new();
    for c in table(root, "UserCompoTable")?.children_named("UserCompo") {
        let key = parse_num(c.attr("key")?)?;
        let opt = |a: &str| c.attrs.get(a).cloned();
        compose.push(ComposeRule {
            state: parse_num(c.attr("state")?)?,
            key: char::from_u32(key).ok_or(format!("UserCompo key={key:#x}"))?,
            next: opt("newstate").map(|s| parse_num(&s)).transpose()?,
            interim: opt("interim"),
            result: opt("result"),
        });
    }

    // 후보 (ConvertHelper 안의 표. 바깥의 빈 <UserCandiTable/>은 무시)
    let helper = table(root, "ConvertHelper")?;
    let candidates = table(helper, "UserCandiTable")?
        .children_named("UserCandi")
        .map(|u| {
            Ok(CandidateEntry {
                key: u.attr("key")?.to_string(),
                candidates: u
                    .children_named("Candidate")
                    .map(|c| c.attr("data").map(str::to_owned))
                    .collect::<Result<_, _>>()?,
            })
        })
        .collect::<Result<_, String>>()?;

    Ok(Converted {
        layout: Layout {
            keys,
            virtual_units,
        },
        mix,
        abbreviations,
        compose,
        candidates,
    })
}
