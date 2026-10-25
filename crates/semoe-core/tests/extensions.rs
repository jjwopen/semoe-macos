//! Phase 4 확장 기능: 기호 확장 단계, 기호 조합, 옛한글 모드, Caps Lock 순아래.

mod common;

use common::{Typist, chord_str};
use semoe_core::Mode;
use semoe_core::key::modifiers;
use semoe_core::{KeyAction, KeyEvent};

fn typed(keys: &[&str]) -> Typist {
    let mut t = Typist::new(Mode::Chord);
    for k in keys {
        t.down(k);
        t.t += 5;
        t.up(k);
        t.wait(100);
    }
    t
}

// ── 기호 확장 (Shift+J → Shift+K·L·; → 기호 하나) ──

#[test]
fn symbol_extension_stages() {
    assert_eq!(typed(&["J", "K", "a"]).finish(), "◇"); // 1단계
    assert_eq!(typed(&["J", "L", "a"]).finish(), "◈"); // 2단계
    assert_eq!(typed(&["J", ":", "a"]).finish(), "◆"); // 3단계
}

#[test]
fn symbol_extension_is_one_shot() {
    // 기호 하나를 넣으면 평소로 돌아와 a는 다시 종성 ㅇ이다.
    let mut t = typed(&["J", "K", "a", "k", "f"]);
    assert_eq!(t.engine.stage(), 0);
    assert_eq!(t.finish(), "◇ㄱㅏ");
}

#[test]
fn ready_stage_falls_back_to_normal_keys() {
    // Shift+J 다음 단계 키가 아니면 그 키는 평소대로 처리되고 확장은 끝난다.
    let mut t = typed(&["J"]);
    assert_eq!(t.engine.stage(), 100);
    t.chord(&["k", "f"]);
    assert_eq!(t.engine.stage(), 0);
    assert_eq!(t.finish(), "가");
}

#[test]
fn escape_cancels_symbol_extension() {
    let mut t = typed(&["J", "K", "esc", "a"]);
    assert_eq!(t.engine.stage(), 0);
    assert_eq!(t.passed, Vec::<String>::new()); // Esc는 취소로 쓰고 앱에 넘기지 않는다
    assert_eq!(t.finish(), "ㅇ");
}

#[test]
fn no_commit_timer_during_symbol_extension() {
    let mut t = Typist::new(Mode::Chord);
    t.down("J");
    assert_eq!(t.last.deadline, None);
    t.up("J");
    t.wait(1000);
    assert_eq!(t.engine.stage(), 100);
}

#[test]
fn circle_keeps_stage_for_ending_abbreviations() {
    // 1단계 d = ○. 원본은 이 키만 단계를 유지(`(T=0x65, …)`)해서, ○ 다음에 1단계 기호를 이어 치면
    // 기호 조합으로 종결어미(니? · 나? 등)가 된다.
    let t = typed(&["J", "K", "d"]);
    assert_eq!(t.engine.stage(), 101);
    assert_eq!(t.marked(), "○"); // 기호 조합 상태 17
}

// ── 기호 조합 (UserCompo) ──

#[test]
fn compose_two_symbols_into_ending() {
    // Shift+K = ※, Shift+F = △ → '가? ' (어느 순서든)
    assert_eq!(typed(&["K", "F"]).finish(), "가? ");
    assert_eq!(typed(&["F", "K"]).finish(), "가? ");
}

#[test]
fn compose_shows_interim_and_uses_native_symbols() {
    // Shift+p = ; (상태 10), 이어서 Shift+1 = ! (원래 QWERTY 문자도 조합에 쓰인다) → '한 '
    let mut t = typed(&["P"]);
    assert_eq!(t.marked(), ";");
    assert_eq!(t.text, "");
    t.down("!");
    assert_eq!(t.text, "한 ");
    assert!(t.passed.is_empty());
}

#[test]
fn compose_without_rule_commits_interim_and_continues() {
    let mut t = typed(&["P", "k"]); // ; 뒤에 한글
    assert_eq!(t.finish(), ";ㄱ");
    let mut t = typed(&["P", "1"]); // ; 뒤에 규칙 없는 원래 문자는 ;를 확정하고 1은 앱에 넘긴다
    assert_eq!(t.finish(), ";");
    assert_eq!(t.passed, ["1"]);
}

#[test]
fn backspace_and_escape_cancel_compose() {
    assert_eq!(typed(&["K", "bksp"]).finish(), "");
    assert_eq!(typed(&["K", "esc"]).finish(), "");
}

