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
    /// `commit`을 넣기 전에 커서 바로 앞에서 지울 문자열. 앞서 확정한 기호 조합 중간 문자열이다
    /// ([`Options::compose_inline`]). 비어 있으면 지우지 않는다.
    pub replace: String,
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
    /// 약어 사전. 옛한글 모드에서는 이 값과 상관없이 꺼진다.
    pub abbreviations: bool,
    /// 기호 조합(원본 `UserCompoTable`): 기호 두 개를 이어 치면 정해진 문자열이 된다 (`※` `△` → `가? `).
    pub compose: bool,
    /// 기호 조합 중간 문자열을 조합 중 문자열로 두지 않고 바로 확정한다. 조합이 이어지면 [`Output::replace`]로
    /// 그 문자열을 바꾼다. 조합 중 문자열이 남은 채 Enter 등을 앱에 넘기면 확정한 글자를 잃는 앱이 있어서다
    /// (grok 웹에서 `?` 뒤 Shift+Enter). 커서 앞 문자열을 바꿀 수 있는 클라이언트에서만 켠다.
    pub compose_inline: bool,
    /// Caps Lock 순아래(Shift Lock). macOS에서는 Caps Lock을 한/영 전환에 쓰는 일이 많아 기본은 끈다.
    pub caps_lock_shift: bool,
    /// 옛한글 모드: 원본의 Scroll Lock 자리(숫자열 옛한글 등)를 쓰고, 약어를 끄고, Shift+Y·U를 방점으로 쓴다.
    pub old_hangul: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            mode: Mode::Chord,
            timeout_ms: 40,
            use_keyup: true,
            abbreviations: true,
            compose: true,
            compose_inline: false,
            caps_lock_shift: false,
            old_hangul: false,
        }
    }
}

/// 옛한글 모드의 방점 (개발 규격 권장: Shift+Y, Shift+U). 원본 XML에는 없어 엔진이 넣는다.
const SINGLE_DOT_TONE_MARK: char = '\u{302E}';
const DOUBLE_DOT_TONE_MARK: char = '\u{302F}';

/// 기호 확장 준비 단계 (Shift+J 뒤). 101~103이 1~3단계다.
const STAGE_READY: u32 = 100;

