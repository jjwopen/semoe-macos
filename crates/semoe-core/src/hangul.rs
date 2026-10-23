//! 첫가끝 자모 → 화면에 보일 문자열.

use crate::data::{CHO_FILLER, JUNG_FILLER};

const CHO_BASE: u32 = 0x1100;
const JUNG_BASE: u32 = 0x1161;
const JONG_BASE: u32 = 0x11A7; // 종성 없음 = 0

/// 현대 초성(U+1100~1112) 순서의 호환 자모
const CHO_COMPAT: &str = "ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ";
/// 현대 중성(U+1161~1175) 순서의 호환 자모
const JUNG_COMPAT: &str = "ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ";
/// 현대 종성(U+11A8~11C2) 순서의 호환 자모
const JONG_COMPAT: &str = "ㄱㄲㄳㄴㄵㄶㄷㄹㄺㄻㄼㄽㄾㄿㅀㅁㅂㅄㅅㅆㅇㅈㅊㅋㅌㅍㅎ";

fn modern_cho(c: char) -> Option<u32> {
    (c as u32).checked_sub(CHO_BASE).filter(|&i| i < 19)
}

fn modern_jung(c: char) -> Option<u32> {
    (c as u32).checked_sub(JUNG_BASE).filter(|&i| i < 21)
}

fn modern_jong(c: char) -> Option<u32> {
    (c as u32)
        .checked_sub(JONG_BASE)
        .filter(|&i| (1..28).contains(&i))
}

/// 현대 자모 하나를 호환 자모로. 옛한글이면 `None`.
pub fn compat(c: char) -> Option<char> {
    let pick = |table: &str, i: u32| table.chars().nth(i as usize);
    modern_cho(c)
        .and_then(|i| pick(CHO_COMPAT, i))
        .or_else(|| modern_jung(c).and_then(|i| pick(JUNG_COMPAT, i)))
        .or_else(|| modern_jong(c).and_then(|i| pick(JONG_COMPAT, i - 1)))
}

/// 조합 중인 음절을 화면에 보일 문자열로 만든다.
///
/// - 현대 한글로 합쳐지면 완성형 한 글자 (`ᄀ ᅡ ᆫ` → `간`)
/// - 합쳐지지 않지만 현대 자모뿐이면 호환 자모를 나열 (`ᄀ` → `ㄱ`, 중성+종성 `ᅵ ᆼ` → `ㅣㅇ`)
/// - 옛한글 자모가 있으면 채움 문자를 넣은 첫가끝 조합 (`ᄩ` → `ᄩᅠ`). 옛한글은 이 꼴이라야 보인다.
///
/// 현대 자모까지 채움 문자 꼴로 내면 앱에 따라 채움 문자가 따로 보이거나 이상하게 합쳐진다 (Phase 3).
pub fn display(cho: Option<char>, jung: Option<char>, jong: Option<char>) -> String {
    match (cho, jung, jong) {
        (None, None, None) => String::new(),
        (Some(l), Some(v), t) => {
            if let (Some(l), Some(v)) = (modern_cho(l), modern_jung(v)) {
                let t = match t {
                    None => Some(0),
                    Some(t) => modern_jong(t),
                };
                if let Some(t) = t {
                    return char::from_u32(0xAC00 + (l * 21 + v) * 28 + t)
                        .unwrap()
                        .to_string();
                }
            }
            compat_all(cho, jung, jong).unwrap_or_else(|| conjoining(cho, jung, jong))
        }
        _ => compat_all(cho, jung, jong).unwrap_or_else(|| conjoining(cho, jung, jong)),
    }
}

/// 있는 자모가 모두 현대 자모면 호환 자모를 차례로 이은 문자열.
fn compat_all(cho: Option<char>, jung: Option<char>, jong: Option<char>) -> Option<String> {
    [cho, jung, jong]
        .into_iter()
        .flatten()
        .map(compat)
        .collect()
}

/// 채움 문자를 넣은 첫가끝 문자열. 약어 사전의 키도 이 꼴이다(초성 + 중성 또는 중성 채움 + 종성?).
pub fn conjoining(cho: Option<char>, jung: Option<char>, jong: Option<char>) -> String {
    [
        Some(cho.unwrap_or(CHO_FILLER)),
        Some(jung.unwrap_or(JUNG_FILLER)),
        jong,
    ]
    .into_iter()
    .flatten()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composes_modern_syllables() {
        assert_eq!(display(Some('ᄀ'), Some('ᅡ'), None), "가");
        assert_eq!(display(Some('ᄀ'), Some('ᅡ'), Some('ᆫ')), "간");
        assert_eq!(display(Some('ᄒ'), Some('ᅳ'), Some('ᆰ')), "흙");
    }

    #[test]
    fn single_modern_jamo_is_compatibility_jamo() {
        assert_eq!(display(Some('ᄀ'), None, None), "ㄱ");
        assert_eq!(display(None, Some('ᅪ'), None), "ㅘ");
        assert_eq!(display(None, None, Some('ᆶ')), "ㅀ");
    }

    #[test]
    fn incomplete_modern_syllables_are_compatibility_jamo() {
        // 초성 없이 중성+종성: 채움 문자(U+115F)를 넣지 않는다
        assert_eq!(display(None, Some('ᅵ'), Some('ᆼ')), "ㅣㅇ");
        assert_eq!(display(None, Some('ᅡ'), Some('ᆫ')), "ㅏㄴ");
        // 중성 없이 초성+종성 (약어가 없거나 꺼졌을 때)
        assert_eq!(display(Some('ᄀ'), None, Some('ᆨ')), "ㄱㄱ");
    }

    #[test]
    fn old_hangul_uses_fillers() {
        assert_eq!(display(Some('ᄩ'), None, None), "ᄩᅠ");
        assert_eq!(display(Some('ᄀ'), Some('ힼ'), None), "ᄀힼ");
        assert_eq!(display(None, Some('ᆞ'), Some('ᆫ')), "ᅟᆞᆫ");
    }
}
