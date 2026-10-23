//! 세모이(세벌식 모아치기 e) 입력 엔진. 플랫폼 중립 순수 로직.

pub mod data;
pub mod engine;
pub mod hangul;
pub mod key;
pub mod keyboard;
pub mod syllable;

pub use engine::{Engine, KeyAction, KeyEvent, Mode, Options, Output};
pub use key::SemoeKey;
pub use keyboard::Keyboard;