/// 키 하나를 자판 데이터로 해석한 결과.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Input {
    Unit(Unit),
    /// 문자. `native`면 그 키의 QWERTY 문자 그대로라, 기호 조합에 쓰이지 않을 때는 앱에 넘긴다
    /// (단축키·자동 반복이 자연스럽다).
    Char {
        ch: char,
        native: bool,
    },
    /// 조합만 확정하고 키는 앱에 넘긴다 (엔진이 모르는 키, 수정키 조합).
    Pass,
    Backspace,
    Space,
    Escape,
    /// 기호 확장 단계 지정 (`C1|100~103`)
    Stage(u32),
    /// 아무것도 하지 않는 자리 (`0`), 또는 알 수 없는 명령
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
    /// 기호 확장 단계 `T`. 키 하나를 처리하면 0으로 돌아간다(한 번짜리). 명령이나 `(T=n, …)`이 다시 정할 수 있다.
    stage: u32,
    /// 기호 조합 상태와, 그 상태에서 보여 줄 중간 문자열
    compose_state: u32,
    compose_interim: String,
    /// `compose_interim`을 이미 확정했는가 (`compose_inline`)
    compose_committed: bool,
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
            stage: 0,
            compose_state: 0,
            compose_interim: String::new(),
            compose_committed: false,
        }
    }

    /// 지금의 기호 확장 단계 (0 = 평소, 100 = 준비, 101~103 = 1~3단계). 껍데기가 상태 표시에 쓸 수 있다.
    pub fn stage(&self) -> u32 {
        self.stage
    }

    pub fn handle_key(&mut self, ev: KeyEvent) -> Output {
        match ev.action {
            KeyAction::Down => self.key_down(ev),
            KeyAction::Up => self.key_up(ev),
        }
    }

    pub fn on_timer(&mut self, now_ms: u64) -> Output {
        let commit = match self.deadline {
            Some(d) if now_ms >= d => self.take_hangul(),
            _ => String::new(),
        };
        self.output(true, commit)
    }

    /// 포커스 이동 등으로 조합을 강제 확정한다.
    pub fn flush(&mut self) -> Output {
        let commit = self.take();
        self.pressed.clear();
        self.stage = 0;
        self.output(true, commit)
    }

    pub fn reset(&mut self) {
        self.syllable.clear();
        self.units.clear();
        self.pressed.clear();
        self.deadline = None;
        self.pending_space = false;
        self.stage = 0;
        self.clear_compose();
    }

    fn chord(&self) -> bool {
        self.options.mode == Mode::Chord
    }

    fn composing_hangul(&self) -> bool {
        !self.syllable.is_empty() || !self.units.is_empty()
    }

    fn abbreviations(&self) -> bool {
        self.options.abbreviations && !self.options.old_hangul
    }

    /// 키를 해석한다. 함께 돌려주는 값은 이 키를 처리한 뒤의 기호 확장 단계(`(T=n, …)`)다.
    fn resolve(&self, ev: &KeyEvent) -> (Input, Option<u32>) {
        if ev.modifiers & (modifiers::CTRL | modifiers::ALT | modifiers::CMD) != 0 {
            return (Input::Pass, None);
        }
        match ev.key {
            SemoeKey::BACKSPACE => return (Input::Backspace, None),
            SemoeKey::SPACE => return (Input::Space, None),
            SemoeKey::ESCAPE => return (Input::Escape, None),
            _ => {}
        }
        let shift = ev.modifiers & modifiers::SHIFT != 0;
        let Some(ch) = ev.key.qwerty(shift) else {
            return (Input::Pass, None);
        };
        if self.options.old_hangul && self.stage == 0 {
            let mark = match ch {
                'Y' => Some(SINGLE_DOT_TONE_MARK),
                'U' => Some(DOUBLE_DOT_TONE_MARK),
                _ => None,
            };
            if let Some(ch) = mark {
                return (Input::Char { ch, native: false }, None);
            }
        }
        let state = KeyState {
            stage: self.stage,
            caps_lock: self.options.caps_lock_shift && ev.modifiers & modifiers::CAPS != 0,
            scroll_lock: self.options.old_hangul,
        };
        match self.kb.action(ch, &state) {
            Some((DataAction::Unit { unit }, set)) => (Input::Unit(*unit), set),
            Some((DataAction::Char { ch: c }, set)) => (
                Input::Char {
                    ch: *c,
                    native: *c == ch,
                },
                set,
            ),
            Some((DataAction::Command { group: 1, n }, _)) => (Input::Stage(*n), None),
            Some((DataAction::Command { .. } | DataAction::Nothing, set)) => (Input::Ignore, set),
            None => (Input::Pass, None),
        }
    }

    fn key_down(&mut self, ev: KeyEvent) -> Output {
        let mut commit = String::new();
        // 타이머 이벤트가 늦게 와서 마감이 지났으면 먼저 확정한다.
        if self.deadline.is_some_and(|d| ev.time_ms >= d) {
            commit += &self.take_hangul();
        }

        let in_stage = self.stage != 0;
        let (input, set_stage) = self.resolve(&ev);
        // 기호 확장은 한 번짜리다: 명령이 단계를 정하지 않는 한, 키 하나를 처리하면 평소로 돌아간다.
        self.stage = match input {
            Input::Stage(n) => n,
            _ => set_stage.unwrap_or(0),
        };

        match input {
            Input::Escape if in_stage || !self.compose_interim.is_empty() => {
                // 기호 확장·기호 조합을 취소한다.
                self.stage = 0;
                self.clear_compose();
                self.output(true, commit)
            }
            Input::Backspace if self.composing_hangul() => {
                if self.chord() {
                    self.units.pop();
                } else {
                    self.syllable.undo(self.kb);
                }
                if self.composing_hangul() {
                    self.arm(ev.time_ms);
                } else {
                    self.deadline = None;
                    self.pending_space = false;
                }
                self.output(true, commit)
            }
            Input::Backspace if self.compose_committed => {
                // 확정한 중간 문자열은 앱이 지운다.
                self.clear_compose();
                self.output(false, commit)
            }
            Input::Backspace if !self.compose_interim.is_empty() || in_stage => {
                self.clear_compose();
                self.output(true, commit)
            }
            Input::Space if self.chord() && self.composing_hangul() && !ev.is_repeat => {
                // 스페이스 모아치기: 음절과 함께 누른 Space는 확정할 때 공백으로 붙인다.
                self.pending_space = true;
                self.pressed.insert(ev.key);
                self.output(true, commit)
            }
            Input::Unit(unit) => {
                commit += &self.take_compose();
                if !ev.is_repeat {
                    commit += &self.add(unit);
                    if self.chord() {
                        self.pressed.insert(ev.key);
                        self.arm(ev.time_ms);
                    }
                }
                self.output(true, commit)
            }
            Input::Char { ch, native } => {
                commit += &self.take_hangul();
                let mut replace = String::new();
                if self.options.compose && self.compose(ch, &mut commit, &mut replace) {
                    return Output {
                        replace,
                        ..self.output(true, commit)
                    };
                }
                if native {
                    self.output(false, commit)
                } else {
                    commit.push(ch);
                    self.output(true, commit)
                }
            }
            Input::Stage(_) | Input::Ignore => {
                commit += &self.take();
                self.output(true, commit)
            }
            Input::Pass | Input::Backspace | Input::Space | Input::Escape => {
                commit += &self.take();
                self.output(false, commit)
            }
        }
    }

    /// 기호 조합에 문자 하나를 넣는다. 조합이 받으면 `true` (확정할 결과는 `commit`에 붙인다).
    /// 지금 상태에 규칙이 없으면 중간 문자열을 확정하고 처음 상태에서 다시 본다.
    /// 앞서 확정한 중간 문자열을 바꿔야 하면 그 문자열을 `replace`에 넣는다.
    fn compose(&mut self, ch: char, commit: &mut String, replace: &mut String) -> bool {
        for attempt in 0..2 {
            if let Some(rule) = self.kb.compose(self.compose_state, ch) {
                let prev = std::mem::take(&mut self.compose_interim);
                if std::mem::take(&mut self.compose_committed) {
                    *replace = prev;
                }
                match rule.next {
                    Some(next) => {
                        self.compose_state = next;
                        self.compose_interim =
                            rule.interim.clone().unwrap_or_else(|| ch.to_string());
                        if self.options.compose_inline {
                            *commit += &self.compose_interim;
                            self.compose_committed = true;
                        }
                    }
                    None => {
                        *commit += rule.result.as_deref().unwrap_or_default();
                        self.compose_state = 0;
                    }
                }
                return true;
            }
            if attempt == 0 && self.compose_state != 0 {
                *commit += &self.take_compose();
            } else {
                break;
            }
        }
        false
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
            commit = self.take_hangul();
            self.syllable.add(self.kb, unit);
        }
        commit
    }

    fn key_up(&mut self, ev: KeyEvent) -> Output {
        if !self.chord() || !self.pressed.remove(&ev.key) {
            return self.output(false, String::new());
        }
        let commit = if self.options.use_keyup && self.pressed.is_empty() && self.composing_hangul()
        {
            self.take_hangul()
        } else {
            String::new()
        };
        self.output(true, commit)
    }

    fn arm(&mut self, t: u64) {
        // 원본 타이머 조건 `T<100 ? 40 : 0`: 기호 확장 중에는 타이머를 쓰지 않는다
        // (규격: Shift+J 뒤에는 일시적으로 이어치기처럼 동작).
        self.deadline =
            (self.chord() && self.stage < STAGE_READY).then_some(t + self.options.timeout_ms);
    }

    /// 조합 중인 한글을 화면에 보일 문자열로.
    fn render_hangul(&self) -> String {
        let abbr = self.abbreviations();
        if self.chord() {
            Syllable::segments(self.kb, &self.units)
                .iter()
                .map(|s| s.render(self.kb, abbr))
                .collect()
        } else {
            self.syllable.render(self.kb, abbr)
        }
    }

    /// 조합 중인 한글을 꺼내고 비운다. 눌린 키 집합은 유지한다(날개셋의 `v`와 같음).
    fn take_hangul(&mut self) -> String {
        let mut s = self.render_hangul();
        if self.pending_space {
            s.push(' ');
        }
        self.syllable.clear();
        self.units.clear();
        self.deadline = None;
        self.pending_space = false;
        s
    }

    /// 기호 조합 중간 문자열을 꺼내고 비운다. 이미 확정했으면 빈 문자열이다.
    fn take_compose(&mut self) -> String {
        let s = std::mem::take(&mut self.compose_interim);
        self.compose_state = 0;
        if std::mem::take(&mut self.compose_committed) {
            String::new()
        } else {
            s
        }
    }

    fn clear_compose(&mut self) {
        self.take_compose();
    }

    /// 조합 중인 것을 모두 꺼낸다 (한글은 기호보다 먼저 확정되므로 둘이 함께 있지는 않다).
    fn take(&mut self) -> String {
        let mut s = self.take_hangul();
        s += &self.take_compose();
        s
    }

    fn output(&self, consumed: bool, commit: String) -> Output {
        Output {
            consumed,
            commit,
            replace: String::new(),
            marked: self.render_hangul()
                + if self.compose_committed {
                    ""
                } else {
                    &self.compose_interim
                },
            deadline: self.deadline,
        }
    }
}
