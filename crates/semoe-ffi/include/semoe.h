#ifndef SEMOE_H
#define SEMOE_H

// cbindgen이 crates/semoe-ffi/src/lib.rs에서 만든 파일. 직접 고치지 말 것.

#include <stdbool.h>
#include <stdint.h>

#define SEMOE_MOD_SHIFT (1 << 0)

#define SEMOE_MOD_CAPS (1 << 1)

#define SEMOE_MOD_CTRL (1 << 2)

#define SEMOE_MOD_ALT (1 << 3)

#define SEMOE_MOD_CMD (1 << 4)

// 엔진 인스턴스 (불투명). 입력 세션마다 하나씩 만든다.
typedef struct SemoeEngine SemoeEngine;

typedef struct {
  // false면 키를 앱에 그대로 넘긴다. commit은 그보다 먼저 넣는다.
  bool consumed;
  // 확정 문자열(UTF-8). 없으면 NULL. 다음 호출 전까지만 유효하다.
  const char *commit;
  // 조합 중 문자열(UTF-8). 빈 문자열이면 조합 없음. 다음 호출 전까지만 유효하다.
  const char *marked;
  // 이 시각(ms)에 semoe_on_timer를 불러야 한다. -1이면 타이머 해제.
  int64_t deadline_ms;
} SemoeOutput;

// 키 동작 (`SEMOE_KEY_DOWN`, `SEMOE_KEY_UP`)
typedef uint32_t SemoeKeyAction;

typedef struct {
  // USB HID usage (Keyboard 페이지). 0이면 엔진이 모르는 키
  uint16_t key;
  SemoeKeyAction action;
  // `SEMOE_MOD_*`
  uint32_t modifiers;
  bool is_repeat;
  // 단조 증가 시각 (ms)
  uint64_t time_ms;
} SemoeKeyEvent;

#define SEMOE_KEY_DOWN 0

#define SEMOE_KEY_UP 1

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

SemoeEngine *semoe_engine_new(void);

// # Safety
// `engine`은 `semoe_engine_new`가 돌려준 포인터이거나 NULL이어야 하며, 한 번만 해제한다.
void semoe_engine_free(SemoeEngine *engine);

SemoeOutput semoe_handle_key(SemoeEngine *engine, SemoeKeyEvent ev);

SemoeOutput semoe_on_timer(SemoeEngine *engine, uint64_t now_ms);

SemoeOutput semoe_flush(SemoeEngine *engine);

void semoe_reset(SemoeEngine *engine);

// 옵션 (알 수 없는 키나 값이면 `false`)
// - `mode`: `chord`(한 번에 모아치기) / `sequential`(이어치기·중종성 모아치기). 바꾸면 조합을 버린다.
// - `timeout_ms`: 1~1000
// - `keyup`, `abbrev`, `compose`, `caps_lock_shift`, `old_hangul`: `on` / `off`
//
// # Safety
// `key`와 `value`는 NUL로 끝나는 유효한 C 문자열이거나 NULL이어야 한다.
bool semoe_set_option(SemoeEngine *engine,
                      const char *key,
                      const char *value);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* SEMOE_H */
