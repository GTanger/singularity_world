#!/usr/bin/env python3
"""
將 Azgaar 世界母圖 `Tanger Full 2026-11-18-19-55.json` 全量專有名詞與編年史（約 2,800 筆）
無損轉譯為《奇點世界》台灣正體中文（zh-TW），並依「兩層自然權重公式」計算全大陸 100 國與 850 城之四族結構。

產出：
1. data/world/tanger_full_zh_v1.json（完整中文化 Azgaar 母圖，保留 orig_name 供雙向追溯）
2. data/world/tanger_glossary_zh_v1.json（中英對照總辭典 ＋ 100 國／850 城四族自然權重表）
3. docs/design/奇點世界_全大陸中文地名政權物產戰史總冊_2026-11-20.md（可讀總覽清冊）
"""

from __future__ import annotations

import copy
import hashlib
import json
import re
from collections import Counter
from pathlib import Path

SRC_PATH = Path("/home/tanger/下載/Tanger Full 2026-11-18-19-55.json")
OUT_FULL_JSON = Path("/home/tanger/Projects/singularity_world/data/world/tanger_full_zh_v1.json")
OUT_GLOSSARY_JSON = Path("/home/tanger/Projects/singularity_world/data/world/tanger_glossary_zh_v1.json")
OUT_DOC_MD = Path("/home/tanger/Projects/singularity_world/docs/design/奇點世界_全大陸中文地名政權物產戰史總冊_2026-11-20.md")


def clean_surrogates(text: str) -> str:
    return text.encode("utf-16", "surrogatepass").decode("utf-16", "replace")


def det_u32(key: str) -> int:
    return int.from_bytes(hashlib.sha256(key.encode("utf-8")).digest()[:4], "big")


# ─── 一、固定枚舉字典（生態系、物產、政體、省制、水系、信仰型態、戰役詞尾） ───

BIOME_ZH = {
    0: "外海大洋",
    1: "赤沙熱漠",
    2: "寒磧冷漠",
    3: "稀樹莽原",
    4: "廣袤草原",
    5: "季風雨林",
    6: "溫帶闊葉林",
    7: "赤道密林",
    8: "溫帶雨林",
    9: "北方針葉林",
    10: "極北苔原",
    11: "萬年冰原",
    12: "蘆葦大澤",
}

# 71 項貿易物產（遵循無火藥物理根律：Saltpeter->硝鹽、Gunpowder->鍛爐助燃礦粉、Artillery->重型扭力投石機）
GOODS_ZH = {
    1: ("原木", "堆"),
    2: ("石材", "垛"),
    3: ("雲紋石", "垛"),
    4: ("鐵礦", "車"),
    5: ("銅礦", "車"),
    6: ("錫礦", "車"),
    7: ("白銀", "錠"),
    8: ("黃金", "錠"),
    9: ("穀麥", "車"),
    10: ("牛隻", "頭"),
    11: ("鮮魚", "車"),
    12: ("野味獸肉", "車"),
    13: ("果釀葡萄酒", "桶"),
    14: ("橄欖", "桶"),
    15: ("野蜂蜜", "桶"),
    16: ("海鹽", "袋"),
    17: ("乾棗", "車"),
    18: ("健馬", "匹"),
    19: ("巨獸長牙", "根"),
    20: ("雙峰駝", "頭"),
    21: ("粗麻", "車"),
    22: ("海珠", "顆"),
    23: ("寶石原礦", "顆"),
    24: ("草木染料", "袋"),
    25: ("樹脂薰香", "箱"),
    26: ("蠶絲綢", "匹"),
    27: ("遠洋香料", "箱"),
    28: ("松脂琥珀", "塊"),
    29: ("厚毛獸皮", "張"),
    30: ("綿羊毛", "捆"),
    31: ("苦役契工", "名"),
    32: ("船用松焦油", "桶"),
    33: ("鞣革硝鹽", "桶"),
    34: ("鍛爐煤礦", "車"),
    35: ("燈用鯨脂油", "桶"),
    36: ("硬紅木", "堆"),
    37: ("海獸厚脂", "桶"),
    38: ("蔗糖", "袋"),
    39: ("山茶葉", "袋"),
    40: ("旱菸葉", "袋"),
    41: ("陶土", "車"),
    42: ("石英白沙", "車"),
    43: ("熟鞣皮革", "卷"),
    44: ("織布", "匹"),
    45: ("成衣斗篷", "套"),
    46: ("窯燒陶器", "車"),
    47: ("吹製琉璃", "車"),
    48: ("麻纜繩索", "卷"),
    49: ("麻紙", "令"),
    50: ("松煙墨汁", "瓶"),
    51: ("裝幀典籍", "冊"),
    52: ("厚帆布", "套"),
    53: ("木造舟船", "艘"),
    54: ("厚底皮靴", "雙"),
    55: ("鞍轡馬具", "套"),
    56: ("橡木酒桶", "只"),
    57: ("青銅錠", "車"),
    58: ("鐵木工具", "套"),
    59: ("鍛鐵兵刃", "套"),
    60: ("鍛爐助燃礦粉", "桶"),
    61: ("重型扭力投石機", "架"),
    62: ("鑄造錢幣", "袋"),
    63: ("金銀首飾", "件"),
    64: ("鹽漬保存糧", "車"),
    65: ("釀造酸醋", "桶"),
    66: ("熟成乳酪", "車"),
    67: ("大麥啤酒", "桶"),
    68: ("烈餾燒酒", "罈"),
    69: ("蜂蠟燭磚", "塊"),
    70: ("草木灰皂", "桶"),
    71: ("花露香膏", "瓶"),
}

CULTURE_ZH = {
    0: "荒野散部",
    1: "青灣海裔",
    2: "銀潮海裔",
    3: "蒼原游牧裔",
    4: "鐵嶺原裔",
    5: "赤沙獵裔",
    6: "霜脊游牧裔",
    7: "寒松原裔",
    8: "翠林古裔",
    9: "珊礁島裔",
    10: "雲澤水裔",
    11: "幽林森裔",
    12: "嵐谷林裔",
    13: "黑磧荒原裔",
}

STATE_FORM_ZH = {
    "Beylik": "邊境伯領",
    "Brotherhood": "誓約兄弟團",
    "Chiefdom": "酋長領",
    "City-state": "自由城邦",
    "Duchy": "公國",
    "Empire": "帝國",
    "Grand Duchy": "大公國",
    "Horde": "荒原部族聯軍",
    "Imamah": "聖裔領",
    "Khaganate": "大汗國",
    "Khanate": "汗國",
    "Kingdom": "王國",
    "League": "商邦同盟",
    "Marches": "邊疆戍衛領",
    "Principality": "親王國",
    "Protectorate": "護民領",
    "Republic": "共和邦",
    "Satrapy": "總督領",
    "See": "聖座領",
    "Shogunate": "軍幕領",
    "Thearchy": "古祀邦",
    "Theocracy": "祭司邦",
    "Ulus": "游牧兀魯思",
    "Union": "聯合邦",
    "United Provinces": "聯省邦",
}

PROV_FORM_ZH = {
    "Barony": "男爵領",
    "Canton": "山谷邦",
    "Captaincy": "都尉轄區",
    "Colony": "拓殖領",
    "Council": "議事轄區",
    "County": "伯爵郡",
    "Deanery": "林谷轄區",
    "Earldom": "方伯領",
    "Governorate": "鎮守轄區",
    "Landgrave": "方伯州",
    "Margrave": "邊防侯領",
    "Parish": "鄉鎮轄區",
    "Province": "行省",
    "Seneschalty": "總管轄區",
    "Shire": "郡",
    "State": "邦領",
    "Territory": "邊領",
}

RIVER_TYPE_ZH = {
    "River": "河",
    "Fork": "支流",
    "Creek": "溪",
    "Brook": "澗",
}

CAMPAIGN_SUFFIX_ZH = {
    "War": "戰爭",
    "Campaign": "戰役",
    "Invasion": "征伐之役",
    "Rebellion": "叛亂之役",
    "Conflict": "邊境衝突",
    "Conquest": "征服之戰",
    "Expedition": "遠征之役",
    "Intervention": "干預之役",
    "Crusade": "討伐遠征",
}

# 已定錨正典地名（東南外海大島 Feature 6）
FIXED_STATE_STEMS = {
    26: "銀鷗",  # fullName 特殊鎖定為「銀鷗海領」
}
FIXED_STATE_FULL = {
    26: "銀鷗海領",
}
FIXED_BURG_NAMES = {
    26: "白鷗港",
    188: "西峽渡口",
    301: "鷗尾漁港",
    147: "灰松林驛",
    137: "海風北塢",
}

# ─── 二、短音譯與風土詞根池（2～3 字，徹底洗除地球現成省市撞名） ───

STEPPE_SYL_1 = ["瑟", "烏", "達", "巴", "科", "阿", "托", "扎", "諾", "呼", "帖", "沙", "博", "額", "圖", "喀", "庫", "塔", "哈", "阿爾"]
STEPPE_SYL_2 = ["爾", "蘭", "圖", "泰", "克", "顏", "倫", "木", "格", "魯", "頓", "蘇", "칸".replace("칸", "汗"), "丹", "莫", "古", "斯", "加", "拉", "溫"]

HIGHLAND_ROOTS = [
    "鐵砧", "黑岩", "霜脊", "赤銅", "深爐", "灰礫", "石槌", "銀脈", "玄武", "斷崖",
    "寒鐵", "礦谷", "熔岩", "堅石", "雪峰", "青岩", "銅錘", "白堊", "燧石", "雲嶺",
    "卡蘭", "格隆", "杜恩", "托倫", "巴頓", "莫爾", "科爾", "達克", "洛克", "奧恩",
]

SYLVAN_ROOTS = [
    "灰松", "銀樺", "翠影", "青苔", "霧杉", "星葉", "白梣", "月溪", "風語", "松露",
    "鹿鳴", "微光", "古橡", "深林", "藤谷", "蕨溪", "琥珀", "冷杉", "柳泉", "蒼苔",
    "艾爾", "洛林", "席爾", "維爾", "法爾", "伊蘭", "諾林", "萊恩", "塞恩", "奧林",
]

COAST_ROOTS = [
    "白浪", "銀潮", "鹽風", "珊礁", "青灣", "海霧", "錨灣", "鯨骨", "星岬", "碎浪",
    "帆影", "灰礁", "海鷗", "潮音", "風帆", "礫灘", "深澳", "螺灣", "浮木", "燈岬",
    "卡斯", "盧卡", "貝爾", "馬丁", "安伯", "雷恩", "克萊", "瓦倫", "哈倫", "韋斯",
]

HEARTLAND_SYL_1 = [
    "羅", "維", "諾", "格", "艾", "奧", "貝", "萊", "科", "達",
    "米", "瓦", "霍", "雷", "索", "阿", "卡", "布", "法", "塔",
    "克", "塞", "莫", "席", "盧", "安", "古", "哈", "韋", "佩",
    "瑪", "納", "薩", "博", "拉", "伊", "泰", "澤", "歐", "迪",
]
HEARTLAND_SYL_2 = [
    "蘭", "爾", "林", "登", "恩", "倫", "斯", "克", "頓", "維",
    "利", "亞", "拉", "納", "德", "曼", "特", "森", "伯", "威",
    "洛", "昂", "尼", "加", "門", "谷", "原", "橋", "泉", "麥",
]

GOOD_SCENIC_PREFIX = {
    4: "鐵爐", 5: "紅銅", 6: "白錫", 7: "銀溪", 8: "金沙",
    9: "金麥", 11: "曬網", 13: "紫藤", 15: "蜂谷", 16: "鹽場",
    18: "牧馬", 22: "珠貝", 23: "晶石", 26: "桑林", 28: "琥珀",
    29: "獵裘", 34: "黑煤", 36: "紅木", 39: "雲茶", 41: "紅陶",
    43: "熟革", 44: "織梭", 48: "麻繩", 53: "船塢", 64: "鹽魚",
    66: "乳酪", 67: "麥酒",
}

FORBIDDEN_REAL_NAMES = {
    "瑞州", "南昌", "台南", "太平", "溫州", "廣州", "荊州", "徐州", "楚州",
    "北京", "南京", "東京", "大京", "長安", "洛陽", "成都", "杭州", "蘇州",
    "台灣", "中國", "日本", "朝鮮", "高麗", "越南", "波斯", "突厥",
}


