#!/usr/bin/env python3
"""
梧桐大街：內室/內間/第N間 → 台灣慣用「有語意的房間名」（前間、裡間、櫃前、庫房等），一句入畫。
- 房間 name、exits[].direction、objects[].name、description 可經、responses.Move 一併更新。
"""
import json
from pathlib import Path

BASE = Path(__file__).resolve().parent.parent / "data" / "rooms" / "梧桐大街"

# 民宅內間：台灣慣用、有畫面感（前間→裡間→愈深）
HOME_INNER_NAMES = [
    "前間", "裡間", "側間", "書隅", "茶間", "寢間", "廊底", "儲間", "廳角",
]
# 商鋪內間：櫃前、庫房、工坊等
SHOP_INNER_NAMES = [
    "櫃前", "庫房", "工坊隅", "展間", "焙間", "理療區", "寢區", "儲區", "後間",
]

# suffix _02=索引0, _03=索引1, ...
SUFFIX_INDEX = {"02": 0, "03": 1, "04": 2, "05": 3, "06": 4, "07": 5, "08": 6, "09": 7, "10": 8}


def room_suffix(rid: str) -> str | None:
    if "_main" in rid or not rid.startswith("wust"):
        return None
    parts = rid.split("_")
    if len(parts) < 3:
        return None
    last = parts[-1]
    if last in SUFFIX_INDEX:
        return last
    return None


def building_prefix(rid: str) -> str | None:
    """e.g. wust07_home01_02 -> wust07_home01"""
    if "_main" in rid:
        return rid.replace("_main", "")
    suf = room_suffix(rid)
    if suf and rid.endswith("_" + suf):
        return rid[: -len(suf) - 1]
    return None


def build_rid_to_semantic_name() -> dict[str, str]:
    """掃描所有房間，為每個內間（_02～_10）決定語意名稱。"""
    rid_to_name: dict[str, str] = {}
    # 依建築前綴分組，同一棟內 _02 用同一個名稱類型
    by_prefix: dict[str, list[str]] = {}
    for path in sorted(BASE.rglob("*.json")):
        if path.name.startswith("wustreet"):
            continue
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except Exception:
            continue
        rid = data.get("id", "")
        suf = room_suffix(rid)
        if not suf:
            continue
        pre = building_prefix(rid)
        if not pre:
            continue
        if pre not in by_prefix:
            by_prefix[pre] = []
        if rid not in by_prefix[pre]:
            by_prefix[pre].append(rid)
    for pre, rids in by_prefix.items():
        is_shop = "shop" in pre
        names = SHOP_INNER_NAMES if is_shop else HOME_INNER_NAMES
        for rid in sorted(rids):
            suf = room_suffix(rid)
            if suf is None:
                continue
            idx = SUFFIX_INDEX.get(suf, 0)
            name = names[idx % len(names)]
            rid_to_name[rid] = name
    return rid_to_name


def update_room(data: dict, rid_to_name: dict[str, str]) -> bool:
    rid = data.get("id", "")
    changed = False
    # 1. 房間 name：改為語意名稱
    if rid in rid_to_name:
        new_name = rid_to_name[rid]
        if data.get("name") != new_name:
            data["name"] = new_name
            changed = True
    # 2. exits[].direction：改為目標房間的語意名稱
    for ex in data.get("exits", []):
        to_rid = ex.get("to", "")
        if not to_rid or to_rid.startswith("wustreet"):
            continue
        if to_rid in rid_to_name:
            new_dir = rid_to_name[to_rid]
            if ex.get("direction") != new_dir:
                ex["direction"] = new_dir
                changed = True
    # 3. objects[].name 與 responses.Move
    for obj in data.get("objects", []):
        to_rid = obj.get("move_to_room_id", "")
        if to_rid and to_rid in rid_to_name:
            new_name = rid_to_name[to_rid]
            if obj.get("name") != new_name:
                obj["name"] = new_name
                changed = True
            resp = obj.get("responses", {}) or {}
            move_text = resp.get("Move", "")
            if move_text and "走去" in move_text:
                # 統一為「你往{名稱}走去」
                expected = "你往" + new_name + "走去。"
                if move_text.strip() != expected:
                    obj.setdefault("responses", {})["Move"] = expected
                    changed = True
    # 4. description：可經 依新 direction 重組
    desc = data.get("description", "")
    if desc and "可經" in desc:
        before = desc.split("可經")[0].rstrip()
        dirs = [ex.get("direction", "") for ex in data.get("exits", []) if ex.get("direction")]
        if dirs:
            new_suffix = "可經〔" + "〕、〔".join(dirs) + "〕前往他處。"
            new_desc = before + new_suffix
            if new_desc != desc:
                data["description"] = new_desc
                changed = True
    return changed


def main():
    rid_to_name = build_rid_to_semantic_name()
    count = 0
    for path in sorted(BASE.rglob("*.json")):
        if path.name.startswith("wustreet"):
            continue
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except Exception:
            continue
        if update_room(data, rid_to_name):
            path.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
            count += 1
            print(path.relative_to(BASE))
    print("\n已更新", count, "個房間檔案（第N間 → 語意房間名，並修正「你往…走去」）。")


if __name__ == "__main__":
    main()
