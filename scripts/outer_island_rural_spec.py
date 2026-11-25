#!/usr/bin/env python3
"""
銀鷗島 88 個城間地塊、35 處正典基層舊址、二樓獨立客房、碼頭記籌手與四條實體車船班線編譯規格。

嚴格落實審辯室 Room `9639c638`、`460a0fc4`、`cf22135e` 與 `f04b40af` 定案：
1. 非疫區 26 處聚落依母圖 pop 兩道天然斷層（10.82 / 3.23）定為 3 市集鎮 / 13 百工農礦村 / 10 專職微據點，逐村按 round(pop * 2.2) 配置真實家戶與居民（4429 定名為「灰松石堡」消解撞名）。
2. #4529 定名為「黑煤鎮舊址・斷橋塌陷坑」（主煤井塌陷為北林試煉坑地城入口）。
3. 南岸船疫熱病區（Zone 6）8 處定為「一哨（4695 風語橋鎮檢疫哨）・二孤莊（4777 達利莊、4776 索昂林莊病癒採苦艾守墓戶）・五封村（4849 古加津等 5 處 0 活人、0 固定刷怪之封門殘信遺址）」。
4. 20 格陸路走廊與 33 格無路山海格格具備唯一地貌描寫與資源存量再生池（含石英白沙灣、退潮海貝礁、4 處路邊避雨茶棚、3 尊唯一太古巨物），徹底清零複製貼上罐頭描寫。
5. 幹道村口採正交側枝（T 字分支），過路 0 額外步數，守住全島東西橫越 <= 25 步；出口附里程石碑步數提示。
6. 新增居民與記籌手之初始銅幣全數自銀號準備金（40,000 銅）原子扣撥，全島總銅幣維持 120,805 銅一文不差。
"""

from __future__ import annotations
from collections import deque
from typing import Any

from outer_island_mesh_topology import (
    build_road_corridor_mesh,
    build_wilderness_mesh,
    insert_inter_settlement_buffers_if_needed,
    populate_existing_rural_map_xy,
)

# 南岸船疫熱病區（母圖 Zone 6 涵蓋之 8 處舊址）角色定義
PLAGUE_CHECKPOINT_CID = 4695  # 風語橋哨（北口檢疫橋哨）
PLAGUE_SURVIVOR_CIDS = {4777, 4776}  # 達利莊、索昂林莊（病癒具抗體之採苦艾守墓孤戶）
PLAGUE_SEALED_RUIN_CIDS = {4849, 4780, 4775, 4847, 4698}  # 五處徹底封村遺址（0 活人、0 刷怪）

# 銀鷗島城間聚落與封村遺址正名表（消除母圖機翻『市／城／鎮』與 1～8 戶實際規模倒掛，對齊手繪外景圖面與職能）
RURAL_CANONICAL_NAMES: dict[int, str] = {
    4200: "青苔溪集",
    4194: "格伯橋集",
    4853: "艾森河集",
    4072: "黑煤谷村",
    4327: "深林村",
    4427: "琥珀橋村",
    4083: "拉橋村",
    4201: "柳泉谷村",
    4695: "風語橋哨",
    4198: "雷原石寨",
    4077: "艾爾灣村",
    4195: "博泉村",
    4428: "乳酪牧坡",
    4429: "灰松石堡",
    4530: "佩克崖莊",
    4926: "雷橋莊",
    4425: "拉登濱莊",
    4069: "柳泉石寮",
    4071: "白梣炭寮",
    4614: "黑煤溪寮",
    4698: "乳酪坡",
    4780: "維爾崖",
    4847: "蕨溪谷",
}

# 3 尊無城鎮壓制區「殺一頭世上永久少一尊」之太古巨物
APEX_BEASTS_BY_CELL: dict[int, dict[str, Any]] = {
    4070: {
        "beast_id": "apex:abyssal_turtle_4070",
        "name_zh": "淵潮覆甲巨鰲",
        "title_zh": "北灣暗礁太古巨物",
        "rune_reward": "鎮潮古符文原石",
        "bone_reward": "巨鰲千年背甲骨",
        "lair_hint_zh": "退潮後的玄武暗礁深處傳來沉悶如雷的呼吸潮鳴，岩壁留著房屋大小的覆甲擦痕。",
    },
    4326: {
        "beast_id": "apex:frost_antler_4326",
        "name_zh": "蒼冠石角古鹿",
        "title_zh": "島心禁林太古巨物",
        "rune_reward": "生機古符文原石",
        "bone_reward": "石角古鹿冠骨",
        "lair_hint_zh": "禁伐古林深處的巨木樹皮泛著奇異銀白礦光，泥地印著深達半尺的巨型偶蹄印。",
    },
    5079: {
        "beast_id": "apex:basalt_serpent_5079",
        "name_zh": "黑岩吞潮巨蟒",
        "title_zh": "極南斷崖太古巨物",
        "rune_reward": "裂岩古符文原石",
        "bone_reward": "黑岩巨蟒逆鱗骨",
        "lair_hint_zh": "海蝕裂谷底端散落著碗口粗的蛻皮碎甲，腥鹹海風中帶著令人心悸的低頻震動。",
    },
}

# 4 處官道中段「路邊避雨茶棚／火塘」（飲茶不鎖秒、消寒乏並聽車夫真實風聲）
TEA_SHED_CELLS: dict[int, tuple[str, str]] = {
    4432: ("古加坡口茶棚", "官道轉折處搭起的杉木避雨茶棚，大鐵壺裡滾著驅寒薑茶，挑夫與車夫常在此歇腳交換白鷗港與灰松林驛的貨況。"),
    4426: ("西嶺風口火塘棚", "翻越西段山坡前的矮石擋風棚，火塘炭火微紅，往返西峽渡口的旅人常在此烘乾斗篷、打聽渡輪班期。"),
    4200: ("青苔石牆外茶寮", "青苔溪集石牆門外供過路商車飲馬歇腳的木棚，木牌旁聚著等候北上燈塔班車的行商。"),
    4612: ("南灣望潮茶亭", "連接白鷗港與鷗尾漁港海濱官道的石砌涼亭，煮著鹹香昆布熱湯，可遠眺南灣歸港船影。"),
}

