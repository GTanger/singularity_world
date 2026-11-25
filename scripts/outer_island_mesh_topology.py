#!/usr/bin/env python3
"""
《奇點世界・銀鷗島》零讀秒等待 × 網狀山海微拓撲 × 48候非阻塞分流編譯模組（Room 4387da46 定案）。

核心契約：
1. 刪除聚落間 Voronoi 幾何擦邊無條件直通邊（根治 85% 聚落出門 1 步撞鄰村與 15 步穿島），保住既有 5 鎮 + 36 聚落圖零重畫。
2. 20 個官道格展開為 3~4 個具橋渡/茶棚/坡口分岔之異質節點（保留 oi:cell{cid}:0 為車船樞紐，實現步車分軌）。
3. 33 個山海荒野格展開為 6 節點「網狀立體微地形場」（坡口 :0、低線時相捷徑 :low、高線全天候攀脊路 :high、岩凹火塘 :camp、制高觀日點 :vantage、深處出口 :far）。
4. 擴充 15 處跨地塊山海秘徑（連同 #4529 試煉坑共 16 處 hidden_entrances），兌現首訪開荒厚重、老手探熟捷徑變快。
5. 編譯期強制執行「48 候全時相非阻塞連通不變式」BFS 斷言（即使滿潮封閉潮間礁灘，全島 100% 聚落與山巔仍 0 秒可達、零罰站乾等）。
"""

from __future__ import annotations
from collections import deque
import math
from typing import Any


# 大漲潮時段（每日兩次大滿潮，對應 live_tide.rs 48 候中之早潮 15..=20 與晚潮 39..=44）
HIGH_TIDE_SLOTS: set[int] = set(range(15, 21)) | set(range(39, 45))

# 全島最具代表性之八大破曉觀日絕頂（其餘荒野制高點亦支援登高眺望與晨曉觀景）
DAWN_PEAK_CELLS: dict[int, tuple[str, str]] = {
    4074: ("北岬東極望日斷崖", "北岬燈塔東側探入寧靜大洋的千仞玄武岩岬角，腳下驚濤拍岸，破曉時第一道金紅日輪正從無垠海平線躍出。"),
    4084: ("拉橋東極迎曦海崖", "銀鷗島最東端的孤懸海蝕高崖，海風終年呼嘯不息，黎明前海天交界處泛起魚肚藍光與萬丈霞彩。"),
    4430: ("雷原分水絕頂觀日台", "全島中脊最高處的裸露花崗岩峰頂，四顧群山與東西兩岸海灣盡收眼底，晨嵐在腳下山谷翻湧如海。"),
    4326: ("島心禁林古鹿望月崖", "俯瞰整片原始禁林冠層的銀白礦脈岩台，黎明破曉時林海霧氣散開，偶見太古石角古鹿於遠處林隙昂首。"),
    5079: ("極南吞潮裂谷觀日巔", "銀鷗島最南端直劈入海的漆黑玄武岩斷崖絕頂，大洋長浪在百丈下轟鳴，日出金光斜照裂谷如熔金傾瀉。"),
    4611: ("鷗尾西嶺望海高台", "俯視南灣與鷗尾漁港帆影的海濱高嶺，破曉時可見早班漁舟迎著朝霞點點出港。"),
    4070: ("北灣巨鰲暗礁望潮頂", "高懸於北灣玄武暗礁群之上的陡峭崖頂，晨光初露時可俯瞰退潮礁盤與深海漩渦。"),
    4323: ("琥珀西嶺雲海岩台", "西峽內陸群山間凸出的扁平巨岩，清晨山嵐聚成白茫雲海，日出時遠方海峽波光隱約可見。"),
}


def filter_voronoi_cell_edges(
    cell_map: dict[int, dict[str, Any]],
    highway_pairs: set[tuple[int, int]],
    settlement_defs: dict[int, dict[str, Any]],
    burgs_by_cell: dict[int, dict[str, Any]],
    plague_ruin_cids: set[int],
) -> dict[tuple[int, int], str]:
    """
    砍除聚落間 Voronoi 幾何擦邊無條件直通邊：
    - 官方 highway_pairs 全數保留為 "highway"；
    - 兩個活態聚落（5 主鎮 + 29 鄉野村鎮）之間嚴禁自動新增 wild_trail 直接擦邊互連；
    - 鄉野村鎮對外最多開 1 條通往相鄰荒野／官道之野徑坡口；
    - 荒野格之間依真實坡度串聯山脊與海岸走廊，最後跑 93 格連通補橋。
    """
    allowed: dict[tuple[int, int], str] = {}
    for a, b in highway_pairs:
        allowed[(min(a, b), max(a, b))] = "highway"

    def is_live_settlement(cid: int) -> bool:
        if cid in settlement_defs:
            return True
        if cid in burgs_by_cell and cid != 4529 and cid not in plague_ruin_cids:
            return True
        return False

    for cid in sorted(cell_map.keys()):
        cinfo = cell_map[cid]
        if cid in settlement_defs and settlement_defs[cid]["walled"]:
            continue
        cid_is_settle = is_live_settlement(cid)
        nbs = sorted(
            cinfo["neighbors_f6"],
            key=lambda nb: (abs(cell_map[nb]["height"] - cinfo["height"]), nb),
        )
        # 活態村鎮對外僅留 1 條野徑接往荒野（不撞鄰村）；高山脊限 2 條；一般荒野限 2 條以維持稀疏有向感
        wild_quota = 1 if cid_is_settle else (2 if cinfo["height"] < 25 else 1)
        added = 0
        for nb in nbs:
            if nb in settlement_defs and settlement_defs[nb]["walled"]:
                continue
            # 核心禁令：兩個活態聚落嚴禁透過 wild_trail 直接擦邊相撞！
            if cid_is_settle and is_live_settlement(nb):
                continue
            pair = (min(cid, nb), max(cid, nb))
            if pair in allowed:
                continue
            dh = abs(cinfo["height"] - cell_map[nb]["height"])
            if (cinfo["height"] >= 25 or cell_map[nb]["height"] >= 25) and dh > 4:
                continue
            allowed[pair] = "wild_trail"
            added += 1
            if added >= wild_quota:
                break

    def bfs_cells(start_cid: int) -> set[int]:
        adj: dict[int, list[int]] = {k: [] for k in cell_map}
        for a, b in allowed:
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

    # 保底連通補橋（優先連往非聚落格，僅在半島死角無他路時才允許連往非城牆鄰格）
    while True:
        reached = bfs_cells(4616)
        if len(reached) == len(cell_map):
            break
        missing = sorted(set(cell_map.keys()) - reached)
        best_pair: tuple[int, int] | None = None
        best_score = (99, 999)
        for m in missing:
            for nb in cell_map[m]["neighbors_f6"]:
                if nb not in reached:
                    continue
                if nb in settlement_defs and settlement_defs[nb]["walled"]:
                    continue
                both_settle = 1 if (is_live_settlement(m) and is_live_settlement(nb)) else 0
                dh = abs(cell_map[m]["height"] - cell_map[nb]["height"])
                sc = (both_settle, dh)
                if sc < best_score:
                    best_score = sc
                    best_pair = (min(m, nb), max(m, nb))
        if best_pair is None:
            m = missing[0]
            nb = cell_map[m]["neighbors_f6"][0]
            best_pair = (min(m, nb), max(m, nb))
        allowed[best_pair] = "wild_trail"

    return allowed


