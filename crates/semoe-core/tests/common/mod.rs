//! 통합 테스트 공용 도구: QWERTY 문자로 키를 누르고 떼는 흉내.

#![allow(dead_code)]

use semoe_core::key::modifiers;
use semoe_core::{Engine, KeyAction, KeyEvent, Mode, Output, SemoeKey};

/// 키 이름: QWERTY 문자 하나, 또는 `space`, `bksp`, `ret`, `esc`, `tab`.
pub fn key(name: &str) -> (SemoeKey, bool) {
    match name {
        "space" => (SemoeKey::SPACE, false),
        "bksp" => (SemoeKey::BACKSPACE, false),
        "ret" => (SemoeKey::RETURN, false),
        "esc" => (SemoeKey::ESCAPE, false),
        "tab" => (SemoeKey::TAB, false),
        _ => {
            let mut it = name.chars();
            let (Some(c), None) = (it.next(), it.next()) else {
                panic!("키 이름 {name:?}")
            };
            SemoeKey::from_qwerty(c).unwrap_or_else(|| panic!("QWERTY에 없는 문자 {c:?}"))
        }
    }
}

pub fn event(name: &str, action: KeyAction, t: u64) -> KeyEvent {
    let (k, shift) = key(name);
    KeyEvent {
        key: k,
        action,
        modifiers: if shift { modifiers::SHIFT } else { 0 },
        is_repeat: false,
        time_ms: t,
    }
}

/// 입력기를 흉내 낸다: 확정된 문자열을 모으고, 엔진이 넘긴 키는 앱이 받은 것으로 기록한다.
pub struct Typist {
    pub engine: Engine,
    pub text: String,
    pub passed: Vec<String>,
    pub last: Output,
    pub t: u64,
}

impl Typist {
    pub fn new(mode: Mode) -> Self {
        let mut engine = Engine::new();
        engine.options.mode = mode;
        Self {
            engine,
            text: String::new(),
            passed: Vec::new(),
            last: Output::default(),
            t: 1000,
        }
    }

    pub fn apply(&mut self, name: &str, out: Output) {
        self.text += &out.commit;
        if !out.consumed {
            self.passed.push(name.to_string());
        }
        self.last = out;
    }

    pub fn down(&mut self, name: &str) {
        let out = self.engine.handle_key(event(name, KeyAction::Down, self.t));
        self.apply(name, out);
    }

    pub fn up(&mut self, name: &str) {
        let out = self.engine.handle_key(event(name, KeyAction::Up, self.t));
        // 키뗌은 앱이 받든 말든 결과에 영향이 없으므로 넘긴 키로 세지 않는다.
        self.text += &out.commit;
        self.last = out;
    }

    pub fn wait(&mut self, ms: u64) {
        self.t += ms;
        if self.last.deadline.is_some_and(|d| d <= self.t) {
            let out = self.engine.on_timer(self.t);
            self.text += &out.commit;
            self.last = out;
        }
    }

    /// 키들을 1 ms 간격으로 누르고 10 ms 뒤 모두 뗀 다음 100 ms 쉰다 (한 번에 모아치기 한 타).
    pub fn chord(&mut self, keys: &[&str]) {
        for k in keys {
            self.down(k);
            self.t += 1;
        }
        self.t += 10;
        for k in keys {
            self.up(k);
            self.t += 1;
        }
        self.wait(100);
    }

    /// 키를 하나씩 눌렀다 뗀다 (이어치기).
    pub fn seq(&mut self, keys: &[&str]) {
        for k in keys {
            self.down(k);
            self.t += 30;
            self.up(k);
            self.wait(100);
        }
    }

    pub fn marked(&self) -> &str {
        &self.last.marked
    }

    /// 조합 중인 것까지 확정한 전체 문자열
    pub fn finish(&mut self) -> String {
        let out = self.engine.flush();
        self.text += &out.commit;
        std::mem::take(&mut self.text)
    }
}

/// `"kfs"` → `["k", "f", "s"]`
pub fn keys(s: &str) -> Vec<String> {
    s.chars().map(|c| c.to_string()).collect()
}

pub fn chord_str(t: &mut Typist, s: &str) {
    let ks = keys(s);
    t.chord(&ks.iter().map(String::as_str).collect::<Vec<_>>());
}
