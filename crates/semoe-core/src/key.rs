//! 플랫폼 중립 물리 키.

/// 물리 키 식별자. USB HID Keyboard/Keypad 페이지(0x07)의 usage ID를 그대로 쓴다.
///
/// macOS 가상 키 코드, Linux evdev 코드, Windows 스캔 코드 모두 HID usage로 바꿀 수 있으므로
/// 각 플랫폼 껍데기가 자기 키 코드를 이 값으로 변환해 넘긴다. 시스템 영문 배열(Dvorak 등)과
/// 무관하게 QWERTY 자리 기준으로 동작한다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SemoeKey(pub u16);

impl SemoeKey {
    /// 엔진이 모르는 키. 조합을 확정하고 앱에 넘긴다.
    pub const NONE: Self = Self(0);

    pub const A: Self = Self(0x04);
    pub const B: Self = Self(0x05);
    pub const C: Self = Self(0x06);
    pub const D: Self = Self(0x07);
    pub const E: Self = Self(0x08);
    pub const F: Self = Self(0x09);
    pub const G: Self = Self(0x0A);
    pub const H: Self = Self(0x0B);
    pub const I: Self = Self(0x0C);
    pub const J: Self = Self(0x0D);
    pub const K: Self = Self(0x0E);
    pub const L: Self = Self(0x0F);
    pub const M: Self = Self(0x10);
    pub const N: Self = Self(0x11);
    pub const O: Self = Self(0x12);
    pub const P: Self = Self(0x13);
    pub const Q: Self = Self(0x14);
    pub const R: Self = Self(0x15);
    pub const S: Self = Self(0x16);
    pub const T: Self = Self(0x17);
    pub const U: Self = Self(0x18);
    pub const V: Self = Self(0x19);
    pub const W: Self = Self(0x1A);
    pub const X: Self = Self(0x1B);
    pub const Y: Self = Self(0x1C);
    pub const Z: Self = Self(0x1D);

    pub const RETURN: Self = Self(0x28);
    pub const ESCAPE: Self = Self(0x29);
    pub const BACKSPACE: Self = Self(0x2A);
    pub const TAB: Self = Self(0x2B);
    pub const SPACE: Self = Self(0x2C);
    pub const SEMICOLON: Self = Self(0x33);
    pub const COMMA: Self = Self(0x36);
    pub const PERIOD: Self = Self(0x37);
    pub const SLASH: Self = Self(0x38);
    pub const DIGIT1: Self = Self(0x1E);
}

impl SemoeKey {
    /// QWERTY 배열에서 이 키(와 Shift)로 나오는 ASCII 문자. 날개셋 `KeyTable`은 이 문자로 키를 찾는다.
    pub fn qwerty(self, shift: bool) -> Option<char> {
        let (lower, upper) = match self.0 {
            u @ 0x04..=0x1D => {
                let c = (b'a' + (u - 0x04) as u8) as char;
                (c, c.to_ascii_uppercase())
            }
            0x1E => ('1', '!'),
            0x1F => ('2', '@'),
            0x20 => ('3', '#'),
            0x21 => ('4', '$'),
            0x22 => ('5', '%'),
            0x23 => ('6', '^'),
            0x24 => ('7', '&'),
            0x25 => ('8', '*'),
            0x26 => ('9', '('),
            0x27 => ('0', ')'),
            0x2D => ('-', '_'),
            0x2E => ('=', '+'),
            0x2F => ('[', '{'),
            0x30 => (']', '}'),
            0x31 => ('\\', '|'),
            0x33 => (';', ':'),
            0x34 => ('\'', '"'),
            0x35 => ('`', '~'),
            0x36 => (',', '<'),
            0x37 => ('.', '>'),
            0x38 => ('/', '?'),
            _ => return None,
        };
        Some(if shift { upper } else { lower })
    }

    /// `qwerty`의 반대: ASCII 문자를 내는 키와 Shift 여부. 테스트와 재생 하네스에서 쓴다.
    pub fn from_qwerty(c: char) -> Option<(SemoeKey, bool)> {
        (0x04..=0x38).map(SemoeKey).find_map(|k| {
            if k.qwerty(false) == Some(c) {
                Some((k, false))
            } else if k.qwerty(true) == Some(c) {
                Some((k, true))
            } else {
                None
            }
        })
    }
}

/// 수정키 비트. C ABI의 `SEMOE_MOD_*`와 같은 값이다.
pub mod modifiers {
    pub const SHIFT: u32 = 1 << 0;
    pub const CAPS: u32 = 1 << 1;
    pub const CTRL: u32 = 1 << 2;
    pub const ALT: u32 = 1 << 3;
    pub const CMD: u32 = 1 << 4;
}
