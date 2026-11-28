//! 第二刀核心器官：全身多部位裝備＝生存與工作工具組 × 材質物理向量 × 零門檻天然原材起手 × 百工自由造物。
//!
//! 嚴守 `DESIGN_KEYWORDS.md` L145 / L154 / L156 / L216：
//! 1. 後台以真實材質物理向量（硬度／韌性／重量／導能率）演算防反震、涉水省力與負重擴容；
//! 2. 前台嚴禁外露原始數值，全數轉譯為台灣白話手感與實效敘述；
//! 3. 野外可徒手拾取硬石、枯枝、韌藤就地綁製初階工具與護具（徹底破除作 A 缺 B 死鎖）；
//! 4. 鍛鐵鋪／百工工坊支援「形制插座 × 自由選材」打造唯一落款器物，所有材料與銅幣嚴格守恆。

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::clock_and_lang::SolarClockSnapshot;
use super::mind::EventPublicity;
use super::movement::{add_carry_item, push_breath, take_carry_item};
use super::persona::ValueAxis;
use super::shop_leases::{actor_operates_shop, infer_shop_kind};
use super::sim_tick::broadcast_scene_event;
use super::types::{
    BoundTool, CarryItem, IslandWorldDelta, SpatialNodeDef, UnifiedMindBody, WorldStaticDoc,
};

/// 後台真實材質物理向量（0..=100，前台僅轉譯為體感白話，絕不直接外露數字）
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct MaterialPhysics {
    pub hardness: u8,
    pub toughness: u8,
    pub weight: u8,
    pub conductivity: u8,
}

/// 穿戴於雙臂、足腿或軀幹的實體裝備件（與背囊 `CarryItem.gear_meta` 守恆流轉）
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct BodyGearPiece {
    pub uid: String,
    pub slug: String,
    pub slot: String, // "arms" | "feet" | "torso"
    pub name: String,
    pub material_slug: String,
    pub material_zh: String,
    pub provenance: String,
    pub wear_stage: u8,
    pub physics: MaterialPhysics,
}

/// 全身多部位工具與護具槽（雙手所持沿用 `bound_weapon` 保持 100% 向下相容）
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct BodyGearRig {
    pub arms: Option<BodyGearPiece>,
    pub feet: Option<BodyGearPiece>,
    pub torso: Option<BodyGearPiece>,
}

pub struct MaterialSpec {
    pub slug: &'static str,
    pub name_zh: &'static str,
    pub prefix_zh: &'static str,
    pub physics: MaterialPhysics,
}

pub const CRAFT_MATERIALS: &[MaterialSpec] = &[
    MaterialSpec {
        slug: "river_flint",
        name_zh: "河灘硬石",
        prefix_zh: "硬石",
        physics: MaterialPhysics {
            hardness: 58,
            toughness: 36,
            weight: 48,
            conductivity: 18,
        },
    },
    MaterialSpec {
        slug: "dry_branch",
        name_zh: "枯枝木桿",
        prefix_zh: "硬木",
        physics: MaterialPhysics {
            hardness: 28,
            toughness: 46,
            weight: 22,
            conductivity: 24,
        },
    },
    MaterialSpec {
        slug: "tough_vine",
        name_zh: "韌藤纖維",
        prefix_zh: "韌藤",
        physics: MaterialPhysics {
            hardness: 18,
            toughness: 64,
            weight: 14,
            conductivity: 26,
        },
    },
    MaterialSpec {
        slug: "driftwood",
        name_zh: "異國刻紋漂流木",
        prefix_zh: "沉木",
        physics: MaterialPhysics {
            hardness: 46,
            toughness: 68,
            weight: 30,
            conductivity: 44,
        },
    },
    MaterialSpec {
        slug: "timber_log",
        name_zh: "山林原木",
        prefix_zh: "松木",
        physics: MaterialPhysics {
            hardness: 48,
            toughness: 62,
            weight: 42,
            conductivity: 30,
        },
    },
    MaterialSpec {
        slug: "animal_hide",
        name_zh: "熟鞣獸皮",
        prefix_zh: "厚皮",
        physics: MaterialPhysics {
            hardness: 34,
            toughness: 76,
            weight: 24,
            conductivity: 34,
        },
    },
    MaterialSpec {
        slug: "iron_ore",
        name_zh: "粗鐵礦石",
        prefix_zh: "鍛鐵",
        physics: MaterialPhysics {
            hardness: 78,
            toughness: 68,
            weight: 72,
            conductivity: 48,
        },
    },
    MaterialSpec {
        slug: "quartz_sand",
        name_zh: "高純石英砂",
        prefix_zh: "晶砂",
        physics: MaterialPhysics {
            hardness: 66,
            toughness: 56,
            weight: 38,
            conductivity: 78,
        },
    },
    MaterialSpec {
        slug: "spider_chitin",
        name_zh: "吞石巨蛛甲片",
        prefix_zh: "蛛甲",
        physics: MaterialPhysics {
            hardness: 86,
            toughness: 80,
            weight: 44,
            conductivity: 66,
        },
    },
    MaterialSpec {
        slug: "spider_carapace",
        name_zh: "吞石巨蛛厚甲殼",
        prefix_zh: "巨蛛甲",
        physics: MaterialPhysics {
            hardness: 88,
            toughness: 82,
            weight: 46,
            conductivity: 68,
        },
    },
    MaterialSpec {
        slug: "lizard_scale",
        name_zh: "嗜石岩蜥厚鱗皮",
        prefix_zh: "岩鱗",
        physics: MaterialPhysics {
            hardness: 74,
            toughness: 78,
            weight: 38,
            conductivity: 54,
        },
    },
    MaterialSpec {
        slug: "black_coal",
        name_zh: "北林精煤",
        prefix_zh: "滲炭",
        physics: MaterialPhysics {
            hardness: 72,
            toughness: 60,
            weight: 54,
            conductivity: 42,
        },
    },
];

