#!/usr/bin/env python3
"""
將梧桐大街擴充建築的「民宅一／民宅二／商鋪二」等改為暢銷作家風格的飾辭命名。
依建築 main room id 對應新名稱，更新：該建築 main 的 name、街道 objects 的 name 與 Move 回應、街道 description 的〔〕、參考文件表格。
"""
import json
from pathlib import Path

BASE = Path(__file__).resolve().parent.parent / "data" / "rooms" / "梧桐大街"
DOC = Path(__file__).resolve().parent.parent / "docs" / "reference" / "梧桐大街—環境概念.md"

# main room id -> 飾辭名稱（暢銷作家文筆：有畫面、有氛圍、略帶詩意）
MAIN_ID_TO_NAME = {
    "wust01_home01_main": "簷蔭宅",
    "wust01_home02_main": "桐影居",
    "wust02_shop02_main": "桐香閣",
    "wust02_home01_main": "綠脈軒",
    "wust02_home02_main": "沉香隅",
    "wust03_shop02_main": "玉光坊",
    "wust03_home01_main": "根鬚齋",
    "wust03_home02_main": "靜謐邸",
    "wust04_home01_main": "翠廊小築",
    "wust04_home02_main": "蔭深居",
    "wust05_shop02_main": "濃蔭棧",
    "wust06_shop02_main": "簷下肆",
    "wust06_home01_main": "溫潤宅",
    "wust06_home02_main": "葉影軒",
    "wust07_shop02_main": "翠脈鋪",
    "wust07_home01_main": "廊下居",
    "wust07_home02_main": "青脈宅",
    "wust08_shop02_main": "桐光肆",
    "wust08_shop03_main": "綠影坊",
    "wust08_home02_main": "幽蔭舍",
    "wust09_home02_main": "隅光邸",
    "wust09_home03_main": "葉脈居",
}

# 舊顯示名 -> 對應的 main id（用於街道 description 替換時辨識哪個建築）
# 街道描述裡是「民宅一」「民宅二」等，需依段替換為對應新名，所以改為依街道檔逐個處理
def main():
    # 1) 更新各建築 main 房間的 name
    for path in BASE.rglob("*.json"):
        if path.name.startswith("wustreet_"):
            continue
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except Exception:
            continue
        rid = data.get("id")
        if rid in MAIN_ID_TO_NAME:
            data["name"] = MAIN_ID_TO_NAME[rid]
            path.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
            print("Room", rid, "->", data["name"])

    # 2) 更新各段街道：object name、Move 回應、description 中的〔舊名〕->〔新名〕
    for seg in range(1, 10):
        seg_dir = BASE / f"梧桐大街{['', '一段', '二段', '三段', '四段', '五段', '六段', '七段', '八段', '九段'][seg]}"
        path = seg_dir / f"wustreet_st{seg}.json"
        if not path.exists():
            continue
        data = json.loads(path.read_text(encoding="utf-8"))
        desc = data.get("description", "")
        for obj in data.get("objects", []):
            mid = obj.get("move_to_room_id")
            if mid in MAIN_ID_TO_NAME:
                old_name = obj.get("name", "")
                new_name = MAIN_ID_TO_NAME[mid]
                obj["name"] = new_name
                obj["responses"] = obj.get("responses") or {}
                obj["responses"]["Move"] = f"你步入{new_name}。"
                if old_name and f"〔{old_name}〕" in desc:
                    desc = desc.replace(f"〔{old_name}〕", f"〔{new_name}〕", 1)
        data["description"] = desc
        path.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
        print("Street st", seg, "updated.")

    # 3) 更新參考文件：建築分配表第三欄 民宅一／民宅二／商鋪二／商鋪三 -> 新名稱
    def old_name(rid):
        if "home01" in rid: return "民宅一"
        if "home02" in rid: return "民宅二"
        if "home03" in rid: return "民宅三"
        if "shop02" in rid: return "商鋪二"
        if "shop03" in rid: return "商鋪三"
        return None

    text = DOC.read_text(encoding="utf-8")
    lines = text.split("\n")
    for i, line in enumerate(lines):
        for rid, new_name in MAIN_ID_TO_NAME.items():
            if rid not in line:
                continue
            old = old_name(rid)
            if old and f"| {old} |" in line:
                lines[i] = line.replace(f"| {old} |", f"| {new_name} |", 1)
                break
    DOC.write_text("\n".join(lines), encoding="utf-8")
    print("Doc updated.")


if __name__ == "__main__":
    main()
