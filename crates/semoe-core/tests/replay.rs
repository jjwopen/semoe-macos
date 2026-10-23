//! 키 이벤트 재생 하네스: `tests/fixtures/*.txt`를 차례로 재생하며 결과를 확인한다.
//!
//! 픽스처 문법 (한 줄에 하나, `#` 뒤는 주석)
//! ```text
//! mode chord | sequential      입력 방식 (기본 chord)
//! option keyup off             엔진 옵션: keyup, abbrev, caps, old_hangul (on/off), timeout <ms>
//! +k 0                         t=0 ms에 k 누름. 키 이름은 QWERTY 문자 하나 또는 space bksp ret esc tab
//! -k 20                        t=20 ms에 k 뗌
//! +k 30 repeat                 자동 반복 keyDown
//! +a 5 cmd                     수정키와 함께 (shift caps ctrl alt cmd, 대문자는 Shift가 붙음)
//! timer 60                     t=60 ms에 타이머 호출
//! flush                        강제 확정
//! commit 간                    지난 확인 뒤로 확정된 문자열 (따옴표로 감싸면 JSON 문자열: "강 ")
//! marked 가                    지금 조합 중인 문자열 (비었으면 "")
//! deadline 40                  지금 마감 시각 (none이면 없음)
//! passed k 1                   지난 확인 뒤로 앱에 넘긴 키들 (없으면 none)
//! ```

mod common;

use std::fs;
use std::path::Path;

use semoe_core::key::modifiers;
use semoe_core::{Engine, KeyAction, KeyEvent, Mode, Output};

fn value(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('"') {
        serde_json::from_str(s).unwrap_or_else(|e| panic!("문자열 {s}: {e}"))
    } else {
        s.to_string()
    }
}

fn run(path: &Path) -> Result<(), String> {
    let text = fs::read_to_string(path).unwrap();
    let mut engine = Engine::new();
    let mut committed = String::new();
    let mut passed: Vec<String> = Vec::new();
    let mut last = Output::default();

    for (n, raw) in text.lines().enumerate() {
        let line = raw.split(" #").next().unwrap().trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let at = |msg: String| format!("{}:{}: {msg}\n    {raw}", path.display(), n + 1);
        let (cmd, rest) = line.split_once(' ').unwrap_or((line, ""));
        let mut feed = |out: Output, name: Option<&str>| {
            committed += &out.commit;
            if let (Some(name), false) = (name, out.consumed) {
                passed.push(name.to_string());
            }
            last = out;
        };
        match cmd {
            "mode" => {
                engine.options.mode = match rest {
                    "chord" => Mode::Chord,
                    "sequential" => Mode::Sequential,
                    _ => return Err(at(format!("모르는 방식 {rest}"))),
                }
            }
            "option" => {
                let (k, v) = rest
                    .split_once(' ')
                    .ok_or_else(|| at("option <이름> <값>".into()))?;
                let on = v == "on";
                let o = &mut engine.options;
                match k {
                    "keyup" => o.use_keyup = on,
                    "abbrev" => o.abbreviations = on,
                    "caps" => o.caps_lock_shift = on,
                    "old_hangul" => o.old_hangul = on,
                    "timeout" => o.timeout_ms = v.parse().map_err(|e| at(format!("{e}")))?,
                    _ => return Err(at(format!("모르는 옵션 {k}"))),
                }
            }
            c if c.starts_with('+') || c.starts_with('-') => {
                let name = &c[1..];
                let mut words = rest.split_whitespace();
                let t: u64 = words
                    .next()
                    .ok_or_else(|| at("시각이 없음".into()))?
                    .parse()
                    .map_err(|e| at(format!("{e}")))?;
                let mut ev: KeyEvent = common::event(
                    name,
                    if c.starts_with('+') {
                        KeyAction::Down
                    } else {
                        KeyAction::Up
                    },
                    t,
                );
                for w in words {
                    match w {
                        "repeat" => ev.is_repeat = true,
                        "shift" => ev.modifiers |= modifiers::SHIFT,
                        "caps" => ev.modifiers |= modifiers::CAPS,
                        "ctrl" => ev.modifiers |= modifiers::CTRL,
                        "alt" => ev.modifiers |= modifiers::ALT,
                        "cmd" => ev.modifiers |= modifiers::CMD,
                        _ => return Err(at(format!("모르는 꼬리말 {w}"))),
                    }
                }
                let out = engine.handle_key(ev);
                feed(out, c.starts_with('+').then_some(name));
            }
            "timer" => {
                let t = rest.parse().map_err(|e| at(format!("{e}")))?;
                feed(engine.on_timer(t), None);
            }
            "flush" => feed(engine.flush(), None),
            "commit" => {
                let want = value(rest);
                if committed != want {
                    return Err(at(format!("확정: {committed:?}, 기대: {want:?}")));
                }
                committed.clear();
            }
            "marked" => {
                let want = value(rest);
                if last.marked != want {
                    return Err(at(format!("조합 중: {:?}, 기대: {want:?}", last.marked)));
                }
            }
            "deadline" => {
                let want = (rest != "none")
                    .then(|| rest.parse::<u64>())
                    .transpose()
                    .map_err(|e| at(format!("{e}")))?;
                if last.deadline != want {
                    return Err(at(format!("마감: {:?}, 기대: {want:?}", last.deadline)));
                }
            }
            "passed" => {
                let want: Vec<String> = if rest == "none" {
                    vec![]
                } else {
                    rest.split_whitespace().map(str::to_owned).collect()
                };
                if passed != want {
                    return Err(at(format!("넘긴 키: {passed:?}, 기대: {want:?}")));
                }
                passed.clear();
            }
            _ => return Err(at(format!("모르는 명령 {cmd}"))),
        }
    }
    if !committed.is_empty() {
        return Err(format!(
            "{}: 확인하지 않은 확정 문자열 {committed:?}",
            path.display()
        ));
    }
    Ok(())
}

#[test]
fn fixtures() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut files: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.retain(|p| p.extension().is_some_and(|e| e == "txt"));
    files.sort();
    assert!(!files.is_empty(), "픽스처가 없음");
    let errors: Vec<String> = files.iter().filter_map(|p| run(p).err()).collect();
    assert!(
        errors.is_empty(),
        "{} / {} 실패\n{}",
        errors.len(),
        files.len(),
        errors.join("\n")
    );
}