pub fn lookup_material_spec(slug: &str) -> Option<&'static MaterialSpec> {
    CRAFT_MATERIALS.iter().find(|m| m.slug == slug)
}

/// 後台物理數據 100% 轉譯為前台去數值白話手感描述
pub fn describe_gear_physics_zh(phys: MaterialPhysics, slot: &str) -> String {
    let hard = if phys.hardness >= 75 {
        "質地極為堅硬"
    } else if phys.hardness >= 50 {
        "骨幹結實耐磕"
    } else {
        "質地偏軟貼身"
    };
    let tough = if phys.toughness >= 72 {
        "韌性極佳不易崩裂"
    } else if phys.toughness >= 52 {
        "耐拉扯抗反震"
    } else {
        "須留神重擊磨損"
    };
    let weight = if phys.weight >= 65 {
        "分量沉手"
    } else if phys.weight >= 36 {
        "份量適中"
    } else {
        "輕便俐落"
    };
    let cond = if phys.conductivity >= 60 {
        "・導能溫潤流暢"
    } else if phys.conductivity >= 40 {
        "・隱約可導引氣息"
    } else {
        ""
    };
    let effect = match slot {
        "arms" => "護腕卸震",
        "feet" => "涉水踏坡省力",
        "torso" => "擴增行囊負重",
        _ => "順手趁力",
    };
    format!("{hard}、{tough}、{weight}{cond}（{effect}）")
}

/// 計算角色穿戴軀幹採集背架後的跨格大宗重貨上限（預設 2 件，穿戴背架依材質物理擴至 3～4 件）
pub fn effective_overland_bulk_cap(actor: &UnifiedMindBody) -> u32 {
    let Some(ref torso) = actor.body_gear.torso else {
        return 2;
    };
    if torso.physics.toughness >= 70 || (torso.physics.hardness >= 65 && torso.physics.toughness >= 58)
    {
        4
    } else if torso.physics.toughness >= 45 {
        3
    } else {
        2
    }
}

/// 足腿靴履對野外步行／涉水疲勞之真實減免量
pub fn feet_fatigue_reduction(actor: &UnifiedMindBody, is_flood: bool) -> u8 {
    let Some(ref feet) = actor.body_gear.feet else {
        return 0;
    };
    if is_flood {
        if feet.physics.toughness >= 65 {
            3
        } else {
            2
        }
    } else if feet.physics.toughness >= 45 {
        1
    } else {
        0
    }
}

/// 雙臂護具是否足以吸收重甲敵手反震（防手腕瘀腫並減輕交鋒疲勞）
pub fn arms_absorbs_wrist_recoil(actor: &UnifiedMindBody) -> bool {
    actor
        .body_gear
        .arms
        .as_ref()
        .is_some_and(|a| a.physics.toughness >= 50 || a.physics.hardness >= 55)
}