def make_phonetic_stem(raw_name: str, culture_id: int, salt: str, used: set[str] | None = None) -> str:
    h = det_u32(f"{raw_name}:{culture_id}:{salt}")
    for attempt in range(400):
        idx = h + attempt * 37
        if culture_id in (3, 6, 13):
            a = STEPPE_SYL_1[idx % len(STEPPE_SYL_1)]
            b = STEPPE_SYL_2[(idx >> 5) % len(STEPPE_SYL_2)]
            cand = f"{a}{b}" if a != b else f"{a}圖"
            if attempt >= 80:
                c = STEPPE_SYL_2[(idx >> 9) % len(STEPPE_SYL_2)]
                cand = f"{a[:1]}{b}{c}"
        else:
            a = HEARTLAND_SYL_1[idx % len(HEARTLAND_SYL_1)]
            b = HEARTLAND_SYL_2[(idx >> 5) % len(HEARTLAND_SYL_2)]
            cand = f"{a}{b}" if a != b else f"{a}爾"
            if attempt >= 80:
                c = HEARTLAND_SYL_2[(idx >> 10) % len(HEARTLAND_SYL_2)]
                cand = f"{a}{b}{c}"[:3]
        if cand in FORBIDDEN_REAL_NAMES:
            continue
        if used is None or cand not in used:
            if used is not None:
                used.add(cand)
            return cand
    fallback = f"阿爾{h % 99}"
    if used is not None:
        used.add(fallback)
    return fallback


# ─── 三、兩層自然權重種族自動計算公式（全大陸 4,015 陸格／850 城適用） ───

def compute_cell_base_race_weights(cell: dict, culture_type: str, prod_good_ids: list[int]) -> dict[str, float]:
    h = float(cell.get("h", 20))
    biome = int(cell.get("biome", 4))
    w_human = 18.0
    w_dwarf = 4.0
    w_elf = 4.0
    w_orc = 6.0

    # 1. 海拔與礦脈 -> 矮人
    if h >= 60:
        w_dwarf += 62.0
    elif h >= 45:
        w_dwarf += 42.0
    elif h >= 33:
        w_dwarf += 16.0
    if any(g in (2, 3, 4, 5, 6, 7, 8, 23, 34, 57, 58, 59) for g in prod_good_ids):
        w_dwarf += 22.0

    # 2. 森林生態系 -> 精靈
    if biome in (6, 7, 8, 9):
        w_elf += 44.0 if h < 45 else 20.0
        if culture_type == "Generic" and h < 30:
            w_human += 18.0
    elif biome == 12:
        w_elf += 14.0
        w_human += 20.0

    # 3. 游牧／狩獵文化與荒漠草原苔原 -> 獸人
    if culture_type in ("Nomadic", "Hunting"):
        w_orc += 58.0
    if biome in (1, 2, 3, 10, 11):
        w_orc += 34.0
    elif biome == 4:
        if culture_type == "Nomadic":
            w_orc += 24.0
        else:
            w_human += 46.0

    # 4. 低海拔河谷與高農商開發 -> 人類
    if h < 32 and culture_type == "Generic" and biome not in (2, 10, 11):
        w_human += 32.0

    total = w_human + w_dwarf + w_elf + w_orc
    return {
        "人類": w_human / total,
        "矮人": w_dwarf / total,
        "精靈": w_elf / total,
        "獸人": w_orc / total,
    }


def compute_burg_race_pct(cell: dict, culture_type: str, burg: dict, deal_count: int) -> dict[str, int]:
    prod_ids = [int(p[0]) for p in burg.get("production", []) if isinstance(p, list) and p]
    base = compute_cell_base_race_weights(cell, culture_type, prod_ids)

    # 第二層：港口與跨國貿易單帶來的混居開放度（0.05 封閉內陸 ~ 0.42 跨海樞紐大港）
    openness = 0.06
    if burg.get("port"):
        openness += 0.22
    if burg.get("capital"):
        openness += 0.08
    openness += min(0.12, deal_count * 0.015)
    openness = min(0.45, openness)

    # 海港開放流特別帶入人類行商與獸人碼頭水手苦力、加上矮人百工與精靈藥師
    maritime_mix = {"人類": 0.44, "獸人": 0.30, "矮人": 0.16, "精靈": 0.10}
    blended = {
        k: (1.0 - openness) * base[k] + openness * maritime_mix[k]
        for k in ("人類", "矮人", "精靈", "獸人")
    }
    # 轉為整數百分比（總和 100）
    raw_ints = {k: int(round(v * 100)) for k, v in blended.items()}
    diff = 100 - sum(raw_ints.values())
    top_k = max(raw_ints, key=lambda k: blended[k])
    raw_ints[top_k] += diff
    return raw_ints


def generate_burg_name_zh(burg: dict, cell: dict, state_stem: str, state_orig_name: str, used_burgs: set[str]) -> str:
    bid = int(burg["i"])
    if bid in FIXED_BURG_NAMES:
        fixed = FIXED_BURG_NAMES[bid]
        used_burgs.add(fixed)
        return fixed

    h = float(cell.get("h", 20))
    biome = int(cell.get("biome", 4))
    cid = int(burg.get("culture", cell.get("culture", 0)) or 0)
    is_port = bool(burg.get("port"))
    grp = str(burg.get("group", "town"))
    prod_ids = [int(p[0]) for p in burg.get("production", []) if isinstance(p, list) and p]
    orig_bn = str(burg.get("name", ""))
    seed = det_u32(f"burg:{bid}:{orig_bn}")

    # 後綴依聚落性質決定
    if is_port:
        suffixes = ["港", "灣", "澳", "津", "塢", "岬"]
    elif grp == "fort" or (burg.get("citadel") and h >= 42):
        suffixes = ["堡", "塞", "關", "石堡", "壘"]
    elif grp == "monastery":
        suffixes = ["修院", "靜院", "鐘塔"]
    elif grp in ("caravanserai", "trading_post"):
        suffixes = ["驛", "商驛", "市集", "駝驛"]
    elif h >= 52:
        suffixes = ["爐堡", "礦城", "岩寨", "峰城", "石城"]
    elif grp in ("capital", "city"):
        suffixes = ["城", "堅城", "都", "要塞"]
    elif grp == "town":
        suffixes = ["鎮", "城", "市", "橋鎮", "谷鎮"]
    else:
        suffixes = ["村", "莊", "屯", "坡", "溪村", "林莊"]

    # 若為該國首府且原名與國名相同，直接沿用國家詞幹（如「佩德公國」首府即「佩德港／佩德城」），記憶零負擔！
    if burg.get("capital") and state_stem and orig_bn and orig_bn == state_orig_name:
        for suf in suffixes:
            if not state_stem.endswith(suf[0]):
                cand_cap = f"{state_stem}{suf}"
                if cand_cap not in used_burgs:
                    used_burgs.add(cand_cap)
                    return cand_cap

    for attempt in range(500):
        idx = seed + attempt * 29
        suf = suffixes[(idx >> 3) % len(suffixes)]

        # 決定詞幹：一半取風土物產意譯詞根，一半取短音譯詞根，確保好記且不重複
        use_scenic = ((idx + attempt) % 2 == 0) and attempt < 220
        if use_scenic:
            if prod_ids and prod_ids[0] in GOOD_SCENIC_PREFIX and attempt < 40:
                stem = GOOD_SCENIC_PREFIX[prod_ids[0]]
            elif is_port:
                stem = COAST_ROOTS[idx % len(COAST_ROOTS)]
            elif h >= 46:
                stem = HIGHLAND_ROOTS[idx % len(HIGHLAND_ROOTS)]
            elif biome in (6, 7, 8, 9):
                stem = SYLVAN_ROOTS[idx % len(SYLVAN_ROOTS)]
            else:
                stem = make_phonetic_stem(str(burg.get("name", "")), cid, f"b{bid}:{attempt}")
        else:
            stem = make_phonetic_stem(str(burg.get("name", "")), cid, f"bp{bid}:{attempt}")

        # 避免詞尾重複字（例如「錨灣」+「灣」-> 改「錨灣港」）
        if stem.endswith(suf[0]):
            suf = suffixes[(idx + 1) % len(suffixes)]
            if stem.endswith(suf[0]):
                continue

        cand = f"{stem}{suf}"
        if cand in FORBIDDEN_REAL_NAMES or len(cand) > 5:
            continue
        if cand not in used_burgs:
            used_burgs.add(cand)
            return cand

    fallback = f"{state_stem}第{bid}鎮"
    used_burgs.add(fallback)
    return fallback


# ─── 四、神祇稱號、信仰、地標與世界編年句子轉譯器 ───

DEITY_EPITHET_WORDS = {
    "Blind": "盲眼", "Blond": "金髮", "Bronze": "青銅", "Brown": "褐岩",
    "Cerberus": "冥犬", "Chief": "大酋", "Coral": "珊礁", "Council": "眾議",
    "Creator": "締造者", "Cruel": "冷酷", "Disturbing": "驚夢", "Divine": "聖耀",
    "Eternal": "永恆", "Fair": "澄明", "Fog": "迷霧", "Forefather": "先祖",
    "Frozen": "霜凍", "Golden": "黃金", "Happy": "歡愉", "Ice": "寒冰",
    "Infallible": "無誤", "Jaguar": "黑豹", "Last": "終末", "Latter": "後世",
    "Light": "晨光", "Loud": "雷鳴", "Lucky": "吉運", "Mad": "狂嘯",
    "Magenta": "絳紫", "Narwhal": "獨角鯨", "New": "新世", "Night": "暗夜",
    "One": "獨一者", "Otherworldly": "幽界", "Pain": "苦難", "Patient": "堅忍",
    "Peaceful": "靜謐", "Pegasus": "飛馬", "Pink": "緋霞", "Plum": "寒梅",
    "Pregnant": "孕育", "Prime": "原初", "Proud": "孤高", "Pure": "純淨",
    "Reaper": "收割者", "Red": "赤紅", "Sage": "賢者", "Sapphire": "蒼藍",
    "Secret": "秘隱", "Selected": "蒙選者", "Serene": "寧靜", "Seven": "七曜",
    "Sister": "姊神", "Sleeping": "沉眠者", "Spirit": "靈主", "Springs": "湧泉",
    "Summer": "仲夏", "Supreme": "至高", "Ten": "十方", "Time": "歲月",
    "Transcendent": "超脫者", "Transcendental": "超凡", "Unfathomable": "淵深莫測者",
    "Unicorn": "獨角獸", "Victory": "勝戰", "Viper": "蝮蛇", "Walrus": "海象",
    "Wise": "睿智", "Wolverine": "狼獾", "Yeti": "雪怪", "Ancient": "古靈",
    "Beaver": "河狸", "Bison": "野牛", "Cockatrice": "蛇雞獸", "Crane": "白鶴",
    "Death": "冥寂", "Deity": "神靈", "Father": "古父", "Fire": "烈火",
    "Gates": "門戶", "God": "古神", "Guide": "引路者", "Heaven": "蒼穹",
    "Life": "生機", "Underworld": "地淵", "Being": "本源",
}


def _ep_chunk_zh(chunk: str) -> str:
    words = re.sub(r"\b(The|the)\b", " ", chunk).split()
    return "".join(DEITY_EPITHET_WORDS.get(w, "") for w in words if DEITY_EPITHET_WORDS.get(w))


def translate_deity_zh(raw_deity: str | None, culture_id: int, rid: int) -> str | None:
    if not raw_deity:
        return None
    if rid == 37 or raw_deity.startswith("Shaoqing,"):
        return "淵潮古靈（深不可測者）"
    if "," in raw_deity:
        dname, ep = raw_deity.split(",", 1)
    else:
        dname, ep = raw_deity, ""
    d_key = dname.strip()
    d_zh = make_phonetic_stem(d_key, culture_id, f"deity:{d_key}")
    if " of " in ep:
        left_ep, right_ep = ep.split(" of ", 1)
        l_zh, r_zh = _ep_chunk_zh(left_ep), _ep_chunk_zh(right_ep)
        ep_zh = f"{r_zh}之{l_zh}" if (l_zh and r_zh) else (l_zh or r_zh)
    else:
        ep_zh = _ep_chunk_zh(ep)
    if ep_zh:
        return f"{d_zh}（{ep_zh}）"
    return d_zh