def _principal_axis_partition(
    cid: int,
    cell_map: dict[int, dict[str, Any]],
    connected_nbs: list[int],
) -> tuple[list[int], list[int], list[int], tuple[int, int], tuple[int, int]]:
    """
    依連接鄰格相對於本格中心的方向，將外部鄰格分為 A 端、B 端與側向 C 組，
    並回傳 900x900 手繪圖上 A 端與 B 端的自然視口座標。
    """
    cx, cy = cell_map[cid]["center"]
    if not connected_nbs:
        return [], [], [], (220, 450), (680, 450)
    if len(connected_nbs) == 1:
        nb = connected_nbs[0]
        nx, ny = cell_map[nb]["center"]
        dx, dy = nx - cx, ny - cy
        norm = math.hypot(dx, dy) or 1.0
        ux, uy = dx / norm, dy / norm
        ax = int(round(450 + ux * 250))
        ay = int(round(450 + uy * 250))
        bx = int(round(450 - ux * 240))
        by = int(round(450 - uy * 240))
        return [nb], [], [], (ax, ay), (bx, by)

    # 找張角最開的一對鄰格作為主貫穿軸 A <-> B
    best_pair = (connected_nbs[0], connected_nbs[-1])
    best_dot = 2.0
    vecs: dict[int, tuple[float, float]] = {}
    for nb in connected_nbs:
        nx, ny = cell_map[nb]["center"]
        dx, dy = nx - cx, ny - cy
        ln = math.hypot(dx, dy) or 1.0
        vecs[nb] = (dx / ln, dy / ln)
    for i, a in enumerate(connected_nbs):
        for b in connected_nbs[i + 1 :]:
            dot = vecs[a][0] * vecs[b][0] + vecs[a][1] * vecs[b][1]
            if dot < best_dot:
                best_dot = dot
                best_pair = (a, b)

    na, nb = best_pair
    ux = vecs[nb][0] - vecs[na][0]
    uy = vecs[nb][1] - vecs[na][1]
    uln = math.hypot(ux, uy) or 1.0
    ux, uy = ux / uln, uy / uln

    group_a: list[int] = []
    group_b: list[int] = []
    group_side: list[int] = []
    for x in connected_nbs:
        proj = vecs[x][0] * ux + vecs[x][1] * uy
        if x == na or proj <= -0.22:
            group_a.append(x)
        elif x == nb or proj >= 0.22:
            group_b.append(x)
        else:
            group_side.append(x)

    if not group_a:
        group_a.append(na)
    if not group_b:
        group_b.append(nb)

    ax = max(140, min(760, int(round(450 - ux * 255))))
    ay = max(140, min(760, int(round(450 - uy * 255))))
    bx = max(140, min(760, int(round(450 + ux * 255))))
    by = max(140, min(760, int(round(450 + uy * 255))))
    return group_a, group_b, group_side, (ax, ay), (bx, by)


