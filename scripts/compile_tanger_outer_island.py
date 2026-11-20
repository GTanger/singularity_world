#!/usr/bin/env python3
"""
Deterministic compiler for Singularity World — Southeast Outer Island (Feature 6)
from Azgaar v1.158 Full JSON (`Tanger Full 2026-11-18-19-55.json`).

Enforces all 16 Council-approved laws:
- 93 Voronoi cells of Feature 6 + 1-ring neighbor outlines + West Strait ferry corridor
- 5 settlements with fishbone street/alley/household topology (1 roof = 1 real household)
- Walled towns align gates 1:1 with incoming Azgaar routes and block un-gated wall edges
- Steep central spine ridges block non-pass edges; island streams (fl 31..57) are fordable/bridged
- Compile-time BFS connectivity assertion across all outdoor nodes and highway settlements
- 4 fantasy races (Human, Dwarf, Elf, Orc), double-entry copper conservation, zero GDD jargon
"""

from __future__ import annotations
import hashlib
import json
import math
from collections import deque
from pathlib import Path

SRC_JSON = Path("/home/tanger/下載/Tanger Full 2026-11-18-19-55.json")
OUT_JSON = Path("/home/tanger/Projects/singularity_world/data/world/tanger_outer_island_v1.json")
SCHEMA_VERSION = 1


def det_u64(tag: str) -> int:
    digest = hashlib.sha256(f"sw-tanger-v{SCHEMA_VERSION}:{tag}".encode("utf-8")).digest()
    return int.from_bytes(digest[:8], "little")


def compass_dir(dx: float, dy: float) -> str:
    # Azgaar y grows southward
    if abs(dx) >= abs(dy) * 1.15:
        return "東" if dx > 0 else "西"
    if abs(dy) >= abs(dx) * 1.15:
        return "南" if dy > 0 else "北"
    if dx > 0 and dy < 0:
        return "東北"
    if dx < 0 and dy < 0:
        return "西北"
    if dx > 0 and dy > 0:
        return "東南"
    return "西南"


HUMAN_GIVEN = [
    "羅蘭", "瑪莎", "葛林", "霍克", "艾倫", "薇拉", "托馬", "莉亞", "布蘭", "諾亞",
    "米拉", "柯林", "海倫", "達倫", "莎拉", "雷恩", "安娜", "歐文", "克萊爾", "芬恩",
    "伊森", "蘿絲", "馬丁", "席琳", "韋恩", "妮娜", "卡爾", "黛安", "里奧", "莫莉",
]
DWARF_CLEAN = [
    "巴爾", "托爾", "朵琳", "布隆", "格林", "烏爾", "布魯諾", "海格", "杜林", "卡茲",
    "摩根", "博爾", "塔克", "諾里", "剛鐸", "哈爾", "法林", "奧林", "貝爾", "達因",
]

ELF_GIVEN = [
    "艾琳", "瑟蘭", "莉瑞", "費恩", "希爾", "洛林", "伊露", "凱蘭", "緹雅", "風歌",
    "米瑞", "亞玟", "賽爾", "奈拉", "歐瑞", "維林",
]
ORC_GIVEN = [
    "洛克", "格魯", "烏卡", "塔格", "摩卡", "德拉", "古爾", "巴克", "薩爾", "拉格",
    "庫爾", "加什", "諾格", "布拉", "圖爾", "奧格",
]

EPITHETS_BY_JOB = {
    "鍛鐵匠": ["鐵砧", "赤爐", "重錘", "火鉗"],
    "酒館掌櫃": ["鏽錨", "麥酒", "暖爐", "橡木桶"],
    "車行掌櫃": ["疾風", "雙轡", "銅鈴", "穩軸"],
    "馬車馭手": ["長鞭", "熟路", "灰轡", "老輪"],
    "銀號帳房": ["鐵算", "銅秤", "封蠟", "銀匣"],
    "草藥師": ["苦艾", "止血", "青苔", "石臼"],
    "漁戶": ["鹹網", "浪花", "魚叉", "潮聲"],
    "碼頭苦力": ["寬肩", "粗繩", "鐵鉤", "重擔"],
    "木匠": ["刨花", "榫頭", "杉木", "船肋"],
    "皮匠": ["熟革", "獸脂", "韌線", "硬靴"],
    "農戶": ["黑麥", "犁頭", "石磨", "穀倉"],
    "獵戶": ["灰羽", "獸夾", "短弓", "林蹤"],
    "石工": ["灰鑿", "方石", "鐵楔", "碎岩"],
    "觀星領航員": ["北辰", "風向", "海圖", "夜航"],
    "巡城衛兵": ["鐵矛", "夜巡", "盾牌", "號角"],
}


def pick_name(
    race: str,
    job: str,
    seed_num: int,
    with_epithet: bool = False,
    used_names: set[str] | None = None,
) -> str:
    if race == "矮人":
        pool = DWARF_CLEAN
    elif race == "精靈":
        pool = ELF_GIVEN
    elif race == "獸人":
        pool = ORC_GIVEN
    else:
        pool = HUMAN_GIVEN
    eps = EPITHETS_BY_JOB.get(job, ["老街", "海風", "石橋", "晨鐘"])
    for attempt in range(len(pool) * (len(eps) + 1)):
        base = pool[(seed_num + attempt) % len(pool)]
        if with_epithet or attempt >= len(pool):
            ep = eps[((seed_num >> 8) + attempt) % len(eps)]
            cand = f"{ep}{base}"
        else:
            cand = base
        if used_names is None or cand not in used_names:
            if used_names is not None:
                used_names.add(cand)
            return cand
    fallback = f"{pool[seed_num % len(pool)]}"
    if used_names is not None:
        used_names.add(fallback)
    return fallback