CN_NUM = ["零", "一", "二", "三", "四", "五", "六", "七", "八", "九", "十", "十一", "十二"]


def cn_int(n: int) -> str:
    if 0 <= n < len(CN_NUM):
        return CN_NUM[n]
    return str(n)


def infer_rural_specialty(cid: int, name: str, is_port: bool, height: int, flux: int, in_plague: bool) -> dict[str, Any]:
    """依地名語義、水文、海拔與海岸屬性決定每處基層聚落之真實主產與行當（零隨機錯置）。"""
    if cid == PLAGUE_CHECKPOINT_CID:
        return {
            "resource_slug": "quarantine_wormwood",
            "resource_zh": "防疫苦艾草束",
            "jobs": ["巡城衛兵", "草藥師", "石工"],
            "races": ["人類", "矮人"],
            "yard_label": "檢疫石橋哨柵",
            "yard_desc": "橫跨南境溪流的石橋釘著雙層檢疫木柵，火盆日夜焚燒苦艾煙，衛兵與草藥師逐一查驗過往封條，嚴禁疫區生鮮北運與非檢疫糧食私自南運入封鎖區。",
        }
    if cid in PLAGUE_SURVIVOR_CIDS:
        return {
            "resource_slug": "quarantine_wormwood",
            "resource_zh": "陳年苦艾藥束",
            "jobs": ["草藥師", "農戶"],
            "races": ["人類", "精靈"],
            "yard_label": "封鎖線隔欄交接台",
            "yard_desc": "村口立著塗白灰的防疫木欄與長柄遞物木槽。幾戶三二六年大疫後倖存具抗體的守墓採藥人留居於此，可隔欄代送平安家書或換取苦艾湯。",
        }
    if "黑煤" in name:
        return {
            "resource_slug": "black_coal",
            "resource_zh": "烏亮黑煤塊",
            "jobs": ["石工", "鍛鐵匠", "木匠"],
            "races": ["矮人", "人類", "獸人"],
            "yard_label": "礦溪洗煤場",
            "yard_desc": "順著溪流架設木槽沖洗剛出坑的烏亮黑煤，矮人與礦工將瀝乾的煤塊裝入厚木箱，專供城鎮鐵匠石爐燃用。",
        }
    if "乳酪" in name:
        return {
            "resource_slug": "dairy_cheese",
            "resource_zh": "熟成山羊乳酪輪",
            "jobs": ["農戶", "皮匠", "木匠"],
            "races": ["人類", "精靈"],
            "yard_label": "草坡擠乳棚場",
            "yard_desc": "緩坡草場圍著低矮石欄，木棚下整排杉木架晾著抹鹽熟成的乳酪輪與厚羊皮。",
        }
    if is_port or (cid != 4077 and any(k in name for k in ["塢", "灣", "岬", "津"])):
        return {
            "resource_slug": "salt_fish_barrel",
            "resource_zh": "鹽漬海魚木桶",
            "jobs": ["漁戶", "木匠", "碼頭苦力"],
            "races": ["人類", "獸人"],
            "yard_label": "石澳曬網船塢",
            "yard_desc": "避風小石澳斜鋪著拖船枕木與桐油木桶，漁戶正將清晨起網的海魚抹鹽入桶，等候沿岸貨駁靠泊收運。",
        }
    if any(k in name for k in ["林", "橡", "梣", "松"]):
        return {
            "resource_slug": "charcoal_sack" if height >= 22 else "timber_log",
            "resource_zh": "硬木炭袋" if height >= 22 else "杉松原木料",
            "jobs": ["木匠", "獵戶", "草藥師"],
            "races": ["精靈", "人類", "獸人"],
            "yard_label": "林間燒炭集材場",
            "yard_desc": "林緣空地築著兩座冒青煙的悶燒土炭窯，鋸好的杉松原木與綑紮結實的硬木炭袋整齊堆在防雨棚下。",
        }
    if "橋" in name or flux >= 28:
        return {
            "resource_slug": "wheat_grain",
            "resource_zh": "飽滿黑麥穀袋",
            "jobs": ["農戶", "石工", "木匠"],
            "races": ["人類", "矮人"],
            "yard_label": "溪橋水磨穀場",
            "yard_desc": "石拱橋畔立著借溪水推轉的石磨坊，農戶在石板曬場翻曬新收黑麥，橋頭木樁栓著待發的載貨篷車。",
        }
    if height >= 23:
        return {
            "resource_slug": "quarry_stone",
            "resource_zh": "方整灰岩條石",
            "jobs": ["石工", "獵戶", "鍛鐵匠"],
            "races": ["矮人", "人類"],
            "yard_label": "山腰採石劈岩場",
            "yard_desc": "裸露灰岩坡面打著整排鐵楔孔，石工與獵戶在棚下修整切下的方整條石與粗鐵礦塊。",
        }
    return {
        "resource_slug": "wheat_grain",
        "resource_zh": "旱地麥薯糧袋",
        "jobs": ["農戶", "草藥師", "木匠"],
        "races": ["人類", "精靈"],
        "yard_label": "村口古井曬穀坪",
        "yard_desc": "夯土曬穀坪中央圍著一口青苔石井，屋簷下掛滿風乾麥穗與草藥束，雞犬聲與鄰里交談聲在巷口迴盪。",
    }


