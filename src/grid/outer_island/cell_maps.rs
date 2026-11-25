//! 銀鷗島手繪單格／單室地景輿圖註冊表（支援地塊外景 `cell_XXXX.webp` 與室內／地底細部 `node_id` 雙層解析）。

pub(crate) struct CellMapSpec {
    pub url: &'static str,
    pub bg_day: &'static str,
    pub bg_night: &'static str,
    #[expect(dead_code, reason = "retained for atlas historical metadata")]
    pub is_subterranean: bool,
    pub glows: &'static [[u16; 3]],
}

pub(crate) struct DungeonTileSpec {
    pub tile_url: &'static str,
    pub room_url: &'static str,
    pub box_xywh: [u16; 4],
    pub anchor: [u16; 2],
    pub label_pos: [u16; 2],
}

const GLOW_VILLAGE_STD: &[[u16; 3]] = &[[460, 470, 105], [560, 360, 85]];
const GLOW_VILLAGE_LARGE: &[[u16; 3]] = &[[450, 460, 115], [320, 360, 90], [590, 560, 90]];
const GLOW_KILN: &[[u16; 3]] = &[[380, 430, 110], [550, 520, 85]];
const GLOW_QUARANTINE: &[[u16; 3]] = &[[450, 480, 115], [450, 310, 85]];
const GLOW_SURVIVOR: &[[u16; 3]] = &[[480, 510, 80]];
const GLOW_NONE: &[[u16; 3]] = &[];

/// 節點級專屬細部圖（如地底試煉坑 6 石室，以及後續村莊茶棚／民宅與五大城鎮街鋪室內圖）。
#[expect(dead_code, reason = "retained for atlas historical metadata")]
pub(crate) fn node_illustrated_map_spec(node_id: &str) -> Option<CellMapSpec> {
    if let Some(sp) = dungeon_room_tile_spec(node_id) {
        return Some(CellMapSpec {
            url: sp.room_url,
            bg_day: "#12100e",
            bg_night: "#12100e",
            is_subterranean: true,
            glows: GLOW_NONE,
        });
    }
    None
}