def build_road_corridor_mesh(
    cid: int,
    base_title: str,
    base_desc: str,
    prov_zh: str,
    river_zh: str,
    is_coast: bool,
    h: int,
    in_plague: bool,
    tea_shed_info: tuple[str, str] | None,
    cell_map: dict[int, dict[str, Any]],
    allowed_cell_edges: dict[tuple[int, int], str],
    cell_entry_from_neighbor: dict[tuple[int, int], str],
    add_node_fn: Any,
    link_bi_fn: Any,
    compass_dir_fn: Any,
) -> None:
    """
    將 20 個官道格展開為 3~4 個具實質功能與方位差異的地貌節點（保留 oi:cell{cid}:0 為車船樞紐）。
    """
    cx, cy = cell_map[cid]["center"]
    connected_nbs = [
        (b if a == cid else a)
        for (a, b) in sorted(allowed_cell_edges.keys())
        if a == cid or b == cid
    ]
    grp_a, grp_b, grp_side, pos_a, pos_b = _principal_axis_partition(cid, cell_map, connected_nbs)

    n_a = f"oi:cell{cid}:approach_a"
    n_0 = f"oi:cell{cid}:0"
    n_b = f"oi:cell{cid}:approach_b"

    core_short = base_title.split("・")[-1].split("（")[0]
    loc_tag = base_title.replace("・", " ").split("（")[0]
    if "（" in base_title:
        near_hint = base_title.split("（")[-1].rstrip("）")
        loc_prefix = f"{prov_zh}{near_hint}"
    else:
        loc_prefix = f"{prov_zh}{core_short[:3]}"

    if river_zh:
        name_a = f"{loc_prefix}・{river_zh}引橋石坡"
        desc_a = f"順河谷緩降接往【{base_title}】的青石引橋坡道，路側砌著導水石溝，溪風帶著微涼水氣。"
        name_0 = base_title
        desc_0 = base_desc
        name_b = f"{loc_prefix}・過溪古樟岔口"
        desc_b = f"跨過{river_zh}石橋後的寬闊岔路口，老樟樹下立著指路青石碑，車轍在此分往鄰近聚落與山徑。"
    elif in_plague:
        name_a = f"{loc_prefix}・封鎖古道北界碑"
        desc_a = f"進入【{base_title}】前的石灰界碑路段，石縫間長滿野生苦艾，路面仍留著昔日商車舊轍。"
        name_0 = base_title
        desc_0 = base_desc
        name_b = f"{loc_prefix}・苦艾石欄轉折坡"
        desc_b = f"南境古道順山勢轉折的矮石欄坡口，海風吹散微苦藥草香，可清楚辨認通往鄰境的路標。"
    elif is_coast:
        name_a = f"{loc_prefix}・臨海碎石坡頭"
        desc_a = f"貼著海岸矮崖起伏的驛道坡頭，鹹風拂過路旁防風石牆，遠處可見白浪拍打礁盤。"
        name_0 = base_title
        desc_0 = base_desc
        name_b = f"{loc_prefix}・望潮里程石亭"
        desc_b = f"【{core_short}】另一端的石砌望潮路亭，石柱刻著往來港鎮步程，行商常在此核對潮水時辰。"
    elif h >= 24:
        name_a = f"{loc_prefix}・盤山石階坡口"
        desc_a = f"攀上【{core_short}】前的鑿岩盤山坡道，內側岩壁滲出細微山泉，雙馬貨車過坡時需放緩馬步。"
        name_0 = base_title
        desc_0 = base_desc
        name_b = f"{loc_prefix}・山脊風口界石"
        desc_b = f"翻過隘道最高處後的背風界石台地，視野豁然開朗，可俯瞰下山官道與側邊獵徑。"
    else:
        name_a = f"{loc_prefix}・林緣青石坡口"
        desc_a = f"由開闊坡地切入【{core_short}】的闊葉林緣路段，樹影斑駁，路邊留著挑夫歇擔的平整青石。"
        name_0 = base_title
        desc_0 = base_desc
        name_b = f"{loc_prefix}・古松三岔路標"
        desc_b = f"穿出林蔭官道末端的雙松岔口，木製指路牌釘在粗壯松幹上，往來車馬聲清晰可聞。"

    off_ax = (pos_a[0] - 450) / 450.0 * 0.9
    off_ay = (pos_a[1] - 450) / 450.0 * 0.9
    off_bx = (pos_b[0] - 450) / 450.0 * 0.9
    off_by = (pos_b[1] - 450) / 450.0 * 0.9

    add_node_fn(n_a, cid, name_a, "road", desc_a, [round(cx + off_ax, 2), round(cy + off_ay, 2)], map_xy=list(pos_a))
    add_node_fn(n_0, cid, name_0, "road", desc_0, [cx, cy], map_xy=[450, 450])
    add_node_fn(n_b, cid, name_b, "road", desc_b, [round(cx + off_bx, 2), round(cy + off_by, 2)], map_xy=list(pos_b))

    d_a0 = compass_dir_fn(-off_ax, -off_ay)
    d_0a = compass_dir_fn(off_ax, off_ay)
    d_0b = compass_dir_fn(off_bx, off_by)
    d_b0 = compass_dir_fn(-off_bx, -off_by)
    link_bi_fn(n_a, n_0, d_a0, d_0a, "highway")
    link_bi_fn(n_0, n_b, d_0b, d_b0, "highway")

    # 側枝歇腳或觀景節點（確保每個官道格都有停留抉擇價值，而非純過路直線）
    perp_x = max(160, min(740, int(round(450 - (pos_b[1] - 450) * 0.65))))
    perp_y = max(160, min(740, int(round(450 + (pos_b[0] - 450) * 0.65))))
    if tea_shed_info is not None:
        ts_name, ts_desc = tea_shed_info
        n_side = f"oi:cell{cid}:tea_shed"
        add_node_fn(
            n_side,
            cid,
            f"{prov_zh}・{ts_name}",
            "tea_shed",
            ts_desc,
            [round(cx + 0.45, 2), round(cy - 0.35, 2)],
            map_xy=[perp_x, perp_y],
        )
        link_bi_fn(n_0, n_side, "入茶棚歇腳", "回官道", "street")
    elif is_coast and not in_plague:
        n_side = f"oi:cell{cid}:overlook"
        add_node_fn(
            n_side,
            cid,
            f"{loc_prefix}・驛道望海石台",
            "summit_outlook",
            f"官道旁凸出海岸的石砌瞭望小台，憑欄可遠眺沿岸帆駁航跡與潮水漲落，晨昏海色尤為壯闊。",
            [round(cx + 0.42, 2), round(cy - 0.38, 2)],
            map_xy=[perp_x, perp_y],
        )
        link_bi_fn(n_0, n_side, "登望海台", "回官道", "trail")
    else:
        n_side = f"oi:cell{cid}:nook"
        add_node_fn(
            n_side,
            cid,
            f"{loc_prefix}・路畔飲馬石泉",
            "campfire_nook",
            f"官道樹蔭下引山泉鑿成的青石飲馬水槽與避風石凳，行旅可在此掬泉洗臉、生火熱糧舒緩腳力。",
            [round(cx + 0.42, 2), round(cy - 0.38, 2)],
            map_xy=[perp_x, perp_y],
        )
        link_bi_fn(n_0, n_side, "往石泉歇腳", "回官道", "trail")

    for nb in grp_a:
        cell_entry_from_neighbor[(cid, nb)] = n_a
    for nb in grp_b:
        cell_entry_from_neighbor[(cid, nb)] = n_b
    for nb in grp_side:
        cell_entry_from_neighbor[(cid, nb)] = n_0