/// 零門檻野外徒手拾取天然原材（硬石、枯枝、韌藤——零工具門檻、每時段每處限一次防機械刷圖）
pub fn apply_gather_wild_raw(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
) -> Result<String, String> {
    let aid = delta
        .resolve_actor_id(actor_key)
        .ok_or("請先向港務官核對通行身分。")?;
    let node_id = delta
        .actors
        .get(&aid)
        .map(|a| a.node_id.clone())
        .ok_or("找不到旅人行蹤")?;
    let node = doc.nodes.get(&node_id).ok_or("四周霧氣迷濛，辨不清地勢。")?;
    if node.indoor || node.dungeon || matches!(node.kind.as_str(), "shop" | "house" | "house_vacant" | "inn_room") {
        return Err("屋舍街鋪之內無天然石枝可撿，出城到河灘、林緣或官道旁就能徒手拾取。".to_string());
    }
    let win_key = format!("raw:{}@{}", node_id, clock.six_phase_key);
    if delta
        .actors
        .get(&aid)
        .is_some_and(|a| a.explored_windows.contains(&win_key))
    {
        return Ok(
            "這片河灘林緣方才已仔細撿過一輪，順手的硬石與韌藤暫時收妥了，換個地界或下個時段再來瞧瞧。"
                .to_string(),
        );
    }

    let cell = doc.cells.get(&node.cell_id.to_string());
    let is_coast_or_river = cell.is_some_and(|c| c.is_coast || c.flux >= 15);
    let picked: Vec<(&str, &str, u16)> = if is_coast_or_river {
        vec![
            ("river_flint", "河灘硬石", 1),
            ("tough_vine", "韌藤纖維", 1),
            ("dry_branch", "枯枝木桿", 1),
        ]
    } else {
        vec![
            ("dry_branch", "枯枝木桿", 1),
            ("tough_vine", "韌藤纖維", 1),
            ("river_flint", "河灘硬石", 1),
        ]
    };

    let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
    super::live_tide::prune_stale_day_window_keys(&mut a.explored_windows, clock.game_days);
    a.explored_windows.insert(win_key);
    for (slug, name, w) in &picked {
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: (*slug).to_string(),
                name: (*name).to_string(),
                count: 1,
                weight_units: *w,
                ..CarryItem::default()
            },
        );
    }
    let (craft_prose, craft_save) = super::martial_art_and_manuals::invoke_craft_phase_effect(a, "harvest");
    a.fatigue = a.fatigue.saturating_add(6u8.saturating_sub(craft_save).max(3)).min(95);
    a.last_active_unix = clock.real_unix;
    let msg = format!(
        "你沿著路畔坡腳與碎石灘彎腰挑揀（{craft_prose}），徒手收起【河灘硬石】、【枯枝木桿】與【韌藤纖維】各一份，隨時可在行囊裡就地綁製初階工具與護具。"
    );
    push_breath(a, msg.clone());
    a.mark_mutated();
    Ok(msg)
}

