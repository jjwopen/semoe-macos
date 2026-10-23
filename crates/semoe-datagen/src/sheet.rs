//! 원본 저장소의 약어 설명서(`세모이 자판 약어 (가나다순).xlsx`)로 약어 표를 교차 검증한다.
//!
//! 설명서에는 약어마다 초성·중성·종성 자리에 누르는 자모가 적혀 있다(예: `v수 없` = 초 `ㅅ`, 종 `ㅄ`,
//! `v`는 공백). 이 자모를 결합 규칙으로 합쳐 만든 첫가끝 문자열이 약어 표에서 같은 글자를 가리키는지
//! 본다. 키 → 결합 규칙 → 낱자 이름 대응표 → 약어 키의 전체 경로를 검사하므로, 대응표에서 두 이름이
//! 서로 바뀐 경우처럼 벌별 도달 가능성 검사로는 못 잡는 오류를 잡는다.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

use semoe_core::data::{Abbreviation, JUNG_FILLER, MixRule, Slot, virtual_unit_char};

use crate::xml::{self, Element};

pub const SHEET: &str = "세모이 자판 설명/세모이 자판 약어 (가나다순).xlsx";

fn zip_text(archive: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Result<String, String> {
    let mut f = archive.by_name(name).map_err(|e| format!("{name}: {e}"))?;
    let mut s = String::new();
    f.read_to_string(&mut s)
        .map_err(|e| format!("{name}: {e}"))?;
    Ok(s)
}

/// 첫 시트의 행들. 각 행은 열 이름(A, B, …) → 값.
pub fn read_rows(path: &Path) -> Result<Vec<BTreeMap<String, String>>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("{}: {e}", path.display()))?;
    let shared: Vec<String> = xml::parse(&zip_text(&mut archive, "xl/sharedStrings.xml")?)?
        .find("sst")
        .ok_or("sharedStrings에 <sst>가 없음")?
        .children_named("si")
        .map(Element::all_text)
        .collect();
    let sheet = xml::parse(&zip_text(&mut archive, "xl/worksheets/sheet1.xml")?)?;
    let data = sheet.find("sheetData").ok_or("<sheetData>가 없음")?;
    let mut rows = Vec::new();
    for row in data.children_named("row") {
        let mut cells = BTreeMap::new();
        for c in row.children_named("c") {
            let col: String = c
                .attr("r")?
                .chars()
                .take_while(char::is_ascii_alphabetic)
                .collect();
            let v = c.find("v").map(|v| v.text.clone()).unwrap_or_default();
            let value = match c.attrs.get("t").map(String::as_str) {
                Some("s") => shared
                    .get(v.parse::<usize>().map_err(|e| format!("셀 {col}: {e}"))?)
                    .cloned()
                    .ok_or(format!("셀 {col}: 공유 문자열 범위 밖"))?,
                Some("inlineStr") => c.all_text(),
                _ => v,
            };
            cells.insert(col, value.trim().to_string());
        }
        rows.push(cells);
    }
    Ok(rows)
}

const CHO: &str = "ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ";
const JUNG: &str = "ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ";
const JONG: &str = "ㄱㄲㄳㄴㄵㄶㄷㄹㄺㄻㄼㄽㄾㄿㅀㅁㅂㅄㅅㅆㅇㅈㅊㅋㅌㅍㅎ";

/// 설명서에 적힌 호환 자모 하나를 그 벌의 첫가끝 자모로.
fn conjoining(slot: Slot, token: &str) -> Option<char> {
    let (table, base) = match slot {
        Slot::Cho => (CHO, 0x1100),
        Slot::Jung => (JUNG, 0x1161),
        Slot::Jong => (JONG, 0x11A8),
    };
    match (slot, token) {
        (Slot::Jung, "ㅗ(V키)") => return Some('ᅩ'),
        (Slot::Jung, "ㆍ") => return Some('ᆞ'),
        (Slot::Jung, "ᆢ") => return Some('ᆢ'),
        _ => {}
    }
    let mut it = token.chars();
    let (Some(c), None) = (it.next(), it.next()) else {
        return None;
    };
    table
        .chars()
        .position(|x| x == c)
        .and_then(|i| char::from_u32(base + i as u32))
}

/// `"ㅅ+ㄹ"`처럼 적힌 칸을 결합 규칙으로 합친다. 빈 칸이면 `None`.
fn cell(slot: Slot, text: &str, mix: &[MixRule]) -> Result<Option<char>, String> {
    let mut cur: Option<char> = None;
    for token in text.split('+').map(str::trim).filter(|t| !t.is_empty()) {
        let c = conjoining(slot, token).ok_or(format!("알 수 없는 자모 표기 {token:?}"))?;
        cur = Some(match cur {
            None => c,
            Some(p) => crate::validate::mix_pair(mix, slot, p, c)
                .ok_or(format!("{p}+{c} 결합 규칙 없음"))?,
        });
    }
    Ok(cur)
}

pub struct SheetCheck {
    pub matched: usize,
    pub errors: Vec<String>,
}

/// 일반 약어 표(A~F열: 번호, 표시되는 글자, 품사, 초성, 중성, 종성)를 약어 표와 맞춰 본다.
/// 같은 시트 G열 이후의 종결어미 약어는 Shift를 쓰는 기호 조합이라 여기서 보지 않는다.
pub fn check(
    rows: &[BTreeMap<String, String>],
    mix: &[MixRule],
    abbreviations: &[Abbreviation],
) -> SheetCheck {
    let by_from: BTreeMap<&str, &str> = abbreviations
        .iter()
        .map(|a| (a.from.as_str(), a.to.as_str()))
        .collect();
    let vowel_shift = virtual_unit_char(500);
    let mut matched = 0;
    let mut errors = Vec::new();
    for r in rows {
        let get = |c: &str| r.get(c).map(String::as_str).unwrap_or("");
        let (no, shown) = (get("A"), get("B"));
        if shown.is_empty() || !no.chars().all(|c| c.is_ascii_digit()) || no.is_empty() {
            continue;
        }
        let text = shown.replace('v', " ");
        let parsed = (|| -> Result<String, String> {
            let cho = cell(Slot::Cho, get("D"), mix)?.ok_or("초성 칸이 비어 있음")?;
            let jung = cell(Slot::Jung, get("E"), mix)?
                .filter(|&c| Some(c) != vowel_shift)
                .unwrap_or(JUNG_FILLER);
            let jong = cell(Slot::Jong, get("F"), mix)?;
            Ok([Some(cho), Some(jung), jong]
                .into_iter()
                .flatten()
                .collect())
        })();
        match parsed {
            Ok(from) => match by_from.get(from.as_str()) {
                Some(&to) if to == text => matched += 1,
                got => errors.push(format!(
                    "설명서 {no}번 {shown:?} (초 {:?} 중 {:?} 종 {:?}) → 키 {from:?}인데 약어 표에는 {got:?}",
                    get("D"),
                    get("E"),
                    get("F")
                )),
            },
            Err(e) => errors.push(format!("설명서 {no}번 {shown:?}: {e}")),
        }
    }
    SheetCheck { matched, errors }
}
