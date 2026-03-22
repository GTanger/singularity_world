#!/usr/bin/env python3
"""將梧桐大街擴充建築的資料夾名由 民宅一／民宅二／商鋪二／商鋪三 改為飾辭名稱。"""
from pathlib import Path

BASE = Path(__file__).resolve().parent.parent / "data" / "rooms" / "梧桐大街"
SEG_DIRS = [None, "梧桐大街一段", "梧桐大街二段", "梧桐大街三段", "梧桐大街四段", "梧桐大街五段", "梧桐大街六段", "梧桐大街七段", "梧桐大街八段", "梧桐大街九段"]

# (段 1-9, 舊資料夾名) -> 新資料夾名
FOLDER_RENAME = {
    (1, "民宅一"): "簷蔭宅",
    (1, "民宅二"): "桐影居",
    (2, "商鋪二"): "桐香閣",
    (2, "民宅一"): "綠脈軒",
    (2, "民宅二"): "沉香隅",
    (3, "商鋪二"): "玉光坊",
    (3, "民宅一"): "根鬚齋",
    (3, "民宅二"): "靜謐邸",
    (4, "民宅一"): "翠廊小築",
    (4, "民宅二"): "蔭深居",
    (5, "商鋪二"): "濃蔭棧",
    (6, "商鋪二"): "簷下肆",
    (6, "民宅一"): "溫潤宅",
    (6, "民宅二"): "葉影軒",
    (7, "商鋪二"): "翠脈鋪",
    (7, "民宅一"): "廊下居",
    (7, "民宅二"): "青脈宅",
    (8, "商鋪二"): "桐光肆",
    (8, "商鋪三"): "綠影坊",
    (8, "民宅二"): "幽蔭舍",
    (9, "民宅二"): "隅光邸",
    (9, "民宅三"): "葉脈居",
}

def main():
    for (seg, old_name), new_name in FOLDER_RENAME.items():
        seg_dir = BASE / SEG_DIRS[seg]
        old_path = seg_dir / old_name
        new_path = seg_dir / new_name
        if old_path.exists() and old_path.is_dir():
            if new_path.exists():
                print("Skip (exists):", new_path)
            else:
                old_path.rename(new_path)
                print(f"  {seg}段 {old_name} -> {new_name}")
        else:
            print("Missing:", old_path)

if __name__ == "__main__":
    main()
