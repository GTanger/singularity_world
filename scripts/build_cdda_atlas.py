#!/usr/bin/env python3
"""將開源 CDDA Chibi_Ultica (UltiCa + MSXotto+) 打包為《奇點世界》單一精煉圖集 atlas.png + atlas.json。"""

import io
import json
import os
import zipfile
from pathlib import Path
from PIL import Image, ImageDraw

ZIP_PATH = Path("/home/tanger/.local/share/tanger/assets/cdda/ChibiUltica-2026-11-15.zip")
OUT_DIR = Path("/home/tanger/Projects/singularity_world/web/assets/cdda")
VERIFY_PATH = Path("/dev/shm/cdda_autotile_verify.png")

# 4-bit NSEW 連通遮罩映射：N=1, E=2, S=4, W=8
# 對應 tile_config.json multitile additional_tiles 索引定義（已逐格圖素核對）
MASK_16_SPECS = (
    (0, "unconnected", None),
    (15, "center", None),
    (5, "edge", 0),        # N + S
    (10, "edge", 1),       # E + W
    (6, "corner", 0),      # S + E
    (12, "corner", 1),     # S + W
    (9, "corner", 2),      # N + W
    (3, "corner", 3),      # N + E
    (14, "t_connection", 0),  # E + S + W
    (13, "t_connection", 1),  # N + S + W
    (11, "t_connection", 2),  # N + E + W
    (7, "t_connection", 3),   # N + E + S
    (4, "end_piece", 0),   # S
    (8, "end_piece", 1),   # W
    (1, "end_piece", 2),   # N
    (2, "end_piece", 3),   # E
)


def pick_idx(val, var_i=0):
    if isinstance(val, int):
        return val
    if isinstance(val, dict):
        return val["sprite"]
    if isinstance(val, list) and val:
        item = val[var_i % len(val)]
        return item["sprite"] if isinstance(item, dict) else item
    return None