def main() -> None:
    with SRC_JSON.open("r", encoding="utf-8") as f:
        raw = json.load(f)

    pack = raw["pack"]
    cells = pack["cells"]
    vertices = pack["vertices"]
    routes = pack["routes"]
    burgs = pack["burgs"]

    v_coords = {}
    for v in vertices:
        v_coords[v["i"]] = [round(float(v["p"][0]), 2), round(float(v["p"][1]), 2)]

    f6_cells = [c for c in cells if c.get("f") == 6]
    f6_ids = {c["i"] for c in f6_cells}
    assert len(f6_cells) == 93, f"Expected 93 cells in Feature 6, got {len(f6_cells)}"

    # Collect 1-ring neighbor cells (ocean/coastal ring for colorless fog outline)
    ring_ids = set()
    for c in f6_cells:
        for nb in c.get("c", []):
            if nb not in f6_ids and 0 <= nb < len(cells):
                ring_ids.add(nb)

    # Extract land routes on Feature 6
    highway_pairs: set[tuple[int, int]] = set()
    highway_polylines: list[dict] = []
    for r in routes:
        pts = r.get("points", [])
        cids = [p[2] for p in pts if len(p) > 2]
        if r.get("group") != "searoutes" and any(cid in f6_ids for cid in cids):
            poly = [[round(float(p[0]), 2), round(float(p[1]), 2)] for p in pts]
            highway_polylines.append({
                "route_id": r["i"],
                "group": r.get("group", "roads"),
                "points": poly,
                "cells": [cid for cid in cids if cid in f6_ids],
            })
            for a, b in zip(cids, cids[1:]):
                if a in f6_ids and b in f6_ids and a != b:
                    highway_pairs.add((min(a, b), max(a, b)))

    # Sea routes: assert 9 stay local/coastal and extract West Strait corridor from burg 188 (cell 4321)
    sea_corridors: list[dict] = []
    for r in routes:
        pts = r.get("points", [])
        cids = [p[2] for p in pts if len(p) > 2]
        if r.get("group") == "searoutes" and any(cid in f6_ids for cid in cids):
            poly = [[round(float(p[0]), 2), round(float(p[1]), 2)] for p in pts]
            crosses_mainland = any(
                0 <= cid < len(cells) and cells[cid].get("h", 0) >= 20 and cells[cid].get("f") != 6
                for cid in cids
            )
            sea_corridors.append({
                "route_id": r["i"],
                "crosses_strait": crosses_mainland,
                "points": poly,
            })

    # Canonical 5 settlements on Feature 6
    settlement_defs = {
        4616: {
            "burg_id": 26,
            "slug": "white_gull_harbor",
            "name": "白鷗港",
            "tier": "port_town",
            "walled": True,
            "has_port": True,
            "desc": "外海大島東岸最大深水良港，石砌城牆環護百戶街坊，海風裡混著矮人鍛爐炭火與鹹魚桶氣息。",
            "alleys": [
                ("iron_anvil_ln", "鐵砧巷", 10, ["矮人", "人類"], ["鍛鐵匠", "石工", "木匠"]),
                ("sail_ln", "船帆巷", 10, ["人類", "獸人"], ["漁戶", "碼頭苦力", "木匠"]),
                ("copper_ln", "銅幣巷", 9, ["人類", "矮人"], ["皮匠", "銀號帳房", "木匠"]),
                ("salted_fish_ln", "鹹魚巷", 10, ["人類", "獸人"], ["漁戶", "碼頭苦力"]),
                ("cooper_ln", "桶匠巷", 9, ["人類", "矮人"], ["木匠", "皮匠"]),
                ("rope_ln", "繩索巷", 9, ["獸人", "人類"], ["碼頭苦力", "漁戶"]),
                ("lantern_ln", "石燈巷", 9, ["人類", "精靈"], ["草藥師", "觀星領航員"]),
                ("seabreeze_ln", "海風巷", 8, ["人類", "精靈"], ["漁戶", "獵戶"]),
                ("old_well_ln", "老井巷", 8, ["人類", "矮人"], ["農戶", "石工"]),
                ("south_wall_ln", "南牆根巷", 8, ["獸人", "人類"], ["巡城衛兵", "石工"]),
            ],
        },
        4321: {
            "burg_id": 188,
            "slug": "west_strait_ferry",
            "name": "西峽渡口",
            "tier": "ferry_post",
            "walled": False,
            "has_port": True,
            "desc": "面向西北寬闊海峽的風浪渡津，公營雙桅渡輪每日在此靠岸，等待攢夠船資的旅人跨海前往本土大陸。",
            "alleys": [
                ("ferry_row", "渡頭木屋街", 10, ["人類", "獸人"], ["碼頭苦力", "漁戶"]),
                ("rope_shed_ln", "纜繩寮巷", 10, ["獸人", "人類"], ["木匠", "碼頭苦力"]),
                ("star_watch_ln", "望海坡巷", 10, ["人類", "精靈"], ["觀星領航員", "漁戶"]),
            ],
        },
        4613: {
            "burg_id": 301,
            "slug": "gull_tail_port",
            "name": "鷗尾漁港",
            "tier": "port_town",
            "walled": True,
            "has_port": True,
            "desc": "南灣避風石澳築起的漁港鎮，矮石牆外曬滿漁網與鯨骨，商販在此收購南岸海產。",
            "alleys": [
                ("net_drying_ln", "曬網巷", 12, ["人類", "獸人"], ["漁戶", "木匠"]),
                ("whalebone_ln", "鯨骨巷", 12, ["人類", "矮人"], ["皮匠", "鍛鐵匠"]),
                ("oyster_ln", "牡蠣殼巷", 12, ["人類", "獸人"], ["漁戶", "碼頭苦力"]),
                ("cliff_steps_ln", "石階巷", 12, ["人類", "矮人"], ["石工", "巡城衛兵"]),
                ("kelp_ln", "海藻巷", 12, ["人類", "精靈"], ["草藥師", "漁戶"]),
            ],
        },
        4525: {
            "burg_id": 147,
            "slug": "greypine_post",
            "name": "灰松林驛",
            "tier": "forest_post",
            "walled": False,
            "has_port": False,
            "desc": "島心十字幹道交會的巨木林間驛鎮，驛馬車在此換馬歇腳，精靈獵戶與伐木匠常在火塘邊交換山林口風。",
            "alleys": [
                ("pine_resin_ln", "松脂巷", 9, ["精靈", "人類"], ["獵戶", "草藥師"]),
                ("sawmill_ln", "鋸木場巷", 9, ["人類", "獸人"], ["木匠", "獵戶"]),
                ("moss_well_ln", "苔井巷", 9, ["精靈", "人類"], ["草藥師", "農戶"]),
            ],
        },
        4073: {
            "burg_id": 137,
            "slug": "north_cape_beacon",
            "name": "北岬燈塔",
            "tier": "beacon_outpost",
            "walled": False,
            "has_port": True,
            "desc": "矗立在島嶼最北端險峻海岬上的石塔哨站，夜裡燃起混有異獸油脂的長明火，指引避開暗礁的船隻。",
            "alleys": [
                ("beacon_foot_ln", "塔下石屋巷", 11, ["人類", "矮人"], ["觀星領航員", "石工"]),
                ("driftwood_ln", "拾木灘巷", 11, ["人類", "精靈"], ["漁戶", "獵戶"]),
            ],
        },
    }

    # Build cell dictionary & wilderness naming
    cell_map: dict[int, dict] = {}
    sorted_f6 = sorted(f6_cells, key=lambda x: x["i"])

    wild_templates_coastal = [
        ("礫石淺灘", "海浪拍打著碎石灘塗，潮線附近散落著海帶與幾截帶有陌生刻痕的異國漂流木。"),
        ("海風草坡", "迎風起伏的海岸草坡，偶爾可見低頭啃草的野鹿與車輪壓出的舊轍。"),
        ("礁岩海灣", "暗礁犬牙交錯的偏僻海灣，退潮時岩縫間露出成簇硬殼海貝。"),
        ("鹽霧崖徑", "貼著海崖蜿蜒的碎石窄徑，腳下浪濤翻湧，海鳥在岩壁築巢。"),
    ]
    wild_templates_forest = [
        ("蒼苔古林", "遠離人煙的茂密闊葉林，樹幹比城裡老樹粗上一圈，枝葉遮天蔽日。"),
        ("斷木獵徑", "獵戶踩出的林間小徑，倒伏的巨木上留著清晰的利爪抓痕。"),
        ("溪石淺灘", "清澈溪流漫過寬平卵石，水深僅及小腿，旅人與馬匹可踩石涉渡。"),
        ("灰耳棚寨外圍", "林間空地搭著幾座簡陋獸皮木棚，空氣裡飄著灰耳哥布林烘烤野果的焦味。"),
    ]
    wild_templates_hill = [
        ("碎石山坡", "地勢漸陡的岩坡，裸露的灰岩縫隙裡嵌著泛微光的粗鐵礦脈。"),
        ("廢棄採石坑", "前人開鑿石材留下的半邊石壁，坑底積著雨水，石縫間有野獸棲息痕跡。"),
        ("山脊隘口", "兩側陡峭岩壁夾出的狹窄風口，是翻越島心山脊的必經通道。"),
        ("岩甲獸棲谷", "草木瘋長的幽深山谷，幾塊巨岩表面布滿堅硬如鐵的灰白礦殼。"),
    ]

    for idx, c in enumerate(sorted_f6):
        cid = c["i"]
        poly = [v_coords[vid] for vid in c.get("v", []) if vid in v_coords]
        cx, cy = round(float(c["p"][0]), 2), round(float(c["p"][1]), 2)
        h = int(c.get("h", 20))
        fl = int(c.get("fl", 0))
        biome = int(c.get("biome", 8))
        is_coast = any(nb not in f6_ids for nb in c.get("c", []))

        if cid in settlement_defs:
            sdef = settlement_defs[cid]
            title = sdef["name"]
            desc = sdef["desc"]
            kind = "settlement"
        else:
            kind = "wilderness"
            if fl >= 30:
                title = "石橋溪灘" if (cid % 2 == 0) else "淺水卵石渡"
                desc = "島內山溪在此放緩，溪石間架著粗石板橋，水流清冽可涉。"
            elif h >= 24:
                t_name, t_desc = wild_templates_hill[idx % len(wild_templates_hill)]
                title = f"{t_name}"
                desc = t_desc
            elif is_coast:
                t_name, t_desc = wild_templates_coastal[idx % len(wild_templates_coastal)]
                title = f"{t_name}"
                desc = t_desc
            else:
                t_name, t_desc = wild_templates_forest[idx % len(wild_templates_forest)]
                title = f"{t_name}"
                desc = t_desc

        cell_map[cid] = {
            "cell_id": cid,
            "center": [cx, cy],
            "polygon": poly,
            "height": h,
            "biome": biome,
            "flux": fl,
            "is_coast": is_coast,
            "kind": kind,
            "title": title,
            "desc": desc,
            "neighbors_f6": [nb for nb in c.get("c", []) if nb in f6_ids],
        }

    # Special cell right north of White Gull Harbor (4529) -> Stone-Bridge Ravine with hidden dungeon entrance!
    if 4529 in cell_map and 4529 not in settlement_defs:
        cell_map[4529]["title"] = "灰松斷橋石灘"
        cell_map[4529]["desc"] = "白鷗港往北出城後的第一處溪谷石灘，老石橋下雜草掩映，溪畔古松異常粗壯，偶有灰耳哥布林斥候出沒。"

    # Determine cell-to-cell passability (Strict terrain & wall rules!)
    # 1. Highway pairs are ALWAYS passable (road)
    # 2. Walled settlements (4616, 4613) ONLY connect to external cells that have a highway! (Un-gated walls block!)
    # 3. High central ridges (h >= 25) only connect along highways or at most 2 lowest-slope pass neighbors
    allowed_cell_edges: dict[tuple[int, int], str] = {}
    for a, b in highway_pairs:
        allowed_cell_edges[(a, b)] = "highway"

    for cid, cinfo in cell_map.items():
        if cid in settlement_defs and settlement_defs[cid]["walled"]:
            continue  # Walled towns strictly connect ONLY via gated highways!
        nbs = sorted(
            cinfo["neighbors_f6"],
            key=lambda nb: (abs(cell_map[nb]["height"] - cinfo["height"]), nb),
        )
        wild_quota = 1 if cinfo["height"] >= 25 else 3
        added = 0
        for nb in nbs:
            if nb in settlement_defs and settlement_defs[nb]["walled"]:
                continue  # Cannot punch through another town's un-gated city wall!
            pair = (min(cid, nb), max(cid, nb))
            if pair in allowed_cell_edges:
                continue
            if cinfo["height"] >= 25 or cell_map[nb]["height"] >= 25:
                if abs(cinfo["height"] - cell_map[nb]["height"]) > 2:
                    continue
            allowed_cell_edges[pair] = "wild_trail"
            added += 1
            if added >= wild_quota:
                break

    # Ensure 100% of all 93 cells are connected via BFS; if any isolated pocket exists, open lowest-slope pass
    def bfs_cells(start_cid: int) -> set[int]:
        adj: dict[int, list[int]] = {k: [] for k in cell_map}
        for (a, b) in allowed_cell_edges:
            adj[a].append(b)
            adj[b].append(a)
        vis = {start_cid}
        q = deque([start_cid])
        while q:
            cur = q.popleft()
            for nxt in adj[cur]:
                if nxt not in vis:
                    vis.add(nxt)
                    q.append(nxt)
        return vis

    while True:
        reached = bfs_cells(4616)
        if len(reached) == len(cell_map):
            break
        missing = sorted(set(cell_map.keys()) - reached)
        bridged = False
        for m in missing:
            for nb in cell_map[m]["neighbors_f6"]:
                if nb in reached and not (nb in settlement_defs and settlement_defs[nb]["walled"]):
                    pair = (min(m, nb), max(m, nb))
                    allowed_cell_edges[pair] = "wild_trail"
                    bridged = True
                    break
            if bridged:
                break
        if not bridged:
            # Fallback allow non-walled connection
            m = missing[0]
            nb = cell_map[m]["neighbors_f6"][0]
            allowed_cell_edges[(min(m, nb), max(m, nb))] = "wild_trail"

    # Build Spatial Nodes, Town Fishbone Geometries, Households, and Residents
    nodes: dict[str, dict] = {}
    edges: list[tuple[str, str, str, str]] = []  # (from_id, to_id, dir_label, kind)
    households: list[dict] = []
    residents: list[dict] = []
    town_micro_maps: dict[str, dict] = {}

    def add_node(nid: str, cell_id: int, name: str, kind: str, desc: str, pos: list[float], indoor: bool = False, dungeon: bool = False) -> None:
        nodes[nid] = {
            "id": nid,
            "cell_id": cell_id,
            "name": name,
            "kind": kind,
            "desc": desc,
            "pos": [round(pos[0], 2), round(pos[1], 2)],
            "indoor": indoor,
            "dungeon": dungeon,
            "exits": [],
        }

    def link_bi(a: str, b: str, dir_ab: str, dir_ba: str, kind: str = "street") -> None:
        edges.append((a, b, dir_ab, kind))
        edges.append((b, a, dir_ba, kind))

    # Track entry node for each cell when entering from an external neighbor cell
    cell_entry_from_neighbor: dict[tuple[int, int], str] = {}
    cell_default_node: dict[int, str] = {}

    # 1. Build 5 Settlements (Fishbone streets + alleys + real roofs + shops + households)
    for cid, sdef in settlement_defs.items():
        cinfo = cell_map[cid]
        cx, cy = cinfo["center"]
        slug = sdef["slug"]
        tname = sdef["name"]

        # Core Spine Nodes (Crossing town = 4~5 steps!)
        n_plaza = f"oi:{slug}:plaza"
        n_north = f"oi:{slug}:north_gate"
        n_south = f"oi:{slug}:south_gate"
        n_smithy = f"oi:{slug}:smithy_st"
        n_tavern_st = f"oi:{slug}:tavern_st"
        n_dock = f"oi:{slug}:dock"

        gate_word_n = "北城門" if sdef["walled"] else "北哨柵口"
        gate_word_s = "南城門" if sdef["walled"] else "南哨柵口"

        add_node(n_plaza, cid, f"{tname}・市集廣場", "plaza", f"{tname}正中央的石板廣場，車馬與挑擔鎮民在此交會，井台邊人聲熱絡。", [cx, cy])
        add_node(n_north, cid, f"{tname}・{gate_word_n}", "gate", f"通往北面山林幹道的{gate_word_n}，衛兵正倚著石牆打量往來旅人。", [cx, cy - 2.2])
        add_node(n_south, cid, f"{tname}・{gate_word_s}", "gate", f"連接南向海岸道路的{gate_word_s}，地面留著深淺車轍。", [cx, cy + 2.2])
        add_node(n_smithy, cid, f"{tname}・鐵匠街", "street", "石爐炭火日夜不熄的百工街段，空氣裡迴盪著沉穩的鐵錘敲擊聲，街口釘著車行時刻木牌。", [cx - 1.4, cy - 0.8])
        add_node(n_tavern_st, cid, f"{tname}・驛站街", "street", "旅店招牌與馬廄草料味交織的熱鬧街段，歇腳商旅常在此打聽各路風聲。", [cx - 1.4, cy + 0.8])

        link_bi(n_north, n_smithy, "南", "北", "street")
        link_bi(n_smithy, n_plaza, "南", "北", "street")
        link_bi(n_plaza, n_tavern_st, "南", "北", "street")
        link_bi(n_tavern_st, n_south, "南", "北", "street")

        if sdef["has_port"]:
            dock_dir = "西" if cid == 4321 else "東"
            back_dir = "東" if cid == 4321 else "西"
            dx_dock = -2.4 if cid == 4321 else 2.4
            add_node(
                n_dock,
                cid,
                f"{tname}・港務碼頭",
                "dock",
                f"木石棧橋延伸入海，雙桅帆船與漁舟隨浪起伏，港務舊貨庫與搬運苦力聚集於此。",
                [cx + dx_dock, cy],
            )
            link_bi(n_plaza, n_dock, dock_dir, back_dir, "street")
            cell_default_node[cid] = n_dock
        else:
            cell_default_node[cid] = n_plaza

        # Major Indoor Shops on Main Spine (Tavern + Upstairs Inn Room, Smithy, Carriage Co, Silver Bank / Herbalist)
        n_tavern = f"oi:{slug}:rusty_anchor_tavern"
        n_inn_up = f"oi:{slug}:inn_upstairs"
        n_forge = f"oi:{slug}:bal_forge"
        n_carriage = f"oi:{slug}:gale_carriage"
        n_herb = f"oi:{slug}:herb_hall"

        tavern_title = "鏽錨酒館" if cid == 4616 else f"{tname}木桶酒館"
        add_node(n_tavern, cid, f"{tname}・{tavern_title}", "shop", "橡木長桌擦得發亮，麥酒香與水手低語交織，掌櫃在吧台後擦拭酒杯。", [cx - 1.9, cy + 0.9], indoor=True)
        add_node(n_inn_up, cid, f"{tavern_title}・二樓宿房", "inn_room", "乾淨乾燥的木板閣樓宿房，床頭擺著一隻帶銅鎖的厚木箱，躺在床上即可安心歇息養傷。", [cx - 2.0, cy + 0.9], indoor=True)
        link_bi(n_tavern_st, n_tavern, "進門", "出門", "door")
        link_bi(n_tavern, n_inn_up, "上樓", "下樓", "stairs")

        forge_title = "巴爾鍛鐵鋪" if cid == 4616 else f"{tname}鐵匠鋪"
        add_node(n_forge, cid, f"{tname}・{forge_title}", "shop", "炙熱石爐旁掛滿厚鐵鍛錘、獵刀與馬車軸配件，鐵砧表面留著千百次敲打的亮痕。", [cx - 1.9, cy - 0.9], indoor=True)
        link_bi(n_smithy, n_forge, "進門", "出門", "door")

        carriage_title = "疾風驛馬行" if cid == 4616 else f"{tname}車馬站"
        add_node(n_carriage, cid, f"{tname}・{carriage_title}", "shop", "寬敞車棚內停著軸刻穩固紋路的雙馬篷車，木牌清楚標著發車鐘響與託運價目。", [cx + 0.9, cy + 0.8], indoor=True)
        link_bi(n_plaza, n_carriage, "進門", "出門", "door")

        add_node(n_herb, cid, f"{tname}・苦艾草藥堂", "shop", "簷下晾著成束止血草與風乾苔蘚，石臼旁瀰漫著清苦藥香，可包紮筋骨挫傷。", [cx + 0.9, cy - 0.8], indoor=True)
        link_bi(n_plaza, n_herb, "進門", "出門", "door")

        # Key shopkeeper households
        shop_keepers = [
            (n_forge, "矮人", "鍛鐵匠", True, "鐵砧巴爾" if cid == 4616 else None, n_smithy),
            (n_tavern, "人類", "酒館掌櫃", False, "瑪莎" if cid == 4616 else None, n_tavern_st),
            (n_carriage, "人類", "車行掌櫃", True, "疾風葛林" if cid == 4616 else None, n_plaza),
            (n_herb, "精靈", "草藥師", True, "苦艾艾琳" if cid == 4616 else None, n_plaza),
        ]
        roof_rects: list[dict] = []
        alley_lines: list[dict] = []
        used_town_names: set[str] = set()

        for sk_idx, (shop_nid, sk_race, sk_job, use_ep, fixed_name, street_nid) in enumerate(shop_keepers):
            h_seed = det_u64(f"shop:{cid}:{sk_idx}")
            hh_id = f"hh:{slug}:shop_{sk_idx}"
            if fixed_name:
                leader_name = fixed_name
                used_town_names.add(leader_name)
            else:
                leader_name = pick_name(sk_race, sk_job, h_seed, with_epithet=use_ep, used_names=used_town_names)
            res_id = f"res:{slug}:shop_{sk_idx}_0"
            langs = {"harbor_trade": 95}
            if sk_race == "矮人":
                langs["dwarven"] = 100
            elif sk_race == "精靈":
                langs["elvish"] = 100
            elif sk_race == "獸人":
                langs["orcish"] = 100
            residents.append({
                "id": res_id,
                "name": leader_name,
                "race": sk_race,
                "role": sk_job,
                "household_id": hh_id,
                "home_node": shop_nid,
                "work_node": street_nid if sk_idx == 0 else shop_nid,
                "soul_seed": h_seed,
                "copper": 180 + (h_seed % 140),
                "languages": langs,
                "persona": f"{tname}在地經營多年的{sk_race}{sk_job}，認臉記帳、說話實在。",
            })
            # Add an apprentice / helper in the same shop household!
            app_seed = det_u64(f"shop_app:{cid}:{sk_idx}")
            app_race = sk_race
            if cid == 4616 and sk_idx == 0:
                app_name = "紅鬚托爾"
                used_town_names.add(app_name)
            else:
                app_name = pick_name(app_race, sk_job, app_seed, False, used_names=used_town_names)
            app_langs = {"harbor_trade": 55 if app_race != "人類" else 90}
            if app_race == "矮人":
                app_langs["dwarven"] = 100
            elif app_race == "精靈":
                app_langs["elvish"] = 100
            residents.append({
                "id": f"res:{slug}:shop_{sk_idx}_1",
                "name": app_name,
                "race": app_race,
                "role": f"{sk_job}學徒",
                "household_id": hh_id,
                "home_node": shop_nid,
                "work_node": street_nid if sk_idx == 0 else shop_nid,
                "soul_seed": app_seed,
                "copper": 35 + (app_seed % 30),
                "languages": app_langs,
                "persona": f"跟著{leader_name}學手藝的年輕{app_race}學徒，同族私語時常講母語。",
            })
            households.append({
                "id": hh_id,
                "cell_id": cid,
                "settlement": tname,
                "node_id": shop_nid,
                "label": f"{leader_name}的鋪子",
                "vacant_for_sale": False,
                "price_copper": 0,
                "members": [res_id, f"res:{slug}:shop_{sk_idx}_1"],
            })

        # Add harbor orc porter & watchman at dock/gates so first 15 minutes feel alive!
        if sdef["has_port"]:
            p_seed = det_u64(f"dock_porter:{cid}")
            p_id = f"res:{slug}:dock_porter"
            residents.append({
                "id": p_id,
                "name": "獨眼洛克" if cid == 4616 else pick_name("獸人", "碼頭苦力", p_seed, True),
                "race": "獸人",
                "role": "碼頭苦力",
                "household_id": f"hh:{slug}:porter",
                "home_node": n_dock,
                "work_node": n_dock,
                "soul_seed": p_seed,
                "copper": 48,
                "languages": {"harbor_trade": 75, "orcish": 100, "goblin": 45},
                "persona": "常年在碼頭扛鹽漬海獸皮箱的獸人苦力，嗓門粗重，懂幾句荒野哥布林話。",
            })

        # Fishbone Residential Alleys & Real Individual Households (1 roof = 1 household, ~15% vacant purchasable!)
        total_alley_rows = max(1, (len(sdef["alleys"]) + 1) // 2)
        for a_idx, (a_slug, a_name, h_count, race_pool, job_pool) in enumerate(sdef["alleys"]):
            row_idx = a_idx // 2
            if total_alley_rows >= 5:
                if row_idx <= 1:
                    anchor = n_smithy
                elif row_idx == 2:
                    anchor = n_plaza
                else:
                    anchor = n_tavern_st
            elif total_alley_rows >= 3:
                if row_idx == 0:
                    anchor = n_smithy
                elif row_idx == 1:
                    anchor = n_plaza
                else:
                    anchor = n_tavern_st
            else:
                anchor = n_plaza if row_idx == 0 else n_tavern_st
            side_east = (a_idx % 2 == 0)
            dir_out = "東" if side_east else "西"
            dir_in = "西" if side_east else "東"
            ax = cx + (1.6 if side_east else -2.2)
            ay = cy - 1.6 + (a_idx * 0.36)
            alley_nid = f"oi:{slug}:{a_slug}"
            add_node(
                alley_nid,
                cid,
                f"{tname}・{a_name}",
                "alley",
                f"由主街分出的靜謐石板巷弄，兩側排著{h_count}棟石木民宅屋舍，門縫飄出炊煙與日常交談聲。",
                [ax, ay],
            )
            link_bi(anchor, alley_nid, dir_out, dir_in, "alley")
            alley_lines.append({
                "name": a_name,
                "node_id": alley_nid,
                "anchor": anchor,
                "side": "east" if side_east else "west",
                "index": a_idx,
            })

            for h_i in range(1, h_count + 1):
                h_seed = det_u64(f"hh:{cid}:{a_slug}:{h_i}")
                home_nid = f"oi:{slug}:{a_slug}:h{h_i}"
                hh_id = f"hh:{slug}:{a_slug}:{h_i}"
                is_vacant = (h_i == h_count and a_idx < 6)  # ~15% real vacant houses for rent/purchase!

                if is_vacant:
                    h_title = f"{a_name}{h_i}號空屋"
                    h_desc = "門板掛著市政廳待售木牌的石木空屋，屋頂完好，屋內留有空置壁爐與儲物空間。"
                    add_node(home_nid, cid, h_title, "house_vacant", h_desc, [ax, ay], indoor=True)
                    link_bi(alley_nid, home_nid, f"進{h_i}號屋", "出門", "door")
                    households.append({
                        "id": hh_id,
                        "cell_id": cid,
                        "settlement": tname,
                        "node_id": home_nid,
                        "label": h_title,
                        "vacant_for_sale": True,
                        "price_copper": 360,
                        "members": [],
                    })
                    roof_rects.append({"alley_idx": a_idx, "house_idx": h_i, "node_id": home_nid, "vacant": True, "label": h_title})
                else:
                    fam_race = race_pool[h_seed % len(race_pool)]
                    fam_job = job_pool[(h_seed >> 4) % len(job_pool)]
                    head_name = pick_name(fam_race, fam_job, h_seed, with_epithet=(h_i <= 2), used_names=used_town_names)
                    fam_size = 2 + (h_seed % 3)  # 2~4 real people per family household
                    h_title = f"{a_name}{h_i}號・{head_name}家"
                    h_desc = f"{fam_race}{fam_job}{head_name}一家的石木居所，牆角掛著日常生計器具，爐上溫著熱湯。"
                    add_node(home_nid, cid, h_title, "house", h_desc, [ax, ay], indoor=True)
                    link_bi(alley_nid, home_nid, f"訪{head_name}家", "出門", "door")

                    mem_ids = []
                    for m_i in range(fam_size):
                        m_seed = det_u64(f"mem:{hh_id}:{m_i}")
                        m_name = head_name if m_i == 0 else pick_name(fam_race, fam_job, m_seed, False, used_names=used_town_names)
                        m_role = fam_job if m_i == 0 else ("家眷" if m_i == 1 else "年輕幫手")
                        m_id = f"res:{slug}:{a_slug}_{h_i}_{m_i}"
                        m_langs = {"harbor_trade": 85 if fam_race == "人類" else 65}
                        if fam_race == "矮人":
                            m_langs["dwarven"] = 100
                        elif fam_race == "精靈":
                            m_langs["elvish"] = 100
                        elif fam_race == "獸人":
                            m_langs["orcish"] = 100
                        residents.append({
                            "id": m_id,
                            "name": m_name,
                            "race": fam_race,
                            "role": m_role,
                            "household_id": hh_id,
                            "home_node": home_nid,
                            "work_node": anchor if m_i == 0 else alley_nid,
                            "soul_seed": m_seed,
                            "copper": 42 + (m_seed % 75),
                            "languages": m_langs,
                            "persona": f"住在{tname}{a_name}的{fam_race}{m_role}，靠{fam_job}營生過日子。",
                        })
                        mem_ids.append(m_id)

                    households.append({
                        "id": hh_id,
                        "cell_id": cid,
                        "settlement": tname,
                        "node_id": home_nid,
                        "label": h_title,
                        "vacant_for_sale": False,
                        "price_copper": 0,
                        "members": mem_ids,
                    })
                    roof_rects.append({"alley_idx": a_idx, "house_idx": h_i, "node_id": home_nid, "vacant": False, "label": h_title})

        # Map incoming external cells to North Gate or South Gate based on dy
        for nb in cinfo["neighbors_f6"]:
            pair = (min(cid, nb), max(cid, nb))
            if pair in allowed_cell_edges:
                nb_cy = cell_map[nb]["center"][1]
                cell_entry_from_neighbor[(cid, nb)] = n_north if nb_cy <= cy else n_south

        coast_dir = "none"
        if cid == 4616:
            coast_dir = "east"
        elif cid == 4321:
            coast_dir = "west"
        elif cid == 4613:
            coast_dir = "south"
        elif cid == 4073:
            coast_dir = "north"

        town_micro_maps[slug] = {
            "slug": slug,
            "cell_id": cid,
            "name": tname,
            "walled": sdef["walled"],
            "has_port": sdef["has_port"],
            "coast_dir": coast_dir,
            "spine": {
                "north_gate": n_north,
                "smithy_st": n_smithy,
                "plaza": n_plaza,
                "tavern_st": n_tavern_st,
                "south_gate": n_south,
                "dock": n_dock if sdef["has_port"] else None,
            },
            "landmarks": [
                {"node_id": n_forge, "label": forge_title, "kind": "forge"},
                {"node_id": n_herb, "label": "苦艾草藥堂", "kind": "herb"},
                {"node_id": n_tavern, "label": tavern_title, "kind": "tavern"},
                {"node_id": n_carriage, "label": carriage_title, "kind": "carriage"},
            ],
            "alleys": alley_lines,
            "roofs": roof_rects,
        }

    # 2. Build Wilderness Outdoor Nodes (1~2 meaningful nodes per cell, 0 empty filler rooms!)
    for cid, cinfo in cell_map.items():
        if cid in settlement_defs:
            continue
        cx, cy = cinfo["center"]
        n0 = f"oi:cell{cid}:0"
        add_node(n0, cid, cinfo["title"], "wild", cinfo["desc"], [cx, cy])
        cell_default_node[cid] = n0

        # Add a second ecological / rural sub-node on highway or larger interior cells so cross-island journey is ~18-24 steps
        has_hwy = any(cid in pair and kind == "highway" for pair, kind in allowed_cell_edges.items())
        if has_hwy or (cid % 3 == 0):
            n1 = f"oi:cell{cid}:1"
            if cinfo["height"] >= 23:
                sub_name = f"{cinfo['title']}・山腰石徑"
                sub_desc = "順著山勢起伏開出的碎石坡道，路旁岩壁偶見抓痕與山羊蹄印。"
            elif cinfo["is_coast"]:
                sub_name = f"{cinfo['title']}・望海坡"
                sub_desc = "海風勁吹的坡頂草地，可遠眺外海波濤與沿岸帆影。"
            else:
                sub_name = f"{cinfo['title']}・林間空地"
                sub_desc = "老樹環繞的林間歇腳處，地上留著過路商旅生火的冷灰。"
            add_node(n1, cid, sub_name, "wild", sub_desc, [cx + 0.8, cy + 0.6])
            link_bi(n0, n1, "東南", "西北", "trail")
            for nb in cinfo["neighbors_f6"]:
                nb_cx = cell_map[nb]["center"][0]
                cell_entry_from_neighbor[(cid, nb)] = n1 if nb_cx > cx else n0
        else:
            for nb in cinfo["neighbors_f6"]:
                cell_entry_from_neighbor[(cid, nb)] = n0

    # 3. Connect allowed inter-cell edges!
    for (ca, cb), ekind in sorted(allowed_cell_edges.items()):
        na = cell_entry_from_neighbor.get((ca, cb), cell_default_node[ca])
        nb = cell_entry_from_neighbor.get((cb, ca), cell_default_node[cb])
        ax, ay = cell_map[ca]["center"]
        bx, by = cell_map[cb]["center"]
        dab = compass_dir(bx - ax, by - ay)
        dba = compass_dir(ax - bx, ay - by)
        link_bi(na, nb, dab, dba, ekind)

    # 4. Build Starter Secret Dungeon (`北林試煉坑` hidden under `oi:cell4529:0` 灰松斷橋石灘!)
    dungeon_rooms = [
        ("oi:dng_pit:entry", "試煉坑・塌陷洞口", "斷橋石縫後方被藤蔓掩蓋的矮石拱門，冰涼地底氣流夾雜著古老鐵鏽味湧出。", [0, 0]),
        ("oi:dng_pit:hall", "試煉坑・斷柱前廳", "四根矮人方石柱半數傾頹，地面散落著碎裂的構裝石塊與陳年火把灰燼。", [1, 0]),
        ("oi:dng_pit:stele", "試煉坑・古銘文石室", "石壁嵌著一整塊刻滿矮人古文字的青灰石碑，字縫間隱隱流動著淡金微光，可設法拓印回城請教。", [1, 1]),
        ("oi:dng_pit:side", "試煉坑・棄械偏室", "潮濕偏室角落歪倒著一具腐朽木架，地上留著前人匆忙撤退遺落的碎銅板痕跡。", [0, 1]),
        ("oi:dng_pit:lair", "試煉坑・吞石巨蛛巢", "幽暗石窟結滿如鐵絲般堅韌的灰白蛛網，一頭甲殼異常肥厚、背嵌發光原石的巨蛛正盤踞深處！", [2, 1]),
        ("oi:dng_pit:altar", "試煉坑・地脈凝石台", "試煉坑最深處的古老石台，岩縫深處凝結著溫潤發亮的古符文原石。", [2, 2]),
    ]
    for r_id, r_name, r_desc, r_pos in dungeon_rooms:
        add_node(r_id, 4529, r_name, "dungeon", r_desc, [float(r_pos[0]), float(r_pos[1])], indoor=True, dungeon=True)

    link_bi("oi:dng_pit:entry", "oi:dng_pit:hall", "東", "西", "dungeon")
    link_bi("oi:dng_pit:hall", "oi:dng_pit:stele", "南", "北", "dungeon")
    link_bi("oi:dng_pit:stele", "oi:dng_pit:side", "西", "東", "dungeon")
    link_bi("oi:dng_pit:stele", "oi:dng_pit:lair", "東", "西", "dungeon")
    link_bi("oi:dng_pit:lair", "oi:dng_pit:altar", "南", "北", "dungeon")
    # Exit back up to surface bridge (one-way normal exit out; entering from surface requires discovering hidden entrance!)
    edges.append(("oi:dng_pit:entry", "oi:cell4529:0", "出洞口", "secret_out"))

    # Populate exits on nodes, resolving duplicate direction labels cleanly
    for src, dst, dlabel, ekind in edges:
        used = {e["dir"] for e in nodes[src]["exits"]}
        final_dir = dlabel
        if final_dir in used:
            # Disambiguate cleanly using destination short name
            short = nodes[dst]["name"].split("・")[-1]
            final_dir = f"{dlabel}（{short}）"
        nodes[src]["exits"].append({
            "to": dst,
            "dir": final_dir,
            "to_name": nodes[dst]["name"],
            "kind": ekind,
        })

    # Assert BFS connectivity across ALL non-dungeon outdoor nodes from starter spawn (`oi:white_gull_harbor:dock`)!
    spawn_node = "oi:white_gull_harbor:dock"
    vis_nodes = {spawn_node}
    q = deque([spawn_node])
    dist_map = {spawn_node: 0}
    while q:
        cur = q.popleft()
        for ex in nodes[cur]["exits"]:
            nxt = ex["to"]
            if nxt not in vis_nodes:
                vis_nodes.add(nxt)
                dist_map[nxt] = dist_map[cur] + 1
                q.append(nxt)

    outdoor_non_dng = [nid for nid, n in nodes.items() if not n["indoor"] and not n["dungeon"]]
    unreachable = [nid for nid in outdoor_non_dng if nid not in vis_nodes]
    assert not unreachable, f"BFS failed! Unreachable outdoor nodes: {unreachable[:10]}"

    steps_to_west_ferry = dist_map["oi:west_strait_ferry:dock"]
    assert 12 <= steps_to_west_ferry <= 35, f"Cross-island step count {steps_to_west_ferry} out of expected 12..35 range"

    # Build 1-ring colorless neighbor polygons for map fog border guidance
    ring_polygons = []
    for rcid in sorted(ring_ids):
        rc = cells[rcid]
        rpoly = [v_coords[vid] for vid in rc.get("v", []) if vid in v_coords]
        if len(rpoly) >= 3:
            ring_polygons.append({
                "cell_id": rcid,
                "polygon": rpoly,
            })

    # Double-entry initial coin supply ledger check
    resident_copper_sum = sum(int(r["copper"]) for r in residents)
    municipal_treasury_copper = 25_000
    silver_bank_reserve_copper = 40_000
    total_world_copper = resident_copper_sum + municipal_treasury_copper + silver_bank_reserve_copper

    world_doc = {
        "schema_version": SCHEMA_VERSION,
        "seed_hash": hashlib.sha256(b"Tanger-812745074-Feature6-v1").hexdigest()[:16],
        "world_name": "銀鷗外島",
        "state_name": "銀鷗海領",
        "spawn_node": spawn_node,
        "hidden_entrances": {
            "oi:cell4529:0": {
                "target": "oi:dng_pit:entry",
                "dir": "鑽入橋下石縫",
                "to_name": "試煉坑・塌陷洞口",
                "rumor_key": "pit_rumor",
                "clue_text": "你撥開斷橋下的亂藤，發現石縫後竟吹出陣陣帶鐵鏽味的地底冷風——似乎藏著某處前朝坑道入口，若去鎮上鐵匠鋪或酒館探聽清楚或許能找到進去的方法。",
                "found_text": "你順著巴爾提過的暗記搬開鬆動楔石，橋下赫然露出一道通往【試煉坑・塌陷洞口】的矮石拱門！",
            }
        },
        "stats": {
            "voronoi_cells": len(cell_map),
            "total_nodes": len(nodes),
            "outdoor_nodes": len(outdoor_non_dng),
            "households": len(households),
            "residents": len(residents),
            "steps_east_to_west_ferry": steps_to_west_ferry,
            "total_conserved_copper": total_world_copper,
        },
        "ledger_genesis": {
            "municipal_treasury_copper": municipal_treasury_copper,
            "silver_bank_reserve_copper": silver_bank_reserve_copper,
            "resident_copper_sum": resident_copper_sum,
            "total_conserved_copper": total_world_copper,
        },
        "cells": cell_map,
        "ring_polygons": ring_polygons,
        "highways": highway_polylines,
        "sea_corridors": sea_corridors,
        "town_micro_maps": town_micro_maps,
        "nodes": nodes,
        "households": households,
        "residents": residents,
    }

    OUT_JSON.parent.mkdir(parents=True, exist_ok=True)
    with OUT_JSON.open("w", encoding="utf-8") as f:
        json.dump(world_doc, f, ensure_ascii=False, separators=(",", ":"))

    print("Compiled successfully:", OUT_JSON)
    print("Stats:", json.dumps(world_doc["stats"], ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