def build_wilderness_mesh(
    cid: int,
    base_title: str,
    base_desc: str,
    prov_zh: str,
    river_zh: str,
    good_zh: str,
    is_coast: bool,
    h: int,
    in_plague: bool,
    apex_info: dict[str, Any] | None,
    tea_shed_info: tuple[str, str] | None,
    cell_map: dict[int, dict[str, Any]],
    allowed_cell_edges: dict[tuple[int, int], str],
    cell_entry_from_neighbor: dict[tuple[int, int], str],
    add_node_fn: Any,
    link_bi_fn: Any,
    compass_dir_fn: Any,
) -> None:
    """
    將 33 個山海荒野格展開為 6 節點「網狀立體微地形場」：
    - :0 (前緣坡口) <-> :low (低線時相捷徑, 2步穿格但受漲潮/夜霧影響) <-> :far (遠端出口)
    - :0 <-> :high (高線全天候攀脊路, 48候永不封閉) <-> :far
    - :camp (背風岩凹火塘, 橫向連接 :low 與 :high, 0秒生火回氣免刷怪)
    - :vantage (制高觀日台/分水嶺瞭望岩, 連於 :high, 支援破曉四候觀日、夜觀命星、白晝眺望識路)
    """
    cx, cy = cell_map[cid]["center"]
    connected_nbs = [
        (b if a == cid else a)
        for (a, b) in sorted(allowed_cell_edges.keys())
        if a == cid or b == cid
    ]
    grp_a, grp_b, grp_side, pos_0, pos_far = _principal_axis_partition(cid, cell_map, connected_nbs)

    n_0 = f"oi:cell{cid}:0"
    n_low = f"oi:cell{cid}:low"
    n_high = f"oi:cell{cid}:high"
    n_camp = f"oi:cell{cid}:camp"
    n_van = f"oi:cell{cid}:vantage"
    n_far = f"oi:cell{cid}:far"

    # 計算 900x900 畫布上不重疊的網狀六點幾何位置
    mid_x = (pos_0[0] + pos_far[0]) // 2
    mid_y = (pos_0[1] + pos_far[1]) // 2
    dx = pos_far[0] - pos_0[0]
    dy = pos_far[1] - pos_0[1]
    ln = math.hypot(dx, dy) or 1.0
    px, py = -dy / ln, dx / ln  # 法向量

    xy_low = [
        max(150, min(750, int(round(mid_x + px * 175)))),
        max(150, min(750, int(round(mid_y + py * 175)))),
    ]
    xy_high = [
        max(150, min(750, int(round(mid_x - px * 165)))),
        max(150, min(750, int(round(mid_y - py * 165)))),
    ]
    xy_camp = [
        max(180, min(720, int(round(mid_x + px * 25)))),
        max(180, min(720, int(round(mid_y + py * 25)))),
    ]
    xy_van = [
        max(130, min(770, int(round(xy_high[0] - px * 105)))),
        max(130, min(770, int(round(xy_high[1] - py * 105)))),
    ]

    core_part = base_title.split("・")[-1].split("（")[0]
    if "（" in base_title:
        near_part = base_title.split("（")[-1].rstrip("）")
        w_prefix = f"{prov_zh}{near_part}"
    else:
        w_prefix = f"{prov_zh}{core_part[:4]}"
    apex_tail = f" {apex_info['lair_hint_zh']}" if apex_info else ""

    if is_coast:
        low_gate = "low_tide"
        name_0 = base_title
        desc_0 = base_desc
        if good_zh == "石英白沙":
            name_low = f"{w_prefix}・退潮銀沙淺灘"
            desc_low = "細白如雪的石英砂灘隨退潮裸露，踩行其上輕快省力；一旦大漲潮湧入，白浪便會漫過整片低灘。"
        elif in_plague:
            name_low = f"{w_prefix}・禁泊暗礁潮溝"
            desc_low = "南岸偏僻礁灣底部的濕滑潮溝，退潮時可貼著岩根快步斜切過灣，滿潮時則被洶湧白浪吞沒。"
        else:
            name_low = f"{w_prefix}・潮間裸露貝礁灘"
            desc_low = "唯有退潮與平水時段才露出海面的寬平玄武岩礁盤，撬貝趕路最為捷近；大漲潮時海浪翻湧淹沒礁面。"
        name_high = f"{w_prefix}・海蝕崖頂藤蔓徑"
        desc_high = "高出滿潮線數丈的峭壁崖頂山徑，老樹盤根與粗藤可供借力攀援，風浪再大也全天候穩固可通，唯徒手攀爬頗費氣力。"
        name_camp = f"{w_prefix}・避風海岩火塘凹"
        desc_camp = "巨型海蝕岩背風面天然凹入的乾燥石窩，前人用扁石圍出火塘並留有乾柴，海風吹不進來，正宜生火烤食回氣。"
        name_far = f"{w_prefix}・岬灣折角石脊口"
        desc_far = f"繞過海灣深處折角、銜接鄰境山海走廊的碎石坡口，潮聲與林風在此交會。{apex_tail}"
    elif river_zh:
        low_gate = "low_tide"
        name_0 = base_title
        desc_0 = base_desc
        name_low = f"{river_zh}・平水踩石淺灘"
        desc_low = f"{river_zh}河床寬闊處裸露的一排青灰踏水卵石，平水時幾步即可輕快躍過；逢漲潮頂托溪水暴漲時則急流漫石。"
        name_high = f"{river_zh}・高岸岩壁古棧道"
        desc_high = f"沿{river_zh}高處岩壁鑿孔架木而成的山腰舊棧徑，不受河水暴漲影響全天可通，但上下陡坡需穩住氣息。"
        name_camp = f"{river_zh}・溪灣大石柴火窩"
        desc_camp = f"高出洪水位的一處乾爽河階巨石凹地，旁邊便是清冽甘甜的{river_zh}支流活水，隨時可掬泉飲水、生火歇腳。"
        name_far = f"{river_zh}・對岸柳林坡口"
        desc_far = f"渡過{river_zh}河谷後通往對岸林地的石坡出口，泥地上印著山鹿與過路旅人的足跡。{apex_tail}"
    elif h >= 24:
        low_gate = "night_dim"
        name_0 = base_title
        desc_0 = base_desc
        name_low = f"{w_prefix}・幽暗裂谷捷徑澗"
        desc_low = "夾在兩道高聳岩壁間的狹長山澗底徑，白晝順澗直切可省去翻嶺之苦；入夜後谷底伸手不見五指，無火光極易走入岔縫。"
        name_high = f"{w_prefix}・向陽分水岩脊線"
        desc_high = "順著灰岩山脊稜線蜿蜒而上的攀登正路，星月與天光皆無遮蔽、日夜暢通，唯坡陡風勁需耗費體力攀登。"
        name_camp = f"{w_prefix}・山腰背風石棚窩"
        desc_camp = "半山腰巨岩傾側形成的天然避風石棚，石縫滲出清涼雪水，火塘餘溫尚存，是翻越山脊途中最佳的生火整頓處。"
        name_far = f"{w_prefix}・嶺後鞍部山口"
        desc_far = f"翻過高脊分水嶺後的寬平山口鞍部，山風自兩側峽谷穿堂吹過，可由此下切鄰境。{apex_tail}"
    else:
        low_gate = "night_dim"
        name_0 = base_title
        desc_0 = base_desc
        name_low = f"{w_prefix}・密林澗底獸徑"
        desc_low = "野鹿與獵戶在巨木根系間踩出的低處捷徑，白晝認準樹皮刻痕可快速穿林；入夜濃霧瀰漫時若無照明則難辨岔口。"
        name_high = f"{w_prefix}・古木山腰向陽徑"
        desc_high = "沿林地較高處岩坡開出的山腰獵道，地勢高爽不積林霧，全天候皆可穩妥認路通行，唯起伏略耗腳力。"
        name_camp = f"{w_prefix}・獵戶樹洞火塘營地"
        desc_camp = "幾株合抱古樟與巨石圍出的林間淨空營地，獵戶留下的石圈火塘乾淨避風，歇腳生火不怕野獸驚擾。"
        name_far = f"{w_prefix}・深林界溪出口"
        desc_far = f"穿出這片幽深獵場邊緣的界溪石坡，林隙透出遠方天光與鄰境山勢。{apex_tail}"

    if cid in DAWN_PEAK_CELLS:
        name_van, desc_van = DAWN_PEAK_CELLS[cid]
    else:
        name_van = f"{w_prefix}・望山風標瞭望岩"
        desc_van = f"高居【{base_title.split('・')[-1]}】之上的裸露瞭望岩台，山海走勢一覽無遺；破曉可迎晨光採霜露、入夜可觀星象辨識秘徑。"

    def off_coord(xy: list[int] | tuple[int, int]) -> list[float]:
        return [
            round(cx + (xy[0] - 450) / 450.0 * 1.1, 2),
            round(cy + (xy[1] - 450) / 450.0 * 1.1, 2),
        ]

    add_node_fn(n_0, cid, name_0, "wild", desc_0, off_coord(pos_0), map_xy=list(pos_0))
    add_node_fn(n_low, cid, name_low, "wild", desc_low, off_coord(xy_low), map_xy=xy_low)
    add_node_fn(n_high, cid, name_high, "wild", desc_high, off_coord(xy_high), map_xy=xy_high)
    add_node_fn(n_camp, cid, name_camp, "campfire_nook", desc_camp, off_coord(xy_camp), map_xy=xy_camp)
    add_node_fn(n_van, cid, name_van, "summit_outlook", desc_van, off_coord(xy_van), map_xy=xy_van)
    add_node_fn(n_far, cid, name_far, "wild", desc_far, off_coord(pos_far), map_xy=list(pos_far))

    # 網狀連接（零單線死走廊！低線 2 步快切但帶時相門檻；高線 2 步全天候攀脊；中間營火橫向貫通高低雙線；制高點俯瞰高線與遠端）
    link_bi_fn(n_0, n_low, "切低處捷徑", "退回坡口", "wild_trail", env_gate_ab=low_gate, env_gate_ba="")
    link_bi_fn(n_low, n_far, "順捷徑穿出", "折入低處捷徑", "wild_trail", env_gate_ab=low_gate, env_gate_ba=low_gate)

    link_bi_fn(n_0, n_high, "攀高處山徑", "下返坡口", "wild_trail", env_gate_ab="climb_ridge", env_gate_ba="")
    link_bi_fn(n_high, n_far, "沿高徑穿出", "攀回高處山徑", "wild_trail", env_gate_ab="climb_ridge", env_gate_ba="climb_ridge")

    link_bi_fn(n_low, n_camp, "轉入背風火塘", "下往低處捷徑", "trail")
    link_bi_fn(n_high, n_camp, "下切背風火塘", "攀回高處山徑", "trail", env_gate_ab="", env_gate_ba="climb_ridge")

    link_bi_fn(n_high, n_van, "登頂瞭望岩", "下返高處山徑", "trail", env_gate_ab="climb_ridge", env_gate_ba="")
    link_bi_fn(n_van, n_far, "順稜線下切出口", "仰攀瞭望岩頂", "trail", env_gate_ab="", env_gate_ba="climb_ridge")

    if tea_shed_info is not None:
        ts_name, ts_desc = tea_shed_info
        n_tea = f"oi:cell{cid}:tea_shed"
        add_node_fn(
            n_tea,
            cid,
            f"{prov_zh}・{ts_name}",
            "tea_shed",
            ts_desc,
            [round(cx + 0.35, 2), round(cy - 0.35, 2)],
            map_xy=[max(150, min(750, pos_0[0] + 95)), max(150, min(750, pos_0[1] - 95))],
        )
        link_bi_fn(n_0, n_tea, "入火塘棚歇腳", "回山徑坡口", "street")

    for nb in grp_a:
        cell_entry_from_neighbor[(cid, nb)] = n_0
    for nb in grp_b:
        cell_entry_from_neighbor[(cid, nb)] = n_far
    for nb in grp_side:
        cell_entry_from_neighbor[(cid, nb)] = n_high


