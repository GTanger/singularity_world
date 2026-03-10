#!/usr/bin/env python3
"""
依「梧桐大街各段設計」新增民宅／商鋪：
一段：+6格民宅、+5格民宅
二段：+9格商鋪、+7格民宅、+6格民宅
三段：+7格商鋪、+6格民宅、+5格民宅
四段：+6格民宅、+7格民宅
五段：+8格商鋪
六段：+9格商鋪、+5格民宅、+7格民宅
七段：+10格商鋪、+8格民宅、+6格民宅
八段：+6格商鋪、+8格商鋪、+6格民宅
九段：+5格民宅、+5格民宅

每棟：main + 02..N 線性鏈，exits 含 ui_hidden，objects 含 Move。
"""
import json
from pathlib import Path

BASE = Path(__file__).resolve().parent.parent / "data" / "rooms" / "梧桐大街"
STREET_IDS = [f"wustreet_st{i}" for i in range(1, 10)]

# (段, 類型, 該類型編號, 格數, 顯示名)
# 段 1～9，類型 home/shop，編號從 1 起（與現有不重複）
NEW_BUILDINGS = [
    (1, "home", 1, 6, "民宅一"),
    (1, "home", 2, 5, "民宅二"),
    (2, "shop", 2, 9, "商鋪二"),
    (2, "home", 1, 7, "民宅一"),
    (2, "home", 2, 6, "民宅二"),
    (3, "shop", 2, 7, "商鋪二"),
    (3, "home", 1, 6, "民宅一"),
    (3, "home", 2, 5, "民宅二"),
    (4, "home", 1, 6, "民宅一"),
    (4, "home", 2, 7, "民宅二"),
    (5, "shop", 2, 8, "商鋪二"),
    (6, "shop", 2, 9, "商鋪二"),
    (6, "home", 1, 5, "民宅一"),
    (6, "home", 2, 7, "民宅二"),
    (7, "shop", 2, 10, "商鋪二"),
    (7, "home", 1, 8, "民宅一"),
    (7, "home", 2, 6, "民宅二"),
    (8, "shop", 2, 6, "商鋪二"),
    (8, "shop", 3, 8, "商鋪三"),
    (8, "home", 2, 6, "民宅二"),
    (9, "home", 2, 5, "民宅二"),
    (9, "home", 3, 5, "民宅三"),
]

DESC_MAIN_HOME = "梧桐濃蔭下的小宅門廳。黑曜石與玉質脈絡嵌合，簷下綠意掩映，空氣中淡淡沉香與恆溫熱氣。"
DESC_MAIN_SHOP = "臨街店鋪門面。櫃檯與貨架與梧桐綠意相映，黑曜石地面、能量導線隱約，富態化材質溫潤。"
DESC_ROOM_HOME = "內室，牆面玉質脈絡與根鬚咬合，光線篩落、靜謐豐饒。"
DESC_ROOM_SHOP = "店內一隅，晶體與木質增生與結構咬合，綠意與微光並存。"


def room_id(seg: int, typ: str, num: int, room: str) -> str:
    t = "home" if typ == "home" else "shop"
    nn = f"{num:02d}"
    return f"wust{seg:02d}_{t}{nn}_{room}"


def make_main_room(seg: int, typ: str, num: int, room_count: int, name: str) -> dict:
    """room_count = 總格數（含 main），內部為 02..room_count 共 room_count-1 間。"""
    rid = room_id(seg, typ, num, "main")
    street = STREET_IDS[seg - 1]
    first_inner = room_id(seg, typ, num, "02")
    desc = DESC_MAIN_HOME if typ == "home" else DESC_MAIN_SHOP
    dir_inner = "內室一" if typ == "home" else "櫃前"
    desc += f"可經〔大街〕、〔{dir_inner}〕前往他處。"
    return {
        "id": rid,
        "name": name,
        "description": desc,
        "tags": ["residence", "indoor"] if typ == "home" else ["shop", "indoor"],
        "zone": "梧桐大街",
        "exits": [
            {"direction": "大街", "to": street, "ui_hidden": True},
            {"direction": dir_inner, "to": first_inner, "ui_hidden": True},
        ],
        "objects": [
            {
                "id": f"{rid}_exit_1",
                "name": "大街",
                "sockets": ["Move"],
                "move_to_room_id": street,
                "responses": {"Move": "你步出，回到梧桐大街。"},
            },
            {
                "id": f"{rid}_exit_2",
                "name": dir_inner,
                "sockets": ["Move"],
                "move_to_room_id": first_inner,
                "responses": {"Move": f"你往{dir_inner}走去。"},
            },
        ],
    }