/// 地塊級 1:1 手繪外景輿圖（涵蓋全部 29 座活態鄉鎮村塢、5 處南岸封村遺址與野外地塊）。
pub(crate) fn cell_illustrated_map_spec(cell_id: u32) -> Option<CellMapSpec> {
    match cell_id {
        // ── 首批 4 處代表地塊 ──
        4432 => Some(CellMapSpec {
            url: "/maps/cells/cell_4432.webp?v=20261123r3",
            bg_day: "#dfd4bc",
            bg_night: "#161914",
            is_subterranean: false,
            glows: &[[520, 575, 95], [575, 315, 105]],
        }),
        4529 => Some(CellMapSpec {
            url: "/maps/cells/cell_4529.webp?v=20261123r3",
            bg_day: "#d6ccb8",
            bg_night: "#141618",
            is_subterranean: false,
            glows: GLOW_NONE,
        }),
        4617 => Some(CellMapSpec {
            url: "/maps/cells/cell_4617.webp?v=20261123r3",
            bg_day: "#dcd3be",
            bg_night: "#131b21",
            is_subterranean: false,
            glows: GLOW_NONE,
        }),
        4698 => Some(CellMapSpec {
            url: "/maps/cells/cell_4698.webp?v=20261123r3",
            bg_day: "#cfc8bc",
            bg_night: "#161518",
            is_subterranean: false,
            glows: GLOW_NONE,
        }),

        // ── 28 座活態鄉野村莊與檢疫據點（第一階段全數換裝） ──
        4069 => Some(CellMapSpec {
            url: "/maps/cells/cell_4069.webp?v=20261123r3",
            bg_day: "#d9d0bd",
            bg_night: "#14181b",
            is_subterranean: false,
            glows: GLOW_VILLAGE_STD,
        }),
        4071 => Some(CellMapSpec {
            url: "/maps/cells/cell_4071.webp?v=20261123r3",
            bg_day: "#dad2be",
            bg_night: "#141918",
            is_subterranean: false,
            glows: GLOW_KILN,
        }),
        4072 => Some(CellMapSpec {
            url: "/maps/cells/cell_4072.webp?v=20261123r3",
            bg_day: "#d5cdbc",
            bg_night: "#13181c",
            is_subterranean: false,
            glows: GLOW_VILLAGE_LARGE,
        }),
        4077 => Some(CellMapSpec {
            url: "/maps/cells/cell_4077.webp?v=20261123r3",
            bg_day: "#dfd5bf",
            bg_night: "#141a1e",
            is_subterranean: false,
            glows: GLOW_VILLAGE_STD,
        }),
        4081 => Some(CellMapSpec {
            url: "/maps/cells/cell_4081.webp?v=20261123r3",
            bg_day: "#dbd3bf",
            bg_night: "#14191b",
            is_subterranean: false,
            glows: GLOW_KILN,
        }),
        4083 => Some(CellMapSpec {
            url: "/maps/cells/cell_4083.webp?v=20261123r3",
            bg_day: "#ded4be",
            bg_night: "#131b20",
            is_subterranean: false,
            glows: GLOW_VILLAGE_LARGE,
        }),
        4194 => Some(CellMapSpec {
            url: "/maps/cells/cell_4194.webp?v=20261123r3",
            bg_day: "#dfd5be",
            bg_night: "#141a1e",
            is_subterranean: false,
            glows: GLOW_VILLAGE_LARGE,
        }),
        4195 => Some(CellMapSpec {
            url: "/maps/cells/cell_4195.webp?v=20261123r3",
            bg_day: "#e0d6bf",
            bg_night: "#141a1e",
            is_subterranean: false,
            glows: GLOW_VILLAGE_STD,
        }),
        4196 => Some(CellMapSpec {
            url: "/maps/cells/cell_4196.webp?v=20261123r3",
            bg_day: "#dcd3be",
            bg_night: "#131b20",
            is_subterranean: false,
            glows: GLOW_VILLAGE_STD,
        }),
        4198 => Some(CellMapSpec {
            url: "/maps/cells/cell_4198.webp?v=20261123r3",
            bg_day: "#d7cebc",
            bg_night: "#161718",
            is_subterranean: false,
            glows: GLOW_VILLAGE_STD,
        }),
        4200 => Some(CellMapSpec {
            url: "/maps/cells/cell_4200.webp?v=20261123r3",
            bg_day: "#ddd4bd",
            bg_night: "#161915",
            is_subterranean: false,
            glows: GLOW_VILLAGE_LARGE,
        }),
        4201 => Some(CellMapSpec {
            url: "/maps/cells/cell_4201.webp?v=20261123r3",
            bg_day: "#dfd5be",
            bg_night: "#161915",
            is_subterranean: false,
            glows: GLOW_VILLAGE_LARGE,
        }),
        4204 => Some(CellMapSpec {
            url: "/maps/cells/cell_4204.webp?v=20261123r3",
            bg_day: "#dcd3be",
            bg_night: "#131b21",
            is_subterranean: false,
            glows: GLOW_VILLAGE_STD,
        }),
        4206 => Some(CellMapSpec {
            url: "/maps/cells/cell_4206.webp?v=20261123r3",
            bg_day: "#dcd3bd",
            bg_night: "#131b21",
            is_subterranean: false,
            glows: GLOW_VILLAGE_STD,
        }),
        4324 => Some(CellMapSpec {
            url: "/maps/cells/cell_4324.webp?v=20261123r3",
            bg_day: "#d9d0ba",
            bg_night: "#161914",
            is_subterranean: false,
            glows: GLOW_KILN,
        }),
        4327 => Some(CellMapSpec {
            url: "/maps/cells/cell_4327.webp?v=20261123r3",
            bg_day: "#d8cfba",
            bg_night: "#151914",
            is_subterranean: false,
            glows: GLOW_VILLAGE_LARGE,
        }),
        4425 => Some(CellMapSpec {
            url: "/maps/cells/cell_4425.webp?v=20261123r3",
            bg_day: "#dfd5be",
            bg_night: "#131a20",
            is_subterranean: false,
            glows: GLOW_VILLAGE_STD,
        }),
        4427 => Some(CellMapSpec {
            url: "/maps/cells/cell_4427.webp?v=20261123r3",
            bg_day: "#dfd4bc",
            bg_night: "#161915",
            is_subterranean: false,
            glows: GLOW_VILLAGE_LARGE,
        }),
        4428 => Some(CellMapSpec {
            url: "/maps/cells/cell_4428.webp?v=20261123r3",
            bg_day: "#e0d6be",
            bg_night: "#161915",
            is_subterranean: false,
            glows: GLOW_VILLAGE_STD,
        }),
        4429 => Some(CellMapSpec {
            url: "/maps/cells/cell_4429.webp?v=20261123r3",
            bg_day: "#d8ceba",
            bg_night: "#161816",
            is_subterranean: false,
            glows: GLOW_KILN,
        }),
        4433 => Some(CellMapSpec {
            url: "/maps/cells/cell_4433.webp?v=20261123r3",
            bg_day: "#ddd3be",
            bg_night: "#131b21",
            is_subterranean: false,
            glows: GLOW_VILLAGE_STD,
        }),
        4530 => Some(CellMapSpec {
            url: "/maps/cells/cell_4530.webp?v=20261123r3",
            bg_day: "#dfd5be",
            bg_night: "#131a20",
            is_subterranean: false,
            glows: GLOW_VILLAGE_STD,
        }),
        4614 => Some(CellMapSpec {
            url: "/maps/cells/cell_4614.webp?v=20261123r3",
            bg_day: "#d5cdbb",
            bg_night: "#151718",
            is_subterranean: false,
            glows: GLOW_VILLAGE_STD,
        }),
        4695 => Some(CellMapSpec {
            url: "/maps/cells/cell_4695.webp?v=20261123r3",
            bg_day: "#dad0bc",
            bg_night: "#15181c",
            is_subterranean: false,
            glows: GLOW_QUARANTINE,
        }),
        4776 => Some(CellMapSpec {
            url: "/maps/cells/cell_4776.webp?v=20261123r3",
            bg_day: "#d6cebc",
            bg_night: "#161816",
            is_subterranean: false,
            glows: GLOW_SURVIVOR,
        }),
        4777 => Some(CellMapSpec {
            url: "/maps/cells/cell_4777.webp?v=20261123r3",
            bg_day: "#d6cebc",
            bg_night: "#161816",
            is_subterranean: false,
            glows: GLOW_SURVIVOR,
        }),
        4853 => Some(CellMapSpec {
            url: "/maps/cells/cell_4853.webp?v=20261123r3",
            bg_day: "#dfd5be",
            bg_night: "#131b20",
            is_subterranean: false,
            glows: GLOW_VILLAGE_LARGE,
        }),
        4926 => Some(CellMapSpec {
            url: "/maps/cells/cell_4926.webp?v=20261123r3",
            bg_day: "#ded4be",
            bg_night: "#131b20",
            is_subterranean: false,
            glows: GLOW_VILLAGE_STD,
        }),

        // ── 其餘 4 處南岸船疫封村遺址（0 活人、0 刷怪、零燈火） ──
        4775 => Some(CellMapSpec {
            url: "/maps/cells/cell_4775.webp?v=20261123r3",
            bg_day: "#cec7bb",
            bg_night: "#141619",
            is_subterranean: false,
            glows: GLOW_NONE,
        }),
        4780 => Some(CellMapSpec {
            url: "/maps/cells/cell_4780.webp?v=20261123r3",
            bg_day: "#cfc8bc",
            bg_night: "#141619",
            is_subterranean: false,
            glows: GLOW_NONE,
        }),
        4847 => Some(CellMapSpec {
            url: "/maps/cells/cell_4847.webp?v=20261123r3",
            bg_day: "#cdc7bb",
            bg_night: "#131619",
            is_subterranean: false,
            glows: GLOW_NONE,
        }),
        4849 => Some(CellMapSpec {
            url: "/maps/cells/cell_4849.webp?v=20261123r3",
            bg_day: "#cbc4b8",
            bg_night: "#131518",
            is_subterranean: false,
            glows: GLOW_NONE,
        }),

        // ── 第二批 52 處官道走廊與山海荒野網狀地景輿圖（100% 全島 93/93 格滿覆蓋） ──
        4068 => Some(wild_spec("/maps/cells/cell_4068.webp?v=20261123r5", true)),
        4070 => Some(wild_spec("/maps/cells/cell_4070.webp?v=20261123r5", true)),
        4074 => Some(wild_spec("/maps/cells/cell_4074.webp?v=20261123r5", true)),
        4075 => Some(wild_spec("/maps/cells/cell_4075.webp?v=20261123r5", true)),
        4076 => Some(wild_spec("/maps/cells/cell_4076.webp?v=20261123r5", false)),
        4082 => Some(wild_spec("/maps/cells/cell_4082.webp?v=20261123r5", true)),
        4084 => Some(wild_spec("/maps/cells/cell_4084.webp?v=20261123r5", true)),
        4193 => Some(wild_spec("/maps/cells/cell_4193.webp?v=20261123r5", true)),
        4197 => Some(wild_spec("/maps/cells/cell_4197.webp?v=20261123r5", false)),
        4199 => Some(wild_spec("/maps/cells/cell_4199.webp?v=20261123r5", false)),
        4202 => Some(wild_spec("/maps/cells/cell_4202.webp?v=20261123r5", true)),
        4203 => Some(wild_spec("/maps/cells/cell_4203.webp?v=20261123r5", true)),
        4205 => Some(wild_spec("/maps/cells/cell_4205.webp?v=20261123r5", true)),
        4207 => Some(wild_spec("/maps/cells/cell_4207.webp?v=20261123r5", true)),
        4322 => Some(wild_spec("/maps/cells/cell_4322.webp?v=20261123r5", true)),
        4323 => Some(wild_spec("/maps/cells/cell_4323.webp?v=20261123r5", false)),
        4325 => Some(wild_spec("/maps/cells/cell_4325.webp?v=20261123r5", false)),
        4326 => Some(wild_spec("/maps/cells/cell_4326.webp?v=20261123r5", false)),
        4328 => Some(wild_spec("/maps/cells/cell_4328.webp?v=20261123r5", false)),
        4329 => Some(wild_spec("/maps/cells/cell_4329.webp?v=20261123r5", true)),
        4426 => Some(wild_spec("/maps/cells/cell_4426.webp?v=20261123r5", true)),
        4430 => Some(wild_spec("/maps/cells/cell_4430.webp?v=20261123r5", false)),
        4431 => Some(wild_spec("/maps/cells/cell_4431.webp?v=20261123r5", false)),
        4434 => Some(wild_spec("/maps/cells/cell_4434.webp?v=20261123r5", true)),
        4521 => Some(wild_spec("/maps/cells/cell_4521.webp?v=20261123r5", true)),
        4522 => Some(wild_spec("/maps/cells/cell_4522.webp?v=20261123r5", true)),
        4523 => Some(wild_spec("/maps/cells/cell_4523.webp?v=20261123r5", true)),
        4524 => Some(wild_spec("/maps/cells/cell_4524.webp?v=20261123r5", true)),
        4526 => Some(wild_spec("/maps/cells/cell_4526.webp?v=20261123r5", false)),
        4527 => Some(wild_spec("/maps/cells/cell_4527.webp?v=20261123r5", false)),
        4528 => Some(wild_spec("/maps/cells/cell_4528.webp?v=20261123r5", false)),
        4611 => Some(wild_spec("/maps/cells/cell_4611.webp?v=20261123r5", true)),
        4612 => Some(wild_spec("/maps/cells/cell_4612.webp?v=20261123r5", true)),
        4615 => Some(wild_spec("/maps/cells/cell_4615.webp?v=20261123r5", false)),
        4693 => Some(wild_spec("/maps/cells/cell_4693.webp?v=20261123r5", true)),
        4694 => Some(wild_spec("/maps/cells/cell_4694.webp?v=20261123r5", false)),
        4696 => Some(wild_spec("/maps/cells/cell_4696.webp?v=20261123r5", false)),
        4697 => Some(wild_spec("/maps/cells/cell_4697.webp?v=20261123r5", true)),
        4778 => Some(wild_spec("/maps/cells/cell_4778.webp?v=20261123r5", true)),
        4779 => Some(wild_spec("/maps/cells/cell_4779.webp?v=20261123r5", true)),
        4781 => Some(wild_spec("/maps/cells/cell_4781.webp?v=20261123r5", false)),
        4846 => Some(wild_spec("/maps/cells/cell_4846.webp?v=20261123r5", true)),
        4848 => Some(wild_spec("/maps/cells/cell_4848.webp?v=20261123r5", true)),
        4850 => Some(wild_spec("/maps/cells/cell_4850.webp?v=20261123r5", true)),
        4851 => Some(wild_spec("/maps/cells/cell_4851.webp?v=20261123r5", false)),
        4852 => Some(wild_spec("/maps/cells/cell_4852.webp?v=20261123r5", true)),
        4927 => Some(wild_spec("/maps/cells/cell_4927.webp?v=20261123r5", true)),
        5006 => Some(wild_spec("/maps/cells/cell_5006.webp?v=20261123r5", true)),
        5007 => Some(wild_spec("/maps/cells/cell_5007.webp?v=20261123r5", true)),
        5008 => Some(wild_spec("/maps/cells/cell_5008.webp?v=20261123r5", true)),
        5009 => Some(wild_spec("/maps/cells/cell_5009.webp?v=20261123r5", true)),
        5079 => Some(wild_spec("/maps/cells/cell_5079.webp?v=20261123r5", true)),

        _ => None,
    }
}