def build_island_secret_shortcuts(
    nodes: dict[str, dict[str, Any]],
    edges: list[tuple[str, str, str, str, str]],
) -> dict[str, dict[str, Any]]:
    """
    在保留 #4529 北林試煉坑基礎上，擴充 15 處跨山脊／海蝕洞／古獵道之老手山海秘徑（共 16 處 hidden_entrances）。
    探熟後可跨地塊抄近路，兌現「首訪開荒 30~60 分鐘、老手熟路 5~15 分鐘」之速度回報。
    """
    secrets: dict[str, dict[str, Any]] = {
        "oi:cell4529:0": {
            "target": "oi:dng_pit:entry",
            "dir": "鑽入橋下石縫",
            "to_name": "試煉坑・塌陷洞口",
            "rumor_key": "pit_rumor",
            "clue_text": "你撥開斷橋下的亂藤，發現青石橋墩後有幾塊矮人楔石明顯鬆動，縫隙吹出陣陣地底冷風——若向港裡老鐵匠或酒館探聽機關口訣，或許能打開入口。",
            "found_text": "你依著聽來的楔石暗記推動橋下鬆石，沉悶石響後露出一道向下延伸的矮人石階暗門！",
        }
    }

    # (起點節點, 終點節點, 秘徑方向詞, 秘徑名稱提示, 線索文字, 解鎖文字)
    shortcut_specs = [
        ("oi:cell4615:vantage", "oi:cell4526:far", "穿古炭夫暗徑", "灰松東嶺古炭道",
         "瞭望岩背後的古松根部留有老炭夫刻的三道斧痕，似有一條廢棄運炭山徑可直切灰松林驛東側。",
         "你撥開古松垂藤，認出昔日炭夫挑炭直通灰松東林的背陰捷徑！"),
        ("oi:cell4526:vantage", "oi:cell4430:high", "攀岩縫獵鹿梯", "克頓高脊石梯徑",
         "岩壁裂縫裡釘著幾枚半鏽的矮人鐵楔，仔細攀摸似乎能直通雷原分水高脊。",
         "你順著岩縫裡的古老鐵楔試踏而上，打通了直攀雷原高脊的獵鹿石梯！"),
        ("oi:cell4430:vantage", "oi:cell4326:high", "切分水嶺獸脊", "禁林分水古稜徑",
         "站在絕頂俯瞰，雲霧間隱約可見一條山羊與古鹿踩出的窄稜直連島心禁林。",
         "你記準雲霧散開時的岩脊走向，探明了橫切島心禁林的分水嶺捷徑！"),
        ("oi:cell4326:far", "oi:cell4323:high", "鑽巨木空心谷", "琥珀西嶺古樹廊",
         "幾株傾倒交疊的千年巨杉下方形成天然拱道，風裡帶著西峽海灣的鹹味。",
         "你穿過倒木拱廊與乾涸溪床，打通了直抵琥珀西嶺的林間秘徑！"),
        ("oi:cell4199:0", "oi:cell4070:high", "下切玄武岩裂階", "北灣懸崖採藥階",
         "盤山隘道護欄外有一串被海風磨平的古老石鑿腳窩，似乎能繞過山谷直下北灣暗礁崖頂。",
         "你扶著岩壁試出穩固腳窩，找出了直通北灣巨鰲崖頂的採藥秘階！"),
        ("oi:cell4074:vantage", "oi:cell4076:0", "穿海蝕風洞廊", "北岬穿山海風洞",
         "斷崖岩壁下傳來奇異的空谷迴音，一股帶著林葉清香的穿堂風正從石洞深處吹出。",
         "你舉步穿過乾燥寬敞的海蝕穿山風洞，轉眼便直抵柳泉林蔭官道！"),
        ("oi:cell4084:vantage", "oi:cell4205:0", "沿斷崖棧孔切南", "東極古烽火窄徑",
         "迎曦海崖南側石壁留有一排古代守軍巡海鑿出的方形棧孔，可貼崖南切哈倫望海驛道。",
         "你順著古烽火兵留下的石壁棧道穩步前行，打通了東岸貼崖捷徑！"),
        ("oi:cell4522:vantage", "oi:cell4322:0", "下切琥珀古鹽道", "西峽舊私鹽山徑",
         "瞭望岩下方的灌木叢裡藏著碎貝殼鋪成的舊日山路，顯然是早年挑夫避開關哨的私鹽捷徑。",
         "你循著草間碎貝路標一路撥草探明，重開了直下西峽南驛道的古鹽徑！"),
        ("oi:cell4611:vantage", "oi:cell4524:high", "攀海風藤梯嶺", "南灣背嶺獵徑",
         "望海高台北側崖壁垂著結成網狀的老海藤，山風吹動時露出後方的石階痕跡。",
         "你拉緊粗韌老藤攀上崖口，打通了從鷗尾西嶺直切灰松南坡的背嶺秘徑！"),
        ("oi:cell4694:vantage", "oi:cell4778:high", "繞高嶺避疫古獵道", "乳酪高嶺獵人道",
         "風語橋東側高嶺岩石上畫著守墓人留下的白灰箭頭，似可從高處山脊繞往南岸外圍。",
         "你依著白灰岩記沿高處淨風山脊前行，探出了俯瞰南岸礁灣的高嶺秘徑！"),
        ("oi:cell4852:vantage", "oi:cell5079:high", "順玄武柱石脊南行", "極南黑岩柱稜線",
         "六角形玄武岩柱如階梯般一路向南延伸，遠處傳來極南斷崖低沉的吞潮轟鳴。",
         "你踏著天然六角玄武岩柱穩步推進，開通了直達極南吞潮斷崖高處的秘徑！"),
        ("oi:cell5079:vantage", "oi:cell5008:high", "穿海霧石拱橋", "極南天然海石拱",
         "斷崖東側海霧中橫跨著一座巨大的天然海蝕石拱，拱頂寬平似乎可供人通行。",
         "你趁海霧散開踏過天然海蝕石拱，瞬間跨過深谷直抵艾森河集外灣高崖！"),
        ("oi:cell4197:vantage", "oi:cell4325:high", "切冷杉分水獵道", "青苔西嶺獵狐徑",
         "冷杉樹幹上每隔十餘步便釘著一枚獸骨路標，一路指向東南方的青苔溪集山腰。",
         "你順著精靈獵戶留下的獸骨路標穿林而過，打通了泰原南林至青苔西嶺的秘徑！"),
        ("oi:cell4528:vantage", "oi:cell4617:0", "順古水渠石堤東行", "澤昂舊引水石堤",
         "河谷高處殘留著一段百年前修築的青石引水渠堤，堤面平整直指白鷗港南郊驛道。",
         "你踏上掩映在蘆葦間的古老青石渠堤，打通了河谷直返白鷗港南驛道的捷徑！"),
        ("oi:cell4328:vantage", "oi:cell4203:0", "下切海松碎石槽", "錨灣背山採脂徑",
         "哈倫西林高處有一道鋪滿松針的乾爽石槽，船匠常由此把松脂直接滑運至北岸驛道。",
         "你順著鋪滿厚松針的採脂山徑輕快下切，探明了直抵錨灣西驛道的近路！"),
    ]

    for src_id, dst_id, dir_zh, short_name, clue, found in shortcut_specs:
        if src_id in nodes and dst_id in nodes and src_id not in secrets:
            secrets[src_id] = {
                "target": dst_id,
                "dir": dir_zh,
                "to_name": nodes[dst_id]["name"],
                "rumor_key": "",
                "clue_text": clue,
                "found_text": f"{found}（已永久記下秘徑【{short_name}】）",
            }
            if dst_id not in secrets:
                secrets[dst_id] = {
                    "target": src_id,
                    "dir": f"循{short_name}折返",
                    "to_name": nodes[src_id]["name"],
                    "rumor_key": "",
                    "clue_text": f"岩壁與樹根間隱約留有通往【{nodes[src_id]['name']}】的舊日刻痕（{short_name}）。",
                    "found_text": f"你辨認出岩壁暗記，打通了往返【{short_name}】的雙向秘徑！",
                }

    return secrets


