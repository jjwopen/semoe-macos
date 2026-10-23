#!/bin/bash
# 원본 저장소(../Semo-e_keyboard)의 날개셋 XML로 data/*.json을 다시 만든다.
#   --check  파일을 쓰지 않고 최신인지만 확인 (CI)
#   SEMOE_SOURCE  원본 저장소 경로 (기본 ../Semo-e_keyboard)
set -euo pipefail
cd "$(dirname "$0")/.."
cargo run --quiet --release -p semoe-datagen -- --source "${SEMOE_SOURCE:-../Semo-e_keyboard}" --data data "$@"
