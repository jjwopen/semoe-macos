//! 골든 테스트: 개발 규격·설명서의 예제 (계획서 Phase 2).
//!
//! 키는 QWERTY 자리로 적는다. 세모이 아랫글쇠:
//! 초성 `y`ㅁ `u`ㄴ `i`ㄷ `o`ㅂ `h`ㅎ `j`ㅇ `k`ㄱ `l`ㅈ `n`ㅅ `m`ㄹ /
//! 중성 `r`ㅓ `t`ㅕ `d`ㅣ `f`ㅏ `g`ㅡ `c`ㅔ `v`ㅗ `b`ㅜ `.`모음시프트 /
//! 종성 `q`ㅅ `w`ㅂ `e`ㄹ `a`ㅇ `s`ㄴ `z`ㅁ `x`ㄱ `;`ㅆ(종성시프트)

mod common;

use common::{Typist, chord_str};
use semoe_core::Mode;

fn chord(seq: &[&str]) -> String {
    let mut t = Typist::new(Mode::Chord);
    for s in seq {
        chord_str(&mut t, s);
    }
    t.finish()
}

#[test]
fn gan_in_any_order() {
    for order in ["kfs", "fks", "skf", "sfk", "ksf", "fsk"] {
        assert_eq!(chord(&[order]), "간", "{order}");
    }
}

#[test]
fn four_chords_heulk_jip_mat_i() {
    // 흙 = ㅎ ㅡ ㄹ+ㄱ, 짚 = ㅈ ㅣ ㅂ+ㅆ(=ㅍ), 맞 = ㅁ ㅏ ㄹ+ㅆ(=ㅈ), 이 = ㅇ ㅣ
    assert_eq!(chord(&["hgex", "ldw;", "yfe;", "jd"]), "흙짚맞이");
}

#[test]
fn abbreviation_shown_while_composing_and_backspace_restores() {
    let mut t = Typist::new(Mode::Chord);
    t.down("k");
    t.down("x"); // 초ㄱ + 종ㄱ
    assert_eq!(t.marked(), "국가");
    t.down("a"); // + 종ㅇ → 종ㄲ
    assert_eq!(t.marked(), "공격");
    t.down("bksp");
    assert_eq!(t.marked(), "국가");
    for k in ["k", "x", "a"] {
        t.up(k);
    }
    assert_eq!(t.finish(), "국가");
}

#[test]
fn vowel_shift_differs_from_left_o() {
    assert_eq!(chord(&["k.g"]), "갸"); // 모음시프트 + ㅡ = ㅑ
    // ㅗ(v) + ㅡ는 옛한글 ᅩ+ᅳ(ힼ)로 가고, 이 음절은 약어 '그곳'이다.
    assert_eq!(chord(&["kvg"]), "그곳");
    let mut t = Typist::new(Mode::Chord);
    t.engine.options.abbreviations = false;
    chord_str(&mut t, "kvg");
    assert_eq!(t.finish(), "고ㅡ"); // 약어가 아니면 옛한글로 내지 않고 나눈다
    t.engine.options.old_hangul = true;
    chord_str(&mut t, "kvg");
    assert_eq!(t.finish(), "ᄀힼ");
}

#[test]
fn old_hangul_clusters_split_outside_old_hangul_mode() {
    // ㅁ(y) + ㄷ(i)는 원본 결합 규칙으로 옛한글 초성 ꥰ가 된다. 약어가 아니므로 낱자로 낸다.
    assert_eq!(chord(&["yi"]), "ㅁㄷ");
    assert_eq!(chord(&["yif"]), "ㅁ다");
    let mut t = Typist::new(Mode::Sequential);
    t.seq(&["y", "i"]);
    assert_eq!(t.marked(), "ㅁㄷ");
    t.seq(&["f"]);
    assert_eq!(t.finish(), "ㅁ다");
    // 옛한글 모드에서는 그대로 낸다.
    let mut t = Typist::new(Mode::Chord);
    t.engine.options.old_hangul = true;
    chord_str(&mut t, "yi");
    assert_eq!(t.finish(), "ꥰᅠ");
}