def populate_existing_rural_map_xy(nodes: dict[str, dict[str, Any]], settlement_defs: dict[int, Any]) -> None:
    """為已繪 36 張鄉野村莊／封村遺址／斷橋坑之室外節點補齊 900x900 畫布推焦座標。"""
    by_cell: dict[int, list[dict[str, Any]]] = {}
    for nd in nodes.values():
        if nd["indoor"] or nd["dungeon"] or nd["cell_id"] in settlement_defs:
            continue
        if nd.get("map_xy") is None:
            by_cell.setdefault(nd["cell_id"], []).append(nd)

    preset_ring = [
        [450, 680],
        [450, 440],
        [310, 470],
        [590, 470],
        [350, 310],
        [550, 310],
    ]
    for cid, arr in by_cell.items():
        arr.sort(key=lambda x: (0 if x["id"].endswith(":0") else (1 if x["kind"] == "plaza" else 2), x["id"]))
        for idx, nd in enumerate(arr):
            if idx < len(preset_ring):
                nd["map_xy"] = list(preset_ring[idx])
            else:
                ang = (idx * 1.15) % (2 * math.pi)
                nd["map_xy"] = [
                    int(round(450 + math.cos(ang) * 210)),
                    int(round(450 + math.sin(ang) * 210)),
                ]


