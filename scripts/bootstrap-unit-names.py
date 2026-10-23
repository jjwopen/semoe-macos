#!/usr/bin/env python3
"""날개셋 낱자 이름 → 유니코드 첫가끝 자모 대응표 초안을 만든다 (한 번만 쓰는 도구).

날개셋 이름은 유니코드 자모 이름을 줄여 쓴 꼴이다(예: ND = NIEUN-TIKEUT, BQ = KAPYEOUNPIEUP,
IYE = I-YE). 이 규칙으로 초안을 만들고, 결과(data/src/unit-names.json)는 사람이 검토해 고친 뒤
semoe-datagen이 도달 가능성 검사로 검증한다. 이후에는 JSON을 직접 고친다.

사용: scripts/bootstrap-unit-names.py <세모이 XML> > data/src/unit-names.json
"""
import json
import re
import sys
import unicodedata

xml = open(sys.argv[1], encoding="utf-8").read()
names = {"cho": set(), "jung": set(), "jong": set()}
slot_of = {"CHO": "cho", "JUNG": "jung", "JONG": "jong"}
for u, a, b, t in re.findall(r'<UnitMix unit="(\w+)" a="(\w+)" b="(\w+)" to="(\w+)"/>', xml):
    names[slot_of[u]] |= {a, b, t}
for v in re.findall(r'H\d\|(_?[A-Za-z][A-Za-z_]*)', xml):
    if v.startswith("_"):
        names["jong"].add(v[1:] + ("_" if len(v) == 2 else ""))
for s in names:
    names[s].discard("500")  # 가상 낱자 (VirtualUnitTable)

# 유니코드 자모 이름 → 문자
uni = {"cho": {}, "jung": {}, "jong": {}}
ranges = [(0x1100, 0x11FF), (0xA960, 0xA97C), (0xD7B0, 0xD7FB)]
for lo, hi in ranges:
    for cp in range(lo, hi + 1):
        n = unicodedata.name(chr(cp), "")
        for prefix, s in (("HANGUL CHOSEONG ", "cho"), ("HANGUL JUNGSEONG ", "jung"), ("HANGUL JONGSEONG ", "jong")):
            if n.startswith(prefix):
                uni[s][n[len(prefix):]] = chr(cp)

CONS = {"G": "KIYEOK", "N": "NIEUN", "D": "TIKEUT", "R": "RIEUL", "M": "MIEUM", "B": "PIEUP", "S": "SIOS",
        "Q": "IEUNG", "J": "CIEUC", "C": "CHIEUCH", "K": "KHIEUKH", "T": "THIEUTH", "P": "PHIEUPH", "H": "HIEUH",
        "Z": "PANSIOS", "V": "YESIEUNG", "X": "YEORINHIEUH"}
VOW = {"A": "A", "AE": "AE", "YA": "YA", "YAE": "YAE", "EO": "EO", "E": "E", "YEO": "YEO", "YE": "YE", "O": "O",
       "WA": "WA", "WAE": "WAE", "OI": "OE", "YO": "YO", "U": "U", "UEO": "WEO", "WE": "WE", "WI": "WI", "YU": "YU",
       "EU": "EU", "EUI": "YI", "I": "I", "F": "ARAEA", "FF": "SSANGARAEA"}
# 확인되지 않은 추정: w가 뒤에 붙으면 치두음, 앞에 붙으면 정치음으로 본다.
UNVERIFIED = []


def cons_parts(name):
    """자음 이름을 유니코드 이름 조각 목록으로. 예: BSG → [PIEUP, SIOS, KIYEOK]"""
    s, parts = name.rstrip("_"), []
    prefix = ""
    if s.startswith("w"):
        prefix, s = "CEONGCHIEUM", s[1:]
    elif s.endswith("w"):
        prefix, s = "CHITUEUM", s[:-1]
    i = 0
    while i < len(s):
        if i + 1 < len(s) and s[i] == s[i + 1]:
            parts.append("SSANG" + CONS[s[i]])
            i += 2
        else:
            parts.append(CONS[s[i]])
            i += 1
        # 입술소리 뒤의 Q는 가벼운(순경음) 표시
        if i < len(s) and s[i] == "Q" and parts[-1].replace("SSANG", "") in ("PIEUP", "MIEUM", "PHIEUPH", "RIEUL") \
                and (i + 1 == len(s)):
            parts[-1] = "KAPYEOUN" + parts[-1]
            i += 1
    if prefix:
        UNVERIFIED.append(name)
        parts = [prefix + p for p in parts]
    return parts


def vow_parts(name):
    s, parts = name.rstrip("_"), []
    while s:
        for n in range(len(s), 0, -1):
            if s[:n] in VOW:
                parts.append(VOW[s[:n]])
                s = s[n:]
                break
        else:
            raise KeyError(name)
    return parts


out = {"_comment": "날개셋 낱자 이름 → 유니코드 첫가끝 자모. scripts/bootstrap-unit-names.py로 초안을 만든 뒤 "
                   "검토해 고친 수작업 원본이다. semoe-datagen이 도달 가능성으로 검증한다.",
       "cho": {}, "jung": {}, "jong": {}, "unverified": []}
missing = []
for s in ("cho", "jung", "jong"):
    for name in sorted(names[s]):
        try:
            parts = vow_parts(name) if s == "jung" else cons_parts(name)
            key = "-".join(parts)
            out[s][name] = uni[s][key]
        except KeyError:
            missing.append(f"{s}:{name}")
out["unverified"] = sorted(set(UNVERIFIED))
json.dump(out, sys.stdout, ensure_ascii=False, indent=2)
print()
if missing:
    print("대응 못 함: " + " ".join(missing), file=sys.stderr)
