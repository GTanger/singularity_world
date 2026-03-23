#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Generate city_life 4F–10F residential lobbies + unit rooms (same layout per floor)."""

from __future__ import annotations

import json
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "data", "rooms", "editor")

# 1–9 unit index -> layout type
UNIT_TYPES: dict[int, str] = {
    1: "t6",
    2: "t5",
    3: "t9",
    4: "t6",
    5: "t9",
    6: "t6",
    7: "t9",
    8: "t5",
    9: "t6",
}

# suffix -> (object_name_for_move, display_name_suffix, description)
ROOM_SPECS: dict[str, list[tuple[str, str, str]]] = {
    "t6": [
        (
            "liv",
            "客廳",
            "門內第一進便是客廳；採光自陽台方向渗入，木作收邊與一組坐臥兼用沙發佔去半面牆，其餘門洞分別通往臥室、浴間與戶外陽台。另可回大廳走廊。",
        ),
        (
            "br1",
            "主臥",
            "主臥不大，床靠內牆，床頭一盞可調角度的閱讀燈；衣櫃滑門軌道輕響，窗外是立面的綠影層疊。可經〔客廳〕回到戶內動線。",
        ),
        (
            "br2",
            "次臥",
            "次臥兼作書桌角，層板與線槽收在牆內；空氣裡有紙張與織品混在一起的淡味。可經〔客廳〕回到戶內動線。",
        ),
        (
            "bt1",
            "主衛",
            "主衛乾溼分區，鏡前燈帶微溫；排水聲隔著門板悶悶傳來。可經〔客廳〕回到戶內動線。",
        ),
        (
            "bt2",
            "次衛",
            "次衛較小，淋浴與馬桶併置；牆磚縫裡嵌著除霉貼條。可經〔客廳〕回到戶內動線。",
        ),
        (
            "bal",
            "陽台",
            "戶外陽台欄杆外是浮生城立面綠被，風從豎井間擠過來；腳下排水格偶爾滴答。可經〔客廳〕回到戶內動線。",
        ),
    ],
    "t5": [
        (
            "liv",
            "客廳",
            "門內第一進便是客廳；採光自陽台方向渗入，沙發與矮櫃佔去主要視線，另側門洞通往兩間臥室與唯一衛浴。另可回大廳走廊。",
        ),
        (
            "br1",
            "主臥",
            "主臥簡潔，床與衣櫃各佔一角；窗框邊緣有細小冷凝水痕。可經〔客廳〕回到戶內動線。",
        ),
        (
            "br2",
            "次臥",
            "次臥牆上釘著幾枚免釘掛鉤，晃著輕物；腳步聲在木地板上略悶。可經〔客廳〕回到戶內動線。",
        ),
        (
            "bat",
            "衛浴",
            "唯一衛浴兼作洗衣角，機臺與洗手檯並排；門口地墊邊緣起毛。可經〔客廳〕回到戶內動線。",
        ),
        (
            "bal",
            "陽台",
            "戶外陽台面積不大，晾衣桿與盆栽併存；遠處可見同棟鄰戶退台的綠意。可經〔客廳〕回到戶內動線。",
        ),
    ],
    "t9": [
        (
            "liv",
            "客廳",
            "門內第一進便是客廳；與餐廳以短廊或開口相連，動線寬敞。採光自兩處陽台方向皆可渗入。另可回大廳走廊。",
        ),
        (
            "din",
            "餐廳",
            "餐廳一張方桌四椅，牆邊矮櫃收著餐具；燈罩邊緣略有黃漬。可經〔客廳〕回到戶內動線。",
        ),
        (
            "br1",
            "主臥",
            "主臥帶獨立衣櫃區，床尾留出走道；窗簾遮光層與紗層各一。可經〔客廳〕回到戶內動線。",
        ),
        (
            "br2",
            "次臥",
            "次臥佈置中性，適合來客短住；牆角插座帶有夜燈孔。可經〔客廳〕回到戶內動線。",
        ),
        (
            "br3",
            "客房",
            "客房較小，一床一桌；門後掛著住戶備用的薄毯。可經〔客廳〕回到戶內動線。",
        ),
        (
            "bt1",
            "主衛",
            "主衛附浴缸與淋浴並列（或乾溼分區），鏡櫃收納充足。可經〔客廳〕回到戶內動線。",
        ),
        (
            "bt2",
            "次衛",
            "次衛緊鄰次臥動線，潔具緊湊；排風扇低鳴。可經〔客廳〕回到戶內動線。",
        ),
        (
            "bala",
            "前陽台",
            "前陽台面朝豎井或外廊一側，欄杆外可見蔓生綠意；地面略向排水口傾斜。可經〔客廳〕回到戶內動線。",
        ),
        (
            "balb",
            "後陽台",
            "後陽台較近廚餐動線，常兼作晾曬與雜物角；風向與前陽台略有不同。可經〔客廳〕回到戶內動線。",
        ),
    ],
}


