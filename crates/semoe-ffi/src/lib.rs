//! 세모이 엔진 C ABI (계획서 4.2). 헤더 `include/semoe.h`는 cbindgen이 이 파일에서 만든다
//! (`scripts/build-rust-universal.sh`, 설정 `cbindgen.toml`).
//!
//! - 반환하는 문자열은 엔진이 소유하며 같은 엔진에 대한 다음 호출 전까지만 유효하다.
//! - 모든 함수는 panic을 잡아 키를 앱에 넘기는 출력(`consumed = false`)으로 바꾼다.

use std::ffi::{CStr, CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

use semoe_core::{Engine, KeyAction, KeyEvent, Mode, Output, SemoeKey};

/// 키 동작 (`SEMOE_KEY_DOWN`, `SEMOE_KEY_UP`)
pub type SemoeKeyAction = u32;
pub const SEMOE_KEY_DOWN: SemoeKeyAction = 0;
pub const SEMOE_KEY_UP: SemoeKeyAction = 1;

// 수정키 비트. semoe_core::key::modifiers와 같은 값이다(테스트로 확인).
pub const SEMOE_MOD_SHIFT: u32 = 1 << 0;
pub const SEMOE_MOD_CAPS: u32 = 1 << 1;
pub const SEMOE_MOD_CTRL: u32 = 1 << 2;
pub const SEMOE_MOD_ALT: u32 = 1 << 3;
pub const SEMOE_MOD_CMD: u32 = 1 << 4;

/// 엔진 인스턴스 (불투명). 입력 세션마다 하나씩 만든다.
pub struct SemoeEngine {
    engine: Engine,
    commit: CString,
    replace: CString,
    marked: CString,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SemoeKeyEvent {
    /// USB HID usage (Keyboard 페이지). 0이면 엔진이 모르는 키
    pub key: u16,
    pub action: SemoeKeyAction,
    /// `SEMOE_MOD_*`
    pub modifiers: u32,
    pub is_repeat: bool,
    /// 단조 증가 시각 (ms)
    pub time_ms: u64,
}

#[repr(C)]
#[derive(Debug)]
pub struct SemoeOutput {
    /// false면 키를 앱에 그대로 넘긴다. commit은 그보다 먼저 넣는다.
    pub consumed: bool,
    /// 확정 문자열(UTF-8). 없으면 NULL. 다음 호출 전까지만 유효하다.
    pub commit: *const c_char,
    /// commit을 넣기 전에 커서 바로 앞에서 지울 문자열(UTF-8). 없으면 NULL. 다음 호출 전까지만 유효하다.
    pub replace: *const c_char,
    /// 조합 중 문자열(UTF-8). 빈 문자열이면 조합 없음. 다음 호출 전까지만 유효하다.
    pub marked: *const c_char,
    /// 이 시각(ms)에 semoe_on_timer를 불러야 한다. -1이면 타이머 해제.
    pub deadline_ms: i64,
}

const EMPTY: &CStr = c"";

impl SemoeOutput {
    fn passthrough() -> Self {
        Self {
            consumed: false,
            commit: ptr::null(),
            replace: ptr::null(),
            marked: EMPTY.as_ptr(),
            deadline_ms: -1,
        }
    }
}

impl SemoeEngine {
    fn export(&mut self, out: Output) -> SemoeOutput {
        // 엔진 출력에는 NUL이 들어갈 수 없지만, 만약을 대비해 실패 시 빈 문자열로 둔다.
        self.commit = CString::new(out.commit).unwrap_or_default();
        self.replace = CString::new(out.replace).unwrap_or_default();
        self.marked = CString::new(out.marked).unwrap_or_default();
        let non_empty = |s: &CString| {
            if s.is_empty() {
                ptr::null()
            } else {
                s.as_ptr()
            }
        };
        SemoeOutput {
            consumed: out.consumed,
            commit: non_empty(&self.commit),
            replace: non_empty(&self.replace),
            marked: self.marked.as_ptr(),
            deadline_ms: out.deadline.map_or(-1, |d| d.min(i64::MAX as u64) as i64),
        }
    }
}

/// 엔진 포인터를 검사하고 panic을 막은 채 `f`를 실행한다.
fn with_engine(engine: *mut SemoeEngine, f: impl FnOnce(&mut Engine) -> Output) -> SemoeOutput {
    if engine.is_null() {
        return SemoeOutput::passthrough();
    }
    // SAFETY: 호출자는 semoe_engine_new가 돌려준, 아직 해제하지 않은 포인터를 넘긴다.
    let handle = unsafe { &mut *engine };
    match catch_unwind(AssertUnwindSafe(|| f(&mut handle.engine))) {
        Ok(out) => handle.export(out),
        Err(_) => {
            handle.engine.reset();
            SemoeOutput::passthrough()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn semoe_engine_new() -> *mut SemoeEngine {
    catch_unwind(|| {
        Box::into_raw(Box::new(SemoeEngine {
            engine: Engine::new(),
            commit: CString::default(),
            replace: CString::default(),
            marked: CString::default(),
        }))
    })
    .unwrap_or(ptr::null_mut())
}

/// # Safety
/// `engine`은 `semoe_engine_new`가 돌려준 포인터이거나 NULL이어야 하며, 한 번만 해제한다.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn semoe_engine_free(engine: *mut SemoeEngine) {
    if !engine.is_null() {
        // SAFETY: 위 계약에 따라 Box::into_raw로 만든 유일한 포인터다.
        drop(unsafe { Box::from_raw(engine) });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn semoe_handle_key(engine: *mut SemoeEngine, ev: SemoeKeyEvent) -> SemoeOutput {
    with_engine(engine, |e| {
        let action = if ev.action == SEMOE_KEY_UP {
            KeyAction::Up
        } else {
            KeyAction::Down
        };
        e.handle_key(KeyEvent {
            key: SemoeKey(ev.key),
            action,
            modifiers: ev.modifiers,
            is_repeat: ev.is_repeat,
            time_ms: ev.time_ms,
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn semoe_on_timer(engine: *mut SemoeEngine, now_ms: u64) -> SemoeOutput {
    with_engine(engine, |e| e.on_timer(now_ms))
}

#[unsafe(no_mangle)]
pub extern "C" fn semoe_flush(engine: *mut SemoeEngine) -> SemoeOutput {
    with_engine(engine, Engine::flush)
}

#[unsafe(no_mangle)]
pub extern "C" fn semoe_reset(engine: *mut SemoeEngine) {
    with_engine(engine, |e| {
        e.reset();
        Output::default()
    });
}

/// 옵션 (알 수 없는 키나 값이면 `false`)
/// - `mode`: `chord`(한 번에 모아치기) / `sequential`(이어치기·중종성 모아치기). 바꾸면 조합을 버린다.
/// - `timeout_ms`: 1~1000
/// - `keyup`, `abbrev`, `compose`, `compose_inline`, `caps_lock_shift`, `old_hangul`: `on` / `off`
///
/// # Safety
/// `key`와 `value`는 NUL로 끝나는 유효한 C 문자열이거나 NULL이어야 한다.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn semoe_set_option(
    engine: *mut SemoeEngine,
    key: *const c_char,
    value: *const c_char,
) -> bool {
    if engine.is_null() || key.is_null() || value.is_null() {
        return false;
    }
    // SAFETY: 위 계약.
    let (key, value) = unsafe { (CStr::from_ptr(key), CStr::from_ptr(value)) };
    let (Ok(key), Ok(value)) = (key.to_str(), value.to_str()) else {
        return false;
    };
    // SAFETY: with_engine과 같은 계약.
    let engine = unsafe { &mut (*engine).engine };
    let flag = |v: &str| match v {
        "on" => Some(true),
        "off" => Some(false),
        _ => None,
    };
    let opts = &mut engine.options;
    match key {
        "timeout_ms" => match value.parse::<u64>() {
            Ok(ms @ 1..=1000) => {
                opts.timeout_ms = ms;
                true
            }
            _ => false,
        },
        "mode" => {
            let mode = match value {
                "chord" => Mode::Chord,
                "sequential" => Mode::Sequential,
                _ => return false,
            };
            if opts.mode != mode {
                opts.mode = mode;
                // 조합 상태는 방식마다 다르다. 호출자가 먼저 semoe_flush로 확정해 둔다.
                engine.reset();
            }
            true
        }
        "keyup" => flag(value).map(|v| opts.use_keyup = v).is_some(),
        "abbrev" => flag(value).map(|v| opts.abbreviations = v).is_some(),
        "compose" => flag(value).map(|v| opts.compose = v).is_some(),
        "compose_inline" => flag(value).map(|v| opts.compose_inline = v).is_some(),
        "caps_lock_shift" => flag(value).map(|v| opts.caps_lock_shift = v).is_some(),
        "old_hangul" => flag(value).map(|v| opts.old_hangul = v).is_some(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(p: *const c_char) -> Option<String> {
        (!p.is_null()).then(|| unsafe { CStr::from_ptr(p) }.to_str().unwrap().to_owned())
    }

    fn key(key: u16, action: u32, t: u64) -> SemoeKeyEvent {
        SemoeKeyEvent {
            key,
            action,
            modifiers: 0,
            is_repeat: false,
            time_ms: t,
        }
    }

    #[test]
    fn gan_through_c_abi() {
        let e = semoe_engine_new();
        for (i, k) in [0x0E, 0x09, 0x16].into_iter().enumerate() {
            let out = semoe_handle_key(e, key(k, SEMOE_KEY_DOWN, i as u64));
            assert!(out.consumed);
            assert_eq!(out.deadline_ms, i as i64 + 40);
        }
        semoe_handle_key(e, key(0x0E, SEMOE_KEY_UP, 10));
        semoe_handle_key(e, key(0x09, SEMOE_KEY_UP, 10));
        let out = semoe_handle_key(e, key(0x16, SEMOE_KEY_UP, 11));
        assert_eq!(s(out.commit).as_deref(), Some("간"));
        assert_eq!(s(out.marked).as_deref(), Some(""));
        assert_eq!(out.deadline_ms, -1);
        unsafe { semoe_engine_free(e) };
    }

    #[test]
    fn modifier_bits_match_core() {
        use semoe_core::key::modifiers as m;
        assert_eq!(
            [
                SEMOE_MOD_SHIFT,
                SEMOE_MOD_CAPS,
                SEMOE_MOD_CTRL,
                SEMOE_MOD_ALT,
                SEMOE_MOD_CMD
            ],
            [m::SHIFT, m::CAPS, m::CTRL, m::ALT, m::CMD]
        );
    }

    #[test]
    fn null_engine_passes_key_through() {
        let out = semoe_handle_key(ptr::null_mut(), key(0x0E, SEMOE_KEY_DOWN, 0));
        assert!(!out.consumed);
        assert!(out.commit.is_null());
    }

    #[test]
    fn options() {
        let e = semoe_engine_new();
        unsafe {
            assert!(semoe_set_option(e, c"timeout_ms".as_ptr(), c"60".as_ptr()));
            assert!(!semoe_set_option(e, c"timeout_ms".as_ptr(), c"0".as_ptr()));
            assert!(semoe_set_option(e, c"keyup".as_ptr(), c"off".as_ptr()));
            assert!(!semoe_set_option(e, c"nope".as_ptr(), c"1".as_ptr()));
            assert!(semoe_set_option(e, c"abbrev".as_ptr(), c"off".as_ptr()));
            assert!(!semoe_set_option(e, c"abbrev".as_ptr(), c"maybe".as_ptr()));
            assert!(!semoe_set_option(e, c"mode".as_ptr(), c"other".as_ptr()));
        }
        let out = semoe_handle_key(e, key(0x0E, SEMOE_KEY_DOWN, 0));
        assert_eq!(out.deadline_ms, 60);
        let out = semoe_handle_key(e, key(0x0E, SEMOE_KEY_UP, 5));
        assert!(out.commit.is_null()); // keyup off
        unsafe { semoe_engine_free(e) };
    }

    #[test]
    fn inline_compose_through_c_abi() {
        let e = semoe_engine_new();
        unsafe {
            assert!(semoe_set_option(
                e,
                c"compose_inline".as_ptr(),
                c"on".as_ptr()
            ))
        };
        let shift = |k, t| SemoeKeyEvent {
            modifiers: SEMOE_MOD_SHIFT,
            ..key(k, SEMOE_KEY_DOWN, t)
        };
        let out = semoe_handle_key(e, shift(0x38, 0)); // ?
        assert_eq!(s(out.commit).as_deref(), Some("?"));
        assert!(out.replace.is_null());
        assert_eq!(s(out.marked).as_deref(), Some(""));
        let out = semoe_handle_key(e, shift(0x09, 10)); // △
        assert_eq!(s(out.replace).as_deref(), Some("?"));
        assert_eq!(s(out.commit).as_deref(), Some("까? "));
        unsafe { semoe_engine_free(e) };
    }

    #[test]
    fn sequential_mode_through_c_abi() {
        let e = semoe_engine_new();
        unsafe {
            assert!(semoe_set_option(
                e,
                c"mode".as_ptr(),
                c"sequential".as_ptr()
            ))
        };
        // k(초ㄱ) f(ㅏ): 키뗌·타이머 없이 조합이 이어진다
        for (i, k) in [0x0E, 0x09].into_iter().enumerate() {
            let out = semoe_handle_key(e, key(k, SEMOE_KEY_DOWN, i as u64 * 100));
            assert_eq!(out.deadline_ms, -1);
            semoe_handle_key(e, key(k, SEMOE_KEY_UP, i as u64 * 100 + 30));
        }
        let out = semoe_flush(e);
        assert_eq!(s(out.commit).as_deref(), Some("가"));
        unsafe { semoe_engine_free(e) };
    }
}
