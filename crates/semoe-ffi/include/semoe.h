// 세모이 엔진 C ABI. crates/semoe-ffi/src/lib.rs와 함께 고친다.
// (Phase 0에서는 손으로 관리하고, Phase 3에서 cbindgen 생성으로 바꾼다.)
#ifndef SEMOE_H
#define SEMOE_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct SemoeEngine SemoeEngine;

typedef uint32_t SemoeKeyAction;
static const SemoeKeyAction SEMOE_KEY_DOWN = 0;
static const SemoeKeyAction SEMOE_KEY_UP = 1;

static const uint32_t SEMOE_MOD_SHIFT = 1u << 0;
static const uint32_t SEMOE_MOD_CAPS = 1u << 1;
static const uint32_t SEMOE_MOD_CTRL = 1u << 2;
static const uint32_t SEMOE_MOD_ALT = 1u << 3;
static const uint32_t SEMOE_MOD_CMD = 1u << 4;

typedef struct {
    uint16_t key;            // USB HID usage (Keyboard 페이지), 0이면 엔진이 모르는 키
    SemoeKeyAction action;
    uint32_t modifiers;      // SEMOE_MOD_*
    bool is_repeat;
    uint64_t time_ms;        // 단조 증가 시각
} SemoeKeyEvent;

typedef struct {
    bool consumed;           // false면 앱으로 그대로 전달 (commit은 먼저 넣는다)
    const char *commit;      // 확정 문자열(UTF-8), 없으면 NULL
    const char *marked;      // 조합 중 문자열, 빈 문자열이면 조합 없음
    int64_t deadline_ms;     // 이 시각에 semoe_on_timer 호출, -1이면 타이머 해제
} SemoeOutput;

SemoeEngine *semoe_engine_new(void);
void semoe_engine_free(SemoeEngine *engine);
SemoeOutput semoe_handle_key(SemoeEngine *engine, SemoeKeyEvent event);
SemoeOutput semoe_on_timer(SemoeEngine *engine, uint64_t now_ms);
SemoeOutput semoe_flush(SemoeEngine *engine);
void semoe_reset(SemoeEngine *engine);
// 옵션: mode = chord | sequential (바꾸면 조합을 버리므로 먼저 semoe_flush), timeout_ms = 1~1000,
//       keyup | abbrev | caps_lock_shift | old_hangul = on | off
bool semoe_set_option(SemoeEngine *engine, const char *key, const char *value);

#ifdef __cplusplus
}
#endif

#endif