def fw_digits(s: str) -> str:
    """ASCII digits -> fullwidth."""
    return "".join(chr(0xFF10 + ord(c) - 48) if c.isdigit() else c for c in s)


def floor_label_cn(f: int) -> str:
    if f < 10:
        return fw_digits(str(f))
    if f == 10:
        return "１０"
    if f < 20:
        return "１" + fw_digits(str(f - 10))
    return fw_digits(str(f))


def unit_code(f: int, digit: int) -> str:
    return f"{f:02d}{digit}"


def room_id(prefix: str, code: str, suffix: str) -> str:
    return f"{prefix}_u{code}_{suffix}"


def build_liv_room(
    prefix: str,
    code_fw: str,
    code_ascii: str,
    lobby_id: str,
    liv_id: str,
    typ: str,
    specs: list[tuple[str, str, str]],
) -> dict:
    liv_spec = specs[0]
    assert liv_spec[0] == "liv"
    _, liv_title, liv_desc = liv_spec
    name = f"戶{code_fw}·{liv_title}"

    objects: list[dict] = [
        {
            "id": lobby_id + "_gate_" + liv_id,
            "name": "大廳",
            "owner": "",
            "sockets": ["Move"],
            "responses": {
                "Move": f'你離開戶內，回到「浮生城{fw_digits(str(prefix.split("_")[-1].replace("f", "")))}樓」走廊大廳。'
            },
            "move_to_room_id": lobby_id,
        }
    ]
    # fix floor in message - prefix is city_life_4f
    flnum = prefix.split("_")[-1].replace("f", "")
    floor_disp = fw_digits(flnum) + "樓"
    objects[0]["responses"]["Move"] = f'你離開戶內，回到「浮生城{floor_disp}」走廊大廳。'

    for suf, title, _ in specs[1:]:
        tgt = room_id(prefix, code_ascii, suf)
        objects.append(
            {
                "id": liv_id + "_to_" + tgt,
                "name": title,
                "owner": "",
                "sockets": ["Move"],
                "responses": {"Move": f'你進入戶{code_fw}的{title}。'},
                "move_to_room_id": tgt,
            }
        )

    # 與 objects 並列：伺服器 handleMove(MoveByExit) 只讀 exits，客廳若僅有 Move 物件則「方向移動」無法進臥衛陽台。
    exits: list[dict] = [{"direction": "大廳", "to": lobby_id}]
    for suf, title, _ in specs[1:]:
        exits.append(
            {"direction": title, "to": room_id(prefix, code_ascii, suf)}
        )

    return {
        "id": liv_id,
        "name": name,
        "description": liv_desc,
        "tags": ["city", "skyscraper", "indoor", "residential"],
        "zone": "citylife",
        "exits": exits,
        "objects": objects,
    }


