//! 세모이 입력 엔진. 계획서 4.3.
//!
//! 순수 로직이다. 시각은 호출자가 넘기고, 타이머가 필요하면 마감 시각만 돌려준다.
//!
//! 입력 방식 (원본 XML의 오토마타)
//! - [`Mode::Chord`] 한 번에 모아치기 (`A|B|C ? 1 : 0`): 어떤 낱자든 같은 음절에 모은다. 모든 키를 떼거나
//!   마지막 입력 뒤 `timeout_ms`가 지나면 확정한다.
//! - [`Mode::Sequential`] 이어치기·중종성 모아치기 (`A ? 1 : B|C ? 2 : 0`, `2: B|C ? 2 : 0`): 초성 뒤에
//!   중·종성이 오면 같은 음절, 중·종성 뒤에 초성이 오면 새 음절이다. 원본은 두 방식을 한 설정으로
//!   제공한다. 순서대로 치면 이어치기, 중·종성을 함께 치면 순서와 무관하게 결합된다.
//!
//! 두 방식 모두 같은 벌 안에서는 결합 규칙으로 합치고, 규칙이 없으면 앞 음절을 확정하고 새 음절을 연다.

use std::collections::BTreeSet;

use crate::data::{KeyAction as DataAction, Slot, Unit};
use crate::key::{SemoeKey, modifiers};
use crate::keyboard::{KeyState, Keyboard};
use crate::syllable::Syllable;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyAction {
    Down,
    Up,
}

#[derive(Clone, Copy, Debug)]
pub struct KeyEvent {
    pub key: SemoeKey,
    pub action: KeyAction,
    pub modifiers: u32,
    pub is_repeat: bool,
    pub time_ms: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Output {
    /// `false`면 플랫폼 껍데기가 키를 앱에 그대로 넘긴다. `commit`은 그보다 먼저 넣는다.
    pub consumed: bool,
    pub commit: String,
    pub marked: String,
    /// 이 시각(ms)에 `on_timer`를 불러야 한다. `None`이면 타이머 해제.
    pub deadline: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Chord,
    Sequential,
}

#[derive(Clone, Debug)]
pub struct Options {
    pub mode: Mode,
    /// 한 번에 모아치기: 마지막 입력 후 강제 확정까지의 시간. 기본 40 ms (원본 `Timer`).
    pub timeout_ms: u64,
    /// 한 번에 모아치기: `false`면 키뗌을 무시하고 타이머만으로 확정한다 (계획서 3.1 방안 C).
    pub use_keyup: bool,
    /// 약어 사전. 옛한글을 직접 입력할 때는 끈다.
    pub abbreviations: bool,
    /// Caps Lock 순아래(Shift Lock). macOS에서는 Caps Lock을 한/영 전환에 쓰는 일이 많아 기본은 끈다.
    pub caps_lock_shift: bool,
    /// 원본의 Scroll Lock 모드(숫자열 옛한글 등). macOS에는 Scroll Lock이 없어 옵션으로 둔다.
    pub old_hangul: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            mode: Mode::Chord,
            timeout_ms: 40,
            use_keyup: true,
            abbreviations: true,
            caps_lock_shift: false,
            old_hangul: false,
        }
    }
}

/// 키 하나를 자판 데이터로 해석한 결과.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Input {
    Unit(Unit),
    /// 문자를 엔진이 넣는다.
    Char(char),
    /// 조합만 확정하고 키는 앱에 넘긴다 (QWERTY와 같은 문자, 엔진이 모르는 키, 수정키 조합).
    Pass,
    Backspace,
    Space,
    /// 아무것도 하지 않는 자리, 또는 아직 구현하지 않은 명령(기호 확장 단계, Phase 4).
    Ignore,
}