def translate_religion_name_zh(r: dict, culture_zh_map: dict[int, str], state_stem_by_token: dict[str, str]) -> str:
    rid = int(r["i"])
    if rid == 37:
        return "淵潮古道"
    raw_n = str(r.get("name", ""))
    cid = int(r.get("culture", 0) or 0)

    # 特殊固定詞組
    special = {
        "Religion of the Golden Beaver": "金狸古宗",
        "Religion of the Immutable Sheep": "磐羊古教",
        "Arcanum of the Peaceful": "靜謐秘儀派",
        "Heresy of the Noble Fire": "聖火異端派",
        "Sect of the Goddess": "古母秘教",
    }
    if raw_n in special:
        return special[raw_n]

    # 比對是否引用已知國名／文化詞根
    matched_stem = None
    body_n = raw_n[4:] if raw_n.startswith("Old ") else raw_n
    for tok, st_zh in state_stem_by_token.items():
        if len(tok) >= 4 and body_n.lower().startswith(tok.lower()[:4]):
            matched_stem = st_zh
            break
    for c_idx, c_en in [
        (1, "Koryo"), (2, "Hantzu"), (3, "Yamoto"), (4, "Turchian"), (5, "Amazi"),
        (6, "Eurabic"), (7, "Efratic"), (8, "Tehrani"), (9, "Maui"), (10, "Carnatic"),
        (11, "Vietic"), (12, "Guantzu"), (13, "Ulus"),
    ]:
        if c_en.lower()[:5] in body_n.lower():
            matched_stem = culture_zh_map[c_idx][:2]
            break

    if not matched_stem:
        matched_stem = make_phonetic_stem(body_n, cid, f"rel:{rid}")

    is_old = raw_n.startswith("Old ")
    prefix = "古" if is_old else ""
    if "Beliefs" in raw_n:
        suf = "舊俗" if is_old else "古俗"
    elif "Spirits" in raw_n:
        suf = "萬靈俗"
    elif "Ancestors" in raw_n:
        suf = "祖靈祀"
    elif "Pantheon" in raw_n or "Gods" in raw_n or "Deities" in raw_n:
        suf = "眾神祀"
    elif "Ethics" in raw_n:
        suf = "禮法派"
    elif "Commandments" in raw_n:
        suf = "戒律派"
    elif "Word of" in raw_n:
        suf = "聖言派"
    elif "Way of" in raw_n:
        suf = "舊道" if is_old else "古道"
    elif "Church" in raw_n:
        suf = "教會"
    elif "Apostates" in raw_n or "Dissenters" in raw_n or "Schism" in raw_n or "Heresy" in raw_n:
        suf = "離教派"
    elif "Sect" in raw_n or "Circle" in raw_n or "Society" in raw_n or "Order" in raw_n:
        suf = "秘社"
    elif "Mysticism" in raw_n or "Occultism" in raw_n:
        suf = "秘術派"
    elif "Faith" in raw_n:
        suf = "信眾派"
    else:
        suf = "宗派"

    return f"{prefix}{matched_stem}{suf}"


INN_NAMES_ZH = {
    "The Fair Tavern": "晴風客棧",
    "The Red Marten": "赤貂酒館",
    "The Dark Snake": "黑蛇酒館",
    "The Major Tavern": "大道酒館",
    "The Flying Inn": "飛羽旅店",
    "The Dark Lion": "黑獅酒館",
    "The Honest Tavern": "老實人酒館",
    "The Far Tavern": "遠途酒館",
    "The Huge Inn": "巨木大客棧",
    "The Light Lion": "白獅酒館",
    "The High Tavern": "高坡酒館",
    "The Golden Crane": "金鶴酒館",
    "The Superior Tavern": "上選酒館",
    "The Peaceful Tavern": "安平酒館",
}

MONTH_ZH = {
    "January": "一月", "February": "二月", "March": "三月", "April": "四月",
    "May": "五月", "June": "六月", "July": "七月", "August": "八月",
    "September": "九月", "October": "十月", "November": "十一月", "December": "十二月",
}

TRANSPORT_EN_TO_ZH = {
    "On foot (laden)": "徒步負重",
    "On foot (light)": "徒步輕裝",
    "Horseback (no spare horse)": "單騎趕路",
    "Horseback (spare horse)": "雙馬換乘急行",
    "Carriage": "雙馬篷車",
    "Stagecoach": "驛站快車",
    "Train": "軌車",
    "Automobile": "自走車",
    "Modern Automobile": "重型自走車",
    "Rowboat": "划槳小舟",
    "Sailing boat": "近岸單桅帆船",
    "Sailing Ship": "遠洋雙桅大帆船",
    "Steamship": "明輪大船",
    "Modern Ship": "遠洋巨舶",
    "Aircraft": "風翼飛舟",
    "Dirigible": "浮空飛艇",
    "Helicopter": "旋翼飛舟",
    "Modern Airplane": "高空飛艇",
    "Teleport": "瞬移陣",
    "Stay": "原地駐留",
}


