#!/usr/bin/env python3
"""
梧桐大街建築結構多樣化：將部分線性鏈建築改為星狀或環狀，使建築結構不重複。
- 星狀 (star)：主廳連所有內室，內室只連回主廳
- 環狀 (cycle)：主廳→內1→內2→…→內N→內1，主廳只連內1
"""
import json
import re
from pathlib import Path
from collections import defaultdict

BASE = Path(__file__).resolve().parent.parent / "data" / "rooms" / "梧桐大街"


def building_prefix(rid: str) -> str | None:
    if "_main" in rid:
        return rid.replace("_main", "")
    for suffix in ["_02", "_03", "_04", "_05", "_06", "_07", "_08", "_09", "_10"]:
        if rid.endswith(suffix):
            return rid[: -len(suffix)]
    parts = rid.split("_")
    if len(parts) >= 3:
        return "_".join(parts[:3])
    return None


def find_building_room_paths(prefix: str) -> list[Path]:
    """回傳該建築所有房間 JSON 路徑（僅 main + _02.._10，不含 hold01 子景點）。"""
    out = []
    for path in BASE.rglob("*.json"):
        if path.name.startswith("wustreet"):
            continue
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except Exception:
            continue
        rid = data.get("id", "")
        if not rid or not rid.startswith(prefix + "_"):
            continue
        if rid == prefix + "_main":
            out.append(path)
            continue
        suffix = rid[len(prefix) + 1 :]
        if suffix.isdigit() or suffix in ("02", "03", "04", "05", "06", "07", "08", "09", "10"):
            out.append(path)
    return sorted(out, key=lambda p: (0 if "_main" in p.name else 1, p.name))


def get_main_and_inner(rooms_data: dict) -> tuple[dict, list[dict]]:
    """rooms_data: rid -> data. 回傳 (main_room_data, [inner_02, inner_03, ...])"""
    main = None
    inners = []
    for rid in sorted(rooms_data.keys()):
        if rid.endswith("_main"):
            main = rooms_data[rid]
            continue
        if "_02" in rid or "_03" in rid or "_04" in rid or "_05" in rid or "_06" in rid or "_07" in rid or "_08" in rid or "_09" in rid or "_10" in rid:
            inners.append(rooms_data[rid])
    inners.sort(key=lambda r: r["id"])
    return main, inners


def update_description_kejing(desc: str, direction_names: list[str]) -> str:
    """將描述中「可經〔X〕、〔Y〕…」替換為新的方向列表。"""
    if "可經" not in desc:
        return desc
    # 保留主體，替換「可經…」整段
    before = desc.split("可經")[0].rstrip()
    new_suffix = "可經〔" + "〕、〔".join(direction_names) + "〕前往他處。"
    return before + new_suffix


def apply_star(paths: list[Path], street_rid: str) -> None:
    """將建築改為星狀：主廳連所有內室，內室只連主廳。"""
    rooms_data = {}
    rid_to_path = {}
    for p in paths:
        data = json.loads(p.read_text(encoding="utf-8"))
        rid = data["id"]
        rooms_data[rid] = data
        rid_to_path[rid] = p
    main, inners = get_main_and_inner(rooms_data)
    if not main or not inners:
        return
    main_rid = main["id"]
    prefix = main_rid.replace("_main", "")

    # 主廳：大街 + 每個內室一個出口（方向用該室的 name）
    main_exits = [{"direction": "大街", "to": street_rid, "ui_hidden": True}]
    main_objects = [{"id": f"{main_rid}_exit_1", "name": "大街", "sockets": ["Move"], "move_to_room_id": street_rid, "responses": {"Move": "你步出，回到梧桐大街。"}}]
    for i, inner in enumerate(inners):
        dir_name = inner.get("name", f"內室{i+1}")
        to_rid = inner["id"]
        main_exits.append({"direction": dir_name, "to": to_rid, "ui_hidden": True})
        main_objects.append({"id": f"{main_rid}_exit_{i+2}", "name": dir_name, "sockets": ["Move"], "move_to_room_id": to_rid, "responses": {"Move": f"你往{dir_name}走去。"}})
    main["exits"] = main_exits
    main["objects"] = main_objects
    main_desc_dirs = ["大街"] + [r.get("name", "") for r in inners]
    main["description"] = update_description_kejing(main.get("description", ""), main_desc_dirs)

    # 各內室：只保留 大街、門廳 -> main
    for inner in inners:
        inner["exits"] = [
            {"direction": "大街", "to": main_rid, "ui_hidden": True},
            {"direction": "門廳", "to": main_rid, "ui_hidden": True},
        ]
        inner["objects"] = [
            {"id": f"{inner['id']}_exit_1", "name": "大街", "sockets": ["Move"], "move_to_room_id": main_rid, "responses": {"Move": "你往大街方向走去。"}},
            {"id": f"{inner['id']}_exit_2", "name": "門廳", "sockets": ["Move"], "move_to_room_id": main_rid, "responses": {"Move": "你往門廳走去。"}},
        ]
        inner["description"] = update_description_kejing(inner.get("description", ""), ["大街", "門廳"])

    for rid, data in rooms_data.items():
        rid_to_path[rid].write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")