const fn wild_spec(url: &'static str, is_coast: bool) -> CellMapSpec {
    CellMapSpec {
        url,
        bg_day: if is_coast { "#dcd3be" } else { "#d8cfba" },
        bg_night: if is_coast { "#121a21" } else { "#141815" },
        is_subterranean: false,
        glows: GLOW_NONE,
    }
}

pub(crate) fn dungeon_room_tile_spec(node_id: &str) -> Option<DungeonTileSpec> {
    match node_id {
        "oi:dng_pit:entry" => Some(DungeonTileSpec {
            tile_url: "/maps/trial_pit/seg_c181917d323d.webp?v=20261123r3",
            room_url: "/maps/trial_pit/rm_8a41e207c9b3.webp?v=20261123r3",
            box_xywh: [0, 0, 318, 312],
            anchor: [164, 178],
            label_pos: [152, 268],
        }),
        "oi:dng_pit:hall" => Some(DungeonTileSpec {
            tile_url: "/maps/trial_pit/seg_f584bfecac23.webp?v=20261123r3",
            room_url: "/maps/trial_pit/rm_3d7c95f104e8.webp?v=20261123r3",
            box_xywh: [263, 0, 360, 330],
            anchor: [454, 162],
            label_pos: [454, 46],
        }),
        "oi:dng_pit:side" => Some(DungeonTileSpec {
            tile_url: "/maps/trial_pit/seg_e1736f150e10.webp?v=20261123r3",
            room_url: "/maps/trial_pit/rm_6f19b82a5d41.webp?v=20261123r3",
            box_xywh: [1, 315, 301, 270],
            anchor: [152, 462],
            label_pos: [138, 542],
        }),
        "oi:dng_pit:stele" => Some(DungeonTileSpec {
            tile_url: "/maps/trial_pit/seg_b4241cd9c5bc.webp?v=20261123r3",
            room_url: "/maps/trial_pit/rm_c450e73b91a6.webp?v=20261123r3",
            box_xywh: [251, 271, 387, 363],
            anchor: [450, 438],
            label_pos: [450, 588],
        }),
        "oi:dng_pit:lair" => Some(DungeonTileSpec {
            tile_url: "/maps/trial_pit/seg_18dd21289df4.webp?v=20261123r3",
            room_url: "/maps/trial_pit/rm_52b8d094e7c2.webp?v=20261123r3",
            box_xywh: [584, 260, 316, 376],
            anchor: [760, 452],
            label_pos: [760, 336],
        }),
        "oi:dng_pit:altar" => Some(DungeonTileSpec {
            tile_url: "/maps/trial_pit/seg_97c8bfa33e77.webp?v=20261123r3",
            room_url: "/maps/trial_pit/rm_9e36a418f0b7.webp?v=20261123r3",
            box_xywh: [565, 581, 335, 319],
            anchor: [748, 696],
            label_pos: [748, 878],
        }),
        _ => None,
    }
}
