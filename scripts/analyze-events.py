#!/usr/bin/env python3
"""Phase 0 이벤트 로그(~/Library/Logs/SemoeIME/events.tsv)를 앱별로 요약한다.

열 설명
- imk down: IMK가 넘긴 keyDown (반복 제외)
- 짝 없음: 탭 기록에서 짝을 찾지 못한 IMK keyDown (탭이 꺼져 있거나 늦은 경우)
- tap up: 탭이 기록해 엔진에 넣은 keyUp (탭이 꺼져 있으면 IMK keyUp)
- 유실: 눌린 뒤 keyUp이 오지 않은 키
- IMK 지연: 물리 keyDown 시각(탭) → IMK keyDown 수신 (ms)
- timer: 마감 시각 → 실제 발화 (ms)
사용: scripts/analyze-events.py [로그 경로]
"""
import csv
import sys
from collections import defaultdict
from pathlib import Path

path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path.home() / "Library/Logs/SemoeIME/events.tsv"


def pct(xs, p):
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(len(xs) * p))] if xs else float("nan")


def new():
    return {"down": 0, "nomatch": 0, "up": 0, "held": set(), "lost": 0, "imk": [], "timer": [], "tapdown": None}


apps = defaultdict(new)
with path.open() as f:
    for row in csv.DictReader(f, delimiter="\t"):
        a = apps[row["bundle_id"]]
        kind, key = row["kind"], row["key_code"]
        if kind == "tapdown":
            a["tapdown"] = float(row["event_ms"])  # 바로 뒤의 IMK down 행과 짝
        elif kind in ("down", "down-nomatch"):
            if row["repeat"] == "1":
                continue
            a["down"] += 1
            if kind == "down-nomatch":
                a["nomatch"] += 1
            elif a["tapdown"] is not None:
                a["imk"].append(float(row["uptime_ms"]) - a["tapdown"])
            a["tapdown"] = None
            if key in a["held"]:
                a["lost"] += 1
            a["held"].add(key)
        elif kind in ("up", "tapup"):
            if key in a["held"]:
                a["up"] += 1
                a["held"].discard(key)
        elif kind == "timer":
            a["timer"].append(float(row["extra"]))

cols = ["imk down", "짝 없음", "tap up", "유실", "IMK p50", "IMK p99", "timer p50", "timer p99", "timer max"]
print(f"{'app':40} " + " ".join(f"{c:>9}" for c in cols))
for app, a in sorted(apps.items()):
    if app in ("-", "?") or not a["down"]:
        continue
    lost = a["lost"] + len(a["held"])
    im, tm = a["imk"], a["timer"]
    vals = [a["down"], a["nomatch"], a["up"], lost]
    print(f"{app:40} " + " ".join(f"{v:9}" for v in vals) + " " +
          " ".join(f"{v:9.2f}" for v in [pct(im, .5), pct(im, .99), pct(tm, .5), pct(tm, .99), max(tm) if tm else float("nan")]))