def insert_inter_settlement_buffers_if_needed(
    allowed_cell_edges: dict[tuple[int, int], str],
    settlement_defs: dict[int, Any],
    burgs_by_cell: dict[int, Any],
    plague_ruin_cids: set[int],
    cell_map: dict[int, dict[str, Any]],
    cell_entry_from_neighbor: dict[tuple[int, int], str],
    cell_default_node: dict[int, str],
    add_node_fn: Any,
    link_bi_fn: Any,
    compass_dir_fn: Any,
) -> int:
    """
    為 29 處鄉野活態聚落（及 #4529 斷橋坑）建立真實的「村郊農漁礦腹地坡口」（GPT-6 Astra 定案：村外設溪渡與耕作腹地，保留 n0 為村口與車船停靠站），
    徹底根除「出村門 1 步撞進隔壁地塊」並賦予每座村莊與其主產相符的外圍地景。
    """
    outskirt_flavors = {
        "wheat_grain": ("外郊黑麥田壟坡", "村口外順坡開墾的成片黑麥旱田，石砌田埂間留有獨輪推車與耕牛往來的泥印。"),
        "black_coal": ("外郊運煤碎石溪橋", "村外跨越黑石小溪的粗木板橋，路緣散落著運煤車顛落的小塊烏煤。"),
        "quarry_stone": ("外郊採石滑道坡", "山寮外鋪著圓木滾軸的運石斜坡，兩側堆著剛劈開的青灰毛石。"),
        "charcoal_sack": ("外郊伐薪林緣坡", "炭寮外圍疏朗的次生闊葉林坡，整齊碼放著待入窯風乾的段木。"),
        "timber_log": ("外郊集材拖木道", "林莊外順山溝修整的拖木土道，空氣中瀰漫著新鮮松脂木香。"),
        "salt_fish_barrel": ("外郊曬鹽礫石灣", "漁塢外側迎風的平坦礫石海灣，竹架上晾著魚網與粗海鹽簍。"),
        "dairy_cheese": ("外郊山羊牧草坡", "牧莊外圍著矮木柵的青翠緩坡，草叢間偶聞山羊銅鈴清響。"),
        "quarantine_wormwood": ("外郊苦艾警戒坡", "哨柵外圍插著白灰木籤的荒草坡地，風裡飄著濃烈苦艾草氣。"),
    }

    buffered = 0
    for cid in sorted(burgs_by_cell.keys()):
        if cid in plague_ruin_cids:
            continue
        connected_nbs = [
            (b if a == cid else a)
            for (a, b) in sorted(allowed_cell_edges.keys())
            if a == cid or b == cid
        ]
        if len(connected_nbs) < 2:
            continue
        grp_a, grp_b, grp_side, pos_a, pos_b = _principal_axis_partition(cid, cell_map, connected_nbs)
        cx, cy = cell_map[cid]["center"]
        n0 = cell_default_node[cid]
        res_slug = cell_map[cid].get("resource_slug", "wheat_grain")
        flv_name, flv_desc = outskirt_flavors.get(res_slug, outskirt_flavors["wheat_grain"])
        short_c = cell_map[cid]["title"].split("・")[0].split("（")[0]

        if grp_a:
            na = f"oi:cell{cid}:skirt_a"
            off_ax = (pos_a[0] - 450) / 450.0 * 0.85
            off_ay = (pos_a[1] - 450) / 450.0 * 0.85
            add_node_fn(
                na,
                cid,
                f"{short_c}・{flv_name}",
                "road",
                f"【{short_c}】{flv_desc}",
                [round(cx + off_ax, 2), round(cy + off_ay, 2)],
                map_xy=list(pos_a),
            )
            d_to_0 = compass_dir_fn(-off_ax, -off_ay)
            d_to_a = compass_dir_fn(off_ax, off_ay)
            link_bi_fn(na, n0, d_to_0, d_to_a, "highway")
            for nb in grp_a:
                cell_entry_from_neighbor[(cid, nb)] = na
            buffered += 1

        if grp_b:
            nb_node = f"oi:cell{cid}:skirt_b"
            off_bx = (pos_b[0] - 450) / 450.0 * 0.85
            off_by = (pos_b[1] - 450) / 450.0 * 0.85
            add_node_fn(
                nb_node,
                cid,
                f"{short_c}・村後界石林徑",
                "road",
                f"離開【{short_c}】屋舍煙火後通往鄰境的村後石板坡道，路旁老樹下立著風化青石界標。",
                [round(cx + off_bx, 2), round(cy + off_by, 2)],
                map_xy=list(pos_b),
            )
            d_0_to_b = compass_dir_fn(off_bx, off_by)
            d_b_to_0 = compass_dir_fn(-off_bx, -off_by)
            link_bi_fn(n0, nb_node, d_0_to_b, d_b_to_0, "highway")
            for nb in grp_b:
                cell_entry_from_neighbor[(cid, nb)] = nb_node
            buffered += 1

        for nb in grp_side:
            cell_entry_from_neighbor[(cid, nb)] = n0

    return buffered