fn typed_inline(keys: &[&str]) -> Typist {
    let mut t = Typist::new(Mode::Chord);
    t.engine.options.compose_inline = true;
    for k in keys {
        t.down(k);
        t.t += 5;
        t.up(k);
        t.wait(100);
    }
    t
}

#[test]
fn inline_compose_commits_interim_right_away() {
    // 조합 중 문자열을 남기지 않는다: Shift+Enter 등을 앱에 넘길 때 앱이 ?를 잃지 않게.
    let mut t = typed_inline(&["?"]);
    assert_eq!(t.marked(), "");
    assert_eq!(t.text, "?");
    t.down("ret");
    assert_eq!(t.passed, ["ret"]);
    assert_eq!(t.last.commit, "");
    t.down("F"); // 조합은 끝났으므로 △는 따로 들어간다
    assert_eq!(t.finish(), "?△");
}

#[test]
fn inline_compose_replaces_committed_interim() {
    let mut t = typed_inline(&["?", "F"]); // ? △ → '까? '
    assert_eq!(t.last.replace, "");
    assert_eq!(t.finish(), "까? ");
    let mut t = typed_inline(&["P"]);
    t.down("!");
    assert_eq!(t.last.replace, ";");
    assert_eq!(t.finish(), "한 ");
    assert_eq!(typed_inline(&[">", ">"]).finish(), ">>");
    assert_eq!(typed_inline(&["K", "F"]).finish(), "가? ");
}

#[test]
fn inline_compose_without_rule_keeps_interim() {
    assert_eq!(typed_inline(&["P", "k"]).finish(), ";ㄱ");
    let mut t = typed_inline(&["P", "1"]);
    assert_eq!(t.finish(), ";");
    assert_eq!(t.passed, ["1"]);
}

#[test]
fn inline_compose_backspace_goes_to_app_and_escape_only_cancels() {
    let mut t = typed_inline(&["K", "bksp", "F"]);
    assert_eq!(t.passed, ["bksp"]); // 확정한 ※는 앱이 지운다
    assert_eq!(t.finish(), "※△"); // 테스트 도구는 앱의 Backspace를 흉내 내지 않는다
    let mut t = typed_inline(&["K", "esc", "F"]);
    assert!(t.passed.is_empty());
    assert_eq!(t.finish(), "※△");
}

#[test]
fn compose_can_be_turned_off() {
    let mut t = Typist::new(Mode::Chord);
    t.engine.options.compose = false;
    for k in ["K", "F"] {
        t.down(k);
        t.up(k);
    }
    assert_eq!(t.finish(), "※△");
}

#[test]
fn hangul_commits_before_compose_starts() {
    let mut t = Typist::new(Mode::Chord);
    chord_str(&mut t, "kfs");
    t.down("K");
    t.up("K");
    t.down("F");
    assert_eq!(t.finish(), "간가? ");
}

// ── 옛한글 모드 ──

fn old_hangul() -> Typist {
    let mut t = Typist::new(Mode::Chord);
    t.engine.options.old_hangul = true;
    t
}

#[test]
fn old_hangul_number_row_and_no_abbreviations() {
    let mut t = old_hangul();
    chord_str(&mut t, "kf4"); // 숫자 4 자리 = 종성 ㄽ
    chord_str(&mut t, "kx"); // 약어(국가)가 꺼져 있다
    assert_eq!(t.finish(), "갌ㄱㄱ");
    let mut t = old_hangul();
    chord_str(&mut t, "k5"); // 숫자 5 자리 = 중성 ᆈ (옛한글)
    assert_eq!(t.finish(), "ᄀᆈ");
}

#[test]
fn old_hangul_tone_marks() {
    let mut t = old_hangul();
    chord_str(&mut t, "kf");
    t.down("Y"); // 방점 하나
    t.down("U"); // 방점 둘
    assert_eq!(t.finish(), "가\u{302E}\u{302F}");
}

// ── Caps Lock 순아래 ──

#[test]
fn caps_lock_shift_lock_only_when_enabled() {
    let caps = |enabled: bool| {
        let mut t = Typist::new(Mode::Chord);
        t.engine.options.caps_lock_shift = enabled;
        let ev = |action, t| KeyEvent {
            modifiers: modifiers::CAPS,
            ..common::event("a", action, t)
        };
        let out = t.engine.handle_key(ev(KeyAction::Down, 0));
        t.apply("a", out);
        let out = t.engine.handle_key(ev(KeyAction::Up, 10));
        t.text += &out.commit;
        t.finish()
    };
    assert_eq!(caps(true), "←");
    assert_eq!(caps(false), "ㅇ");
}