/// 零門檻就地綁製初階工具與護具（隨時隨地單次決策完工，無需工坊或銅幣）
pub fn apply_field_bind_craft(
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    recipe: &str,
) -> Result<String, String> {
    let aid = delta
        .resolve_actor_id(actor_key)
        .ok_or("請先向港務官核對通行身分。")?;
    let reqs: &[(&str, u32)] = match recipe.trim() {
        "stone_hoe" => &[("river_flint", 1), ("dry_branch", 1), ("tough_vine", 1)],
        "stone_spear" => &[("river_flint", 1), ("dry_branch", 1), ("tough_vine", 1)],
        "vine_bracers" => &[("tough_vine", 2)],
        "reed_boots" => &[("tough_vine", 1), ("dry_branch", 1)],
        "vine_packframe" => &[("dry_branch", 2), ("tough_vine", 1)],
        _ => return Err("未知的就地綁製形制。".to_string()),
    };

    {
        let a = delta.actors.get(&aid).ok_or("找不到旅人行蹤")?;
        for &(slug, need) in reqs {
            let have: u32 = a
                .backpack
                .iter()
                .filter(|i| i.slug == slug && i.bound_owner.is_none() && i.stolen_from.is_none())
                .map(|i| i.count)
                .sum();
            if have < need {
                let mname = lookup_material_spec(slug).map_or(slug, |m| m.name_zh);
                return Err(format!(
                    "手頭【{mname}】尚不足 {need} 份，可先在城外河灘林緣徒手拾取原材。"
                ));
            }
        }
    }

    let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
    for &(slug, need) in reqs {
        let _ = take_carry_item(&mut a.backpack, slug, need);
    }
    a.mind.ltm.mastery.vessel_serial = a.mind.ltm.mastery.vessel_serial.saturating_add(1);
    let uid = format!("fb:{}:{}", a.actor_id, a.mind.ltm.mastery.vessel_serial);
    let maker = a.display_name.clone();
    let msg = match recipe.trim() {
        "stone_hoe" | "stone_spear" => {
            let (slug, name) = if recipe.trim() == "stone_hoe" {
                ("field_stone_hoe", "石刃木鋤")
            } else {
                ("field_stone_spear", "硬石長叉")
            };
            let tool = BoundTool {
                uid: uid.clone(),
                slug: slug.to_string(),
                name: name.to_string(),
                provenance: format!("由{maker}以河灘硬石、枯枝與韌藤就地綁製"),
                wear_stage: 0,
                grip_familiarity: 10,
                rune_mark: None,
            };
            add_carry_item(
                &mut a.backpack,
                CarryItem {
                    slug: slug.to_string(),
                    name: name.to_string(),
                    count: 1,
                    weight_units: 2,
                    vessel_meta: Some(tool),
                    ..CarryItem::default()
                },
            );
            format!("你熟練纏緊韌藤將硬石卡進木桿，就地綁成一把趁手的【{name}】，已收入行囊（可隨時換握上手）。")
        }
        "vine_bracers" => {
            let piece = BodyGearPiece {
                uid,
                slug: "gear_vine_bracers".to_string(),
                slot: "arms".to_string(),
                name: "粗藤護腕".to_string(),
                material_slug: "tough_vine".to_string(),
                material_zh: "韌藤纖維".to_string(),
                provenance: format!("由{maker}以雙股韌藤就地編緊"),
                wear_stage: 0,
                physics: MaterialPhysics {
                    hardness: 24,
                    toughness: 64,
                    weight: 14,
                    conductivity: 24,
                },
            };
            add_carry_item(
                &mut a.backpack,
                CarryItem {
                    slug: piece.slug.clone(),
                    name: piece.name.clone(),
                    count: 1,
                    weight_units: 1,
                    gear_meta: Some(piece),
                    ..CarryItem::default()
                },
            );
            "你將韌藤反覆絞編壓實，就地綁成一副【粗藤護腕】收入行囊（穿戴於雙臂可卸震護腕）。".to_string()
        }
        "reed_boots" => {
            let piece = BodyGearPiece {
                uid,
                slug: "gear_reed_boots".to_string(),
                slot: "feet".to_string(),
                name: "編藤涉水履".to_string(),
                material_slug: "tough_vine".to_string(),
                material_zh: "韌藤木片".to_string(),
                provenance: format!("由{maker}以枯枝削底、韌藤縛腳就地編成"),
                wear_stage: 0,
                physics: MaterialPhysics {
                    hardness: 30,
                    toughness: 58,
                    weight: 18,
                    conductivity: 22,
                },
            };
            add_carry_item(
                &mut a.backpack,
                CarryItem {
                    slug: piece.slug.clone(),
                    name: piece.name.clone(),
                    count: 1,
                    weight_units: 1,
                    gear_meta: Some(piece),
                    ..CarryItem::default()
                },
            );
            "你削平枯枝作底、以韌藤交叉捆牢，編成一雙【編藤涉水履】收入行囊（穿戴上腳可省下野外與涉水氣力）。".to_string()
        }
        _ => {
            let piece = BodyGearPiece {
                uid,
                slug: "gear_vine_packframe".to_string(),
                slot: "torso".to_string(),
                name: "藤編採集背架".to_string(),
                material_slug: "dry_branch".to_string(),
                material_zh: "枯枝韌藤".to_string(),
                provenance: format!("由{maker}以兩根硬枝架骨、韌藤紮肩帶編成"),
                wear_stage: 0,
                physics: MaterialPhysics {
                    hardness: 34,
                    toughness: 56,
                    weight: 22,
                    conductivity: 20,
                },
            };
            add_carry_item(
                &mut a.backpack,
                CarryItem {
                    slug: piece.slug.clone(),
                    name: piece.name.clone(),
                    count: 1,
                    weight_units: 2,
                    gear_meta: Some(piece),
                    ..CarryItem::default()
                },
            );
            "你以枯枝紮成背骨、絞藤作肩帶，就地編成一具【藤編採集背架】收入行囊（背上身可多扛一份大宗重貨）。".to_string()
        }
    };
    a.fatigue = (a.fatigue + 6).min(95);
    a.last_active_unix = clock.real_unix;
    push_breath(a, msg.clone());
    a.mark_mutated();
    Ok(msg)
}

