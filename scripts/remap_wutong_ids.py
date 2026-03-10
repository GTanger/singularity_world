#!/usr/bin/env python3
"""
梧桐大街房間 ID 改為飛霜格式：第一層 wust01_～wust09_，第二層 hold01/home01/shop01，第三層自編。
替換 data/rooms/梧桐大街 下所有 JSON 的 id / to / move_to_room_id，並重新命名檔案。
"""
import json
import os
import re
from pathlib import Path

BASE = Path(__file__).resolve().parent.parent / "data" / "rooms" / "梧桐大街"

# 舊 ID → 新 ID（先列長的，避免部分替換錯誤）
ID_MAP = {
    # 一段：長青公所 hold01，內部 01～15，子景點
    "wutong_changqing_gongsuo": "wust01_hold01_main",
    "wutong_quanxiang_yu": "wust01_hold01_quanxiang",
    "wutong_guigen_she": "wust01_hold01_guigen",
    "wutong_moran_ju": "wust01_hold01_moran",
    "wutong_yunxiu_zhai": "wust01_hold01_yunxiu",
    **{f"wutong_cq_{i:02d}": f"wust01_hold01_{i:02d}" for i in range(1, 16)},
    # 二段：沉香理療坊 shop01
    "wutong_chenxiang_liaofang": "wust02_shop01_main",
    **{f"wutong_chenxiang_{i:02d}": f"wust02_shop01_{i:02d}" for i in range(2, 9)},
    # 三段：翡翠茶莊 shop01
    "wutong_feicui_chazhuang": "wust03_shop01_main",
    **{f"wutong_feicui_{i:02d}": f"wust03_shop01_{i:02d}" for i in range(2, 8)},
    # 四段：生機醫館 shop01、凝光藝廊 shop02
    "wutong_shengji_yiguan": "wust04_shop01_main",
    **{f"wutong_shengji_{i:02d}": f"wust04_shop01_{i:02d}" for i in range(2, 10)},
    "wutong_ningguang_yilang": "wust04_shop02_main",
    **{f"wutong_ningguang_{i:02d}": f"wust04_shop02_{i:02d}" for i in range(2, 7)},
    # 五段：翠蔭廣庭 hold01、能量織造坊 shop01
    "wutong_cuiyin_guangting": "wust05_hold01_main",
    **{f"wutong_cy_{i:02d}": f"wust05_hold01_{i:02d}" for i in range(1, 21)},
    "wutong_cuiying_ge": "wust05_hold01_cuiying",
    "wutong_yulin_she": "wust05_hold01_yulin",
    "wutong_qiushi_ju": "wust05_hold01_qiushi",
    "wutong_weiguang_yu": "wust05_hold01_weiguang",
    "wutong_nengliang_zhizaofang": "wust05_shop01_main",
    **{f"wutong_nengliang_{i:02d}": f"wust05_shop01_{i:02d}" for i in range(2, 11)},
    # 六段：精金鍛造鋪 shop01
    "wutong_jingjin_duanzaopu": "wust06_shop01_main",
    **{f"wutong_jingjin_{i:02d}": f"wust06_shop01_{i:02d}" for i in range(2, 9)},
    # 七段：豐饒食堂 shop01
    "wutong_fengrao_shitang": "wust07_shop01_main",
    **{f"wutong_fengrao_{i:02d}": f"wust07_shop01_{i:02d}" for i in range(2, 8)},
    # 八段：晶華雜貨鋪 shop01、厚土居 home01
    "wutong_jinghua_zahuopu": "wust08_shop01_main",
    **{f"wutong_jinghua_{i:02d}": f"wust08_shop01_{i:02d}" for i in range(2, 10)},
    "wutong_houtu_ju": "wust08_home01_main",
    **{f"wutong_houtu_{i:02d}": f"wust08_home01_{i:02d}" for i in range(2, 7)},
    # 九段：鳴蟬室 home01
    "wutong_mingchan_shi": "wust09_home01_main",
    **{f"wutong_mingchan_{i:02d}": f"wust09_home01_{i:02d}" for i in range(2, 6)},
}

# 依 key 長度由長到短排序，避免短 key 先替換造成錯誤（例如 wutong_cy_1 被 wutong_cy 先替換）
SORTED_OLD_IDS = sorted(ID_MAP.keys(), key=len, reverse=True)


def replace_ids_in_text(text: str) -> str:
    for old in SORTED_OLD_IDS:
        new = ID_MAP[old]
        text = text.replace(f'"{old}"', f'"{new}"')
    return text


def main():
    all_jsons = list(BASE.rglob("*.json"))
    # 先記錄哪些檔需要更名（依「替換前」的 id）
    to_rename: list[tuple[Path, str]] = []  # (path, new_id)
    for path in all_jsons:
        if path.name.startswith("wustreet_st"):
            continue
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            continue
        room_id = data.get("id")
        if room_id in ID_MAP:
            new_id = ID_MAP[room_id]
            if path.stem != new_id:
                to_rename.append((path, new_id))

    # 第一輪：對所有 JSON 做內容替換
    for path in all_jsons:
        raw = path.read_text(encoding="utf-8")
        new_raw = replace_ids_in_text(raw)
        if new_raw != raw:
            path.write_text(new_raw, encoding="utf-8")

    # 第二輪：依記錄更名；若無記錄（內容已替換過）則依目前 id 與檔名不一致者更名
    renamed: list[tuple[Path, Path]] = []
    for path, new_id in to_rename:
        if not path.exists():
            continue
        new_path = path.parent / f"{new_id}.json"
        if path != new_path:
            if new_path.exists():
                print("Skip (exists):", new_path)
            else:
                path.rename(new_path)
                renamed.append((path, new_path))

    if not renamed:
        # 內容已是新 id，依 id 與檔名對齊更名
        for path in BASE.rglob("*.json"):
            if path.name.startswith("wustreet_st"):
                continue
            try:
                data = json.loads(path.read_text(encoding="utf-8"))
            except json.JSONDecodeError:
                continue
            rid = data.get("id", "")
            if rid and path.stem != rid:
                new_path = path.parent / f"{rid}.json"
                if not new_path.exists():
                    path.rename(new_path)
                    renamed.append((path, new_path))

    print("Renamed files:", len(renamed))
    for a, b in renamed[:25]:
        print(f"  {a.name} -> {b.name}")
    if len(renamed) > 25:
        print(f"  ... and {len(renamed) - 25} more.")


if __name__ == "__main__":
    main()