def main() -> None:
    raw = json.loads(SRC_PATH.read_text(encoding="utf-8"))
    zh_doc = copy.deepcopy(raw)
    pack = zh_doc["pack"]
    cells = pack["cells"]
    cell_by_id = {c["i"]: c for c in cells if isinstance(c, dict) and "i" in c}

    # 1. 轉譯設定與紀元名稱（2026 NE -> 太陽曆 329 年／古星曆 2026 年對照）
    if "info" in zh_doc and isinstance(zh_doc["info"], dict):
        zh_doc["info"]["orig_mapName"] = zh_doc["info"].get("mapName", "")
        zh_doc["info"]["mapName"] = "蒼淵與銀鷗諸海"
    settings = zh_doc.get("settings", {})
    if isinstance(settings, dict) and "mapName" in settings:
        settings["orig_mapName"] = settings["mapName"]
        settings["mapName"] = "蒼淵與銀鷗諸海"
    map_opts = settings.get("options", {}).get("map", {})
    if "lore" in map_opts:
        map_opts["lore"]["orig_name"] = map_opts["lore"].get("name", "")
        map_opts["lore"]["name"] = "蒼淵與銀鷗諸海"
        map_opts["lore"]["orig_calendar"] = copy.deepcopy(map_opts["lore"].get("calendar", {}))
        map_opts["lore"]["calendar"] = {
            "year": 2026,
            "era": "古星曆",
            "eraShort": "古曆",
            "solarYear": 329,
        }
    if "military" in map_opts and "units" in map_opts["military"]:
        unit_zh_map = {
            "levies": "徵召民兵",
            "infantry": "重步兵",
            "archers": "弓弩手",
            "cavalry": "騎兵",
            "artillery": "重型扭力投石隊",
            "fleet": "風帆艦隊",
        }
        for u in map_opts["military"]["units"]:
            u["orig_name"] = u["name"]
            u["name"] = unit_zh_map.get(u["name"], u["name"])
    if "transports" in map_opts:
        for tr in map_opts["transports"]:
            tr["orig_name"] = tr["name"]
            tr["name"] = TRANSPORT_EN_TO_ZH.get(tr["name"], "車馬行旅")

    # 2. 轉譯 13 種生態系與 71 種貿易物產與 14 支文化
    for b in pack.get("biomes", []):
        if isinstance(b, dict) and "i" in b:
            b["orig_name"] = b["name"]
            b["name"] = BIOME_ZH.get(b["i"], b["name"])

    good_en_to_zh: dict[str, str] = {}
    for g in pack.get("goods", []):
        if isinstance(g, dict) and "i" in g:
            gi = int(g["i"])
            g["orig_name"] = g["name"]
            if gi in GOODS_ZH:
                zh_n, zh_u = GOODS_ZH[gi]
                good_en_to_zh[g["orig_name"].lower()] = zh_n
                g["name"] = zh_n
                g["orig_unit"] = g.get("unit", "")
                g["unit"] = zh_u

    orig_culture_type_by_id: dict[int, str] = {}
    for c in pack.get("cultures", []):
        if isinstance(c, dict) and "i" in c:
            ci = int(c["i"])
            orig_culture_type_by_id[ci] = str(c.get("type", "Generic"))
            c["orig_name"] = c["name"]
            c["name"] = CULTURE_ZH.get(ci, f"古裔{ci}")

    # 統計每座城鎮涉及的跨國貿易單數
    deals_per_burg: Counter[int] = Counter()
    for dl in pack.get("deals", []):
        if isinstance(dl, list) and len(dl) >= 2:
            deals_per_burg[int(dl[0])] += 1
            deals_per_burg[int(dl[1])] += 1

    # 3. 轉譯 100 國家政權詞幹與全稱（確保唯一，且建立英文詞根 -> 中文詞幹對照表）
    used_state_stems: set[str] = set(FIXED_STATE_STEMS.values())
    state_stem_zh: dict[int, str] = {}
    state_full_zh: dict[int, str] = {}
    en_token_to_zh: dict[str, str] = {}

    for s in pack.get("states", []):
        if not isinstance(s, dict) or not s.get("i") or s.get("removed"):
            if isinstance(s, dict) and s.get("i") == 0:
                s["orig_name"] = s.get("name", "Neutrals")
                s["name"] = "無主荒野"
            continue
        sid = int(s["i"])
        orig_n = str(s.get("name", ""))
        orig_full = str(s.get("fullName", orig_n))
        orig_form = str(s.get("formName") or s.get("form") or "Kingdom")
        cid = int(s.get("culture", 0) or 0)

        if sid in FIXED_STATE_STEMS:
            stem = FIXED_STATE_STEMS[sid]
            full = FIXED_STATE_FULL[sid]
        else:
            stem = make_phonetic_stem(orig_n, cid, f"state:{sid}", used=used_state_stems)
            form_zh = STATE_FORM_ZH.get(orig_form, "王國")
            full = f"{stem}{form_zh}"

        state_stem_zh[sid] = stem
        state_full_zh[sid] = full

        # 註冊英文原名與形容詞變化（如 Pinavia / Pinavian, Malapetia / Malapetian, Seryurt / Seryurtan）
        clean_en = orig_n.strip()
        if clean_en:
            en_token_to_zh[clean_en] = stem
            for suf in ("an", "ian", "ese", "ish", "i", "n"):
                en_token_to_zh[clean_en + suf] = stem
                if clean_en.endswith(("a", "e", "i", "o", "u")):
                    en_token_to_zh[clean_en[:-1] + suf] = stem
                if clean_en.endswith("ia"):
                    en_token_to_zh[clean_en[:-2] + "ian"] = stem
                    en_token_to_zh[clean_en[:-2] + "ese"] = stem
                if clean_en.endswith("y"):
                    en_token_to_zh[clean_en[:-1] + "ian"] = stem

        s["orig_name"] = orig_n
        s["orig_fullName"] = orig_full
        s["name"] = stem if sid != 26 else "銀鷗海領"
        s["formName"] = STATE_FORM_ZH.get(orig_form, "王國")
        s["fullName"] = full

    # 4. 轉譯 850 座城鎮聚落（100% 唯一，並同步算出每城四族比例；不覆寫國家詞幹表！）
    used_burgs: set[str] = set()
    burg_zh_by_id: dict[int, str] = {}
    burg_orig_to_zh: dict[str, str] = {}
    port_orig_to_zh: dict[str, str] = {}
    burg_races_by_id: dict[int, dict[str, int]] = {}

    for b in pack.get("burgs", []):
        if not isinstance(b, dict) or not b.get("i") or b.get("removed"):
            continue
        bid = int(b["i"])
        cid_cell = int(b.get("cell", 0))
        cell = cell_by_id.get(cid_cell, {"h": 22, "biome": 4, "culture": b.get("culture", 1)})
        sid = int(b.get("state", 0) or 0)
        st_stem = state_stem_zh.get(sid, "邊境")
        st_orig = pack["states"][sid].get("orig_name", "") if sid < len(pack["states"]) and isinstance(pack["states"][sid], dict) else ""
        orig_bn = str(b.get("name", ""))

        zh_bn = generate_burg_name_zh(b, cell, st_stem, st_orig, used_burgs)
        burg_zh_by_id[bid] = zh_bn
        stripped_b_stem = re.sub(
            r"(堅城|石堡|岩寨|要塞|林驛|渡口|漁港|北塢|橋鎮|谷鎮|溪村|林莊|港|灣|澳|津|塢|岬|城|市|鎮|村|莊|屯|堡|塞|關|驛|坡|都)$",
            "",
            zh_bn,
        )
        if len(stripped_b_stem) < 2:
            stripped_b_stem = zh_bn
        if orig_bn and orig_bn not in burg_orig_to_zh:
            burg_orig_to_zh[orig_bn] = zh_bn
        if orig_bn and b.get("port") and orig_bn not in port_orig_to_zh:
            port_orig_to_zh[orig_bn] = zh_bn
        if orig_bn and orig_bn not in en_token_to_zh:
            en_token_to_zh[orig_bn] = zh_bn
        if orig_bn:
            for suf in ("an", "ian", "ese", "ish", "n"):
                cand_k = orig_bn + suf
                if cand_k not in en_token_to_zh:
                    en_token_to_zh[cand_k] = stripped_b_stem
                if orig_bn.endswith(("a", "e", "i", "o", "u")):
                    cand_k2 = orig_bn[:-1] + suf
                    if cand_k2 not in en_token_to_zh:
                        en_token_to_zh[cand_k2] = stripped_b_stem

        cul_id = int(b.get("culture", cell.get("culture", 0)) or 0)
        cul_type = orig_culture_type_by_id.get(cul_id, "Generic")
        burg_races_by_id[bid] = compute_burg_race_pct(cell, cul_type, b, deals_per_burg[bid])

        b["orig_name"] = orig_bn
        b["name"] = zh_bn
        b["race_pct"] = burg_races_by_id[bid]

    # 計算各國整體四族比例（由轄下陸格與城鎮加權匯總）
    state_races_by_id: dict[int, dict[str, int]] = {}
    cells_by_state: dict[int, list[dict]] = {}
    for c in cells:
        if isinstance(c, dict) and c.get("h", 0) >= 20 and c.get("state"):
            cells_by_state.setdefault(int(c["state"]), []).append(c)

    for sid in state_stem_zh:
        scells = cells_by_state.get(sid, [])
        acc = {"人類": 0.0, "矮人": 0.0, "精靈": 0.0, "獸人": 0.0}
        if scells:
            for c in scells:
                ct = orig_culture_type_by_id.get(int(c.get("culture", 0) or 0), "Generic")
                w = compute_cell_base_race_weights(c, ct, [])
                for k in acc:
                    acc[k] += w[k]
            tot = sum(acc.values())
            pcts = {k: int(round(acc[k] * 100 / tot)) for k in acc}
            diff = 100 - sum(pcts.values())
            top_k = max(pcts, key=lambda k: acc[k])
            pcts[top_k] += diff
            state_races_by_id[sid] = pcts
        else:
            state_races_by_id[sid] = {"人類": 70, "矮人": 10, "精靈": 10, "獸人": 10}
        pack["states"][sid]["race_pct"] = state_races_by_id[sid]

    # 5. 轉譯 841 省分（若省治為已知城鎮則優先沿用省治詞根，否則產生唯一省名）
    used_provs: set[str] = set()
    prov_zh_by_id: dict[int, str] = {}
    for p in pack.get("provinces", []):
        if not isinstance(p, dict) or not p.get("i") or p.get("removed"):
            continue
        pid = int(p["i"])
        orig_pn = str(p.get("name", ""))
        orig_pform = str(p.get("formName", "Province"))
        pform_zh = PROV_FORM_ZH.get(orig_pform, "行省")
        bid = int(p.get("burg", 0) or 0)
        sid = int(p.get("state", 0) or 0)
        cid = int(pack["states"][sid].get("culture", 0)) if sid < len(pack["states"]) and isinstance(pack["states"][sid], dict) else 0

        if bid in burg_zh_by_id:
            # 取城鎮名稱作為省名核心（如「白鷗港鄉鎮轄區」->「白鷗轄區」）
            base_stem = re.sub(
                r"(堅城|石堡|岩寨|要塞|林驛|渡口|漁港|北塢|橋鎮|谷鎮|溪村|林莊|港|灣|澳|津|塢|岬|城|市|鎮|村|莊|屯|堡|塞|關|驛|坡|都)$",
                "",
                burg_zh_by_id[bid],
            )
            if len(base_stem) < 2:
                base_stem = burg_zh_by_id[bid]
        else:
            base_stem = make_phonetic_stem(orig_pn, cid, f"prov:{pid}")

        full_p = f"{base_stem}{pform_zh}"
        if full_p in used_provs:
            extra = make_phonetic_stem(orig_pn, cid, f"prov_alt:{pid}")
            base_stem = extra
            full_p = f"{base_stem}{pform_zh}"
        used_provs.add(full_p)
        prov_zh_by_id[pid] = full_p

        p["orig_name"] = orig_pn
        p["orig_fullName"] = p.get("fullName", "")
        p["name"] = base_stem
        p["formName"] = pform_zh
        p["fullName"] = full_p

    # 6. 轉譯 192 條河川水系
    used_rivers: set[str] = set()
    river_orig_to_zh: dict[str, str] = {}
    for rv in pack.get("rivers", []):
        if not isinstance(rv, dict) or not rv.get("i"):
            continue
        rid = int(rv["i"])
        orig_rn = str(rv.get("name", ""))
        orig_rt = str(rv.get("type", "River"))
        rt_zh = RIVER_TYPE_ZH.get(orig_rt, "河")
        rcells = rv.get("cells", [])
        first_c = cell_by_id.get(rcells[0], {}) if rcells else {}
        cid = int(first_c.get("culture", 0) or 0)

        stem = make_phonetic_stem(orig_rn, cid, f"riv:{rid}", used=None)
        full_rv = f"{stem}{rt_zh}"
        if full_rv in used_rivers:
            stem = make_phonetic_stem(orig_rn, cid, f"riv2:{rid}", used=None)
            full_rv = f"{stem}{rt_zh}"
        used_rivers.add(full_rv)
        if orig_rn:
            river_orig_to_zh[orig_rn] = stem
            river_orig_to_zh[f"{orig_rn} {orig_rt}"] = full_rv

        rv["orig_name"] = orig_rn
        rv["orig_type"] = orig_rt
        rv["name"] = stem
        rv["type"] = rt_zh

    # 7. 轉譯 76 門民間信仰與學派
    rel_orig_to_zh: dict[str, str] = {}
    REL_TYPE_ZH = {"Folk": "民間古俗", "Organized": "制度教派", "Cult": "秘儀宗", "Heresy": "離教支派"}
    REL_FORM_ZH = {
        "Shamanism": "薩滿通靈", "Animism": "萬物有靈", "Ancestor Worship": "祖靈崇祀",
        "Nature Worship": "自然古祀", "Polytheism": "多神祀典", "Monotheism": "一神教義",
        "Dualism": "雙元教義", "Pantheism": "泛靈論", "Non-theism": "無神哲思",
        "Deism": "自然神論", "Ethical": "倫理禮法", "Philosophical": "哲人學派",
        "Syncretism": "諸教合流", "Cult": "秘儀結社", "Dark Cult": "暗夜秘社",
    }
    used_rels: set[str] = set()
    for r in pack.get("religions", []):
        if not isinstance(r, dict) or not r.get("i") or r.get("removed"):
            if isinstance(r, dict) and r.get("i") == 0:
                r["orig_name"] = r.get("name", "No religion")
                r["name"] = "無特定信仰"
            continue
        rid = int(r["i"])
        orig_rn = str(r.get("name", ""))
        cid = int(r.get("culture", 0) or 0)
        zh_rn = translate_religion_name_zh(r, CULTURE_ZH, en_token_to_zh)
        if zh_rn in used_rels:
            zh_rn = f"{make_phonetic_stem(orig_rn, cid, f'rel_u:{rid}')}{zh_rn[-2:]}"
        used_rels.add(zh_rn)
        if orig_rn:
            rel_orig_to_zh[orig_rn] = zh_rn

        r["orig_name"] = orig_rn
        r["name"] = zh_rn
        if r.get("deity"):
            r["orig_deity"] = r["deity"]
            r["deity"] = translate_deity_zh(r["deity"], cid, rid)
        if r.get("type") in REL_TYPE_ZH:
            r["orig_type"] = r["type"]
            r["type"] = REL_TYPE_ZH[r["type"]]
        if r.get("form") in REL_FORM_ZH:
            r["orig_form"] = r["form"]
            r["form"] = REL_FORM_ZH[r["form"]]

    # 8. 轉譯 506 場戰役（campaigns）與 146 支常備軍團（military）
    camp_orig_to_zh: dict[str, str] = {}
    for s in pack.get("states", []):
        if not isinstance(s, dict) or not s.get("i") or s.get("removed"):
            continue
        sid = int(s["i"])
        st_stem = state_stem_zh.get(sid, "邊境")

        for cp in s.get("campaigns", []):
            orig_cn = str(cp.get("name", ""))
            cp["orig_name"] = orig_cn
            atk = int(cp.get("attacker", 0) or 0)
            dfd = int(cp.get("defender", 0) or 0)
            last_w = orig_cn.split()[-1] if orig_cn.split() else "War"
            suf_zh = CAMPAIGN_SUFFIX_ZH.get(last_w, "戰役")

            if sid == 26 and orig_cn == "Yaowuan Invasion":
                zh_cn = "銀鷗島征荒之役"
            elif "-" in orig_cn.split()[0]:
                a_stem = state_stem_zh.get(atk, st_stem)
                d_stem = "荒野散部" if dfd == 0 else state_stem_zh.get(dfd, make_phonetic_stem(orig_cn, 0, "cpd"))
                zh_cn = f"{a_stem}－{d_stem}{suf_zh}"
            elif dfd == 0:
                a_stem = state_stem_zh.get(atk, st_stem)
                zh_cn = f"{a_stem}征荒之役"
            else:
                other_id = dfd if atk == sid else atk
                o_stem = state_stem_zh.get(other_id) if other_id in state_stem_zh else make_phonetic_stem(orig_cn, 0, f"cp:{orig_cn}")
                zh_cn = f"{o_stem}{suf_zh}"

            cp["name"] = zh_cn
            camp_orig_to_zh[orig_cn] = zh_cn

        for reg in s.get("military", []):
            orig_mn = str(reg.get("name", ""))
            orig_note = clean_surrogates(str(reg.get("note", "")))
            reg["orig_name"] = orig_mn
            reg["orig_note"] = orig_note

            m_cell = int(reg.get("cell", 0) or 0)
            c_obj = cell_by_id.get(m_cell, {})
            p_id = int(c_obj.get("province", 0) or 0)
            b_id = int(c_obj.get("burg", 0) or 0)
            loc_zh = burg_zh_by_id.get(b_id) or (prov_zh_by_id.get(p_id, st_stem)[:2])
            num_m = re.match(r"(\d+)(?:st|nd|rd|th)", orig_mn)
            ord_num = num_m.group(1) if num_m else str(int(reg.get("n", 0)) + 1)
            is_fleet = "Fleet" in orig_mn or reg.get("u", {}).get("fleet")
            zh_mn = f"{st_stem}第{ord_num}（{loc_zh}）{'艦隊' if is_fleet else '軍團'}"
            reg["name"] = zh_mn

            # 轉譯軍團歷史駐防備註
            u_map = reg.get("u", {})
            comp_parts = []
            u_labels = [
                ("infantry", "重步兵", 1000),
                ("archers", "弓弩手", 1000),
                ("cavalry", "騎兵", 1000),
                ("artillery", "重型扭力投石隊", 1000),
                ("levies", "徵召民兵", 1000),
                ("fleet", "戰船", 1),
            ]
            for uk, ulab, mult in u_labels:
                if uk in u_map and u_map[uk]:
                    cnt = max(1, int(round(float(u_map[uk]) * mult)))
                    comp_parts.append(f"{ulab} {cnt} {'艘' if uk == 'fleet' else '人'}")

            yr_m = re.search(r"in (\d+) Nostow Era during the ([^.]+)\.", orig_note)
            if yr_m:
                yr_val = yr_m.group(1)
                cp_en = yr_m.group(2).strip()
                cp_zh = camp_orig_to_zh.get(cp_en, f"{st_stem}戰役")
                origin_str = f"於古星曆 {yr_val} 年【{cp_zh}】期間成軍，常駐【{loc_zh}】。"
            else:
                origin_str = f"{st_stem}常備駐防部隊，駐紮於【{loc_zh}】。"

            comp_str = "、".join(comp_parts) if comp_parts else "常備戍衛編制"
            reg["note"] = f"{origin_str}古星曆 2026 年編制：{comp_str}。"

    # 9. 轉譯 states[0].diplomacy（12 場全大陸跨國大戰編年史）
    sorted_en_tokens = sorted(en_token_to_zh.keys(), key=len, reverse=True)

    def lookup_entity_zh(en_phrase: str) -> str:
        p = en_phrase.strip()
        if p in en_token_to_zh:
            return en_token_to_zh[p]
        for tok in sorted_en_tokens:
            if len(tok) >= 4 and (p.lower() == tok.lower() or p.lower().startswith(tok.lower())):
                return en_token_to_zh[tok]
        return make_phonetic_stem(p, 0, f"unk:{p}")

    if pack.get("states") and isinstance(pack["states"][0], dict) and "diplomacy" in pack["states"][0]:
        orig_chron = pack["states"][0]["diplomacy"]
        pack["states"][0]["orig_diplomacy"] = copy.deepcopy(orig_chron)
        zh_chron = []
        for row in orig_chron:
            if not isinstance(row, list) or not row:
                continue
            new_row = []
            # 首項為戰爭名稱（例如 'Koaiakia-Kuhuanian War'）
            wtitle = str(row[0])
            if wtitle in camp_orig_to_zh:
                new_row.append(camp_orig_to_zh[wtitle])
            elif "-" in wtitle:
                left, right_rest = wtitle.split("-", 1)
                right = right_rest.replace(" War", "").strip()
                new_row.append(f"{lookup_entity_zh(left)}－{lookup_entity_zh(right)}大戰")
            else:
                new_row.append(f"{lookup_entity_zh(wtitle.replace(' War', ''))}戰爭")

            for line in row[1:]:
                sline = str(line)
                m1 = re.match(r"^(.+) declared a war on its rival (.+)$", sline)
                m2 = re.match(r"^Frightened by (.+), (.+) severed the defense pact with (.+)$", sline)
                m3 = re.match(r"^Being already at war, (.+) severed the defense pact with (.+)$", sline)
                m4 = re.match(r"^(.+)'s (ally|vassal) (.+) joined the war on (attackers|defenders) side$", sline)
                m5 = re.match(r"^(.+)'s (ally|vassal) (.+) avoided entering the war$", sline)
                if m1:
                    new_row.append(f"{lookup_entity_zh(m1.group(1))}向宿敵{lookup_entity_zh(m1.group(2))}宣戰")
                elif m2:
                    new_row.append(f"震懾於{lookup_entity_zh(m2.group(1))}兵威，{lookup_entity_zh(m2.group(2))}撕毀了與{lookup_entity_zh(m2.group(3))}的協防盟約")
                elif m3:
                    new_row.append(f"因自身已陷入戰火，{lookup_entity_zh(m3.group(1))}解除了與{lookup_entity_zh(m3.group(2))}的協防盟約")
                elif m4:
                    rel_w = "盟邦" if m4.group(2) == "ally" else "附庸邦"
                    side_w = "攻方" if m4.group(4) == "attackers" else "守方"
                    new_row.append(f"{lookup_entity_zh(m4.group(1))}的{rel_w}{lookup_entity_zh(m4.group(3))}投入{side_w}參戰")
                elif m5:
                    rel_w = "盟邦" if m5.group(2) == "ally" else "附庸邦"
                    new_row.append(f"{lookup_entity_zh(m5.group(1))}的{rel_w}{lookup_entity_zh(m5.group(3))}按兵不動、規避參戰")
                else:
                    new_row.append(f"{lookup_entity_zh(sline)}捲入戰局")
            zh_chron.append(new_row)
        pack["states"][0]["diplomacy"] = zh_chron

    # 10. 轉譯 14 處戰區／災疫區（pack.zones）
    ZONE_TYPE_ZH = {
        "Invasion": "戰火侵攻區", "Crusade": "宗教討伐區", "Disease": "疫病蔓延區",
        "Disaster": "地動災區", "Eruption": "火山噴發區", "Avalanche": "雪崩封山區",
        "Fault": "地裂斷層帶", "Tsunami": "海嘯衝擊岸", "Famine": "歉收飢荒區",
    }
    for z in pack.get("zones", []):
        if not isinstance(z, dict) or "name" not in z:
            continue
        orig_zn = str(z["name"])
        orig_zt = str(z.get("type", ""))
        orig_znote = clean_surrogates(str(z.get("note", "")))
        z["orig_name"] = orig_zn
        z["orig_type"] = orig_zt
        z["orig_note"] = orig_znote
        z["type"] = ZONE_TYPE_ZH.get(orig_zt, "特殊區域")

        if orig_zn == "Scarlet Fever":
            p_m = re.search(r"port of ([A-Za-z' -]+),", orig_znote)
            port_zh = port_orig_to_zh.get(p_m.group(1).strip(), lookup_entity_zh(p_m.group(1))) if p_m else "外港"
            z["name"] = "猩紅熱疫區"
            z["note"] = f"由遠洋船隻帶入【{port_zh}】的猩紅熱疫情，正隨往來行商與旅人沿幹道擴散。"
        elif orig_zn == "Deadly Fever":
            p_m = re.search(r"port of ([A-Za-z' -]+),", orig_znote)
            port_zh = port_orig_to_zh.get(p_m.group(1).strip(), lookup_entity_zh(p_m.group(1))) if p_m else "南岸漁港"
            z["name"] = "南岸船疫熱病區"
            z["note"] = f"由外海船隻傳入【{port_zh}】的惡性熱病，隨商旅沿島內道路蔓延，各鎮草藥需求大增。"
        elif orig_zn == "Lethal Cough":
            p_m = re.search(r"port of ([A-Za-z' -]+),", orig_znote)
            port_zh = port_orig_to_zh.get(p_m.group(1).strip(), lookup_entity_zh(p_m.group(1))) if p_m else "海港"
            z["name"] = "血咳疫區"
            z["note"] = f"自【{port_zh}】靠岸船隻傳開的重症咳疫，正順著商道向內陸蔓延。"
        elif orig_zt == "Invasion":
            m_inv = re.search(r"Troops of (.+) have crossed into (.+) in the ", orig_znote)
            a_zh = lookup_entity_zh(m_inv.group(1)) if m_inv else lookup_entity_zh(orig_zn.split()[0])
            d_zh = lookup_entity_zh(m_inv.group(2)) if m_inv else "鄰邦"
            z["name"] = f"{a_zh}侵攻戰區"
            z["note"] = f"{a_zh}大軍越界攻入{d_zh}邊境富庶地帶，劫掠圍城並佔領要地，沿途百姓紛紛逃難。"
        elif orig_zt == "Crusade":
            m_cr = re.match(r"^(.+) is condemned as heresy\.", orig_znote)
            her_zh = rel_orig_to_zh.get(m_cr.group(1), lookup_entity_zh(orig_zn.split()[0]) + "離教派") if m_cr else "異端宗派"
            z["name"] = f"討伐{her_zh}遠征區"
            z["note"] = f"【{her_zh}】被斥為異端，討伐軍四處搜剿信眾並焚毀其祭壇祠堂。"
        elif orig_zt == "Disaster":
            loc_zh = lookup_entity_zh(orig_zn.split()[0])
            z["name"] = f"{loc_zh}強震帶"
            z["note"] = f"劇烈地動重創【{loc_zh}】及周邊鄉鎮，震塌石木屋舍並阻斷多處山道。"
        elif orig_zt == "Eruption":
            loc_zh = lookup_entity_zh(orig_zn.split()[0])
            z["name"] = f"{loc_zh}火山噴發帶"
            z["note"] = f"【{loc_zh}火山】猛烈噴發，火山灰與熔岩覆蓋山麓，山腳居民被迫遷離。"
        elif orig_zt == "Avalanche":
            loc_zh = lookup_entity_zh(orig_zn.split()[0])
            z["name"] = f"{loc_zh}雪崩封隘區"
            z["note"] = f"大雪崩掩埋了此處高山隘口，在積雪清通前車馬無法通行。"
        elif orig_zt == "Fault":
            loc_zh = lookup_entity_zh(orig_zn.split()[0])
            z["name"] = f"{loc_zh}地裂斷層"
            z["note"] = f"地面常年龜裂震動的深層斷層帶，極少有人敢在此處築屋定居。"
        elif orig_zt == "Tsunami":
            loc_zh = lookup_entity_zh(orig_zn.split()[0])
            z["name"] = f"{loc_zh}巨浪災岸"
            z["note"] = f"外海狂暴巨浪拍碎沿岸港棧並淹沒低地漁村，岸邊滿布船隻殘骸。"
        elif orig_zt == "Famine":
            loc_zh = lookup_entity_zh(orig_zn.split()[0])
            z["name"] = f"{loc_zh}荒年飢饉區"
            z["note"] = f"【{loc_zh}】貧瘠鄉野連年歉收，百姓無錢向市集購糧，飢荒蔓延。"
        else:
            z["name"] = f"{lookup_entity_zh(orig_zn)}異象區"
            z["note"] = "此區域近期發生劇烈地貌或人文變故。"

    # 11. 轉譯 96 處世界奇觀與地標（pack.markers，含完整中文說明與損壞字元清理）
    for m in pack.get("markers", []):
        if not isinstance(m, dict) or "name" not in m:
            continue
        orig_mn = clean_surrogates(str(m.get("name", "")))
        orig_mt = str(m.get("type", ""))
        orig_note = clean_surrogates(str(m.get("note", "")))
        m_cell = int(m.get("cell", 0) or 0)
        c_obj = cell_by_id.get(m_cell, {})
        b_id = int(c_obj.get("burg", 0) or 0)
        b_zh = burg_zh_by_id.get(b_id)
        cid = int(c_obj.get("culture", 0) or 0)

        m["orig_name"] = orig_mn
        m["orig_note"] = orig_note

        if orig_mt == "guild-halls":
            # 例：'Bovakapa Jewelry Guild'
            gm = re.match(r"^(.+)\s+([A-Za-z ]+)\s+Guild$", orig_mn)
            place_zh = b_zh or (lookup_entity_zh(gm.group(1)) if gm else make_phonetic_stem(orig_mn, cid, "gh"))
            g_en = gm.group(2).strip().lower() if gm else "craft"
            g_zh = good_en_to_zh.get(g_en, "百工")
            m["name"] = f"{place_zh}{g_zh}總行會"
            m["note"] = f"【{place_zh}】以精製{g_zh}聞名遠近，行會總堂掌握獨門工藝秘傳，連外邦市集的行情皆受其匠師左右。"
        elif orig_mt == "volcanoes":
            stem = lookup_entity_zh(orig_mn.replace(" Volcano", ""))
            m["name"] = f"{stem}火山"
            m["note"] = "高聳入雲的休眠巨型火山，峰頂終年積雪，山腹裂隙隱隱透出硫磺地熱。"
        elif orig_mt == "hot-springs":
            stem = make_phonetic_stem(orig_mn, cid, f"mk:{m['i']}")
            m["name"] = f"{stem}地熱溫泉"
            m["note"] = "終年湧出溫潤泉水的天然地熱山谷，水溫宜人，過路商旅常在此浸泡舒緩筋骨疲勞。"
        elif orig_mt == "water-sources":
            stem = make_phonetic_stem(orig_mn, cid, f"mk:{m['i']}")
            m["name"] = f"{stem}微光靈泉"
            m["note"] = "古老傳聞中記載的澄澈湧泉，即使在幽暗夜色下泉水仍泛著淡淡微光。"
        elif orig_mt == "mines":
            mm = re.search(r"([a-z ]+) mining town", orig_mn.lower())
            g_en = mm.group(1).strip() if mm else "iron"
            g_zh = good_en_to_zh.get(g_en, "礦石")
            pop_m = re.search(r"of (\d+) people", orig_note)
            pop_s = pop_m.group(1) if pop_m else "數千"
            place_zh = b_zh or make_phonetic_stem(orig_mn.split("—")[0], cid, f"mk:{m['i']}")
            m["name"] = f"{place_zh}・{g_zh}礦場"
            m["note"] = f"緊鄰大型{g_zh}礦脈興起的採礦聚落（約 {pop_s} 口礦工與家眷聚居於此）。"
        elif orig_mt == "bridges":
            rv_m = re.search(r"over the ([A-Za-z' ]+) River", orig_note)
            rv_zh = (river_orig_to_zh.get(rv_m.group(1).strip(), make_phonetic_stem(rv_m.group(1), cid, "brv")) + "河") if rv_m else "大河"
            place_zh = b_zh or make_phonetic_stem(orig_mn.replace(" Bridge", ""), cid, f"mk:{m['i']}")
            m["name"] = f"{place_zh}古石橋"
            m["note"] = f"橫跨【{rv_zh}】的堅固石拱大橋，自破曉至暮色皆有滿載貨車與駝隊絡繹過橋趕集。"
        elif orig_mt == "merchant-houses":
            fam_zh = make_phonetic_stem(orig_mn.replace("House of ", ""), cid, f"mk:{m['i']}")
            place_zh = b_zh or "大城"
            m["name"] = f"{fam_zh}大商號"
            m["note"] = f"發跡於【{place_zh}】的豪商世家，旗下掌櫃與行商遍布各大市集，往來大宗貨單多經其手。"
        elif orig_mt == "inns":
            mid_val = m["i"]
            m["name"] = INN_NAMES_ZH.get(orig_mn, f"{make_phonetic_stem(orig_mn, cid, f'inn:{mid_val}')}路邊酒館")
            m["note"] = "幹道旁遠近馳名的大客棧，長桌邊總聚滿歇腳車夫與行商，供應熱騰騰的烤肉與在地佳釀。"
        elif orig_mt == "lighthouses":
            stem = b_zh or make_phonetic_stem(orig_mn.replace(" Lighthouse", ""), cid, f"mk:{m['i']}")
            m["name"] = f"{stem}望海燈塔"
            m["note"] = "矗立於險峻海岬上的石砌高塔，夜裡燃起油脂長明火，為外海迷航帆船指引礁石方位。"
        elif orig_mt == "waterfalls":
            stem = make_phonetic_stem(orig_mn.replace(" Waterfall", ""), cid, f"mk:{m['i']}")
            m["name"] = f"{stem}飛瀑"
            m["note"] = "自斷崖奔騰直下的壯闊瀑布，水霧聲在數里外清晰可聞。"
        elif orig_mt == "battlefields":
            stem = make_phonetic_stem(orig_mn.replace(" Battlefield", ""), cid, f"mk:{m['i']}")
            cp_m = re.search(r"battle of the ([^.]+)\.", orig_note)
            yr_m = re.search(r"([A-Za-z]+) (\d+), (\d+) Nostow Era", orig_note)
            cp_zh = camp_orig_to_zh.get(cp_m.group(1).strip(), "昔日大戰") if cp_m else "昔日大戰"
            if yr_m:
                dt_zh = f"古星曆 {yr_m.group(3)} 年{MONTH_ZH.get(yr_m.group(1), '')}{yr_m.group(2)}日"
            else:
                dt_zh = "古星曆年間"
            m["name"] = f"{stem}古戰場"
            m["note"] = f"【{cp_zh}】留下的慘烈古戰場（{dt_zh}），荒草泥土間仍偶見殘破兵甲。"
        elif orig_mt == "dungeons":
            m["name"] = f"未探古地城（{m_cell}號石窟）"
            m["note"] = "掩藏於荒野地底的古老石窟遺跡，深處結構與盤踞之物尚無人摸清。"
        elif orig_mt == "sea-monsters":
            stem = make_phonetic_stem(orig_mn.replace(" Monster", ""), cid, f"mk:{m['i']}")
            m["name"] = f"{stem}深海巨獸出沒處"
            m["note"] = "老水手口耳相傳的凶險海域，據說水下盤踞著身長逾七丈的翻浪海獸。"
        elif orig_mt == "hill-monsters":
            stem = make_phonetic_stem(orig_mn.replace(" Cyclops", ""), cid, f"mk:{m['i']}")
            m["name"] = f"{stem}獨眼山怪嶺"
            m["note"] = f"盤踞在【{stem}丘陵】一帶的魁梧獨眼異獸，常襲擾落單過路行旅。"
        elif orig_mt in ("sacred-forests", "sacred-pineries", "sacred-palm-groves"):
            stem = make_phonetic_stem(orig_mn.split()[0], cid, f"mk:{m['i']}")
            kind_w = "古松聖林" if orig_mt == "sacred-pineries" else ("棕櫚聖林" if orig_mt == "sacred-palm-groves" else "古木聖林")
            m["name"] = f"{stem}{kind_w}"
            m["note"] = "受在地古老信仰敬畏守護的原始林地，外人不得擅自伐木驚擾。"
        elif orig_mt == "brigands":
            stem = make_phonetic_stem(orig_mn.split()[0], cid, f"mk:{m['i']}")
            beast_map = {"Jaguars": "黑豹幫", "Falcons": "蒼鷹幫", "Tigers": "猛虎寨", "Spiders": "毒蛛幫"}
            b_suf = beast_map.get(orig_mn.split()[-1], "山賊團")
            m["name"] = f"{stem}{b_suf}"
            m["note"] = "出沒於荒野幹道與林間隘口的強盜團夥，專盯無護衛押運的落單貨車。"
        elif orig_mt == "pirates":
            m["name"] = "黑帆海盜出沒海域"
            m["note"] = "常有劫掠商船的海盜快船在此片水道出沒。"
        elif orig_mt == "statues":
            stem = make_phonetic_stem(orig_mn.split()[0], cid, f"mk:{m['i']}")
            kind_w = "古符文石碑" if "Runestone" in orig_mn else ("古石像" if "Statue" in orig_mn else "古紀念方尖碑")
            m["name"] = f"{stem}{kind_w}"
            m["note"] = "矗立於荒野的前代石刻遺跡，表面鑿滿無人能直接通讀的遠古銘文，可拓印回城請博學匠師辨認。"
        elif orig_mt == "ruins":
            kind_w = "古要塞廢墟" if "Fort" in orig_mn else "古祭壇廢墟"
            m["name"] = f"傾頹的{kind_w}"
            m["note"] = "早年荒廢傾頹的石砌遺跡，人煙散盡後已為荒氣與未知生物盤踞，深處或留有前人舊物。"
        elif orig_mt == "libraries":
            stem = b_zh or lookup_entity_zh(orig_mn.replace(" Library", ""))
            m["name"] = f"{stem}古藏書閣"
            m["note"] = "收藏大量前代海圖、地方志與罕見古籍抄本的石造藏書樓。"
        elif orig_mt == "circuses":
            m["name"] = "巡迴百戲雜技團"
            m["note"] = "沿各城巡迴演出的熱鬧馬戲百戲帳篷，只在當地短暫停留數日。"
        elif orig_mt == "jousts":
            place_zh = b_zh or lookup_entity_zh(orig_mn.split()[0])
            m["name"] = f"{place_zh}武藝演武會"
            m["note"] = f"各路好手齊聚【{place_zh}】切磋兵刃武藝的演武集會，勝者可獲豐厚賞金與名聲。"
        elif orig_mt == "fairs":
            place_zh = b_zh or lookup_entity_zh(orig_mn.split()[0])
            m["name"] = f"{place_zh}歲時大市集"
            m["note"] = f"於【{place_zh}】舉辦的盛大定期市集，匯聚在地佳釀、香膏與外邦奇貨。"
        elif orig_mt == "customs-houses":
            cm = re.search(r"house of (.+) on the road from (.+?)\.", orig_note)
            owner_zh = lookup_entity_zh(cm.group(1)) if cm else "邊境"
            from_zh = lookup_entity_zh(cm.group(2)) if cm else "鄰邦"
            gate_zh = b_zh or make_phonetic_stem(orig_mn.split()[0], cid, f"mk:{m['i']}")
            m["name"] = f"{gate_zh}邊境稅關"
            m["note"] = f"【{owner_zh}】設於通往【{from_zh}】幹道上的關卡稅所，過往貨車皆須在此過磅查驗並繳納關稅。"
        elif orig_mt == "canoes":
            m["name"] = "河畔小渡津"
            m["note"] = "設於大河岸邊的簡易木棧渡口，神色疲憊的老船夫在此擺渡過河旅人。"
        elif orig_mt == "migration":
            m["name"] = "荒原駝群遷徙帶"
            m["note"] = "成百上千頭雙峰野駝正沿荒原水草帶浩蕩遷徙。"
        elif orig_mt == "mirage":
            m["name"] = "迷途海市蜃樓"
            m["note"] = "荒漠熱浪折射出的虛幻水城倒影，常引誘缺水旅人偏離正道。"
        elif orig_mt == "caves":
            stem = make_phonetic_stem(orig_mn.split()[0], cid, f"mk:{m['i']}")
            m["name"] = f"{stem}無底深淵"
            m["note"] = f"深不見底的天然地裂岩窟，在地人傳言其下連通幽深地脈，至今無人探盡。"
        elif orig_mt == "necropolises":
            stem = make_phonetic_stem(orig_mn.split()[0], cid, f"mk:{m['i']}")
            m["name"] = f"{stem}古墓群"
            m["note"] = "終年陰冷幽暗的遠古石室墓群，蜿蜒甬道間迴盪著低沉風聲。"
        elif orig_mt == "encounters":
            m["name"] = f"荒野行旅奇遇點（{m_cell}號路段）"
            m["note"] = "常有獨行旅人、游方匠師或異族商隊在此路段歇腳交會。"
        elif orig_mt == "party":
            m["name"] = "遠行探險隊營地"
            m["note"] = "一支結伴遠行的探險隊伍近期在此紮營休整。"
        else:
            mid_val = m["i"]
            m["name"] = f"{make_phonetic_stem(orig_mn, cid, f'mk:{mid_val}')}地標"
            m["note"] = "記載於地方古圖上的特殊地景。"

    # 12. 轉譯 9 大海洋、大陸、湖泊與島嶼（pack.features）
    FEATURE_FIXED_ZH = {
        1: "寧靜大洋",
        2: "蒼淵主陸",
        3: "澄鏡湖",
        4: "嵐礁嶼",
        5: "青石嶼",
        6: "銀鷗島",
        7: "孤帆嶼",
        8: "霧潮島",
        9: "白礁嶼",
    }
    named_features_count = 0
    for ft in pack.get("features", []):
        if not isinstance(ft, dict) or "name" not in ft:
            continue
        fid = int(ft.get("i", 0))
        orig_fn = clean_surrogates(str(ft["name"]))
        ft["orig_name"] = orig_fn
        if fid in FEATURE_FIXED_ZH:
            ft["name"] = FEATURE_FIXED_ZH[fid]
        else:
            suf_f = "大洋" if ft.get("type") == "ocean" else ("湖" if ft.get("type") == "lake" else "島")
            ft["name"] = f"{make_phonetic_stem(orig_fn, 0, f'ft:{fid}')}{suf_f}"
        named_features_count += 1

    # 13. 轉譯 326 條具名陸路驛道、山林棧道與海上航線（pack.routes）
    # 重新排序 en_token_to_zh 以納入城鎮衍生詞根（如 Ruizhouan -> 白鷗）
    sorted_en_tokens = sorted(en_token_to_zh.keys(), key=len, reverse=True)
    ROUTE_ADJ_ZH = {
        "serpentine": "盤蛇", "winding": "蜿蜒", "sacred": "聖林", "silent": "靜謐",
        "echoing": "迴音", "misty": "迷霧", "cracked": "裂石", "shrouded": "幽霧",
        "glowing": "微光", "luminous": "澄光", "glimmering": "星爍", "crimson": "緋紅",
        "scarlet": "赤紅", "azure": "蒼藍", "sapphire": "碧海", "infernal": "焦岩",
        "molten": "熔火", "flame": "赤焰", "ember": "餘燼", "frozen": "寒冰",
        "frost": "霜雪", "stardust": "星塵", "celestial": "星穹", "solar": "日耀",
        "lunar": "月華", "moon": "月影", "shadow": "暗影", "shadowy": "幽影",
        "spectral": "迷離", "ghost": "孤魂", "wraith": "荒魂", "eldritch": "古秘",
        "ethereal": "空靈", "dragon": "蒼龍", "phoenix": "赤凰", "falcon": "蒼鷹",
        "eagle": "飛鷲", "raven": "寒鴉", "spike": "尖石", "thorn": "荊棘",
        "jade": "翠玉", "amethyst": "紫晶", "golden": "金輝", "silvered": "銀輝",
        "platinum": "白銀", "iron": "黑鐵", "imperial": "王家", "royal": "王室",
        "queen": "王后", "military": "軍屯", "verdant": "翠微", "whispering": "風語",
        "tranquil": "靜水", "everlasting": "長生", "velvet": "絨苔", "paved": "石板",
        "cobbled": "卵石", "rustic": "古樸", "breezy": "海風", "obscure": "隱霧",
        "fading": "殘陽", "twisted": "盤曲", "shaky": "險棧", "titan": "巨岩",
        "flower": "繁花", "divine": "神祀", "enchanted": "迷蹤", "forest": "林間",
        "old": "古", "new": "新", "great": "大", "long": "長",
    }
    ROUTE_TAIL_BY_GROUP = {
        "roads": {
            "highway": "馳道", "road": "驛道", "route": "商道", "way": "大道",
        },
        "trails": {
            "pass": "隘道", "trail": "山徑", "path": "古小徑", "track": "林道",
        },
        "searoutes": {
            "water way": "水道", "passage": "海道", "lane": "航道", "route": "航線",
        },
    }
    used_route_names: set[str] = set()
    named_routes_count = 0
    for rt in pack.get("routes", []):
        if not isinstance(rt, dict) or not rt.get("name"):
            continue
        rid = int(rt["i"])
        orig_rn = clean_surrogates(str(rt["name"])).strip()
        grp = str(rt.get("group", "roads"))
        rt["orig_name"] = orig_rn

        tail_map = ROUTE_TAIL_BY_GROUP.get(grp, ROUTE_TAIL_BY_GROUP["roads"])
        body = orig_rn
        if body.lower().startswith("the "):
            body = body[4:].strip()
        suf_zh = "商道" if grp == "roads" else ("山徑" if grp == "trails" else "航線")
        for en_tail in sorted(tail_map.keys(), key=len, reverse=True):
            if body.lower().endswith(" " + en_tail):
                suf_zh = tail_map[en_tail]
                body = body[: -(len(en_tail) + 1)].strip()
                break

        words = body.split()
        zh_parts: list[str] = []
        rem_words: list[str] = []
        for w in words:
            wl = w.lower()
            if wl in ROUTE_ADJ_ZH:
                zh_parts.append(ROUTE_ADJ_ZH[wl])
            else:
                rem_words.append(w)
        if rem_words:
            rem_phrase = " ".join(rem_words)
            ent_zh = lookup_entity_zh(rem_phrase)
            # 若實體本身已帶海港／聚落尾綴，避免「白鷗港水道」重複，收斂為詞幹
            ent_stem = re.sub(
                r"(堅城|石堡|岩寨|要塞|林驛|渡口|漁港|北塢|橋鎮|谷鎮|溪村|林莊|港|灣|澳|津|塢|岬|城|市|鎮|村|莊|屯|堡|塞|關|驛|坡|都)$",
                "",
                ent_zh,
            )
            if len(ent_stem) < 2:
                ent_stem = ent_zh
            zh_parts.append(ent_stem)
        if not zh_parts:
            zh_parts.append(make_phonetic_stem(orig_rn, 0, f"rt:{rid}"))

        base_rt_zh = "".join(zh_parts) + suf_zh
        cand_rt_zh = base_rt_zh
        if cand_rt_zh in used_route_names:
            # 依起點細胞所在省／國詞幹區分同名古道（如多條「靜水驛道」）
            pts = rt.get("points", [])
            c0_id = int(pts[0][2]) if pts and isinstance(pts[0], list) and len(pts[0]) >= 3 else 0
            c0 = cell_by_id.get(c0_id, {})
            st0 = state_stem_zh.get(int(c0.get("state", 0) or 0), "")
            pv0 = prov_zh_by_id.get(int(c0.get("province", 0) or 0), "")[:2]
            loc_tag = pv0 or st0 or make_phonetic_stem(orig_rn, int(c0.get("culture", 0) or 0), f"rtu:{rid}")
            cand_rt_zh = f"{loc_tag}・{base_rt_zh}"
            _dirs = ("東", "西", "南", "北", "上", "下")
            seq = 0
            while cand_rt_zh in used_route_names:
                cand_rt_zh = f"{loc_tag}{_dirs[seq % len(_dirs)]}・{base_rt_zh}"
                seq += 1
        used_route_names.add(cand_rt_zh)
        rt["name"] = cand_rt_zh
        named_routes_count += 1

    # 14. 轉譯跨大陸行旅／走私暗線（pack.journeys）
    JOURNEY_TYPE_ZH = {
        "Smuggling run": "私貨走私暗線",
        "Trade route": "長途商旅路線",
        "Expedition": "遠征探險路線",
        "Pilgrimage": "朝聖苦行路線",
    }
    for jn in pack.get("journeys", []):
        if not isinstance(jn, dict):
            continue
        orig_jn = clean_surrogates(str(jn.get("name", "")))
        orig_jt = clean_surrogates(str(jn.get("type", "")))
        jn["orig_name"] = orig_jn
        jn["orig_type"] = orig_jt
        jn["name"] = "檀香走私暗線" if orig_jn == "The Sandalwood Run" else f"{lookup_entity_zh(orig_jn)}遠行路線"
        jn["type"] = JOURNEY_TYPE_ZH.get(orig_jt, "行旅路線")
        for seg in jn.get("segments", []):
            if not isinstance(seg, dict):
                continue
            s_orig = clean_surrogates(str(seg.get("name", "")))
            t_orig = str(seg.get("transport", ""))
            seg["orig_name"] = s_orig
            seg["orig_transport"] = t_orig
            seg["transport"] = TRANSPORT_EN_TO_ZH.get(t_orig, "車馬行旅")

            c_from = cell_by_id.get(int(seg.get("from", 0) or 0), {})
            c_to = cell_by_id.get(int(seg.get("to", 0) or 0), {})
            b_to = burg_zh_by_id.get(int(c_to.get("burg", 0) or 0))
            st_to = state_stem_zh.get(int(c_to.get("state", 0) or 0))

            m_s = None
            if s_orig.startswith("Loading at "):
                pl = lookup_entity_zh(s_orig.replace("Loading at ", "").replace(" quay", ""))
                seg["name"] = f"於【{pl}】碼頭暗夜裝船"
            elif s_orig.startswith("Out of "):
                pl = lookup_entity_zh(s_orig.replace("Out of ", "").replace(" port", ""))
                seg["name"] = f"揚帆駛離【{pl}】外海"
            elif s_orig.startswith("A night at anchor near "):
                pl = lookup_entity_zh(s_orig.replace("A night at anchor near ", ""))
                seg["name"] = f"於【{pl}】近岸避風夜泊"
            elif s_orig.startswith("A night near "):
                pl = b_to or lookup_entity_zh(s_orig.replace("A night near ", ""))
                seg["name"] = f"於【{pl}】郊野露宿一夜"
            elif s_orig.startswith("Last stretch to "):
                pl = b_to or st_to or lookup_entity_zh(s_orig.replace("Last stretch to ", ""))
                seg["name"] = f"趕赴【{pl}】最後一段路程"
            elif s_orig.endswith(" to the forest"):
                pl = lookup_entity_zh(s_orig.replace(" to the forest", ""))
                seg["name"] = f"自【{pl}】棄道潛入深林"
            elif s_orig == "Cache dug in the forest":
                seg["name"] = "於密林深處掘窖藏匿私貨"
            elif s_orig.startswith("Around the tollgates to "):
                pl = b_to or st_to or lookup_entity_zh(s_orig.replace("Around the tollgates to ", ""))
                seg["name"] = f"繞避官道稅關潛往【{pl}】"
            elif s_orig == "Into the grasslands":
                seg["name"] = "橫越無路廣袤大草原"
            elif s_orig.startswith("Palms greased in "):
                pl = b_to or lookup_entity_zh(s_orig.replace("Palms greased in ", ""))
                seg["name"] = f"於【{pl}】塞錢買通關節"
            elif s_orig.startswith("Off-road to "):
                pl = b_to or lookup_entity_zh(s_orig.replace("Off-road to ", ""))
                seg["name"] = f"避開驛道荒野疾行至【{pl}】"
            else:
                pl = b_to or st_to or lookup_entity_zh(s_orig)
                seg["name"] = f"行經【{pl}】路段"

    # ─── 五、嚴格自動化驗收斷言（0 英文殘留、0 簡體字、100% 唯一城名） ───
    SIMPLIFIED_CHECK = set("们这来个时国动进还发说过对开会经现点样体机实见关门问间长东车马风鸟鱼龙铁矿镇")

    def assert_pure_zhtw(label: str, val: str) -> None:
        if re.search(r"[A-Za-z]", val):
            raise AssertionError(f"[{label}] 殘留英文字母: {val}")
        bad_s = [ch for ch in val if ch in SIMPLIFIED_CHECK]
        if bad_s:
            raise AssertionError(f"[{label}] 發現簡體字 {bad_s}: {val}")

    assert_pure_zhtw("info.mapName", zh_doc["info"]["mapName"])
    for tr in map_opts.get("transports", []):
        assert_pure_zhtw("transport", tr["name"])
    for u in map_opts.get("military", {}).get("units", []):
        assert_pure_zhtw("unit", u["name"])
    for b in pack["biomes"]:
        if isinstance(b, dict) and "name" in b:
            assert_pure_zhtw(f"biome.{b['i']}", b["name"])
    for g in pack["goods"]:
        if isinstance(g, dict) and "name" in g:
            assert_pure_zhtw(f"good.{g['i']}", g["name"])
            assert_pure_zhtw(f"good_unit.{g['i']}", g["unit"])
    for c in pack["cultures"]:
        if isinstance(c, dict) and "name" in c:
            assert_pure_zhtw(f"culture.{c['i']}", c["name"])

    for s in pack["states"]:
        if isinstance(s, dict) and s.get("i") and not s.get("removed"):
            assert_pure_zhtw(f"state.{s['i']}", s["fullName"])
            for cp in s.get("campaigns", []):
                assert_pure_zhtw(f"campaign.{s['i']}", cp["name"])
            for reg in s.get("military", []):
                assert_pure_zhtw(f"reg.{s['i']}", reg["name"])
                assert_pure_zhtw(f"reg_note.{s['i']}", reg["note"])

    for row in pack["states"][0].get("diplomacy", []):
        for line in row:
            assert_pure_zhtw("chronicle", line)

    all_b_names = []
    for b in pack["burgs"]:
        if isinstance(b, dict) and b.get("i") and not b.get("removed"):
            assert_pure_zhtw(f"burg.{b['i']}", b["name"])
            all_b_names.append(b["name"])
    assert len(all_b_names) == len(set(all_b_names)), "城鎮中文名稱出現重複！"

    for p in pack["provinces"]:
        if isinstance(p, dict) and p.get("i") and not p.get("removed"):
            assert_pure_zhtw(f"prov.{p['i']}", p["fullName"])
    for rv in pack["rivers"]:
        if isinstance(rv, dict) and rv.get("i"):
            assert_pure_zhtw(f"river.{rv['i']}", rv["name"] + rv["type"])
    for r in pack["religions"]:
        if isinstance(r, dict) and r.get("i") and not r.get("removed"):
            assert_pure_zhtw(f"rel.{r['i']}", r["name"])
            if r.get("deity"):
                assert_pure_zhtw(f"deity.{r['i']}", r["deity"])
    for z in pack["zones"]:
        if isinstance(z, dict) and "name" in z:
            assert_pure_zhtw("zone.name", z["name"])
            assert_pure_zhtw("zone.note", z["note"])
    for m in pack["markers"]:
        if isinstance(m, dict) and "name" in m:
            assert_pure_zhtw(f"marker.{m['i']}.name", m["name"])
            assert_pure_zhtw(f"marker.{m['i']}.note", m["note"])
    for ft in pack["features"]:
        if isinstance(ft, dict) and "name" in ft:
            assert_pure_zhtw(f"feature.{ft.get('i')}", ft["name"])
    for rt in pack["routes"]:
        if isinstance(rt, dict) and rt.get("name"):
            assert_pure_zhtw(f"route.{rt.get('i')}", rt["name"])
    for jn in pack.get("journeys", []):
        if isinstance(jn, dict):
            assert_pure_zhtw("journey.name", jn["name"])
            assert_pure_zhtw("journey.type", jn["type"])
            for seg in jn.get("segments", []):
                assert_pure_zhtw("journey.seg", seg["name"])
                assert_pure_zhtw("journey.tr", seg["transport"])

    # ─── 六、落盤輸出 JSON 與 Markdown 總冊 ───
    OUT_FULL_JSON.parent.mkdir(parents=True, exist_ok=True)
    OUT_FULL_JSON.write_text(json.dumps(zh_doc, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")

    glossary = {
        "schema": "singularity_world/tanger_glossary_zh/v1",
        "source_file": str(SRC_PATH),
        "counts": {
            "features": named_features_count,
            "biomes": len(BIOME_ZH),
            "cultures": len(CULTURE_ZH),
            "states": len(state_full_zh),
            "provinces": len(prov_zh_by_id),
            "burgs": len(burg_zh_by_id),
            "religions": len(used_rels),
            "rivers": len(used_rivers),
            "routes": named_routes_count,
            "goods": len(GOODS_ZH),
            "zones": len(pack.get("zones", [])),
            "markers": len(pack.get("markers", [])),
            "campaigns": sum(len(s.get("campaigns", [])) for s in pack["states"] if isinstance(s, dict) and s.get("i")),
            "regiments": sum(len(s.get("military", [])) for s in pack["states"] if isinstance(s, dict) and s.get("i")),
            "journeys": len(pack.get("journeys", [])),
        },
        "features": {
            str(ft["i"]): {"zh": ft["name"], "orig": ft.get("orig_name", ""), "type": ft.get("subtype") or ft.get("type", "")}
            for ft in pack["features"] if isinstance(ft, dict) and "name" in ft
        },
        "cultures": {
            str(c["i"]): {"zh": c["name"], "orig": c.get("orig_name", ""), "type": orig_culture_type_by_id.get(c["i"], "")}
            for c in pack["cultures"] if isinstance(c, dict) and "i" in c
        },
        "goods": {
            str(g["i"]): {"zh": g["name"], "orig": g.get("orig_name", ""), "unit_zh": g["unit"], "orig_unit": g.get("orig_unit", "")}
            for g in pack["goods"] if isinstance(g, dict) and "i" in g
        },
        "states": {
            str(s["i"]): {
                "zh": s["fullName"],
                "stem": s["name"],
                "orig": s.get("orig_fullName", ""),
                "capital_zh": burg_zh_by_id.get(int(s.get("capital", 0) or 0), ""),
                "culture_zh": CULTURE_ZH.get(int(s.get("culture", 0) or 0), ""),
                "race_pct": s.get("race_pct", {}),
                "campaigns": [{"zh": cp["name"], "orig": cp.get("orig_name", ""), "start": cp.get("start"), "end": cp.get("end")} for cp in s.get("campaigns", [])],
            }
            for s in pack["states"] if isinstance(s, dict) and s.get("i") and not s.get("removed")
        },
        "provinces": {
            str(p["i"]): {"zh": p["fullName"], "orig": p.get("orig_fullName", ""), "state_id": p.get("state"), "burg_id": p.get("burg")}
            for p in pack["provinces"] if isinstance(p, dict) and p.get("i") and not p.get("removed")
        },
        "burgs": {
            str(b["i"]): {
                "zh": b["name"],
                "orig": b.get("orig_name", ""),
                "cell": b.get("cell"),
                "state_id": b.get("state"),
                "feature": b.get("feature"),
                "port": bool(b.get("port")),
                "walls": bool(b.get("walls")),
                "race_pct": b.get("race_pct", {}),
            }
            for b in pack["burgs"] if isinstance(b, dict) and b.get("i") and not b.get("removed")
        },
        "religions": {
            str(r["i"]): {
                "zh": r["name"],
                "orig": r.get("orig_name", ""),
                "type_zh": r.get("type", ""),
                "form_zh": r.get("form", ""),
                "deity_zh": r.get("deity"),
                "orig_deity": r.get("orig_deity"),
            }
            for r in pack["religions"] if isinstance(r, dict) and r.get("i") and not r.get("removed")
        },
        "rivers": {
            str(rv["i"]): {"zh": rv["name"] + rv["type"], "orig": f"{rv.get('orig_name', '')} {rv.get('orig_type', '')}".strip()}
            for rv in pack["rivers"] if isinstance(rv, dict) and rv.get("i")
        },
        "routes": {
            str(rt["i"]): {"zh": rt["name"], "orig": rt.get("orig_name", ""), "group": rt.get("group", "")}
            for rt in pack["routes"] if isinstance(rt, dict) and rt.get("name")
        },
        "zones": {
            str(z["i"]): {"zh": z["name"], "orig": z.get("orig_name", ""), "type_zh": z["type"], "note_zh": z["note"]}
            for z in pack.get("zones", []) if isinstance(z, dict) and "name" in z
        },
        "markers": {
            str(m["i"]): {"zh": m["name"], "orig": m.get("orig_name", ""), "type": m.get("type", ""), "cell": m.get("cell"), "note_zh": m["note"]}
            for m in pack.get("markers", []) if isinstance(m, dict) and "name" in m
        },
        "journeys": [
            {
                "i": jn.get("i"),
                "zh": jn.get("name"),
                "orig": jn.get("orig_name"),
                "type_zh": jn.get("type"),
                "segments": [
                    {"i": sg.get("i"), "zh": sg.get("name"), "orig": sg.get("orig_name"), "transport_zh": sg.get("transport")}
                    for sg in jn.get("segments", [])
                ],
            }
            for jn in pack.get("journeys", []) if isinstance(jn, dict)
        ],
    }
    OUT_GLOSSARY_JSON.write_text(json.dumps(glossary, ensure_ascii=False, indent=2), encoding="utf-8")

    # 產出可讀 Markdown 總冊
    f6_burgs = [b for b in pack["burgs"] if isinstance(b, dict) and b.get("i") and b.get("feature") == 6]
    f6_burgs.sort(key=lambda x: (-float(x.get("population", 0)), x["i"]))

    md_lines = [
        "# 《奇點世界》Azgaar 母圖全大陸正體中文專有名詞、四族結構與五百年戰史總冊",
        "",
        f"- **原始母圖**：`{SRC_PATH}`（Azgaar v1.158.0，種子 `812745074`）",
        f"- **完整中文化母圖**：`{OUT_FULL_JSON}`",
        f"- **結構化對照辭典**：`{OUT_GLOSSARY_JSON}`",
        "- **轉譯規模**：9 大海洋陸塊群島、13 種生態系、14 支文化語系、100 國、841 省、850 座不重名城鎮、191 條河川、326 條具名陸海商道、76 門民間信仰與神祇、71 種貿易物產、14 處戰區災疫、96 處世界奇觀地標、506 場歷史戰役、146 支常備軍團、12 場當下跨國大戰編年與 1 條 13 段跨陸走私暗線（全數通過 0 英文殘留與 0 簡體字檢查）。",
        "",
        "---",
        "",
        "## 一、9 大海洋、大陸、湖泊與島嶼（`pack.features`）及 14 支地域文化（`pack.cultures`）",
        "",
        "### 1-1. 大洋、主大陸與外海島嶼",
        "",
        "| 編號 | 中文定名 | 母圖原名 | 地貌型態 | 定位說明 |",
        "| :---: | :--- | :--- | :---: | :--- |",
    ]
    ft_desc = {
        1: ("大洋", "環繞主大陸與外海諸島的廣闊大洋"),
        2: ("主大陸", "面積逾 62 萬平方里、百國林立的主大陸"),
        3: ("內陸淡水湖", "主大陸中西部兩河匯聚之清澈大湖"),
        4: ("近海島嶼", "主大陸東南近岸島嶼"),
        5: ("近海島嶼", "主大陸東南近岸岩嶼"),
        6: ("外海大島", "玩家開局大島（銀鷗海領所在，面積逾萬平方里）"),
        7: ("外海島嶼", "銀鷗島西南外海孤嶼"),
        8: ("外海島嶼", "南方航線中途大嶼"),
        9: ("外海島嶼", "南方外海礁嶼"),
    }
    for ft in pack["features"]:
        if isinstance(ft, dict) and "name" in ft:
            fid = int(ft["i"])
            kind_s, desc_s = ft_desc.get(fid, ("島嶼", "外海島嶼"))
            md_lines.append(f"| `{fid}` | **{ft['name']}** | `{ft.get('orig_name', '')}` | {kind_s} | {desc_s} |")

    md_lines += [
        "",
        "### 1-2. 14 支地域文化與方言底色",
        "",
        "| 編號 | 中文名稱 | 原名 | 文化型態 | 主要生態與四族演化底色 |",
        "| :---: | :--- | :--- | :--- | :--- |",
    ]
    cul_desc_map = {
        0: "萬年冰原與高山荒野（哥布林與退化荒野散部）",
        1: "溫帶雨林與河口大澤（人類與精靈混居海灣）",
        2: "東南溫帶雨林群島（銀鷗海領所在，海港四族通商）",
        3: "北方 664 格廣袤草原（獸人與半獸人游牧大汗國主力）",
        4: "林原交界大邦（人類農商與矮人山麓工坊）",
        5: "南方稀樹莽原（獸人與人類狩獵部族）",
        6: "高海拔寒漠高原（獸人與矮人耐寒游牧支系）",
        7: "草原與北方泰加針葉林（人類、精靈與獸人交界）",
        8: "中央 913 格闊葉大林與山麓（精靈古林與人類王國並立）",
        9: "溫暖海島與岬角（海洋商旅與漁戶）",
        10: "南方雨林與大澤（水鄉農商與草藥精靈）",
        11: "高覆蓋闊葉深林（精靈高佔比林間聚落）",
        12: "溫帶闊葉與雨林谷地（精靈與人類混居山谷）",
        13: "高寒荒漠高原（高寒獸人兀魯思與高山矮人礦寨）",
    }
    for ci in sorted(CULTURE_ZH):
        c_obj = pack["cultures"][ci]
        md_lines.append(
            f"| `{ci}` | **{c_obj['name']}** | `{c_obj.get('orig_name', '')}` | `{orig_culture_type_by_id.get(ci, '')}` | {cul_desc_map.get(ci, '')} |"
        )

    md_lines += [
        "",
        "## 二、東南外海開局大島（`銀鷗島 Feature 6` / `銀鷗海領 State 26`）全 40 聚落與周邊航道",
        "",
        "> 註：遊戲內實裝精煉為 **5 座核心據點**（白鷗港、西峽渡口、鷗尾漁港、灰松林驛、海風北塢），其餘 35 點為島內農莊、漁澳與山林聚落舊址，現已全數完成中文定名：",
        "",
        "| 城鎮 ID | 中文定名 | 母圖原名 | 地塊 | 港口／城牆 | 兩層公式四族比例（人／矮／精／獸） |",
        "| :---: | :--- | :--- | :---: | :---: | :--- |",
    ]
    for b in f6_burgs:
        rp = b["race_pct"]
        pw = ("海港" if b.get("port") else "內陸") + ("・石牆" if b.get("walls") else "")
        md_lines.append(
            f"| `{b['i']}` | **{b['name']}** | `{b.get('orig_name', '')}` | `{b.get('cell')}` | {pw} | 人 {rp['人類']}%・矮 {rp['矮人']}%・精 {rp['精靈']}%・獸 {rp['獸人']}% |"
        )

    md_lines += [
        "",
        "### 2-2. 銀鷗島連結外海之具名海上航道（摘自 `pack.routes`）",
        "",
        "| 路線 ID | 中文航道名 | 母圖原名 | 類型 |",
        "| :---: | :--- | :--- | :---: |",
    ]
    for rid_pick in (687, 689, 695, 696, 706):
        for rt in pack["routes"]:
            if isinstance(rt, dict) and rt.get("i") == rid_pick and rt.get("name"):
                md_lines.append(f"| `{rid_pick}` | **{rt['name']}** | `{rt.get('orig_name', '')}` | 海上航道 |")

    md_lines += [
        "",
        "## 三、全大陸 100 國家政權、首府、戰史數與四族結構總表（`pack.states`）",
        "",
        "| 國號 ID | 中文國名 | 母圖原名 | 首府 | 地域方言 | 主導四族結構（人／矮／精／獸） | 歷史戰役數 |",
        "| :---: | :--- | :--- | :--- | :--- | :--- | :---: |",
    ]
    for s in pack["states"]:
        if not isinstance(s, dict) or not s.get("i") or s.get("removed"):
            continue
        rp = s["race_pct"]
        cap_zh = burg_zh_by_id.get(int(s.get("capital", 0) or 0), "—")
        cul_zh = CULTURE_ZH.get(int(s.get("culture", 0) or 0), "—")
        cp_cnt = len(s.get("campaigns", []))
        md_lines.append(
            f"| `{s['i']}` | **{s['fullName']}** | `{s.get('orig_fullName', '')}` | {cap_zh} | {cul_zh} | 人 {rp['人類']}%・矮 {rp['矮人']}%・精 {rp['精靈']}%・獸 {rp['獸人']}% | {cp_cnt} |"
        )

    md_lines += [
        "",
        "## 四、71 種大陸貿易物產與計量單位全表（`pack.goods`）",
        "",
        "> 註：遵循《奇點世界》無火藥科技樹物理鐵律，母圖中涉及硝石／火藥／火砲之三項物產（`33`、`60`、`61`）已無損轉譯為冶金與冷兵器攻城器械物資：",
        "",
        "| 編號 | 中文物產名 | 計量單位 | 母圖原名 | 編號 | 中文物產名 | 計量單位 | 母圖原名 |",
        "| :---: | :--- | :---: | :--- | :---: | :--- | :---: | :--- |",
    ]
    g_list = [g for g in pack["goods"] if isinstance(g, dict) and "i" in g]
    half = (len(g_list) + 1) // 2
    for idx_g in range(half):
        gL = g_list[idx_g]
        left_s = f"`{gL['i']}` | **{gL['name']}** | {gL['unit']} | `{gL.get('orig_name', '')}`"
        if idx_g + half < len(g_list):
            gR = g_list[idx_g + half]
            right_s = f"`{gR['i']}` | **{gR['name']}** | {gR['unit']} | `{gR.get('orig_name', '')}`"
        else:
            right_s = "— | — | — | —"
        md_lines.append(f"| {left_s} | {right_s} |")

    md_lines += [
        "",
        "## 五、76 門民間信仰、教派與主神尊號總表（`pack.religions`）",
        "",
        "| 編號 | 中文信仰名稱 | 教派型態／教義 | 主神尊號 | 母圖原名 |",
        "| :---: | :--- | :--- | :--- | :--- |",
    ]
    for r in pack["religions"]:
        if isinstance(r, dict) and r.get("i") and not r.get("removed"):
            tf_s = f"{r.get('type', '—')}・{r.get('form', '—')}"
            dt_s = r.get("deity") or "（泛靈無具名主神）"
            md_lines.append(f"| `{r['i']}` | **{r['name']}** | {tf_s} | {dt_s} | `{r.get('orig_name', '')}` |")

    md_lines += [
        "",
        "## 六、14 處大陸戰火與災疫區（`pack.zones`）",
        "",
        "| 編號 | 中文區域名稱 | 類型 | 世界內現況記述 |",
        "| :---: | :--- | :--- | :--- |",
    ]
    for z in pack.get("zones", []):
        if isinstance(z, dict) and "name" in z:
            md_lines.append(f"| `{z.get('i')}` | **{z['name']}** | {z['type']} | {z['note']} |")

    md_lines += [
        "",
        "## 七、96 處世界奇觀、行會、古戰場與險境傳聞全錄（`pack.markers`）",
        "",
        "| 編號 | 中文地標名稱 | 地塊 | 在地傳聞與背景記述 | 母圖原名 |",
        "| :---: | :--- | :---: | :--- | :--- |",
    ]
    for m in pack.get("markers", []):
        if isinstance(m, dict) and "name" in m:
            md_lines.append(
                f"| `{m.get('i')}` | **{m['name']}** | `{m.get('cell')}` | {m['note']} | `{m.get('orig_name', '')}` |"
            )

    md_lines += [
        "",
        "## 八、當下 12 場跨國大戰與盟約背棄編年（`states[0].diplomacy`）",
        "",
    ]
    for idx, row in enumerate(pack["states"][0].get("diplomacy", []), 1):
        if not row:
            continue
        md_lines.append(f"### {idx}. 【{row[0]}】")
        for step_line in row[1:]:
            md_lines.append(f"- {step_line}")
        md_lines.append("")

    if pack.get("journeys"):
        jn0 = pack["journeys"][0]
        md_lines += [
            f"## 九、跨海陸走私暗線紀實：【{jn0['name']}】（`pack.journeys[0]`，{jn0['type']}）",
            "",
            "| 段次 | 路程記述 | 交通方式 | 起訖地塊 |",
            "| :---: | :--- | :---: | :---: |",
        ]
        for sg in jn0.get("segments", []):
            md_lines.append(
                f"| `{sg.get('i')}` | {sg['name']} | {sg['transport']} | `{sg.get('from')} → {sg.get('to')}` |"
            )
        md_lines.append("")

    OUT_DOC_MD.parent.mkdir(parents=True, exist_ok=True)
    OUT_DOC_MD.write_text("\n".join(md_lines) + "\n", encoding="utf-8")

    print("TRANSLATION_COMPLETE", json.dumps(glossary["counts"], ensure_ascii=False))


if __name__ == "__main__":
    main()
