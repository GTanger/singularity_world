#!/usr/bin/env python3
"""一次性產生浮生城3F病房走廊與病房房間 JSON（data/rooms/editor/）。"""
from __future__ import annotations

import json
import os

ROOT = os.path.join(os.path.dirname(__file__), "..", "data", "rooms", "editor")

TAGS = ["city", "skyscraper", "indoor"]

TYPE_LABEL = {
    "quad": "四人病房",
    "double": "雙人病房",
    "single": "單人病房",
    "icu": "加護病房",
    "hospice": "安寧病房",
}

WARD_DESC = {
    "quad": "四張床擺成兩兩對腳，簾軌在天花拉出橢圓陰影；窗邊留一條窄縫透氣。",
    "double": "雙床間隔著可收合屏風，床頭卡夾寫著體溫紀錄時段。",
    "single": "單人房門上有觀察窗，室內儀器低鳴混著加濕器的細響。",
    "icu": "多參數監護儀佔據牆面，導線束成一束垂向床側；護士站對講偶爾滋一聲。",
    "hospice": "光線調得暖和，置物枱擺著紙巾與水杯；牆角加濕器吐著細霧。",
}

HALL_TITLE = {
    "內科一": "內科病房一號走廊",
    "內科二": "內科病房二號走廊",
    "外科一": "外科病房一號走廊",
    "外科二": "外科病房二號走廊",
    "兒內一": "兒童內科病房一號走廊",
    "兒內二": "兒童內科病房二號走廊",
    "兒外一": "兒童外科病房一號走廊",
    "兒外二": "兒童外科病房二號走廊",
    "加寧房": "加護病房及安寧病房走廊",
}

# (cor_id, 大廳格線名, 走廊敘事一句, 病房類型 x6, 六個顯示標籤 r01..r06)
# 標籤順序對應房號 01..06；描述裡 2x3 排版：上排 6,4,2 下排 5,3,1
WINGS = [
    (
        "city_life_3f_cor_med1",
        "內科一",
        "門牌漆成淺藍底白字，推床輪痕在防滑膠上拉出淺溝；護理站燈還亮著，對講偶爾滋一聲。",
        ("quad", "quad", "quad", "double", "double", "single"),
        ("內101", "內102", "內103", "內104", "內105", "內106"),
    ),
    (
        "city_life_3f_cor_med2",
        "內科二",
        "燈帶比一號略暗，牆上動線圖邊角捲起，有人用膠帶補了一截。",
        ("quad", "quad", "quad", "double", "double", "single"),
        ("內201", "內202", "內203", "內204", "內205", "內206"),
    ),
    (
        "city_life_3f_cor_surg1",
        "外科一",
        "消毒水味較濃，器械推車靠牆鎖輪，踢腳板邊留著淡黃碘漬。",
        ("quad", "quad", "quad", "double", "double", "single"),
        ("外101", "外102", "外103", "外104", "外105", "外106"),
    ),
    (
        "city_life_3f_cor_surg2",
        "外科二",
        "換藥車停在轉角，垃圾袋口打結規矩，標籤寫「敷料／針具」分欄。",
        ("quad", "quad", "quad", "double", "double", "single"),
        ("外201", "外202", "外203", "外204", "外205", "外206"),
    ),
    (
        "city_life_3f_cor_pcm1",
        "兒內一",
        "牆腰漆淺黃，身高貼紙邊緣被小孩抠得起角；空氣裡有淡淡的果汁味消毒劑。",
        ("quad", "quad", "quad", "double", "double", "single"),
        ("兒內101", "兒內102", "兒內103", "兒內104", "兒內105", "兒內106"),
    ),
    (
        "city_life_3f_cor_pcm2",
        "兒內二",
        "護理台貼著卡通貼紙，推床輪子在轉彎處磨出兩道淺弧。",
        ("quad", "quad", "quad", "double", "double", "single"),
        ("兒內201", "兒內202", "兒內203", "兒內204", "兒內205", "兒內206"),
    ),
    (
        "city_life_3f_cor_pcs1",
        "兒外一",
        "牆上掛著簡筆剖面圖，門口鞋套盒快見底。",
        ("quad", "quad", "quad", "double", "double", "single"),
        ("兒外101", "兒外102", "兒外103", "兒外104", "兒外105", "兒外106"),
    ),
    (
        "city_life_3f_cor_pcs2",
        "兒外二",
        "廊燈偏白，推床防撞條磨得發亮，轉角鏡面貼了「慢行」膠字。",
        ("quad", "quad", "quad", "double", "double", "single"),
        ("兒外201", "兒外202", "兒外203", "兒外204", "兒外205", "兒外206"),
    ),
    (
        "city_life_3f_cor_icu",
        "加寧房",
        "加護與安寧共用的緩衝廊；腳步與對講都壓得低，安全燈在地面投出一長條綠。",
        ("icu", "icu", "icu", "hospice", "hospice", "hospice"),
        ("加護01", "加護02", "加護03", "安寧04", "安寧05", "安寧06"),
    ),
]