def emit_floor(floor: int) -> None:
    prefix = f"city_life_{floor}f"
    lobby_id = prefix
    fl_w = floor_label_cn(floor)
    grid_top = f"｜｜｜｜｜上樓梯｜電梯間｜下樓梯｜｜｜｜｜"
    c1, c2, c3 = unit_code(floor, 1), unit_code(floor, 2), unit_code(floor, 3)
    c4, c5, c6 = unit_code(floor, 4), unit_code(floor, 5), unit_code(floor, 6)
    c7, c8, c9 = unit_code(floor, 7), unit_code(floor, 8), unit_code(floor, 9)
    g1, g2, g3 = fw_digits(c1), fw_digits(c2), fw_digits(c3)
    g4, g5, g6 = fw_digits(c4), fw_digits(c5), fw_digits(c6)
    g7, g8, g9 = fw_digits(c7), fw_digits(c8), fw_digits(c9)
    grid_mid = (
        f"｜{g1}｜　　　　　　　　　　　｜{g9}｜\n"
        f"｜{g2}｜　　※浮生城{fl_w}樓※　　｜{g8}｜\n"
        f"｜{g3}｜　　　　　　　　　　　｜{g7}｜\n"
        f"｜｜｜｜｜{g4}｜{g5}｜{g6}｜｜｜｜｜"
    )
    desc = grid_top + "\n" + grid_mid

    down = f"city_life_{floor - 1}f" if floor > 4 else "city_life_3f"
    up = f"city_life_{floor + 1}f" if floor < 10 else None

    exits: list[dict] = [
        {"direction": "電梯間", "to": "city_life_elevator"},
        {"direction": "下樓梯", "to": down},
    ]
    if up:
        exits.append({"direction": "上樓梯", "to": up})

    objects: list[dict] = [
        {
            "id": "city_life_elevator",
            "name": "電梯間",
            "owner": "",
            "sockets": ["Move"],
            "responses": {"Move": "你前往「電梯間」。"},
            "move_to_room_id": "city_life_elevator",
        },
        {
            "id": down,
            "name": "下樓梯",
            "owner": "",
            "sockets": ["Move"],
            "responses": {
                "Move": f'你前往「浮生城{fw_digits(str(floor - 1))}樓」。'
                if floor > 4
                else '你前往「浮生城3F」。'
            },
            "move_to_room_id": down,
        },
    ]
    if up:
        objects.append(
            {
                "id": up,
                "name": "上樓梯",
                "owner": "",
                "sockets": ["Move"],
                "responses": {"Move": f'你前往「浮生城{fw_digits(str(floor + 1))}樓」。'},
                "move_to_room_id": up,
            }
        )

    for digit in range(1, 10):
        c = unit_code(floor, digit)
        cfw = fw_digits(c)
        liv_id = room_id(prefix, c, "liv")
        exits.append({"direction": cfw, "to": liv_id})
        objects.append(
            {
                "id": liv_id + "_from_lobby",
                "name": cfw,
                "owner": "",
                "sockets": ["Move"],
                "responses": {"Move": f'你進入戶{cfw}，來到客廳。'},
                "move_to_room_id": liv_id,
            }
        )

    lobby = {
        "id": lobby_id,
        "name": f"浮生城{floor}F",
        "description": desc,
        "tags": ["city", "skyscraper"],
        "zone": "citylife",
        "exits": exits,
        "objects": objects,
    }

    lobby_path = os.path.join(OUT, f"{lobby_id}.json")
    with open(lobby_path, "w", encoding="utf-8") as fp:
        json.dump(lobby, fp, ensure_ascii=False, indent=2)
        fp.write("\n")

    for digit in range(1, 10):
        c = unit_code(floor, digit)
        cfw = fw_digits(c)
        typ = UNIT_TYPES[digit]
        specs = ROOM_SPECS[typ]
        liv_id = room_id(prefix, c, "liv")

        liv_data = build_liv_room(prefix, cfw, c, lobby_id, liv_id, typ, specs)
        with open(os.path.join(OUT, f"{liv_id}.json"), "w", encoding="utf-8") as fp:
            json.dump(liv_data, fp, ensure_ascii=False, indent=2)
            fp.write("\n")

        for suf, title, rdesc in specs[1:]:
            rid = room_id(prefix, c, suf)
            leaf = {
                "id": rid,
                "name": f"戶{cfw}·{title}",
                "description": rdesc,
                "tags": ["city", "skyscraper", "indoor", "residential"],
                "zone": "citylife",
                "exits": [{"direction": "客廳", "to": liv_id}],
                "objects": [
                    {
                        "id": rid + "_to_liv",
                        "name": "客廳",
                        "owner": "",
                        "sockets": ["Move"],
                        "responses": {"Move": f"你回到戶{cfw}的客廳。"},
                        "move_to_room_id": liv_id,
                    }
                ],
            }
            with open(os.path.join(OUT, f"{rid}.json"), "w", encoding="utf-8") as fp:
                json.dump(leaf, fp, ensure_ascii=False, indent=2)
                fp.write("\n")


def main() -> None:
    for floor in range(4, 11):
        emit_floor(floor)
    print("Wrote city_life_4f..10f lobbies + residential unit JSON under data/rooms/editor/")


if __name__ == "__main__":
    main()
