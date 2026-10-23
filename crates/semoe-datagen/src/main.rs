//! 날개셋 세모이 XML → 세모이 엔진 데이터(`data/*.json`).
//!
//! ```text
//! semoe-datagen [--source <Semo-e_keyboard 경로>] [--data <data 경로>] [--check]
//! ```
//! - 입력: 원본 저장소의 두 XML, `data/src/unit-names.json`, `data/src/abbreviations-extra.json`
//! - 출력: `data/layout.json`, `unit-mix.json`, `abbreviations.json`, `compose.json`, `candidates.json`, `meta.json`
//! - `--check`: 파일을 쓰지 않고, 다시 만든 결과가 지금 파일과 같은지만 본다(CI용).

mod convert;
mod expr;
mod sheet;
mod units;
mod validate;
mod xml;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde::Serialize;
use serde_json::{Value, json};

const CHORD_XML: &str = "세모이 (한 번에 모아치기).xml";
const SEQUENTIAL_XML: &str = "세모이 (이어치기 & 중종성 모아치기).xml";

struct Args {
    source: PathBuf,
    data: PathBuf,
    check: bool,
}

fn args() -> Result<Args, String> {
    let mut a = Args {
        source: "../Semo-e_keyboard".into(),
        data: "data".into(),
        check: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--source" => a.source = it.next().ok_or("--source 값이 없음")?.into(),
            "--data" => a.data = it.next().ok_or("--data 값이 없음")?.into(),
            "--check" => a.check = true,
            "-h" | "--help" => {
                println!(
                    "사용: semoe-datagen [--source <Semo-e_keyboard>] [--data <data>] [--check]"
                );
                std::process::exit(0);
            }
            _ => return Err(format!("알 수 없는 인자 {arg}")),
        }
    }
    Ok(a)
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// 배열을 항목 하나에 한 줄씩 쓴다. 생성물을 커밋해 diff로 검토하기 쉽게 한다.
fn json_lines<T: Serialize>(items: &[T]) -> String {
    let body: Vec<String> = items
        .iter()
        .map(|i| serde_json::to_string(i).unwrap())
        .collect();
    format!("[\n{}\n]\n", body.join(",\n"))
}