/// 百工自由造物（形制插座 × 自由選材）：`param = "<form>|<material_slug>"`
pub fn apply_craft_custom_gear(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    param: &str,
) -> Result<String, String> {
    let aid = delta
        .resolve_actor_id(actor_key)
        .ok_or("請先向港務官核對通行身分。")?;
    let mut parts = param.split('|');
    let form = parts.next().unwrap_or("").trim();
    let mat_slug = parts.next().unwrap_or("").trim();
    let Some(mat) = lookup_material_spec(mat_slug) else {
        return Err("請挑選行囊裡實有的鍛造主材。".to_string());
    };
    let (slot, form_name_zh, base_w) = match form {
        "heavy_pick" => ("hands", "開山鍛鎬", 3u16),
        "carving_blade" => ("hands", "百工彎刀", 2u16),
        "arm_bracers" => ("arms", "護臂護腕", 2u16),
        "stride_boots" => ("feet", "釘底涉水靴", 2u16),
        "cargo_frame" => ("torso", "模組採集背架", 3u16),
        _ => return Err("請先選定欲打造的器物形制。".to_string()),
    };

    let (node_id, maker_name) = {
        let a = delta.actors.get(&aid).ok_or("找不到旅人行蹤")?;
        (a.node_id.clone(), a.display_name.clone())
    };
    let node = doc.nodes.get(&node_id).ok_or("找不到所在工坊")?;
    let is_workshop = node.kind == "shop"
        && (infer_shop_kind(node) == "smithy"
            || node.name.contains("鍛鐵")
            || node.name.contains("鐵匠")
            || node.name.contains("集坊"));
    if !is_workshop {
        return Err("百工形制打造須在開爐的鍛鐵鋪或百工集坊砧台旁進行。".to_string());
    }
    if clock.is_night {
        return Err("夜深工坊已封爐落栓，待晨鐘開爐後再動工打造。".to_string());
    }
    if super::live_commodities::is_smithy_forge_cold(delta, &node_id) {
        return Err("鍛爐裡的精煤與木炭已熄火，須先補炭重燃爐火方能開工打造。".to_string());
    }

    let need_cnt: u32 = if matches!(mat.slug, "spider_chitin" | "spider_carapace" | "lizard_scale") {
        1
    } else {
        2
    };
    let have_mat: u32 = delta
        .actors
        .get(&aid)
        .map_or(0, |a| {
            a.backpack
                .iter()
                .filter(|i| i.slug == mat.slug && i.bound_owner.is_none() && i.stolen_from.is_none())
                .map(|i| i.count)
                .sum()
        });
    if have_mat < need_cnt {
        return Err(format!(
            "行囊裡的【{}】尚不足 {} 份，無法以此料開爐打造【{}{}】。",
            mat.name_zh, need_cnt, mat.prefix_zh, form_name_zh
        ));
    }

    let self_operates = actor_operates_shop(delta, &aid, &node_id);
    let smith_opt = delta.node_occupants.get(&node_id).and_then(|occ| {
        occ.iter()
            .find(|&oid| {
                oid != &aid
                    && delta.actors.get(oid).is_some_and(|o| {
                        o.alive
                            && o.controller_id.is_none()
                            && (o.role.contains("鍛鐵") || o.role.contains("木匠"))
                    })
            })
            .cloned()
    });
    if !self_operates && smith_opt.is_none() {
        return Err("工坊師傅與學徒此刻不在爐邊，暫無人看火借砧。".to_string());
    }
    let fee = if self_operates { 0i64 } else { 4i64 };
    if fee > 0 {
        let Some(ref sid) = smith_opt else {
            return Err("工坊師傅此刻不在爐邊。".to_string());
        };
        if !delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "craft_custom_gear",
            &aid,
            sid,
            fee,
            format!("借爐共鍛{}{}火炭工錢", mat.prefix_zh, form_name_zh),
        ) {
            return Err("手頭銅幣不足 4 枚，付不起開爐借砧的火炭工錢。".to_string());
        }
    }
    let _ = super::live_commodities::consume_smithy_fuel_one(delta, smith_opt.as_deref(), &node_id);

    let full_name = format!("{}{}", mat.prefix_zh, form_name_zh);
    let item_slug = format!("craft_{}_{}", form, mat.slug);
    let sense_desc = describe_gear_physics_zh(mat.physics, slot);

    let msg = {
        let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
        let _ = take_carry_item(&mut a.backpack, mat.slug, need_cnt);
        a.mind.ltm.mastery.vessel_serial = a.mind.ltm.mastery.vessel_serial.saturating_add(1);
        a.mind.ltm.mastery.craft_works = a.mind.ltm.mastery.craft_works.saturating_add(1);
        a.mind.ltm.mastery.distinct_recipes.insert(item_slug.clone());
        let uid = format!("cg:{}:{}", a.actor_id, a.mind.ltm.mastery.vessel_serial);
        let prov = format!(
            "由{}於{}以【{}】親手鍛製",
            maker_name,
            node.name.split('・').next_back().unwrap_or(&node.name),
            mat.name_zh
        );
        if slot == "hands" {
            let tool = BoundTool {
                uid,
                slug: item_slug.clone(),
                name: full_name.clone(),
                provenance: prov,
                wear_stage: 0,
                grip_familiarity: 14,
                rune_mark: None,
            };
            add_carry_item(
                &mut a.backpack,
                CarryItem {
                    slug: item_slug,
                    name: full_name.clone(),
                    count: 1,
                    weight_units: base_w,
                    vessel_meta: Some(tool),
                    ..CarryItem::default()
                },
            );
        } else {
            let piece = BodyGearPiece {
                uid,
                slug: item_slug.clone(),
                slot: slot.to_string(),
                name: full_name.clone(),
                material_slug: mat.slug.to_string(),
                material_zh: mat.name_zh.to_string(),
                provenance: prov,
                wear_stage: 0,
                physics: mat.physics,
            };
            add_carry_item(
                &mut a.backpack,
                CarryItem {
                    slug: item_slug,
                    name: full_name.clone(),
                    count: 1,
                    weight_units: base_w,
                    gear_meta: Some(piece),
                    ..CarryItem::default()
                },
            );
        }
        let (forge_prose, forge_save) =
            super::martial_art_and_manuals::invoke_craft_phase_effect(a, "forge");
        a.fatigue = a.fatigue.saturating_add(12u8.saturating_sub(forge_save).max(6)).min(95);
        a.last_active_unix = clock.real_unix;
        let fee_note = if fee > 0 {
            format!("（付妥 {fee} 枚銅幣爐炭費）")
        } else {
            "（自家工坊免付借爐費）".to_string()
        };
        let m = format!(
            "爐火驟旺、鐵錘起落間（{forge_prose}），你以【{}】打造出一件【{}】{}——{}，已收入行囊。",
            mat.name_zh, full_name, fee_note, sense_desc
        );
        push_breath(a, m.clone());
        a.mark_mutated();
        m
    };

    broadcast_scene_event(
        doc,
        delta,
        clock,
        &node_id,
        &aid,
        &maker_name,
        EventPublicity::PublicLoud,
        None,
        42,
        18,
        ValueAxis::CraftPride,
        "craft_custom_gear",
        &format!("{maker_name}在工坊爐邊以【{}】親手鍛成【{full_name}】", mat.name_zh),
    );
    Ok(msg)
}