def apply_cycle(paths: list[Path], street_rid: str) -> None:
    """將建築改為環狀：主廳→內1→內2→…→內N→內1。最後一室與第一室互加「迴廊」雙向出口，地圖檢視器才會畫出實線。"""
    rooms_data = {}
    rid_to_path = {}
    for p in paths:
        data = json.loads(p.read_text(encoding="utf-8"))
        rid = data["id"]
        rooms_data[rid] = data
        rid_to_path[rid] = p
    main, inners = get_main_and_inner(rooms_data)
    if not main or len(inners) < 2:
        return
    main_rid = main["id"]
    first_room = inners[0]
    first_rid = first_room["id"]
    last = inners[-1]
    last_rid = last["id"]
    prev_rid = inners[-2]["id"]  # 倒數第二室
    last_name = last.get("name", "末室")

    # 第一室：加「迴廊」→ 最後一室（雙向才讓地圖檢視器畫實線）
    if not any(e.get("to") == last_rid for e in first_room.get("exits", [])):
        first_room["exits"] = list(first_room.get("exits", [])) + [
            {"direction": "迴廊", "to": last_rid, "ui_hidden": True},
        ]
        first_room["objects"] = list(first_room.get("objects", [])) + [
            {"id": f"{first_rid}_exit_huilang", "name": "迴廊", "sockets": ["Move"], "move_to_room_id": last_rid, "responses": {"Move": "你經迴廊往深處走去。"}},
        ]
        desc = first_room.get("description", "")
        if "可經" in desc and "〔迴廊〕" not in desc:
            first_room["description"] = desc.replace("前往他處。", "、〔迴廊〕前往他處。")
        rooms_data[first_rid] = first_room

    # 最後一室：大街->main、前一間->prev_rid、迴廊->first_rid
    last_exits = [
        {"direction": "大街", "to": main_rid, "ui_hidden": True},
        {"direction": "迴廊", "to": first_rid, "ui_hidden": True},
    ]
    prev_dir_name = "前一間"
    for e in last["exits"]:
        if e.get("to") == prev_rid:
            last_exits.insert(1, e)
            prev_dir_name = e.get("direction", prev_dir_name)
            break
    last["exits"] = last_exits
    last_objs = [
        {"id": f"{last_rid}_exit_1", "name": "大街", "sockets": ["Move"], "move_to_room_id": main_rid, "responses": {"Move": "你往大街方向走去。"}},
        {"id": f"{last_rid}_exit_2", "name": prev_dir_name, "sockets": ["Move"], "move_to_room_id": prev_rid, "responses": {"Move": f"你往{prev_dir_name}走去。"}},
        {"id": f"{last_rid}_exit_3", "name": "迴廊", "sockets": ["Move"], "move_to_room_id": first_rid, "responses": {"Move": "你經迴廊往內室方向走去。"}},
    ]
    last["objects"] = last_objs
    desc = last.get("description", "")
    if "可經" in desc:
        before = desc.split("可經")[0].rstrip()
        last["description"] = before + "可經〔大街〕、〔" + prev_dir_name + "〕、〔迴廊〕前往他處。"
    rooms_data[last_rid] = last

    for rid, data in rooms_data.items():
        rid_to_path[rid].write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")


def get_street_for_main(main_data: dict) -> str:
    for e in main_data.get("exits", []):
        if "st" in e.get("to", "") and "wustreet" in e.get("to", ""):
            return e["to"]
    return "wustreet_st1"


def main():
    # 重複結構對照：每組保留一棟線性，其餘改為星狀或環狀
    # 格式: (結構簽名, [(prefix, 改為), ...]) 改為 = "star" | "cycle"
    conversions = [
        ("graph-9-15", [("wust06_shop02", "star")]),
        ("path-8", [("wust06_shop01", "cycle")]),
        ("path-9", [("wust08_shop01", "star")]),
        ("graph-8-13", [("wust07_home01", "star"), ("wust08_shop03", "cycle")]),
        ("graph-7-11", [("wust03_shop02", "star"), ("wust04_home02", "cycle"), ("wust02_home01", "cycle"), ("wust06_home02", "cycle")]),
        ("graph-5-7", [("wust03_home02", "star"), ("wust06_home01", "cycle"), ("wust01_home02", "cycle"), ("wust09_home02", "cycle")]),
        ("graph-6-9", [("wust02_home02", "star"), ("wust03_home01", "cycle"), ("wust04_home01", "star"), ("wust07_home02", "cycle"), ("wust01_home01", "cycle"), ("wust08_home02", "cycle")]),
    ]
    done = set()
    for sig, building_changes in conversions:
        for prefix, target in building_changes:
            if prefix in done:
                continue
            paths = find_building_room_paths(prefix)
            if not paths:
                print("skip (no paths):", prefix)
                continue
            data0 = json.loads(paths[0].read_text(encoding="utf-8"))
            street = get_street_for_main(data0) if data0.get("id", "").endswith("_main") else "wustreet_st1"
            for p in paths:
                d = json.loads(p.read_text(encoding="utf-8"))
                if d.get("id", "").endswith("_main"):
                    street = get_street_for_main(d)
                    break
            if target == "star":
                apply_star(paths, street)
                print("OK star", prefix)
            else:
                apply_cycle(paths, street)
                print("OK cycle", prefix)
            done.add(prefix)
    print("\n建築結構多樣化完成。")


if __name__ == "__main__":
    main()