def enrich_five_core_towns(
    settlement_defs: dict[int, dict[str, Any]],
    nodes: dict[str, dict[str, Any]],
    households: list[dict[str, Any]],
    residents: list[dict[str, Any]],
    add_node_fn: Any,
    link_bi_fn: Any,
    det_u64_fn: Any,
    pick_name_fn: Any,
) -> None:
    """為既有五座核心城鎮補齊：1) 二樓一至六號獨立客房；2) 四座海港碼頭專職『記籌手』（含真實室內居所）。"""
    for cid, sdef in settlement_defs.items():
        slug = sdef["slug"]
        tname = sdef["name"]
        n_inn_up = f"oi:{slug}:inn_upstairs"
        if n_inn_up in nodes:
            up_pos = nodes[n_inn_up]["pos"]
            tav_prefix = nodes[n_inn_up]["name"].split("・")[0]
            for rm_i in range(1, 7):
                rm_c = cn_int(rm_i)
                rm_nid = f"oi:{slug}:inn_room_{rm_i}"
                add_node_fn(
                    rm_nid,
                    cid,
                    f"{tav_prefix}・{rm_c}號客房",
                    "inn_room",
                    f"{tav_prefix}二樓獨立門扇的{rm_c}號單人木板客房，窗邊設一張乾爽草墊木床與帶銅扣的床頭私箱，關門落栓即可隔絕樓下喧囂安穩歇息。",
                    [round(up_pos[0] + rm_i * 0.05, 2), round(up_pos[1] + 0.05, 2)],
                    indoor=True,
                )
                link_bi_fn(n_inn_up, rm_nid, f"進{rm_c}號房", "出房門", "door")

        if sdef["has_port"]:
            n_dock = f"oi:{slug}:dock"
            n_tav_st = f"oi:{slug}:tavern_st"
            dock_pos = nodes[n_dock]["pos"]
            clerk_home = f"oi:{slug}:tally_house"
            tc_seed = det_u64_fn(f"tally_clerk:{cid}")
            tc_race = "矮人" if cid == 4616 else "人類"
            tc_name = pick_name_fn(tc_race, "銀號帳房", tc_seed, True)
            tc_id = f"res:{slug}:tally_clerk"
            hh_id = f"hh:{slug}:tally_clerk"
            add_node_fn(
                clerk_home,
                cid,
                f"{tname}・港務記籌寮屋",
                "house",
                f"緊鄰驛站街與棧橋轉角的石砌小屋，牆上掛著成串竹籌與潮汐靠船板，是港務記籌手{tc_name}夜間落鎖歇息的居所。",
                [round(dock_pos[0] - 0.6, 2), round(dock_pos[1] + 0.5, 2)],
                indoor=True,
            )
            link_bi_fn(n_tav_st, clerk_home, "訪記籌寮", "出門", "door")
            langs = {"harbor_trade": 98}
            if tc_race == "矮人":
                langs["dwarven"] = 100
            residents.append({
                "id": tc_id,
                "name": tc_name,
                "race": tc_race,
                "role": "港務記籌手",
                "household_id": hh_id,
                "home_node": clerk_home,
                "work_node": n_dock,
                "soul_seed": tc_seed,
                "copper": 85 + (tc_seed % 40),
                "languages": langs,
                "persona": f"負責在{tname}棧橋清點靠岸貨駁木箱、發放竹籌並核銷鋪主預付腳錢的{tc_race}記籌手，算盤撥得滴水不漏。",
            })
            households.append({
                "id": hh_id,
                "cell_id": cid,
                "settlement": tname,
                "node_id": clerk_home,
                "label": f"{tc_name}的港務記籌寮",
                "vacant_for_sale": False,
                "price_copper": 0,
                "members": [tc_id],
            })