/// 守恆穿戴行囊內之多部位裝備（雙臂護具／足腿靴履／軀幹背架），舊件原物退回行囊
pub fn apply_equip_body_gear(
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    slug: &str,
) -> Result<String, String> {
    let aid = delta
        .resolve_actor_id(actor_key)
        .ok_or("請先向港務官核對通行身分。")?;
    let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
    let pos = a
        .backpack
        .iter()
        .position(|i| i.slug == slug && i.count > 0 && i.gear_meta.is_some())
        .ok_or("行囊裡找不到這件可穿戴的護具或背架。")?;
    let item = if a.backpack[pos].count > 1 {
        a.backpack[pos].count -= 1;
        let mut one = a.backpack[pos].clone();
        one.count = 1;
        one
    } else {
        a.backpack.remove(pos)
    };
    let Some(piece) = item.gear_meta else {
        add_carry_item(&mut a.backpack, item);
        return Err("這件物件並非穿戴式多部位裝備。".to_string());
    };
    let slot_label = match piece.slot.as_str() {
        "arms" => "雙臂護具",
        "feet" => "足腿靴履",
        "torso" => "軀幹背負",
        _ => {
            add_carry_item(
                &mut a.backpack,
                CarryItem {
                    slug: piece.slug.clone(),
                    name: piece.name.clone(),
                    count: 1,
                    weight_units: item.weight_units,
                    gear_meta: Some(piece),
                    ..CarryItem::default()
                },
            );
            return Err("未知的裝備穿戴部位。".to_string());
        }
    };
    let prev_opt = match piece.slot.as_str() {
        "arms" => a.body_gear.arms.replace(piece.clone()),
        "feet" => a.body_gear.feet.replace(piece.clone()),
        _ => a.body_gear.torso.replace(piece.clone()),
    };
    let swap_note = if let Some(prev) = prev_opt {
        let pn = prev.name.clone();
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: prev.slug.clone(),
                name: prev.name.clone(),
                count: 1,
                weight_units: 2,
                gear_meta: Some(prev),
                ..CarryItem::default()
            },
        );
        format!("，原穿戴的【{pn}】已原封收妥回行囊")
    } else {
        String::new()
    };
    let desc = describe_gear_physics_zh(piece.physics, &piece.slot);
    let msg = format!("你將【{}】穿戴妥當於{}位{}（{}）。", piece.name, slot_label, swap_note, desc);
    a.last_active_unix = clock.real_unix;
    push_breath(a, msg.clone());
    a.mark_mutated();
    Ok(msg)
}