def grid_labels(labels: tuple[str, ...]) -> tuple[str, str]:
    """r01..r06 labels -> (top_row, bottom_row) 如範例 106/104/102 與 105/103/101。"""
    # index 0 = r01 .. 5 = r06
    l1, l2, l3, l4, l5, l6 = labels
    top = f"｜{l6}｜{l4}｜{l2}｜"
    bot = f"｜{l5}｜{l3}｜{l1}｜"
    return top, bot


def write_json(path: str, data: dict) -> None:
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
        f.write("\n")


def ward_room(cor_id: str, wing: str, idx: int, type_key: str) -> dict:
    hall_title = HALL_TITLE[wing]
    n = f"{idx:02d}"
    rid = f"{cor_id}_r{n}"
    type_zh = TYPE_LABEL[type_key]
    name = f"{wing}·{type_zh}{n}"
    desc = WARD_DESC[type_key]
    return {
        "id": rid,
        "name": name,
        "description": desc + f"\n可經〔走廊〕回到「{hall_title}」。",
        "tags": TAGS,
        "zone": "citylife",
        "exits": [{"direction": "走廊", "to": cor_id}],
        "objects": [
            {
                "id": f"{rid}_to_hall",
                "name": "走廊",
                "owner": "",
                "sockets": ["Move"],
                "responses": {"Move": f"你回到「{hall_title}」。"},
                "move_to_room_id": cor_id,
            }
        ],
    }


def corridor_doc(
    cor_id: str,
    wing: str,
    hall_line: str,
    types: tuple[str, ...],
    labels: tuple[str, ...],
) -> dict:
    hall_title = HALL_TITLE[wing]
    top, bot = grid_labels(labels)
    body = f"{hall_title}。{hall_line}" if hall_line else hall_title
    # 「大廳」無括號，詞界替換為 Move（與 objects 裡 name「大廳」一致）
    wayfinding = "牆上標示著動線指示：大廳←人→病房"
    desc = f"{top}\n\n{body}\n{wayfinding}\n\n{bot}"
    objects = [
        {
            "id": f"{cor_id}_to_lobby",
            "name": "大廳",
            "owner": "",
            "sockets": ["Move"],
            "responses": {"Move": "你回到「浮生城3F」。"},
            "move_to_room_id": "city_life_3f",
        }
    ]
    for i, (t, lab) in enumerate(zip(types, labels, strict=True), start=1):
        n = f"{i:02d}"
        rid = f"{cor_id}_r{n}"
        type_zh = TYPE_LABEL[t]
        objects.append(
            {
                "id": f"{cor_id}_obj_{n}",
                "name": lab,
                "owner": "",
                "sockets": ["Move"],
                "responses": {"Move": f"你推門進入「{wing}·{type_zh}{n}」。"},
                "move_to_room_id": rid,
            }
        )
    return {
        "id": cor_id,
        "name": hall_title,
        "description": desc,
        "tags": TAGS,
        "zone": "citylife",
        "exits": [{"direction": "大廳", "to": "city_life_3f"}],
        "objects": objects,
    }


def main() -> None:
    os.makedirs(ROOT, exist_ok=True)
    for cor_id, wing, hall_line, types, labels in WINGS:
        cdoc = corridor_doc(cor_id, wing, hall_line, types, labels)
        write_json(os.path.join(ROOT, f"{cor_id}.json"), cdoc)
        for i, (t, _) in enumerate(zip(types, labels, strict=True), start=1):
            wdoc = ward_room(cor_id, wing, i, t)
            write_json(os.path.join(ROOT, f"{cor_id}_r{i:02d}.json"), wdoc)
    print(f"wrote {len(WINGS)} corridors + {len(WINGS)*6} wards under {ROOT}")


if __name__ == "__main__":
    main()