fn git_commit(dir: &Path) -> String {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

fn mode_meta(root: &xml::Element) -> Value {
    let automata: Vec<Value> = root
        .find("AutomataTable")
        .map(|t| {
            t.children_named("Automata")
                .map(|a| json!({ "state": a.attrs.get("state"), "value": a.attrs.get("value") }))
                .collect()
        })
        .unwrap_or_default();
    let extra: Vec<Value> = root
        .find("Extra")
        .map(|e| {
            e.children
                .iter()
                .map(|c| json!({ "name": c.name, "attrs": c.attrs }))
                .collect()
        })
        .unwrap_or_default();
    json!({
        "automata": automata,
        "key_program": root.find("KeyProgramTable").is_some_and(|t| !t.children.is_empty()),
        "extra": extra,
    })
}

fn run() -> Result<(), String> {
    let a = args()?;
    let chord = xml::parse(&read(&a.source.join(CHORD_XML))?)?;
    let sequential = xml::parse(&read(&a.source.join(SEQUENTIAL_XML))?)?;

    // 두 입력 방식이 공유하는 표는 같아야 하나로 합칠 수 있다.
    for t in convert::SHARED_TABLES {
        if chord.find(t) != sequential.find(t) {
            return Err(format!(
                "두 XML의 <{t}>가 다름. 데이터를 하나로 합칠 수 없음"
            ));
        }
    }
    let candi = |r: &xml::Element| {
        r.find("ConvertHelper")
            .and_then(|h| h.find("UserCandiTable"))
            .cloned()
    };
    if candi(&chord) != candi(&sequential) {
        return Err("두 XML의 후보 표(ConvertHelper/UserCandiTable)가 다름".into());
    }

    let mut names = units::UnitNames::from_json(&read(&a.data.join("src/unit-names.json"))?)?;
    let virtuals = convert::virtual_units(&chord, &mut names)?;
    let mut c = convert::convert(&chord, &names, &virtuals)?;
    let extras = validate::extra_abbreviations(
        &read(&a.data.join("src/abbreviations-extra.json"))?,
        &c.layout,
        &c.mix,
    )?;
    let extra_count = extras.len();
    c.abbreviations.extend(extras);

    let report = validate::check(&c, &names);
    for w in &report.warnings {
        eprintln!("경고: {w}");
    }
    if !report.errors.is_empty() {
        for e in &report.errors {
            eprintln!("오류: {e}");
        }
        return Err(format!("검증 오류 {}건", report.errors.len()));
    }
    // 원본 설명서와 교차 검증 (키 → 결합 → 이름 대응 → 약어 키 전체 경로)
    let rows = sheet::read_rows(&a.source.join(sheet::SHEET))?;
    let sc = sheet::check(&rows, &c.mix, &c.abbreviations);
    if !sc.errors.is_empty() {
        for e in &sc.errors {
            eprintln!("오류: {e}");
        }
        return Err(format!(
            "약어 설명서와 불일치 {}건 (일치 {})",
            sc.errors.len(),
            sc.matched
        ));
    }
    if validate::vowel_shift(&c.layout).is_none() {
        return Err("모음시프트(가상 낱자 500)가 없음".into());
    }

    let reach = validate::reachable(&c.layout, &c.mix);
    let meta = json!({
        "_comment": "semoe-datagen이 만든 파일. 직접 고치지 말고 scripts/gen-data.sh로 다시 만든다.",
        "source": {
            "repository": "https://github.com/Sinseiki/Semo-e_keyboard",
            "commit": git_commit(&a.source),
            "files": [CHORD_XML, SEQUENTIAL_XML, sheet::SHEET],
            "license": "CC BY-SA 4.0",
        },
        "counts": {
            "keys": c.layout.keys.len(),
            "mix_rules": c.mix.len(),
            "abbreviations_original": c.abbreviations.len() - extra_count,
            "abbreviations_extra": extra_count,
            "compose_rules": c.compose.len(),
            "candidate_keys": c.candidates.len(),
            "abbreviations_checked_against_sheet": sc.matched,
            "reachable_units": reach.iter().map(|(s, set)| (units::slot_key(*s), set.len())).collect::<std::collections::BTreeMap<_, _>>(),
        },
        "unverified_unit_names": names.unverified,
        // 입력 방식마다 다른 부분. 동작은 엔진(semoe-core)이 구현하고, 여기에는 원본 식을 기록만 한다.
        "modes": { "chord": mode_meta(&chord), "sequential": mode_meta(&sequential) },
    });

    let outputs = [
        (
            "layout.json",
            serde_json::to_string_pretty(&json!({
                "virtual_units": c.layout.virtual_units,
                "keys": "@KEYS@",
            }))
            .unwrap()
            .replace("\"@KEYS@\"", json_lines(&c.layout.keys).trim_end())
                + "\n",
        ),
        ("unit-mix.json", json_lines(&c.mix)),
        ("abbreviations.json", json_lines(&c.abbreviations)),
        ("compose.json", json_lines(&c.compose)),
        ("candidates.json", json_lines(&c.candidates)),
        (
            "meta.json",
            serde_json::to_string_pretty(&meta).unwrap() + "\n",
        ),
    ];

    let mut stale = Vec::new();
    for (name, text) in &outputs {
        let path = a.data.join(name);
        if a.check {
            if fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
                stale.push(name.to_string());
            }
        } else {
            fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    if !stale.is_empty() {
        return Err(format!(
            "생성물이 최신이 아님: {} — scripts/gen-data.sh를 실행하세요",
            stale.join(", ")
        ));
    }
    println!(
        "{} {}: 키 {}, 결합 규칙 {}, 약어 {} (+추가 {}, 설명서 대조 {}), 기호 조합 {}, 후보 {} (경고 {})",
        if a.check { "확인" } else { "생성" },
        a.data.display(),
        c.layout.keys.len(),
        c.mix.len(),
        c.abbreviations.len() - extra_count,
        extra_count,
        sc.matched,
        c.compose.len(),
        c.candidates.len(),
        report.warnings.len(),
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("semoe-datagen: {e}");
            ExitCode::FAILURE
        }
    }
}