pub struct Engine {
    pub options: Options,
    kb: &'static Keyboard,
    /// 이어치기: 조합 중인 음절
    syllable: Syllable,
    /// 한 번에 모아치기: 이번 chord에 들어온 낱자들 (들어온 순서). 음절로 나누는 일은 표시·확정할 때 한다.
    units: Vec<Unit>,
    pressed: BTreeSet<SemoeKey>,
    deadline: Option<u64>,
    pending_space: bool,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self::with_keyboard(Keyboard::builtin())
    }

    pub fn with_keyboard(kb: &'static Keyboard) -> Self {
        Self {
            options: Options::default(),
            kb,
            syllable: Syllable::default(),
            units: Vec::new(),
            pressed: BTreeSet::new(),
            deadline: None,
            pending_space: false,
        }
    }

    pub fn handle_key(&mut self, ev: KeyEvent) -> Output {
        match ev.action {
            KeyAction::Down => self.key_down(ev),
            KeyAction::Up => self.key_up(ev),
        }
    }

    pub fn on_timer(&mut self, now_ms: u64) -> Output {
        let commit = match self.deadline {
            Some(d) if now_ms >= d => self.take(),
            _ => String::new(),
        };
        self.output(true, commit)
    }

    /// 포커스 이동 등으로 조합을 강제 확정한다.
    pub fn flush(&mut self) -> Output {
        let commit = self.take();
        self.pressed.clear();
        self.output(true, commit)
    }

    pub fn reset(&mut self) {
        self.syllable.clear();
        self.units.clear();
        self.pressed.clear();
        self.deadline = None;
        self.pending_space = false;
    }

    fn chord(&self) -> bool {
        self.options.mode == Mode::Chord
    }

    fn composing(&self) -> bool {
        !self.syllable.is_empty() || !self.units.is_empty()
    }

    fn resolve(&self, ev: &KeyEvent) -> Input {
        if ev.modifiers & (modifiers::CTRL | modifiers::ALT | modifiers::CMD) != 0 {
            return Input::Pass;
        }
        match ev.key {
            SemoeKey::BACKSPACE => return Input::Backspace,
            SemoeKey::SPACE => return Input::Space,
            _ => {}
        }
        let shift = ev.modifiers & modifiers::SHIFT != 0;
        let Some(ch) = ev.key.qwerty(shift) else {
            return Input::Pass;
        };
        let state = KeyState {
            stage: 0,
            caps_lock: self.options.caps_lock_shift && ev.modifiers & modifiers::CAPS != 0,
            scroll_lock: self.options.old_hangul,
        };
        match self.kb.action(ch, &state) {
            Some((DataAction::Unit { unit }, _)) => Input::Unit(*unit),
            // 원래 그 키의 문자면 앱이 직접 처리하게 넘긴다(단축키·자동 반복이 자연스럽다).
            Some((DataAction::Char { ch: c }, _)) if *c == ch => Input::Pass,
            Some((DataAction::Char { ch: c }, _)) => Input::Char(*c),
            Some((DataAction::Nothing | DataAction::Command { .. }, _)) => Input::Ignore,
            None => Input::Pass,
        }
    }

    fn key_down(&mut self, ev: KeyEvent) -> Output {
        let mut commit = String::new();
        // 타이머 이벤트가 늦게 와서 마감이 지났으면 먼저 확정한다.
        if self.deadline.is_some_and(|d| ev.time_ms >= d) {
            commit += &self.take();
        }

        match self.resolve(&ev) {
            Input::Backspace if self.composing() => {
                if self.chord() {
                    self.units.pop();
                } else {
                    self.syllable.undo(self.kb);
                }
                if self.composing() {
                    self.arm(ev.time_ms);
                } else {
                    self.deadline = None;
                    self.pending_space = false;
                }
                self.output(true, commit)
            }
            Input::Space if self.chord() && self.composing() && !ev.is_repeat => {
                // 스페이스 모아치기: 음절과 함께 누른 Space는 확정할 때 공백으로 붙인다.
                self.pending_space = true;
                self.pressed.insert(ev.key);
                self.output(true, commit)
            }
            Input::Unit(unit) => {
                if !ev.is_repeat {
                    commit += &self.add(unit);
                    if self.chord() {
                        self.pressed.insert(ev.key);
                        self.arm(ev.time_ms);
                    }
                }
                self.output(true, commit)
            }
            Input::Char(c) => {
                commit += &self.take();
                commit.push(c);
                self.output(true, commit)
            }
            Input::Ignore => {
                commit += &self.take();
                self.output(true, commit)
            }
            Input::Pass | Input::Backspace | Input::Space => {
                commit += &self.take();
                self.output(false, commit)
            }
        }
    }

    /// 낱자를 넣는다. 새 음절을 열어야 하면 앞 음절을 확정해 그 문자열을 돌려준다.
    fn add(&mut self, unit: Unit) -> String {
        if self.chord() {
            self.units.push(unit);
            return String::new();
        }
        let mut commit = String::new();
        // 이어치기: 중·종성 뒤에 초성이 오면 새 음절 (오토마타 상태 2에서 A → 0)
        let new_syllable = unit.slot == Slot::Cho
            && (self.syllable.has(Slot::Jung) || self.syllable.has(Slot::Jong));
        if new_syllable || !self.syllable.add(self.kb, unit) {
            commit = self.take();
            self.syllable.add(self.kb, unit);
        }
        commit
    }

    fn key_up(&mut self, ev: KeyEvent) -> Output {
        if !self.chord() || !self.pressed.remove(&ev.key) {
            return self.output(false, String::new());
        }
        let commit = if self.options.use_keyup && self.pressed.is_empty() && self.composing() {
            self.take()
        } else {
            String::new()
        };
        self.output(true, commit)
    }

    fn arm(&mut self, t: u64) {
        self.deadline = self.chord().then_some(t + self.options.timeout_ms);
    }

    /// 조합 중인 것을 화면에 보일 문자열로.
    fn render(&self) -> String {
        let abbr = self.options.abbreviations;
        if self.chord() {
            Syllable::segments(self.kb, &self.units)
                .iter()
                .map(|s| s.render(self.kb, abbr))
                .collect()
        } else {
            self.syllable.render(self.kb, abbr)
        }
    }

    /// 조합 중인 것을 꺼내고 상태를 비운다. 눌린 키 집합은 유지한다(날개셋의 `v`와 같음).
    fn take(&mut self) -> String {
        let mut s = self.render();
        if self.pending_space {
            s.push(' ');
        }
        self.syllable.clear();
        self.units.clear();
        self.deadline = None;
        self.pending_space = false;
        s
    }

    fn output(&self, consumed: bool, commit: String) -> Output {
        Output {
            consumed,
            commit,
            marked: self.render(),
            deadline: self.deadline,
        }
    }
}