#[test]
fn jong_combinations() {
    assert_eq!(chord(&["kfsa"]), "갆"); // ㄶ = ㄴ + ㅇ
    assert_eq!(chord(&["kfaz"]), "갍"); // ㄾ = ㅇ + ㅁ
    assert_eq!(chord(&["kfaw"]), "갎"); // ㄿ = ㅇ + ㅂ
    assert_eq!(chord(&["kf;"]), "갔"); // 종성시프트 단독 = ㅆ
    assert_eq!(chord(&["kf;a"]), "갏"); // ㅆ + ㅇ = ㅀ
}

#[test]
fn space_chord() {
    let mut t = Typist::new(Mode::Chord);
    t.chord(&["k", "f", "a", "space"]);
    assert_eq!(t.finish(), "강 ");
}

#[test]
fn extra_abbreviation_ppyam() {
    assert_eq!(chord(&["ojz"]), "뺨");
    assert_eq!(chord(&["zjo"]), "뺨");
}

#[test]
fn dwaet_in_any_order() {
    // 초ㄷ ㅗ ㅣ ㅏ 종ㅆ: 어느 둘이 먼저 결합해도 ㅙ가 된다 (Phase 0에서 됬ㅏ로 쪼개지던 경우)
    for order in ["ivdf;", ";fdvi", "di;vf", "fdvi;", "v;ifd"] {
        assert_eq!(chord(&[order]), "됐", "{order}");
    }
}

#[test]
fn symbol_abbreviations() {
    assert_eq!(chord(&["nm"]), "? ");
    assert_eq!(chord(&["ui"]), ", ");
    assert_eq!(chord(&["io"]), ". ");
    assert_eq!(chord(&["yu"]), "! ");
}

#[test]
fn layout_symbols_and_passthrough() {
    let mut t = Typist::new(Mode::Chord);
    t.down("k");
    t.down("p"); // p = 마침표: 조합을 확정하고 '.'
    assert_eq!(t.text, "ㄱ.");
    t.up("k");
    t.up("p");
    t.down("P"); // Shift+p = 세미콜론
    t.down(":"); // Shift+; = 콜론(원래 문자이므로 앱에 넘김)
    t.down("1");
    t.down("ret");
    assert_eq!(t.finish(), "ㄱ.;");
    assert_eq!(t.passed, [":", "1", "ret"]);
}

#[test]
fn shifted_letters_are_symbols() {
    let mut t = Typist::new(Mode::Chord);
    t.down("A");
    t.down("R");
    assert_eq!(t.finish(), "←☆");
}

// ── 이어치기 · 중종성 모아치기 ──

fn seq(s: &str) -> String {
    let mut t = Typist::new(Mode::Sequential);
    let ks = common::keys(s);
    t.seq(&ks.iter().map(String::as_str).collect::<Vec<_>>());
    t.finish()
}

#[test]
fn sequential_typing() {
    assert_eq!(seq("kfs"), "간");
    assert_eq!(seq("kfskf"), "간가");
    assert_eq!(seq("hgexldw;yfe;jd"), "흙짚맞이");
    // 초성끼리는 결합한다: ㅇ + ㄱ = ㄲ
    assert_eq!(seq("jkf"), "까");
}

#[test]
fn sequential_jung_jong_chord_any_order() {
    for order in [["f", "s"], ["s", "f"]] {
        let mut t = Typist::new(Mode::Sequential);
        t.seq(&["k"]);
        t.chord(&order);
        assert_eq!(t.finish(), "간");
    }
}

#[test]
fn sequential_has_no_timer_or_keyup_commit() {
    let mut t = Typist::new(Mode::Sequential);
    t.down("k");
    t.down("f");
    assert_eq!(t.last.deadline, None);
    t.up("k");
    t.up("f");
    t.wait(1000);
    assert_eq!(t.marked(), "가");
    t.down("space"); // 이어치기에서 Space는 확정하고 앱에 넘긴다
    assert_eq!(t.text, "가");
    assert_eq!(t.passed, ["space"]);
}

#[test]
fn sequential_abbreviation_kukka() {
    // 초ㄱ 뒤 종ㄱ = 국가, 이어서 초ㄱ이 오면 확정
    assert_eq!(seq("kxkf"), "국가가");
}