/// 守恆卸下指定部位裝備並原物放回行囊
pub fn apply_unequip_body_gear(
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    slot: &str,
) -> Result<String, String> {
    let aid = delta
        .resolve_actor_id(actor_key)
        .ok_or("請先向港務官核對通行身分。")?;
    let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
    let taken = match slot.trim() {
        "arms" => a.body_gear.arms.take(),
        "feet" => a.body_gear.feet.take(),
        "torso" => a.body_gear.torso.take(),
        _ => return Err("請指定要卸下的身體部位（雙臂／足腿／軀幹）。".to_string()),
    };
    let Some(piece) = taken else {
        return Err("該部位目前並未穿戴裝備。".to_string());
    };
    let pname = piece.name.clone();
    add_carry_item(
        &mut a.backpack,
        CarryItem {
            slug: piece.slug.clone(),
            name: piece.name.clone(),
            count: 1,
            weight_units: 2,
            gear_meta: Some(piece),
            ..CarryItem::default()
        },
    );
    let msg = format!("你解下身上的【{pname}】，摺疊收妥放回行囊。");
    a.last_active_unix = clock.real_unix;
    push_breath(a, msg.clone());
    a.mark_mutated();
    Ok(msg)
}

/// 序列化全身多部位裝備（零原始數值，純手感與實效）＋ 可就地綁製清單供前端繪製
pub fn build_body_gear_view_json(viewer: &UnifiedMindBody) -> Value {
    let fmt_piece = |opt: &Option<BodyGearPiece>| -> Value {
        match opt {
            Some(p) => json!({
                "slug": p.slug,
                "slot": p.slot,
                "name": p.name,
                "material": p.material_zh,
                "provenance": p.provenance,
                "sense": describe_gear_physics_zh(p.physics, &p.slot),
            }),
            None => Value::Null,
        }
    };
    let count_slug = |slug: &str| -> u32 {
        viewer
            .backpack
            .iter()
            .filter(|i| i.slug == slug && i.bound_owner.is_none() && i.stolen_from.is_none())
            .map(|i| i.count)
            .sum()
    };
    let flint = count_slug("river_flint");
    let branch = count_slug("dry_branch");
    let vine = count_slug("tough_vine");
    let field_recipes = vec![
        json!({ "id": "stone_hoe", "name": "石刃木鋤", "slot": "雙手工具", "need": "硬石・枯枝・韌藤", "ready": flint >= 1 && branch >= 1 && vine >= 1 }),
        json!({ "id": "stone_spear", "name": "硬石長叉", "slot": "雙手長械", "need": "硬石・枯枝・韌藤", "ready": flint >= 1 && branch >= 1 && vine >= 1 }),
        json!({ "id": "vine_bracers", "name": "粗藤護腕", "slot": "雙臂護具", "need": "韌藤 × 2", "ready": vine >= 2 }),
        json!({ "id": "reed_boots", "name": "編藤涉水履", "slot": "足腿靴履", "need": "韌藤・枯枝", "ready": vine >= 1 && branch >= 1 }),
        json!({ "id": "vine_packframe", "name": "藤編採集背架", "slot": "軀幹背負", "need": "枯枝 × 2・韌藤", "ready": branch >= 2 && vine >= 1 }),
    ];
    let bulk_cap = effective_overland_bulk_cap(viewer);
    let bulk_sense = match bulk_cap {
        4 => "裝有堅實採集背架，徒步可比尋常旅人多扛兩份大宗重貨",
        3 => "背著輕便採集背架，徒步可多扛一份大宗重貨",
        _ => "未佩採集背架，跨城徒步僅宜攜帶隨身樣品",
    };
    json!({
        "arms": fmt_piece(&viewer.body_gear.arms),
        "feet": fmt_piece(&viewer.body_gear.feet),
        "torso": fmt_piece(&viewer.body_gear.torso),
        "bulk_sense": bulk_sense,
        "field_recipes": field_recipes,
    })
}