def main():
    if not ZIP_PATH.exists():
        raise FileNotFoundError(f"找不到 {ZIP_PATH}")

    zf = zipfile.ZipFile(ZIP_PATH)
    cfg = json.loads(zf.read("ChibiUltica/tile_config.json"))

    sheets = []
    base = 0
    for s in cfg["tiles-new"]:
        fname = s["file"]
        sw = s.get("sprite_width") or 32
        sh = s.get("sprite_height") or 32
        ox = s.get("sprite_offset_x") or 0
        oy = s.get("sprite_offset_y") or 0
        img = Image.open(io.BytesIO(zf.read("ChibiUltica/" + fname))).convert("RGBA")
        cols = img.width // sw
        cnt = cols * (img.height // sh)
        sheets.append({
            "file": fname,
            "img": img,
            "sw": sw,
            "sh": sh,
            "ox": ox,
            "oy": oy,
            "cols": cols,
            "base": base,
            "end": base + cnt,
        })
        base += cnt

    id_map = {}
    for s in cfg["tiles-new"]:
        for t in s.get("tiles", []):
            tid = t.get("id")
            for k in (tid if isinstance(tid, list) else [tid]):
                if isinstance(k, str) and k not in id_map:
                    id_map[k] = t

    def get_sprite(idx):
        for sh in sheets:
            if sh["base"] <= idx < sh["end"]:
                li = idx - sh["base"]
                r, c = divmod(li, sh["cols"])
                crop = sh["img"].crop((c * sh["sw"], r * sh["sh"], (c + 1) * sh["sw"], (r + 1) * sh["sh"]))
                return crop, sh["ox"], sh["oy"], sh["file"]
        raise KeyError(f"sprite index out of range: {idx}")

    extracted = []

    def add_sprite(key, tile_id, use_bg=False, sub=None, sub_pos=None, var_i=0, center_in_32=False, tint=None):
        t = id_map.get(tile_id)
        if not t:
            raise KeyError(f"missing CDDA tile id: {tile_id} (for {key})")
        val = t.get("bg") if use_bg else t.get("fg")
        if sub is not None:
            subs = {at["id"]: at["fg"] for at in t.get("additional_tiles", [])}
            val = subs.get(sub, val)
            if sub_pos is not None and isinstance(val, list):
                val = val[sub_pos]
        idx = pick_idx(val, var_i=var_i)
        if idx is None:
            raise ValueError(f"null sprite index for {key} -> {tile_id}")
        sp, ox, oy, fn = get_sprite(idx)
        if fn.startswith("filler"):
            raise ValueError(f"filler sprite {idx} for {key} -> {tile_id}")
        if sp.getbbox() is None:
            raise ValueError(f"empty transparent sprite for {key} -> {tile_id}")
        if tint == "slate":
            # 將紅瓦四連通圖磚無損轉色為台式青黑瓦（保留原始瓦片明暗紋理）
            px = sp.copy().load()
            for yy in range(sp.height):
                for xx in range(sp.width):
                    r, g, b, a = px[xx, yy]
                    if a > 0:
                        lum = int(0.38 * r + 0.42 * g + 0.20 * b)
                        px[xx, yy] = (
                            max(0, min(255, int(lum * 0.74))),
                            max(0, min(255, int(lum * 0.80))),
                            max(0, min(255, int(lum * 0.90))),
                            a,
                        )
            sp = sp.copy()
            sp_px = sp.load()
            for yy in range(sp.height):
                for xx in range(sp.width):
                    sp_px[xx, yy] = px[xx, yy]
        if center_in_32 and sp.width <= 32 and sp.height <= 32:
            ox = (32 - sp.width) // 2
            oy = (32 - sp.height) // 2
        extracted.append((key, sp, ox, oy))

    def add_16(prefix, tile_id, tint=None):
        for mask, sub, sub_pos in MASK_16_SPECS:
            add_sprite(f"{prefix}_{mask}", tile_id, sub=sub, sub_pos=sub_pos, tint=tint)

    # 1. 地形與水岸
    for v in range(4):
        add_sprite(f"ground_grass_{v}", "t_grass", var_i=v)
    add_sprite("ground_red", "t_soil")
    add_sprite("ground_dirt", "t_dirt", sub="center")
    add_sprite("ground_pebble", "t_dirtfloor")
    add_sprite("ground_sand", "t_sand", sub="center")
    add_sprite("ground_mudflat", "t_mud")
    add_sprite("ground_rock", "t_rock_floor", sub="center")
    add_16("ground_cliff", "t_rock")
    add_16("ground_water_sh", "t_water_sh")
    add_16("ground_water_dp", "t_water_dp")
    add_sprite("ground_floor", "t_floor", sub="center")
    add_sprite("ground_tatami_0", "f_tatami", var_i=0)
    add_sprite("ground_tatami_1", "f_tatami", var_i=1)
    add_16("ground_paving", "t_sidewalk")
    add_sprite("ground_bridge", "t_riverbridge_dp")

    # 2. 建築牆門屋頂（四連通 16 態）
    add_16("wall_wood", "t_wall_log")
    add_16("wall_adobe", "t_adobe_brick_wall")
    add_16("wall_brick", "t_brick_wall")
    add_sprite("door_closed", "t_door_c")
    add_sprite("door_open", "t_door_o")
    add_16("roof_tile", "t_brick_roof")
    add_16("roof_shingle", "t_brick_roof", tint="slate")

    # 3. 家具、街燈、自然物與農田五階
    add_sprite("struct_firering", "f_firering")
    add_sprite("struct_fire", "fd_fire")
    add_sprite("struct_woodstove", "f_woodstove")
    add_sprite("struct_fireplace", "f_fireplace")
    add_sprite("struct_anvil", "f_anvil")
    add_sprite("struct_workbench", "f_workbench")
    add_sprite("struct_counter", "f_counter")
    add_sprite("struct_table", "f_table")
    add_sprite("struct_chair", "f_chair", var_i=0)
    add_sprite("struct_rack", "f_rack_wood")
    add_sprite("struct_bed", "f_bed")
    add_sprite("struct_makeshift_bed", "f_makeshift_bed")
    add_sprite("struct_street_light", "f_street_light")

    add_sprite("farm_ridge", "t_dirtmound")
    add_sprite("farm_seed", "f_plant_seed")
    add_sprite("farm_seedling", "f_plant_seedling")
    add_sprite("farm_mature", "f_plant_mature")
    add_sprite("farm_harvest", "f_plant_harvest")
    add_sprite("farm_herb_seed", "f_chamomile")
    add_sprite("farm_herb_mature", "f_carrot_wild")

    add_sprite("prop_bamboo_tall", "t_bamboo_tall")
    add_sprite("prop_bamboo_young", "t_bamboo")
    add_sprite("prop_shoot", "cattail_stalk", center_in_32=True)
    add_sprite("prop_herb", "f_carrot_wild")
    add_sprite("prop_stone_0", "f_boulder_small")
    add_sprite("prop_stone_1", "f_boulder_medium")

    # 4. 全 16 種物資道具（對應 RimItemKind::code 與 short_char）
    item_map = {
        "item_bamboo": "raw_bamboo",
        "item_stone": "rock",
        "item_shoot": "cattail_stalk",
        "item_meat": "meat",
        "item_roast_shoot": "veggy_cooked",
        "item_roast_meat": "meat_cooked",
        "item_blade": "knife_hunting",
        "item_pick": "primitive_axe",
        "item_hat": "straw_fedora",
        "item_pack": "wicker_backpack",
        "item_herb": "wild_herbs",
        "item_salve": "jar_glass_sealed",
        "item_tuber": "potato",
        "item_roast_tuber": "chestnut_roasted",
        "item_alloy_blade": "machete",
        "item_alloy_pick": "bronze_pickaxe",
    }
    for k, tid in item_map.items():
        add_sprite(k, tid, center_in_32=True)

    # 5. 野生動物
    add_sprite("beast_boar", "mon_boar_wild")
    add_sprite("beast_pheasant", "mon_pheasant")

    # 6. MSXotto+ 人類紙娃娃完整圖層
    add_sprite("pd_shadow", "player_male", use_bg=True)
    add_sprite("pd_body_m", "player_male")
    add_sprite("pd_body_f", "player_female")
    for ec in ["brown", "amber", "gray", "hazel"]:
        add_sprite(f"pd_eye_{ec}", f"overlay_mutation_eye_color_var_{ec}")

    hair_styles = ["buzzcut", "crewcut", "short", "short_no_fringe", "medium", "long", "ponytail"]
    hair_colors = ["black", "brown", "gray", "white"]
    for g_tag, cd_g in [("m", "male"), ("f", "female")]:
        for st in hair_styles:
            for col in hair_colors:
                tid = f"overlay_{cd_g}_mutation_hair_{st}_var_{col}"
                if tid in id_map:
                    add_sprite(f"pd_hair_{g_tag}_{st}_{col}", tid)

    add_sprite("pd_shoes_boots", "overlay_worn_boots")
    add_sprite("pd_shoes_hiking", "overlay_worn_boots_hiking")
    add_sprite("pd_shoes_dress", "overlay_worn_dress_shoes")
    add_sprite("pd_pants_work", "overlay_worn_pants")
    add_sprite("pd_pants_cargo", "overlay_worn_pants_cargo")
    add_sprite("pd_pants_jeans", "overlay_worn_jeans")

    m_shirts = [
        ("0", "overlay_male_worn_tunic"),
        ("1", "overlay_male_worn_dress_shirt_var_white"),
        ("2", "overlay_male_worn_dress_shirt_var_blue"),
        ("3", "overlay_male_worn_dress_shirt_var_gray"),
        ("4", "overlay_male_worn_polo_shirt"),
        ("5", "overlay_male_worn_longshirt"),
    ]
    f_shirts = [
        ("0", "overlay_female_worn_tunic"),
        ("1", "overlay_female_worn_dress_shirt_var_white"),
        ("2", "overlay_female_worn_dress_shirt_var_blue"),
        ("3", "overlay_female_worn_dress_shirt_var_gray"),
        ("4", "overlay_female_worn_polo_shirt"),
        ("5", "overlay_female_worn_longshirt"),
    ]
    for s_tag, tid in m_shirts:
        add_sprite(f"pd_shirt_m_{s_tag}", tid)
    for s_tag, tid in f_shirts:
        add_sprite(f"pd_shirt_f_{s_tag}", tid)

    add_sprite("pd_vest_cloth", "overlay_worn_vest")
    add_sprite("pd_vest_leather", "overlay_worn_vest_leather")
    add_sprite("pd_apron_white", "overlay_worn_apron_cotton")
    add_sprite("pd_apron_floral", "overlay_worn_apron_cotton_var_floral_apron_cotton")
    add_sprite("pd_apron_leather", "overlay_worn_apron_leather")
    add_sprite("pd_gloves_leather", "overlay_worn_gloves_leather")
    add_sprite("pd_coat_lab", "overlay_worn_coat_lab")

    add_sprite("pd_hat_straw", "overlay_worn_straw_hat")
    add_sprite("pd_pack_basket", "overlay_worn_straw_basket")
    add_sprite("pd_pack_wicker", "overlay_worn_wicker_backpack")
    add_sprite("pd_pack_travel", "overlay_worn_backpack")

    add_sprite("pd_wield_blade", "overlay_wielded_knife_hunting")
    add_sprite("pd_wield_alloy_blade", "overlay_wielded_machete")
    add_sprite("pd_wield_pick", "overlay_wielded_hoe")
    add_sprite("pd_wield_alloy_pick", "overlay_wielded_pickaxe")

    # Shelf / row-packing into power-of-2-friendly clean atlas with 2px padding
    pad = 2
    max_w = 1024
    items_sorted = sorted(extracted, key=lambda x: (-x[1].height, -x[1].width, x[0]))
    cur_x, cur_y, row_h = pad, pad, 0
    placements = {}
    for key, sp, ox, oy in items_sorted:
        if cur_x + sp.width + pad > max_w:
            cur_x = pad
            cur_y += row_h + pad
            row_h = 0
        placements[key] = (cur_x, cur_y, sp, ox, oy)
        cur_x += sp.width + pad
        row_h = max(row_h, sp.height)

    atlas_h = cur_y + row_h + pad
    atlas = Image.new("RGBA", (max_w, atlas_h), (0, 0, 0, 0))
    meta_frames = {}
    for key, (x, y, sp, ox, oy) in sorted(placements.items()):
        atlas.paste(sp, (x, y))
        meta_frames[key] = {
            "x": x,
            "y": y,
            "w": sp.width,
            "h": sp.height,
            "ox": ox,
            "oy": oy,
        }

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    atlas.save(OUT_DIR / "atlas.png", optimize=True)
    with open(OUT_DIR / "atlas.json", "w", encoding="utf-8") as f:
        json.dump({"tile_w": 32, "tile_h": 32, "frames": meta_frames}, f, ensure_ascii=False, indent=2)

    print(f"Packed {len(meta_frames)} sprites -> {OUT_DIR / 'atlas.png'} ({max_w}x{atlas_h})")

    # 生成綜合離線驗收圖 /dev/shm/cdda_autotile_verify.png
    v = Image.new("RGBA", (1120, 660), (46, 58, 42, 255))
    vd = ImageDraw.Draw(v)

    def blit2(k, gx, gy, px0, py0, dx=0, dy=0):
        fr = meta_frames[k]
        sp = atlas.crop((fr["x"], fr["y"], fr["x"] + fr["w"], fr["y"] + fr["h"])).resize(
            (fr["w"] * 2, fr["h"] * 2), Image.Resampling.NEAREST
        )
        v.alpha_composite(sp, (px0 + gx * 64 + fr["ox"] * 2 + dx, py0 + gy * 64 + fr["oy"] * 2 + dy))

    vd.text((20, 10), "1. 6x5 商住一體木屋+門+灶+鐵砧+貨架+榻席+床+街燈 (2x)", fill=(245, 240, 215, 255))
    bx, by = 20, 32
    layout = [
        "######",
        "#..#.#",
        "##D#.#",
        "#....#",
        "###D##",
    ]
    for ry in range(6):
        for rx in range(7):
            blit2("ground_grass_0", rx, ry, bx, by)
            if ry == 5:
                blit2("ground_paving_15", rx, ry, bx, by)
    for ry in range(5):
        for rx in range(6):
            ch = layout[ry][rx]
            if ry <= 2:
                blit2("ground_tatami_0", rx, ry, bx, by)
            else:
                blit2("ground_floor", rx, ry, bx, by)
            if ch == "#":
                m = 0
                if ry > 0 and layout[ry-1][rx] in ("#", "D"): m |= 1
                if rx < 5 and layout[ry][rx+1] in ("#", "D"): m |= 2
                if ry < 4 and layout[ry+1][rx] in ("#", "D"): m |= 4
                if rx > 0 and layout[ry][rx-1] in ("#", "D"): m |= 8
                blit2(f"wall_wood_{m}", rx, ry, bx, by)
            elif ch == "D":
                blit2("door_closed" if ry == 2 else "door_open", rx, ry, bx, by)
    blit2("struct_woodstove", 1, 3, bx, by)
    blit2("struct_anvil", 2, 3, bx, by)
    blit2("struct_workbench", 4, 3, bx, by)
    blit2("struct_rack", 4, 2, bx, by)
    blit2("item_roast_meat", 4, 2, bx, by)
    blit2("struct_bed", 1, 1, bx, by)
    blit2("struct_street_light", 6, 5, bx, by)

    # 2) 6x5 紅瓦屋頂 + 農田五階 + 竹林溪流木橋 + 野獸
    vd.text((510, 10), "2. 紅瓦屋頂 + 竹林溪橋 + 農田五階 + 山豬/竹雞 (2x)", fill=(245, 240, 215, 255))
    rx0, ry0 = 510, 32
    for ry in range(5):
        for rx in range(5):
            blit2("ground_grass_1", rx, ry, rx0, ry0)
            m = 0
            if ry > 0: m |= 1
            if rx < 4: m |= 2
            if ry < 4: m |= 4
            if rx > 0: m |= 8
            blit2(f"roof_tile_{m}", rx, ry, rx0, ry0)
    # Stream + bridge + bamboo + farm stages
    fx0, fy0 = 510, 380
    for rx in range(8):
        blit2("ground_grass_2", rx, 0, fx0, fy0)
        blit2("ground_water_sh_10", rx, 1, fx0, fy0)
    blit2("ground_bridge", 3, 1, fx0, fy0)
    blit2("prop_bamboo_tall", 0, 0, fx0, fy0)
    blit2("prop_shoot", 1, 0, fx0, fy0)
    blit2("beast_boar", 5, 0, fx0, fy0)
    blit2("beast_pheasant", 6, 0, fx0, fy0)
    for i, fk in enumerate(["farm_ridge", "farm_seed", "farm_seedling", "farm_mature", "farm_harvest"]):
        blit2("ground_red", i, 2, fx0, fy0)
        blit2("farm_ridge", i, 2, fx0, fy0)
        if i > 0:
            blit2(fk, i, 2, fx0, fy0)

    # 3) 紙娃娃角色實裝預覽 + 全 16 種道具一覽
    vd.text((20, 420), "3. 紙娃娃全圖層實測 + 16 種物資道具 (2x)", fill=(245, 240, 215, 255))
    dolls = [
        ("林阿木", ["pd_shadow", "pd_body_m", "pd_eye_brown", "pd_shoes_hiking", "pd_pants_cargo", "pd_shirt_m_3", "pd_apron_leather", "pd_gloves_leather", "pd_hair_m_crewcut_black", "pd_wield_alloy_pick"]),
        ("劉春水", ["pd_shadow", "pd_body_f", "pd_eye_hazel", "pd_shoes_dress", "pd_pants_work", "pd_shirt_f_0", "pd_apron_white", "pd_hair_f_long_black", "pd_wield_blade"]),
        ("張濟安", ["pd_shadow", "pd_body_m", "pd_eye_brown", "pd_shoes_dress", "pd_pants_work", "pd_shirt_m_2", "pd_coat_lab", "pd_hair_m_medium_gray"]),
        ("周阿茶", ["pd_shadow", "pd_body_f", "pd_eye_amber", "pd_shoes_dress", "pd_pants_work", "pd_shirt_f_5", "pd_apron_floral", "pd_hair_f_medium_black"]),
        ("戴笠獵人", ["pd_shadow", "pd_body_m", "pd_eye_brown", "pd_shoes_boots", "pd_pants_jeans", "pd_shirt_m_0", "pd_vest_leather", "pd_pack_wicker", "pd_hair_m_short_black", "pd_hat_straw", "pd_wield_alloy_blade"]),
        ("採藥女", ["pd_shadow", "pd_body_f", "pd_eye_hazel", "pd_shoes_boots", "pd_pants_cargo", "pd_shirt_f_1", "pd_vest_cloth", "pd_pack_basket", "pd_hair_f_ponytail_black", "pd_hat_straw", "pd_wield_pick"]),
    ]
    for di, (dname, layers) in enumerate(dolls):
        px = 20 + di * 80
        py = 456
        for lk in layers:
            blit2(lk, 0, 0, px, py)
        vd.text((px + 6, py + 72), dname, fill=(235, 235, 220, 255))

    for ii, ik in enumerate(item_map):
        ix = 20 + ii * 48
        iy = 570
        fr = meta_frames[ik]
        sp = atlas.crop((fr["x"], fr["y"], fr["x"] + fr["w"], fr["y"] + fr["h"])).resize(
            (fr["w"] * 2, fr["h"] * 2), Image.Resampling.NEAREST
        )
        v.alpha_composite(sp, (ix, iy))

    v.save(VERIFY_PATH)
    print(f"Verification image saved -> {VERIFY_PATH}")


if __name__ == "__main__":
    main()
