//! 순서 무관성 (계획서 Phase 2): 같은 chord는 누르는 순서와 상관없이 같은 결과를 내야 한다.
//!
//! 어떤 순서로도 한 음절이 되지 않는 chord(예: 초ㄱ+초ㅁ, 규칙 없음)는 들어온 순서대로 나뉘므로 이
//! 성질의 대상이 아니다. 옛한글 모드가 아닐 때 약어가 아닌 옛한글 음절(예: 초ㅁ+초ㄷ → ꥰ)도 낱자로
//! 나뉘므로 마찬가지다. 한 음절이 되는 chord는 결과까지 같아야 한다.

mod common;

use std::collections::BTreeSet;

use common::Typist;
use proptest::prelude::*;
use semoe_core::Mode;
use semoe_core::data::{KeyAction, Unit};
use semoe_core::keyboard::{KeyState, Keyboard};
use semoe_core::syllable::Syllable;

/// 아랫글쇠에서 낱자가 나오는 키 (모음시프트 포함)
const HANGUL_KEYS: &str = "yuiohjklnmrtdfgcvb.qweaszx;";

fn unit(c: char) -> Unit {
    match Keyboard::builtin().action(c, &KeyState::default()) {
        Some((KeyAction::Unit { unit }, _)) => *unit,
        other => panic!("{c:?}: {other:?}"),
    }
}

fn permutations(v: &[char]) -> Vec<Vec<char>> {
    if v.len() <= 1 {
        return vec![v.to_vec()];
    }
    (0..v.len())
        .flat_map(|i| {
            let mut rest = v.to_vec();
            let c = rest.remove(i);
            permutations(&rest).into_iter().map(move |mut p| {
                p.insert(0, c);
                p
            })
        })
        .collect()
}

/// 한 음절이 되면 그 결과. 옛한글이라 낱자로 나뉘면 `None`.
fn whole(keys: &[char]) -> Option<String> {
    let kb = Keyboard::builtin();
    let units: Vec<Unit> = keys.iter().map(|&c| unit(c)).collect();
    let s = Syllable::whole(kb, &units)?;
    let shown = s.render(kb, true, false);
    (shown == s.render(kb, true, true)).then_some(shown)
}

fn typed(keys: &[char]) -> String {
    let mut t = Typist::new(Mode::Chord);
    let names: Vec<String> = keys.iter().map(|c| c.to_string()).collect();
    t.chord(&names.iter().map(String::as_str).collect::<Vec<_>>());
    t.finish()
}

/// 키 2~3개 chord 전부: 한 음절이 되는지와 그 결과가 순서와 무관하다.
#[test]
fn all_small_chords_are_order_independent() {
    let keys: Vec<char> = HANGUL_KEYS.chars().collect();
    let mut checked = 0;
    for a in 0..keys.len() {
        for b in a + 1..keys.len() {
            for c in (b + 1..=keys.len()).map(Some).chain([None]) {
                let set: Vec<char> = [
                    Some(keys[a]),
                    Some(keys[b]),
                    c.and_then(|c| keys.get(c).copied()),
                ]
                .into_iter()
                .flatten()
                .collect();
                let results: BTreeSet<Option<String>> =
                    permutations(&set).iter().map(|p| whole(p)).collect();
                assert_eq!(results.len(), 1, "{set:?}: {results:?}");
                checked += 1;
            }
        }
    }
    assert!(checked > 3000);
}

/// 원본 규칙으로 두 가지 답이 나오는 종성 묶음은 코드 순으로 정해 한 가지로 고정된다.
#[test]
fn ambiguous_jong_groups_are_fixed() {
    for (keys, want) in [("qea", "ㄳ"), ("es;", "ㅀ"), ("qeas", "ㅀ"), ("wes;", "ㄿ")] {
        let set: Vec<char> = keys.chars().collect();
        for p in permutations(&set) {
            assert_eq!(whole(&p).as_deref(), Some(want), "{p:?}");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        // 2~5키 무작위 chord는 옛한글로 나뉘어 대상에서 빠지는 경우가 많다.
        max_global_rejects: 10_000,
        ..ProptestConfig::with_cases(2000)
    })]

    /// 키 2~5개를 무작위 두 순서로 눌렀을 때, 한 음절이 되는 chord면 엔진 출력이 같다.
    #[test]
    fn engine_chord_output_is_order_independent(
        set in proptest::sample::subsequence(HANGUL_KEYS.chars().collect::<Vec<_>>(), 2..=5),
        seed_a in any::<u64>(),
        seed_b in any::<u64>(),
    ) {
        let shuffle = |seed: u64| {
            let mut v = set.clone();
            let mut x = seed | 1;
            for i in (1..v.len()).rev() {
                x ^= x << 13; x ^= x >> 7; x ^= x << 17;
                v.swap(i, (x % (i as u64 + 1)) as usize);
            }
            v
        };
        let (a, b) = (shuffle(seed_a), shuffle(seed_b));
        prop_assume!(whole(&a).is_some());
        prop_assert_eq!(typed(&a), typed(&b), "{:?} / {:?}", a, b);
    }
}