/// 在野外／城外掛載「徒手拾撿原材」實物插座，在開爐鍛鐵鋪／集坊掛載「百工形制×選材打造」砧台插座
pub fn append_gear_and_forage_scene_objects<F>(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    viewer: &UnifiedMindBody,
    node: &SpatialNodeDef,
    clock: &SolarClockSnapshot,
    objects: &mut Vec<Value>,
    push_audit: &mut F,
) where
    F: FnMut(&str, &str, &str, &str, &str, &str),
{
    let is_outdoor_forageable = !node.indoor
        && !node.dungeon
        && matches!(node.kind.as_str(), "wild" | "village_gate" | "campfire_nook" | "summit_outlook");
    if is_outdoor_forageable {
        let cell_is_coast = doc
            .cells
            .get(&node.cell_id.to_string())
            .is_some_and(|c| c.is_coast);
        let obj_name = if cell_is_coast {
            "碎石潮灘與海藤"
        } else {
            "林緣硬石與枯枝韌藤"
        };
        objects.push(json!({
            "object_id": "obj:wild_raw_materials",
            "name": obj_name,
            "cue": "徒手即可拾撿硬石、枯枝與韌藤，供就地綁製初階工具護具",
            "direct_action": { "id": "gather_wild_raw", "param": "" },
            "options": [],
        }));
        push_audit(
            "gather_wild_raw",
            "",
            "徒手拾撿天然原材",
            "河灘硬石・枯枝・韌藤",
            "object",
            "obj:wild_raw_materials",
        );
    }

    let is_open_workshop = !clock.is_night
        && node.kind == "shop"
        && (infer_shop_kind(node) == "smithy"
            || node.name.contains("鍛鐵")
            || node.name.contains("鐵匠")
            || node.name.contains("集坊"))
        && !super::live_commodities::is_smithy_forge_cold(delta, &node.id);
    if is_open_workshop {
        let mut craft_opts: Vec<Value> = Vec::new();
        let forms: &[(&str, &str)] = &[
            ("arm_bracers", "護臂護腕（防反震護腕）"),
            ("stride_boots", "釘底涉水靴（涉水山路省力）"),
            ("cargo_frame", "模組採集背架（擴增負重）"),
            ("heavy_pick", "開山鍛鎬（雙手鈍器）"),
            ("carving_blade", "百工彎刀（雙手利刃）"),
        ];
        for mat in CRAFT_MATERIALS {
            let need: u32 = if matches!(mat.slug, "spider_chitin" | "spider_carapace" | "lizard_scale") {
                1
            } else {
                2
            };
            let have: u32 = viewer
                .backpack
                .iter()
                .filter(|i| i.slug == mat.slug && i.bound_owner.is_none() && i.stolen_from.is_none())
                .map(|i| i.count)
                .sum();
            if have < need {
                continue;
            }
            for &(f_id, f_desc) in forms {
                let param = format!("{f_id}|{}", mat.slug);
                let label = format!("以【{}】打造「{}{}」", mat.name_zh, mat.prefix_zh, f_desc.split('（').next().unwrap_or(f_desc));
                craft_opts.push(json!({
                    "id": "craft_custom_gear",
                    "param": param,
                    "label": label,
                    "sub": format!("{}・耗料 {} 份（借爐火炭費 4 銅，自家鋪免付）", f_desc, need),
                }));
            }
            if craft_opts.len() >= 10 {
                break;
            }
        }
        let cue = if craft_opts.is_empty() {
            "備妥任意 2 份同類原材（粗鐵礦／原木／漂流木／石英砂／硬石／韌藤或 1 份巨蛛甲片）即可自由選材開爐打造"
        } else {
            "行囊裡已有可用主材，點開可依形制 × 材質親手打造全身工具與護具"
        };
        objects.push(json!({
            "object_id": "obj:craft_anvil_bench",
            "name": "百工鍛造砧台",
            "cue": cue,
            "desc": "工坊的厚重鍛砧與配料架：形制決定用途插座，主材決定硬度、韌性、重量與導能手感，無職業限制。",
            "direct_action": Value::Null,
            "options": craft_opts,
        }));
    }
}
