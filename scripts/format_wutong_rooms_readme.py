#!/usr/bin/env python3
"""
依 data/rooms/README.md 約定，為梧桐大街一段～九段「內部建築」房間補齊格式：
- exits 每項加上 ui_hidden: true
- 每個出口對應一個 objects 項：Move、move_to_room_id、responses.Move
- 描述中若尚無 〔〕 可點擊處，則補一句「可經〔X〕、〔Y〕…前往他處。」
"""
import json
import re
from pathlib import Path

BASE = Path(__file__).resolve().parent.parent / "data" / "rooms" / "梧桐大街"


def format_room(path: Path) -> bool:
    data = json.loads(path.read_text(encoding="utf-8"))
    rid = data.get("id", "")
    if not rid or rid.startswith("wustreet_st"):
        return False
    exits = data.get("exits")
    if not exits:
        return False

    # 1) 為每個 exit 加 ui_hidden: true
    for e in exits:
        e["ui_hidden"] = True

    # 2) 為每個 exit 建立對應 object（若尚無）
    objects = list(data.get("objects") or [])
    existing_move_ids = {o["move_to_room_id"] for o in objects if o.get("move_to_room_id")}
    for i, e in enumerate(exits):
        to_id = e["to"]
        direction = e.get("direction", "出口")
        oid = f"{rid}_exit_{i + 1}"
        if any(o.get("id") == oid for o in objects):
            continue
        # 回大街的 Move 回應稍具體
        if to_id.startswith("wustreet_st"):
            move_resp = "你步出，回到梧桐大街。"
        else:
            move_resp = f"你往{direction}走去。"
        objects.append({
            "id": oid,
            "name": direction,
            "sockets": ["Move"],
            "move_to_room_id": to_id,
            "responses": {"Move": move_resp},
        })
        existing_move_ids.add(to_id)
    data["objects"] = objects

    # 3) 描述中若沒有任何 〔〕，補一句可經某某前往
    desc = data.get("description", "")
    if "〔" not in desc or "〕" not in desc:
        names = [e.get("direction", "") for e in exits]
        names = [n for n in names if n]
        if names:
            bracket = "、".join(f"〔{n}〕" for n in names)
            data["description"] = desc.rstrip().rstrip("。") + "。可經" + bracket + "前往他處。"
    data["exits"] = exits

    path.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
    return True


def main():
    count = 0
    for path in sorted(BASE.rglob("*.json")):
        if path.name.startswith("wustreet_st"):
            continue
        try:
            if format_room(path):
                count += 1
                print(path.relative_to(BASE))
        except Exception as e:
            print("Error", path, e)
    print("Formatted", count, "rooms.")


if __name__ == "__main__":
    main()
