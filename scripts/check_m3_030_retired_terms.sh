#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

PATTERN='短市|岸邊短市|短市包貨單|短市外谷地路|短市人群|小市'

PATHS=(
  "data/world/singularity_world_skeleton.json"
  "data/world/tradable_materials_p0.json"
  "web/sw-room-desc.js"
  "src/game/player_scene.rs"
  "src/server/handler/movement.rs"
  "src/server/action/object.rs"
  "src/server/action/look.rs"
  "src/server/m3_029.rs"
  "src/server/simulation_loop/movement.rs"
)

cd "$ROOT"

existing=()
for path in "${PATHS[@]}"; do
  if [[ -e "$path" ]]; then
    existing+=("$path")
  fi
done

if (( ${#existing[@]} == 0 )); then
  echo "M3-030 retired-term check: no target files found" >&2
  exit 2
fi

if rg -n --color=never "$PATTERN" "${existing[@]}"; then
  echo
  echo "M3-030 retired-term check FAILED: live player-facing paths still contain retired naming." >&2
  echo "Replace approved terms first; do not count docs/candidate notes as runtime proof." >&2
  exit 1
fi

echo "M3-030 retired-term check passed: no retired naming in live player-facing paths."