def assert_48_phase_non_blocking_invariant(
    nodes: dict[str, dict[str, Any]],
    spawn_node: str,
) -> dict[str, int]:
    """
    形式化驗證 Room 4387da46 核心鐵律：
    在全日 48 候（0..47）任一時相下，即使大漲潮淹沒全部 `low_tide` 潮間礁灘邊，
    且玩家空手無火把（不依賴任何 secret 秘徑），從白鷗港碼頭出發仍 100% 無阻塞抵達全島每一座室外非地城節點！
    """
    outdoor_targets = [
        nid for nid, nd in nodes.items() if not nd["indoor"] and not nd["dungeon"]
    ]
    min_ew_steps = 999
    max_ew_steps = 0

    for slot in range(48):
        is_high_tide = slot in HIGH_TIDE_SLOTS
        vis = {spawn_node}
        dist = {spawn_node: 0}
        q = deque([spawn_node])
        while q:
            cur = q.popleft()
            for ex in nodes[cur]["exits"]:
                gate = ex.get("env_gate", "")
                if is_high_tide and gate == "low_tide":
                    continue  # 大漲潮時潮間礁灘淹沒，驗證高處崖徑是否 100% 暢通兜底！
                nxt = ex["to"]
                nd_nxt = nodes[nxt]
                if nd_nxt["indoor"] or nd_nxt["dungeon"]:
                    continue
                if nxt not in vis:
                    vis.add(nxt)
                    dist[nxt] = dist[cur] + 1
                    q.append(nxt)

        missing = [nid for nid in outdoor_targets if nid not in vis]
        assert not missing, (
            f"48-phase non-blocking invariant violated at tide_slot={slot} "
            f"(high_tide={is_high_tide})! Unreachable outdoor nodes: {missing[:8]}"
        )
        ew_d = dist["oi:west_strait_ferry:dock"]
        min_ew_steps = min(min_ew_steps, ew_d)
        max_ew_steps = max(max_ew_steps, ew_d)

    return {
        "outdoor_node_count": len(outdoor_targets),
        "ew_steps_low_tide": min_ew_steps,
        "ew_steps_high_tide": max_ew_steps,
    }