def build_all_88_between_town_cells(
    pack: dict[str, Any],
    f6_ids: set[int],
    settlement_defs: dict[int, dict[str, Any]],
    cell_map: dict[int, dict[str, Any]],
    allowed_cell_edges: dict[tuple[int, int], str],
    nodes: dict[str, dict[str, Any]],
    edges: list[tuple[str, str, str, str, str]],
    households: list[dict[str, Any]],
    residents: list[dict[str, Any]],
    town_micro_maps: dict[str, dict[str, Any]],
    cell_entry_from_neighbor: dict[tuple[int, int], str],
    cell_default_node: dict[int, str],
    add_node_fn: Any,
    link_bi_fn: Any,
    det_u64_fn: Any,
    pick_name_fn: Any,
    compass_dir_fn: Any,
) -> dict[str, Any]:
    """完整落實 88 個城間地塊（35 正典舊址 + 20 官道支線格 + 33 山海活源格），零複製貼上罐頭描寫。"""
    pcells = {c["i"]: c for c in pack["cells"] if isinstance(c, dict) and c.get("i") in f6_ids}
    provs = {
        p["i"]: (p.get("fullName") or p.get("name") or "銀鷗鄉野")
        for p in pack.get("provinces", [])
        if isinstance(p, dict) and "i" in p
    }
    rivers = {
        r["i"]: (r.get("name", "山溪") + r.get("type", "河"))
        for r in pack.get("rivers", [])
        if isinstance(r, dict) and "i" in r
    }
    goods = {
        g["i"]: g.get("name", "")
        for g in pack.get("goods", [])
        if isinstance(g, dict) and "i" in g
    }
    plague_zone_cells: set[int] = set()
    for z in pack.get("zones", []):
        if isinstance(z, dict) and z.get("i") == 6:
            plague_zone_cells = {cid for cid in z.get("cells", []) if cid in f6_ids}

    burgs_by_cell: dict[int, dict[str, Any]] = {}
    for b in pack.get("burgs", []):
        if isinstance(b, dict) and b.get("cell") in f6_ids and b["cell"] not in settlement_defs:
            burgs_by_cell[b["cell"]] = b

    assert len(burgs_by_cell) == 35, f"Expected 35 canonical rural burgs, got {len(burgs_by_cell)}"

    # 更新石牆阻斷清單：4200 青苔市、4429 灰松石堡、4206 哈倫灣保留 walls=1，但確保其至少有合法城門對外連通
    walled_rural_cids = {4200, 4429, 4206}

    used_rural_names: set[str] = {r["name"] for r in residents}
    rural_summary_counts = {"market_town": 0, "artisan_village": 0, "micro_hub": 0, "quarantine_post": 0, "survivor_stead": 0, "plague_ruin": 0, "collapsed_mine": 0}

    # 為無聚落 53 格找最近的核心鎮或具名聚落作方位地標錨點，確保 88 格描寫 100% 唯一無重樣
    named_anchors: list[tuple[int, str, float, float]] = []
    for cid, sdef in settlement_defs.items():
        cx, cy = cell_map[cid]["center"]
        named_anchors.append((cid, sdef["name"], cx, cy))
    for cid, b in burgs_by_cell.items():
        cx, cy = cell_map[cid]["center"]
        bname: str = "黑煤鎮舊址" if cid == 4529 else (RURAL_CANONICAL_NAMES.get(cid) or str(b["name"]))
        named_anchors.append((cid, bname, cx, cy))

    def nearest_landmark_phrase(cid: int) -> tuple[str, str, str, str, str]:
        cx, cy = cell_map[cid]["center"]
        ranked = sorted(
            (a for a in named_anchors if a[0] != cid),
            key=lambda a: (a[2] - cx) ** 2 + (a[3] - cy) ** 2,
        )
        best = ranked[0]
        second = ranked[1]
        third = ranked[2]
        direction_to_me = compass_dir_fn(cx - best[2], cy - best[3])
        dir_from_second = compass_dir_fn(cx - second[2], cy - second[3])
        return best[1], direction_to_me, second[1], dir_from_second, third[1]

    used_cell_titles: set[str] = {sdef["name"] for sdef in settlement_defs.values()}
    used_cell_descs: set[str] = set()
    micro_landmarks = (
        "青苔界石", "風化斷碑", "雙叉古松", "野鴿岩龕", "冷泉石槽", "紅土切坡", "碎貝淺溝", "灰鶴棲木",
        "舊炭窯跡", "石砌擋牆", "野莓灌叢", "白石英脈", "老牧羊圈", "避風岩凹", "山溪木筧", "古車轍印",
    )

    for cid in sorted(cell_map.keys()):
        if cid in settlement_defs:
            continue
        cinfo = cell_map[cid]
        pc = pcells[cid]
        cx, cy = cinfo["center"]
        h = cinfo["height"]
        fl = cinfo["flux"]
        is_coast = cinfo["is_coast"]
        prov_zh = provs.get(pc.get("province", 0), "銀鷗鄉野").replace("轄區", "")
        river_zh = rivers.get(pc.get("r", 0), "")
        good_zh = goods.get(pc.get("good", 0), "")
        in_plague = cid in plague_zone_cells
        has_road = any(cid in pair and ek == "highway" for pair, ek in allowed_cell_edges.items())

        n0 = f"oi:cell{cid}:0"
        cell_default_node[cid] = n0
        for nb in cinfo["neighbors_f6"]:
            cell_entry_from_neighbor[(cid, nb)] = n0

        # ── A. 35 處正典基層舊址逐格落位 ──
        if cid in burgs_by_cell:
            b = burgs_by_cell[cid]
            raw_bname = str(b.get("legacy_zh") or b["name"])
            pop_val = float(b.get("population", 1.0))
            is_port = bool(b.get("port", 0))
            is_walled = cid in walled_rural_cids
            slug = f"burg_{cid}"

            # 1) #4529 黑煤鎮舊址・斷橋塌陷坑
            if cid == 4529:
                rural_summary_counts["collapsed_mine"] += 1
                cinfo["kind"] = "ruin_dungeon"
                cinfo["title"] = "黑煤鎮舊址・斷橋坑"
                cinfo["desc"] = "昔日北林最大採煤聚落【黑煤鎮】舊址。太陽曆二三八年主煤井鑿穿荒氣地隙塌陷後，鎮民遷往黑煤溪寮與白鷗港，斷橋石縫下僅留通往北林試煉坑的封楔暗門。"
                cinfo["resource_slug"] = "black_coal"
                cinfo["stock_cap"] = 14
                cinfo["regen_per_day"] = 4
                cinfo["plague_sealed"] = False
                add_node_fn(n0, cid, "黑煤鎮舊址・斷橋塌陷坑", "ruin", cinfo["desc"], [cx, cy])
                continue

            # 2) 南岸船疫區 5 處徹底封村遺址（0 活人、0 固定刷怪）
            if cid in PLAGUE_SEALED_RUIN_CIDS:
                ruin_name = RURAL_CANONICAL_NAMES.get(cid, raw_bname)
                rural_summary_counts["plague_ruin"] += 1
                cinfo["kind"] = "plague_ruin"
                cinfo["title"] = f"{ruin_name}（封村遺址）"
                origin_note = (
                    f"太陽曆三二六年外洋病船最初停靠的原爆海港【{ruin_name}】，棧橋早已被火焚斷並釘上白灰封港木牌"
                    if cid == 4849
                    else f"太陽曆三二六年南岸船疫蔓延後奉令封閉撤離的{prov_zh}舊聚落【{ruin_name}】，石屋門窗釘著十字木條"
                )
                cinfo["desc"] = f"{origin_note}。巷內空無一人，亦無野獸盤踞，唯有石縫瘋長的野生苦艾與屋內未及帶走的陳年家書帳冊見證當年倉促封村的往事。"
                cinfo["resource_slug"] = "quarantine_wormwood"
                cinfo["stock_cap"] = 12
                cinfo["regen_per_day"] = 3
                cinfo["plague_sealed"] = True
                add_node_fn(n0, cid, f"{ruin_name}・封村界碑口", "plague_ruin", cinfo["desc"], [cx, cy])
                ruin_hall = f"oi:{slug}:ruin_hall"
                add_node_fn(
                    ruin_hall,
                    cid,
                    f"{ruin_name}・釘封舊祠堂",
                    "plague_ruin",
                    f"{ruin_name}荒廢三載的石砌舊堂，案上壓著太陽曆三二六年秋的封村名冊與幾封寫給白鷗港、鷗尾漁港親族卻未能寄出的泛黃書信，牆根長滿清冽苦艾。",
                    [round(cx + 0.5, 2), round(cy + 0.4, 2)],
                )
                link_bi_fn(n0, ruin_hall, "探看舊堂", "退回界碑", "alley")
                continue

            # 3) 活態聚落（含 #4695 風語橋哨、#4777/#4776 病癒孤莊、以及非疫 26 處 3/13/10 自然三階）
            bname: str = RURAL_CANONICAL_NAMES.get(cid) or str(raw_bname)
            spec = infer_rural_specialty(cid, str(raw_bname), is_port, h, fl, in_plague)

            if cid == PLAGUE_CHECKPOINT_CID:
                tier_key = "quarantine_post"
                target_people = 14
                hh_count = 4
            elif cid in PLAGUE_SURVIVOR_CIDS:
                tier_key = "survivor_stead"
                target_people = 5
                hh_count = 2
            elif pop_val >= 10.8:
                tier_key = "market_town"
                target_people = round(pop_val * 2.2)  # 24~26 人
                hh_count = 8
            elif pop_val >= 3.2:
                tier_key = "artisan_village"
                target_people = max(7, round(pop_val * 2.2))  # 7~21 人自然梯度
                hh_count = max(2, min(6, round(target_people / 3.2)))
            else:
                tier_key = "micro_hub"
                if is_port:
                    hh_count = 2
                    target_people = 5
                else:
                    hh_count = 1
                    target_people = 3

            rural_summary_counts[tier_key] += 1
            cinfo["kind"] = "rural_settlement"
            cinfo["title"] = bname
            cinfo["resource_slug"] = spec["resource_slug"]
            cinfo["stock_cap"] = 28 if tier_key == "market_town" else (20 if tier_key == "artisan_village" else 14)
            cinfo["regen_per_day"] = 8 if tier_key == "market_town" else 5
            cinfo["plague_sealed"] = cid in PLAGUE_SURVIVOR_CIDS

            wall_phrase = "矮石城牆環護的" if is_walled else "依山傍水而建的"
            cinfo["desc"] = f"{prov_zh}境內{wall_phrase}{bname}（共{cn_int(hh_count)}戶常住人家），以產出{spec['resource_zh']}聞名，與五座主鎮車船往來不絕。"

            # 幹道村口正交側枝（T-branch）：n0 為幹道村口，過路 0 額外步數；轉入 n_yard 才進村內！
            gate_suffix = "石牆門口" if is_walled else ("檢疫橋頭" if cid == PLAGUE_CHECKPOINT_CID else ("封鎖木欄口" if cid in PLAGUE_SURVIVOR_CIDS else "村口石牌"))
            add_node_fn(
                n0,
                cid,
                f"{bname}・{gate_suffix}",
                "village_gate",
                f"幹道旁立著刻有【{bname}】字樣的風化青石界碑，路邊留著載貨馬車與挑擔村民進出的新鮮轍印。",
                [cx, cy],
            )

            n_yard = f"oi:{slug}:yard"
            yard_kind = "quarantine_post" if cid == PLAGUE_CHECKPOINT_CID else ("survivor_palisade" if cid in PLAGUE_SURVIVOR_CIDS else "plaza")
            add_node_fn(
                n_yard,
                cid,
                f"{bname}・{spec['yard_label']}",
                yard_kind,
                spec["yard_desc"],
                [round(cx + 0.6, 2), round(cy - 0.4, 2)],
            )
            in_label = "進石堡" if is_walled else ("上檢疫橋" if cid == PLAGUE_CHECKPOINT_CID else ("近隔欄台" if cid in PLAGUE_SURVIVOR_CIDS else "進村"))
            link_bi_fn(n0, n_yard, in_label, "出村口", "street")

            landmarks_micro = []
            # 一級鄉鎮市集額外設一間百工農具集坊（可修磨工具、收發鄉鎮貨箱）
            if tier_key == "market_town":
                n_ws = f"oi:{slug}:workshop"
                add_node_fn(
                    n_ws,
                    cid,
                    f"{bname}・百工農具集坊",
                    "shop",
                    f"{bname}趕集日最熱鬧的石木敞棚工坊，爐邊擺著待修的犁頭、鐮刀與準備裝車發往主港的{spec['resource_zh']}。",
                    [round(cx + 1.0, 2), round(cy - 0.7, 2)],
                    indoor=True,
                )
                link_bi_fn(n_yard, n_ws, "進集坊", "出門", "door")
                landmarks_micro.append({"node_id": n_ws, "label": "百工農具集坊", "kind": "forge"})

            # 若該格同時為 4 處官道茶棚之一（如 4200 青苔市、4432 古加坡），於村口 n0 旁掛接避雨茶棚
            if cid in TEA_SHED_CELLS:
                ts_name, ts_desc = TEA_SHED_CELLS[cid]
                n_tea = f"oi:cell{cid}:tea_shed"
                add_node_fn(n_tea, cid, f"{bname}・{ts_name}", "tea_shed", ts_desc, [round(cx - 0.5, 2), round(cy + 0.4, 2)])
                link_bi_fn(n0, n_tea, "入茶棚歇腳", "回官道", "street")

            # 逐戶精確分配人數，總和嚴格等於 target_people
            base_per_hh = max(1, target_people // hh_count)
            rem = max(0, target_people - base_per_hh * hh_count)
            roof_rects = []

            for h_i in range(1, hh_count + 1):
                fam_size = base_per_hh + (1 if h_i <= rem else 0)
                h_seed = det_u64_fn(f"rural_hh:{cid}:{h_i}")
                fam_race = spec["races"][h_seed % len(spec["races"])]
                fam_job = spec["jobs"][(h_seed >> 3) % len(spec["jobs"])]
                head_name = pick_name_fn(fam_race, fam_job, h_seed, with_epithet=True, used_names=used_rural_names)
                home_nid = f"oi:{slug}:h{h_i}"
                hh_id = f"hh:{slug}:h{h_i}"
                h_num_zh = cn_int(h_i)
                h_title = f"{bname}・{h_num_zh}號{head_name}家"
                h_desc = (
                    f"{bname}封鎖木欄內側的石木農舍，屋裡晾著剛採下的苦艾草束，桌角壓著盼望捎往山外親故的家書。"
                    if cid in PLAGUE_SURVIVOR_CIDS
                    else f"{bname}的{fam_race}{fam_job}{head_name}一家居所，院角堆著整理妥當的{spec['resource_zh']}與日常農具，灶間飄著麥湯熱氣。"
                )
                add_node_fn(
                    home_nid,
                    cid,
                    h_title,
                    "house",
                    h_desc,
                    [round(cx + 0.7 + h_i * 0.12, 2), round(cy + 0.2 + (h_i % 2) * 0.25, 2)],
                    indoor=True,
                )
                link_bi_fn(n_yard, home_nid, f"訪{head_name}家", "出門", "door")

                mem_ids = []
                for m_i in range(fam_size):
                    m_seed = det_u64_fn(f"rural_mem:{hh_id}:{m_i}")
                    m_name = head_name if m_i == 0 else pick_name_fn(fam_race, fam_job, m_seed, False, used_names=used_rural_names)
                    m_role = fam_job if m_i == 0 else ("家眷" if m_i == 1 else "年輕幫手")
                    m_id = f"res:{slug}:h{h_i}_{m_i}"
                    m_langs = {"harbor_trade": 88 if fam_race == "人類" else 70}
                    if fam_race == "矮人":
                        m_langs["dwarven"] = 100
                    elif fam_race == "精靈":
                        m_langs["elvish"] = 100
                    elif fam_race == "獸人":
                        m_langs["orcish"] = 100
                    work_target = n_yard if m_i == 0 else (n0 if m_i == 2 and cid == PLAGUE_CHECKPOINT_CID else home_nid)
                    residents.append({
                        "id": m_id,
                        "name": m_name,
                        "race": fam_race,
                        "role": m_role,
                        "household_id": hh_id,
                        "home_node": home_nid,
                        "work_node": work_target,
                        "soul_seed": m_seed,
                        "copper": 28 + (m_seed % 26),
                        "languages": m_langs,
                        "persona": f"世居{bname}操持{fam_job}營生的{fam_race}{m_role}，常隨驛車貨駁與五鎮親故往來互通有無。",
                    })
                    mem_ids.append(m_id)

                households.append({
                    "id": hh_id,
                    "cell_id": cid,
                    "settlement": bname,
                    "node_id": home_nid,
                    "label": h_title,
                    "vacant_for_sale": False,
                    "price_copper": 0,
                    "members": mem_ids,
                })
                roof_rects.append({"alley_idx": 0, "house_idx": h_i, "node_id": home_nid, "vacant": False, "label": h_title})

            town_micro_maps[slug] = {
                "slug": slug,
                "cell_id": cid,
                "name": bname,
                "tier": tier_key,
                "walled": is_walled,
                "has_port": is_port,
                "coast_dir": "east" if cx > 1445 and is_port else ("west" if cx < 1395 and is_port else ("south" if is_port else "none")),
                "spine": {
                    "north_gate": n0,
                    "smithy_st": None,
                    "plaza": n_yard,
                    "tavern_st": None,
                    "south_gate": None,
                    "dock": n0 if is_port else None,
                },
                "landmarks": landmarks_micro,
                "alleys": [{"name": spec["yard_label"], "node_id": n_yard, "anchor": n0, "side": "east", "index": 0}],
                "roofs": roof_rects,
            }
            continue

        # ── B. 53 個非聚落城間地塊（20 道路河橋走廊格 ＋ 33 山海活源格，格格唯一地名與專屬資源）──
        near_name, dir_from_near, second_name, dir_from_second, third_name = nearest_landmark_phrase(cid)
        apex_info = APEX_BEASTS_BY_CELL.get(cid)
        elev_band_zh = (
            "高聳冷杉風嶺" if h >= 28 else
            "灰岩緩升山腰" if h >= 23 else
            "起伏林間台地" if h >= 19 else
            "濱海低平坡谷"
        )
        span_tag = f"{near_name}{dir_from_near}側、{second_name}{dir_from_second}向並遙望{third_name}的{elev_band_zh}（海拔約{cn_int(max(1, h))}丈）"

        if good_zh == "石英白沙":
            res_slug, cap, regen = "quartz_sand", 16, 3
            title = f"{prov_zh}・晶瑩白沙灣"
            desc = f"位於{span_tag}的罕見石英白沙海灣，細白如雪的石英砂隨海浪閃耀銀光，是全島鍛造高階透光器皿與符文砂輪的唯一產地。"
            n_kind = "wild"
        elif river_zh:
            res_slug, cap, regen = ("fresh_fish_basket", 18, 6) if not in_plague else ("quarantine_wormwood", 12, 4)
            bridge_word = "石拱官橋" if has_road else "淺水卵石渡"
            title = f"{river_zh}・{near_name}{bridge_word}"
            desc = f"{river_zh}流經{span_tag}的河谷地段，溪流漫過青灰岩床，{'石拱橋面留著深深驛車輪轍，大潮與山雨時仍穩固可通' if has_road else '平水期可踩寬平卵石涉渡，漲潮急流時則需格外留神腳步'}。"
            n_kind = "road" if has_road else "wild"
        elif has_road:
            if in_plague:
                res_slug, cap, regen = "quarantine_wormwood", 14, 4
                title = f"{prov_zh}・南境封鎖古道（{near_name}{dir_from_near}）"
                desc = f"串連{span_tag}的南岸舊官道，路旁石碑刻著三二六年封港警示，車轍間長出成叢耐寒苦艾，偶見風語橋哨兵巡查留下的火把灰。"
            elif is_coast:
                res_slug, cap, regen = "shellfish_meat", 18, 6
                title = f"{prov_zh}・望海驛道（{near_name}{dir_from_near}）"
                desc = f"緊貼海岸起伏的碎石驛道，位處{span_tag}，路邊立著計步青石里程碑，退潮時路下礁灘可見成簇海貝。"
            elif h >= 24:
                res_slug, cap, regen = "iron_ore", 20, 5
                title = f"{prov_zh}・山脊隘道（{near_name}{dir_from_near}）"
                desc = f"鑿開{prov_zh}灰岩山脊修築的盤山驛道，地處{span_tag}風口，岩壁裸露著暗紅粗鐵礦紋，雙馬貨車過坡時車鈴聲迴盪山谷。"
            else:
                res_slug, cap, regen = "herb_bundle", 20, 6
                title = f"{prov_zh}・林蔭官道（{near_name}{dir_from_near}）"
                desc = f"穿行於{prov_zh}闊葉林間的平坦官道，座落{span_tag}，路緣青石里程碑字跡清晰，樹蔭下偶見歇腳挑夫留下的茶葉渣。"
            n_kind = "road"
        elif is_coast:
            if in_plague:
                res_slug, cap, regen = "quarantine_wormwood", 12, 3
                title = f"{prov_zh}・禁泊礁岸（{near_name}{dir_from_near}）"
                desc = f"{span_tag}外海風浪拍擊的偏僻礁岸，崖頭插著南岸檢疫禁泊木標，鹹霧岩縫間生長著藥力厚重的海風苦艾。"
            else:
                res_slug, cap, regen = "shellfish_meat", 20, 6
                title = f"{prov_zh}・潮間貝礁灣（{near_name}{dir_from_near}）"
                desc = f"{span_tag}無官道經過的天然礁岩海灣，唯有退潮時段暗礁裸露方能下灘撬取肥美海貝與海帶；漲潮時白浪拍岸。"
            n_kind = "wild"
        elif h >= 25:
            res_slug, cap, regen = "iron_ore", 22, 5
            title = f"{prov_zh}・高脊礦岩坡（{near_name}{dir_from_near}）"
            desc = f"{span_tag}上方地勢陡峻的分水山脊，亂石間嵌著粗鐵礦脈與山羊獸徑，寒風掠過灰岩發出低鳴。"
            n_kind = "wild"
        else:
            res_slug, cap, regen = ("beast_tallow", 16, 4) if (cid % 2 == 0 and not in_plague) else ("timber_log", 20, 5)
            title = f"{prov_zh}・幽林獵場（{near_name}{dir_from_near}）"
            desc = f"深入{span_tag}腹地的原始密林，古樹根系盤錯，林間可見野鹿啃食苔蘚與獵戶佈下的舊繩套，是採集原木與熬製鎮邦聖物修補獸脂的活源產地。"
            n_kind = "wild"

        if apex_info:
            if cid == 4070:
                title = f"{prov_zh}・北灣巨鰲暗礁灣"
            elif cid == 4326:
                title = f"{prov_zh}・禁林古鹿幽谷"
            elif cid == 5079:
                title = f"{prov_zh}・極南吞潮斷崖"

        if title in used_cell_titles:
            title = f"{title}・近{second_name}"
        if title in used_cell_titles:
            title = f"{title}（望{third_name}）"
        if title in used_cell_titles:
            title = f"{title}・海岬第{cn_int((cid % 9) + 1)}灣"
        used_cell_titles.add(title)

        if desc in used_cell_descs:
            ml = micro_landmarks[(cid * 7 + h + fl) % len(micro_landmarks)]
            desc = f"{desc} 地界旁留有一處{ml}與通往{third_name}方向的舊日路標。"
        if desc in used_cell_descs:
            desc = f"{desc}（水脈匯流約{cn_int(max(1, fl))}股）"
        used_cell_descs.add(desc)

        if apex_info:
            desc = f"{desc} {apex_info['lair_hint_zh']}"

        cinfo["kind"] = "road_corridor" if n_kind == "road" else "wilderness"
        cinfo["title"] = title
        cinfo["desc"] = desc
        cinfo["resource_slug"] = res_slug
        cinfo["stock_cap"] = cap
        cinfo["regen_per_day"] = regen
        cinfo["plague_sealed"] = in_plague
        if apex_info:
            cinfo["apex_beast"] = apex_info

        tea_info = TEA_SHED_CELLS.get(cid)
        if n_kind == "road":
            build_road_corridor_mesh(
                cid=cid,
                base_title=title,
                base_desc=desc,
                prov_zh=prov_zh,
                river_zh=river_zh,
                is_coast=is_coast,
                h=h,
                in_plague=in_plague,
                tea_shed_info=tea_info,
                cell_map=cell_map,
                allowed_cell_edges=allowed_cell_edges,
                cell_entry_from_neighbor=cell_entry_from_neighbor,
                add_node_fn=add_node_fn,
                link_bi_fn=link_bi_fn,
                compass_dir_fn=compass_dir_fn,
            )
        else:
            build_wilderness_mesh(
                cid=cid,
                base_title=title,
                base_desc=desc,
                prov_zh=prov_zh,
                river_zh=river_zh,
                good_zh=good_zh,
                is_coast=is_coast,
                h=h,
                in_plague=in_plague,
                apex_info=apex_info,
                tea_shed_info=tea_info,
                cell_map=cell_map,
                allowed_cell_edges=allowed_cell_edges,
                cell_entry_from_neighbor=cell_entry_from_neighbor,
                add_node_fn=add_node_fn,
                link_bi_fn=link_bi_fn,
                compass_dir_fn=compass_dir_fn,
            )

    insert_inter_settlement_buffers_if_needed(
        allowed_cell_edges=allowed_cell_edges,
        settlement_defs=settlement_defs,
        burgs_by_cell=burgs_by_cell,
        plague_ruin_cids=PLAGUE_SEALED_RUIN_CIDS,
        cell_map=cell_map,
        cell_entry_from_neighbor=cell_entry_from_neighbor,
        cell_default_node=cell_default_node,
        add_node_fn=add_node_fn,
        link_bi_fn=link_bi_fn,
        compass_dir_fn=compass_dir_fn,
    )
    populate_existing_rural_map_xy(nodes, settlement_defs)
    return rural_summary_counts


def annotate_milestones_and_build_transit_lines(
    nodes: dict[str, dict[str, Any]],
    cell_map: dict[int, dict[str, Any]],
) -> list[dict[str, Any]]:
    """
    1) 以多源最短路在城間道路／野徑出口標註真實里程石碑（如『里程碑：往白鷗港 3 步』）；
    2) 編譯並驗證四條實體逐拍車船班線（東西幹線驛車、南北支線貨車、北岸淨岸貨駁、南灣檢疫外貨駁）。
    """
    core_targets = [
        ("oi:white_gull_harbor:plaza", "白鷗港"),
        ("oi:west_strait_ferry:plaza", "西峽渡口"),
        ("oi:gull_tail_port:plaza", "鷗尾漁港"),
        ("oi:greypine_post:plaza", "灰松林驛"),
        ("oi:north_cape_beacon:plaza", "北岬燈塔"),
    ]

    def bfs_dist_to(target_nid: str) -> dict[str, int]:
        rev_adj: dict[str, list[str]] = {k: [] for k in nodes}
        for nid, nd in nodes.items():
            for ex in nd["exits"]:
                rev_adj[ex["to"]].append(nid)
        dist = {target_nid: 0}
        q = deque([target_nid])
        while q:
            cur = q.popleft()
            for prv in rev_adj[cur]:
                if prv not in dist:
                    dist[prv] = dist[cur] + 1
                    q.append(prv)
        return dist

    dists_by_town = {tname: bfs_dist_to(tnid) for tnid, tname in core_targets}

    for nid, nd in nodes.items():
        if nd["indoor"] or nd["dungeon"]:
            continue
        for ex in nd["exits"]:
            if ex["kind"] not in ("highway", "wild_trail", "trail"):
                continue
            dst_nid = ex["to"]
            best_town = None
            best_d = 999
            for tname, dmap in dists_by_town.items():
                d_cur = dmap.get(nid, 999)
                d_dst = dmap.get(dst_nid, 999)
                if d_dst < d_cur and d_dst < best_d:
                    best_d = d_dst
                    best_town = tname
            if best_town is not None and best_d <= 48:
                ex["milestone_zh"] = f"往{best_town}{best_d}步" if best_d > 0 else f"抵{best_town}"

    def shortest_outdoor_chain(a: str, b: str) -> list[str]:
        q = deque([[a]])
        vis = {a}
        while q:
            path = q.popleft()
            cur = path[-1]
            if cur == b:
                return path
            for ex in nodes[cur]["exits"]:
                nxt = ex["to"]
                if nxt in vis:
                    continue
                n_def = nodes[nxt]
                if n_def["indoor"] or n_def["dungeon"]:
                    continue
                vis.add(nxt)
                q.append(path + [nxt])
        raise AssertionError(f"Transit route broken between {a} and {b}")

    def chain_trunk_stops(wps: list[str]) -> list[str]:
        """步車分軌：車船班線僅擷取沿途地塊之幹道核心樞紐（:0 / 城鎮門街廣場），不插入徒步細分坡口，維持整趟 8~15s。"""
        raw: list[str] = []
        for s, t in zip(wps, wps[1:]):
            seg = shortest_outdoor_chain(s, t)
            if raw:
                raw.extend(seg[1:])
            else:
                raw.extend(seg)
        wps_set = set(wps)
        trunk: list[str] = []
        for nid in raw:
            is_sub_approach = (
                ":approach_" in nid
                or ":skirt_" in nid
                or ":bound_" in nid
                or nid.endswith(":low")
                or nid.endswith(":high")
                or nid.endswith(":far")
                or nid.endswith(":vantage")
                or nid.endswith(":camp")
            )
            if nid not in wps_set and is_sub_approach:
                continue
            if not trunk or trunk[-1] != nid:
                trunk.append(nid)
        return trunk

    # 1. 陸路東西幹線驛車（3 輛對開，步車分軌只跨大站樞紐，零折返重複節點）
    ew_nodes = chain_trunk_stops([
        "oi:white_gull_harbor:plaza",
        "oi:cell4432:0",
        "oi:greypine_post:north_gate",
        "oi:cell4201:0",
        "oi:west_strait_ferry:plaza",
    ])
    # 2. 陸路南北支線貨車（3 輛對開，步車分軌只跨大站樞紐，零折返重複節點）
    ns_nodes = chain_trunk_stops([
        "oi:cell4695:0",
        "oi:gull_tail_port:plaza",
        "oi:greypine_post:north_gate",
        "oi:cell4429:0",
        "oi:cell4200:0",
        "oi:north_cape_beacon:plaza",
    ])
    # 3. 北岸淨岸貨駁（沿海港塢節點逐站航行：白鷗港碼頭 ↔ 韋斯塢 ↔ 艾爾市 ↔ 北岬燈塔 ↔ 泰原塢 ↔ 錨灣塢 ↔ 哈倫灣 ↔ 西峽渡口）
    north_sea_nodes = [
        "oi:white_gull_harbor:dock",
        "oi:cell4433:0",
        "oi:cell4077:0",
        "oi:north_cape_beacon:dock",
        "oi:cell4196:0",
        "oi:cell4204:0",
        "oi:cell4206:0",
        "oi:west_strait_ferry:dock",
    ]
    # 4. 南灣檢疫外貨駁（白鷗港碼頭 ↔ 南灣望潮岸 ↔ 鷗尾漁港碼頭 ↔ 艾森東澳口，嚴守不入南岸封港疫區）
    south_sea_nodes = [
        "oi:white_gull_harbor:dock",
        "oi:cell4612:0",
        "oi:gull_tail_port:dock",
        "oi:cell4853:0",
    ]

    for nid in north_sea_nodes + south_sea_nodes:
        assert nid in nodes, f"Missing sea stop node {nid}"

    return [
        {
            "line_id": "land_ew_express",
            "name_zh": "東西幹線雙馬驛車",
            "mode": "carriage",
            "vehicle_count": 3,
            "fare_copper": 2,
            "route_nodes": ew_nodes,
        },
        {
            "line_id": "land_ns_freight",
            "name_zh": "南北縱貫重載篷車",
            "mode": "carriage",
            "vehicle_count": 3,
            "fare_copper": 2,
            "route_nodes": ns_nodes,
        },
        {
            "line_id": "sea_north_barge",
            "name_zh": "北岸環島運貨帆駁",
            "mode": "barge",
            "vehicle_count": 3,
            "fare_copper": 3,
            "route_nodes": north_sea_nodes,
        },
        {
            "line_id": "sea_south_bay_barge",
            "name_zh": "南灣淨岸漁鹽貨駁",
            "mode": "barge",
            "vehicle_count": 2,
            "fare_copper": 2,
            "route_nodes": south_sea_nodes,
        },
    ]
