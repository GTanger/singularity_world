#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
  cat >&2 <<'USAGE'
Usage: scripts/preview_m3_030_candidate.sh A|B|C

Dry-run only. Prints the approved-name mapping for M3-030 ferry-local naming.
It does not edit runtime files.
USAGE
}

if [[ $# -ne 1 ]]; then
  usage
  exit 2
fi

candidate="$1"

case "$candidate" in
  A)
    place="岸邊攤棚"
    road="攤棚外谷地路"
    crowd="攤邊人群"
    near_side="攤棚那頭"
    edge="棚腳"
    price_board="攤棚價牌"
    goods_basket="貨籃"
    ;;
  B)
    place="渡口貨攤"
    road="貨攤外谷地路"
    crowd="看貨人群"
    near_side="貨攤那頭"
    edge="貨攤棚腳"
    price_board="貨攤價牌"
    goods_basket="小件貨籃"
    ;;
  C)
    place="渡口市集"
    road="市集外谷地路"
    crowd="看貨人群"
    near_side="市集那頭"
    edge="市集棚腳"
    price_board="市集價牌"
    goods_basket="小件貨籃"
    ;;
  *)
    usage
    exit 2
    ;;
esac

board="收貨牌"

cat <<EOF
M3-030 candidate $candidate dry-run mapping

Core names:
  岸邊短市      -> $place
  短市包貨單    -> $board
  短市外谷地路  -> $road
  短市人群      -> $crowd

Phrase rules:
  短市那頭      -> $near_side
  短市棚腳      -> $edge
  短市價牌      -> $price_board
  短貨籃        -> $goods_basket
  短貨          -> 小件貨 / 零散貨 (choose by sentence)
  短市的人      -> ${place}邊的人 / 貨攤邊的人 (choose by sentence)
  往短市方向    -> 往${place}那邊
  短市送來      -> ${place}送來 / 收貨處送來 (choose by sentence)
  短市收滿      -> 收貨牌翻面 / 收貨處收滿 (choose by sentence)

Do not change:
  M4/S/E internal ids
  coordinates
  object action kinds
  NPC entity ids
  money/inventory/source-stock rules
EOF

echo
echo "Current retired-term hits in live target files:"
"$ROOT/scripts/check_m3_030_retired_terms.sh" || true