def make_inner_room(seg: int, typ: str, num: int, suf: int, room_count: int) -> dict:
    """suf = 02, 03, ..., room_count（總格數）。前一間 suf-1，後一間 suf+1，大街經 main。"""
    suf_str = f"{suf:02d}"
    rid = room_id(seg, typ, num, suf_str)
    main_rid = room_id(seg, typ, num, "main")
    desc = DESC_ROOM_HOME if typ == "home" else DESC_ROOM_SHOP
    exits = []
    objects = []
    # 大街（經 main）
    exits.append({"direction": "大街", "to": main_rid, "ui_hidden": True})
    objects.append({
        "id": f"{rid}_exit_1",
        "name": "大街",
        "sockets": ["Move"],
        "move_to_room_id": main_rid,
        "responses": {"Move": "你往大街方向走去。"},
    })
    # 前一間：門廳(main) 或 內室X/內間X
    if suf == 2:
        prev_rid, dir_prev = main_rid, "門廳"
    else:
        prev_rid = room_id(seg, typ, num, f"{suf - 1:02d}")
        dir_prev = "內室一" if suf == 3 else (f"內室{suf - 2}" if typ == "home" else "櫃前" if suf == 3 else f"內間{suf - 2}")
    exits.append({"direction": dir_prev, "to": prev_rid, "ui_hidden": True})
    objects.append({
        "id": f"{rid}_exit_2",
        "name": dir_prev,
        "sockets": ["Move"],
        "move_to_room_id": prev_rid,
        "responses": {"Move": f"你往{dir_prev}走去。"},
    })
    # 後一間
    if suf < room_count:
        next_rid = room_id(seg, typ, num, f"{suf + 1:02d}")
        dir_next = "內室二" if suf == 2 else (f"內室{suf - 1}" if typ == "home" else "內間一" if suf == 2 else f"內間{suf - 1}")
        exits.append({"direction": dir_next, "to": next_rid, "ui_hidden": True})
        objects.append({
            "id": f"{rid}_exit_3",
            "name": dir_next,
            "sockets": ["Move"],
            "move_to_room_id": next_rid,
            "responses": {"Move": f"你往{dir_next}走去。"},
        })
    dir_names = [e["direction"] for e in exits]
    desc += "可經" + "、".join(f"〔{d}〕" for d in dir_names) + "前往他處。"
    room_name = "內室一" if suf == 2 else (f"內室{suf - 1}" if typ == "home" else "櫃前" if suf == 2 else f"內間{suf - 2}")
    if suf > 2 and typ == "shop":
        room_name = "內間一" if suf == 3 else f"內間{suf - 2}"
    return {
        "id": rid,
        "name": room_name,
        "description": desc,
        "tags": ["residence", "indoor"] if typ == "home" else ["shop", "indoor"],
        "zone": "梧桐大街",
        "exits": exits,
        "objects": objects,
    }


def write_room(path: Path, data: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")


def add_building(seg: int, typ: str, num: int, room_count: int, name: str) -> tuple[str, str]:
    """room_count = 總格數（含 main）。產生 main + 02, 03, ..., room_count。"""
    seg_dir = BASE / f"梧桐大街{['', '一段', '二段', '三段', '四段', '五段', '六段', '七段', '八段', '九段'][seg]}"
    folder = seg_dir / name
    rid_main = room_id(seg, typ, num, "main")
    write_room(folder / f"{rid_main}.json", make_main_room(seg, typ, num, room_count, name))
    for suf in range(2, room_count + 1):
        data = make_inner_room(seg, typ, num, suf, room_count)
        write_room(folder / f"{data['id']}.json", data)
    return rid_main, name


def update_street(seg: int, new_entries: list[tuple[str, str]]) -> None:
    """new_entries: [(room_id, display_name)]"""
    seg_dir = BASE / f"梧桐大街{['', '一段', '二段', '三段', '四段', '五段', '六段', '七段', '八段', '九段'][seg]}"
    path = seg_dir / f"wustreet_st{seg}.json"
    data = json.loads(path.read_text(encoding="utf-8"))
    st_prev = f"wustreet_st{seg - 1}" if seg > 1 else None
    st_next = f"wustreet_st{seg + 1}" if seg < 9 else None
    for rid, disp_name in new_entries:
        data["exits"].append({"direction": rid, "to": rid, "ui_hidden": True})
        part = rid.replace(f"wust{seg:02d}_", "").replace("_main", "")  # e.g. home01, shop02
        obj_id = f"wu_st{seg}_obj_{part}"
        data["objects"].append({
            "id": obj_id,
            "name": disp_name,
            "sockets": ["Move"],
            "move_to_room_id": rid,
            "responses": {"Move": f"你步入{disp_name}。"},
        })
    # 更新 description：在「路側」後補上新建築〔名稱〕
    desc = data["description"]
    if "可經" in desc:
        part, rest = desc.split("可經", 1)
        existing = []
        for o in data["objects"]:
            if o.get("move_to_room_id") and not o["move_to_room_id"].startswith("wustreet"):
                existing.append(o["name"])
        # 保持原有「路側…」敘述，只確保新建築在 objects 裡有對應 name
        pass
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")


def main():
    by_seg = {}
    for seg, typ, num, room_count, name in NEW_BUILDINGS:
        by_seg.setdefault(seg, []).append((typ, num, room_count, name))

    for seg in range(1, 10):
        entries = by_seg.get(seg, [])
        new_entries = []
        for typ, num, room_count, name in entries:
            rid_main, disp = add_building(seg, typ, num, room_count, name)
            new_entries.append((rid_main, disp))
        if new_entries:
            update_street(seg, new_entries)

    print("Added", sum(len(e) for e in by_seg.values()), "buildings.")
    for seg, items in sorted(by_seg.items()):
        print(f"  段{seg}:", ", ".join(f"{t}{n}({c}格){name}" for t, n, c, name in items))


if __name__ == "__main__":
    main()
