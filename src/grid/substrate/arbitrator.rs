//! 玩家與 NPC 共用的純 Rust 四步仲裁管線（`Observe -> Declare Intent -> Resolve -> Update`）
//! ＋ `cell_overlay` × `entities.inventory` 同交易原子 CAS（微步 3b′）。
//!
//! 鐵律：
//! 1. 玩家與 NPC 100% 共用 `execute_world_intent`，不另立第二套分派器。
//! 2. 所有物資產出嚴守 `zl_item_*` 命名，與 `cell_overlay` 在同一 PostgreSQL Transaction
//!    內搭配 `SELECT inventory, inventory_seq ... FOR UPDATE` 原子提交，杜絕無中生有與雙花。

use super::cache::{invalidate_chunk, put_cached_overlay, remove_cached_overlay};
use super::facade::CellFacade;
use super::hydrology::{WorldClockSnapshot, derive_cell_field};
use super::l0_cell::L0Cell;
use super::runtime;
use super::settle::{is_record_pristine, settle_cell_on_observe};
use super::slots::{
    FieldState, S0Companion, S0Pavement, SlotStack, V1Plugin, is_campfire_extinguished_by_rain,
};
use crate::pg::cell_overlay::{
    CellOverlayRecord, OverlayInvTxResult, commit_overlay_and_inventory_tx,
};
use crate::store::sql::DbPool;

/// 強型別世界意圖（玩家與 NPC 共用之 `Declare Intent` 階段契約）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldIntent {
    /// 劈竹開路：移除 `V1` 合抱刺竹叢，保留 `U1` 老竹鞭，產出 `zl_item_bamboo_culm`。
    ClearBamboo,
    /// 掘除露土老竹鞭：須先清空 `V1`，移除 `U1` 並彈出活體共生項，產出 `zl_item_bamboo_rhizome`。
    UprootRhizome,
    /// 伐取刺竹稈（不清空整叢，扣減單格儲量 `charge`），產出 `zl_item_bamboo_culm`。
    GatherBambooCulm,
    /// 採集地表 `S0` 伴生物（乾竹葉／枯黃竹籜／破土莿竹筍／竹根木耳）。
    GatherCompanion(S0Companion),
    /// 攏乾竹葉生起竹葉營火（佔用 1 個 `S0` 伴生槽；建築與石板街及高水期禁生野火）。
    IgniteCampfire,
    /// 撲滅竹葉營火。
    ExtinguishCampfire,
}

/// 仲裁結果（統一回傳給玩家 WS 層或 NPC 決策層）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArbitrationOutcome {
    pub success: bool,
    pub narrative: String,
    pub yielded_items: Vec<(&'static str, i32)>,
    pub new_inventory: Option<String>,
    pub new_inventory_seq: Option<i64>,
    pub vit_delta: i32,
}

/// Step 3（純函式 `Resolve`）：依當下快照、場域與行動者背包判定規則，產出目標髒格與背包增減量。
pub fn resolve_world_intent(
    existing: Option<&CellOverlayRecord>,
    gx: i32,
    gy: i32,
    l0: L0Cell,
    world_seed: u64,
    clock: WorldClockSnapshot,
    field: FieldState,
    actor_inventory_json: &str,
    intent: WorldIntent,
) -> Result<
    (
        CellOverlayRecord,
        Vec<(&'static str, i32)>,
        String,
        i32,
        bool,
    ),
    String,
> {
    let pristine = super::slots::derive_pristine_slots(gx, gy, l0, world_seed);
    let mut rec = existing
        .cloned()
        .unwrap_or_else(|| CellOverlayRecord::from_stack(gx, gy, pristine));
    let mut deltas: Vec<(&'static str, i32)> = Vec::new();
    let mut vit_delta = 0i32;
    let mut require_positive_charge = false;

    match intent {
        WorldIntent::ClearBamboo => {
            if rec.stack.v1 != V1Plugin::ThornyBambooClump {
                return Err("此處已無擋路的合抱刺竹叢。".to_string());
            }
            if !crate::grid::zhulin::inventory_has_capability_tag(
                actor_inventory_json,
                "heavy_chop",
            ) {
                return Err(
                    "合抱刺竹叢竹竿粗硬如鐵、刺枝交錯，徒手難以開路；需持有「竹鞭根槌」或「籜繩竹鞭石斧」方能一舉劈開竹障！".to_string(),
                );
            }
            let has_stone_axe = actor_inventory_json.contains("zl_item_bamboo_stone_axe");
            let yield_qty = if has_stone_axe { 3 } else { 2 };
            rec.stack.clear_v1_bamboo();
            rec.stack.s0_pave = match rec.stack.s0_pave {
                S0Pavement::BambooTrampledPath { pave_heat } => S0Pavement::BambooTrampledPath {
                    pave_heat: pave_heat.max(48),
                },
                S0Pavement::BareSoil => S0Pavement::BambooTrampledPath { pave_heat: 48 },
                other => other,
            };
            let _ = rec.stack.add_companion(S0Companion::DryBambooLeaf);
            rec.roll_day = clock.game_day.max(1);
            rec.charge = -1;
            deltas.push(("zl_item_bamboo_culm", yield_qty));
            let tool_desc = if has_stone_axe {
                "揮動「籜繩竹鞭石斧」順著竹節一氣呵成連番斜劈"
            } else {
                "掄起「竹鞭根槌」順著竹節猛力砸斷交錯刺枝"
            };
            let narrative = format!(
                "你{tool_desc}，將擋路的「合抱刺竹叢」連根伐開，闢出一條透光通風的【疏竹小徑】，收起 {yield_qty} 根「刺竹稈」入袋，林下也露出盤錯的【露土老竹鞭】！"
            );
            Ok((rec, deltas, narrative, vit_delta, require_positive_charge))
        }

        WorldIntent::UprootRhizome => {
            if !rec.stack.exposes_bamboo_rhizome_socket() {
                return Err("須先劈除地上合抱刺竹叢，露出土面老竹鞭後方可掘取。".to_string());
            }
            let had_shoot = rec
                .stack
                .s0_companions
                .contains(&S0Companion::EmergingBambooShoot);
            let had_ear = rec
                .stack
                .s0_companions
                .contains(&S0Companion::BambooRootEar);
            if !rec.stack.uproot_u1_rhizome() {
                return Err("此處並無可掘除的老竹鞭。".to_string());
            }
            rec.roll_day = clock.game_day.max(1);
            deltas.push(("zl_item_bamboo_rhizome", 2));
            let mut extra_note = String::new();
            if had_shoot {
                deltas.push(("zl_item_bamboo_shoot", 1));
                extra_note.push_str("，連帶掘出一支破土莿竹筍");
            }
            if had_ear {
                deltas.push(("zl_item_bamboo_fungus", 1));
                extra_note.push_str("與一簇竹根木耳");
            }
            let narrative = format!(
                "你順著裂土縫隙撬起盤錯的【露土老竹鞭】並斬斷根脈，收入 2 截「老竹鞭」{extra_note}；這格已徹底絕去竹潮根基。"
            );
            Ok((rec, deltas, narrative, vit_delta, require_positive_charge))
        }

        WorldIntent::GatherBambooCulm => {
            if rec.stack.v1 != V1Plugin::ThornyBambooClump {
                return Err("此處已無可伐取的合抱刺竹叢。".to_string());
            }
            if !crate::grid::zhulin::inventory_has_capability_tag(
                actor_inventory_json,
                "heavy_chop",
            ) {
                return Err(
                    "合抱刺竹稈粗硬如鐵，需持有「竹鞭根槌」或「籜繩竹鞭石斧」方能伐取刺竹稈。"
                        .to_string(),
                );
            }
            let cur_charge = if rec.charge < 0 { 6 } else { rec.charge };
            if cur_charge <= 0 {
                return Err("這叢刺竹的筆直成材已伐完，須待次日新篁抽長。".to_string());
            }
            require_positive_charge = existing.is_some();
            rec.charge = cur_charge - 1;
            rec.roll_day = clock.game_day.max(1);
            deltas.push(("zl_item_bamboo_culm", 1));
            let narrative = "你挑選一株筆直老練的刺竹稈順節伐下，收進隨身行囊。".to_string();
            Ok((rec, deltas, narrative, vit_delta, require_positive_charge))
        }

        WorldIntent::GatherCompanion(comp) => {
            if !rec.stack.s0_companions.contains(&comp) {
                return Err(format!("腳邊已無「{}」可供採集。", comp.label_zh()));
            }
            let (item_id, qty, desc) = match comp {
                S0Companion::DryBambooLeaf => {
                    ("zl_item_dry_leaves", 2, "俯身攏起兩把乾燥脆響的乾竹葉")
                }
                S0Companion::BambooSheath => {
                    ("zl_item_bamboo_sheath", 2, "拾起兩片寬厚堅韌的枯黃竹籜")
                }
                S0Companion::EmergingBambooShoot => (
                    "zl_item_bamboo_shoot",
                    1,
                    "撬開鬆土挖起一支飽滿清甜的破土莿竹筍",
                ),
                S0Companion::BambooRootEar => (
                    "zl_item_bamboo_fungus",
                    1,
                    "沿著濕潤竹根細心採下一簇肥厚的竹根木耳",
                ),
                _ => return Err(format!("「{}」無法直接徒手採收。", comp.label_zh())),
            };
            rec.stack.s0_companions.retain(|c| *c != comp);
            rec.roll_day = clock.game_day.max(1);
            if rec.charge < 0 {
                rec.charge = 0;
            }
            deltas.push((item_id, qty));
            let narrative = format!("你{desc}，妥善收入行囊。");
            Ok((rec, deltas, narrative, vit_delta, require_positive_charge))
        }

        WorldIntent::IgniteCampfire => {
            if rec.stack.has_campfire() {
                return Err("此處已燃著一簇溫暖的竹葉營火。".to_string());
            }
            if matches!(rec.stack.v1, V1Plugin::ArcadeShophouse { .. })
                || matches!(
                    rec.stack.s0_pave,
                    S0Pavement::StoneStreet | S0Pavement::NarrowRailBed
                )
            {
                return Err("石板街面與騎樓街屋內禁生野火。".to_string());
            }
            if field.flood_active {
                return Err("高水期低地積水漫流，地面潮濕泥濘，無法就地生火。".to_string());
            }
            if is_campfire_extinguished_by_rain(field.weather_wetting, rec.stack.v1) {
                return Err("頭頂無濃密竹冠遮雨，雨絲直落，野地無法引燃竹葉營火。".to_string());
            }
            let ground_has_leaves = rec
                .stack
                .s0_companions
                .contains(&S0Companion::DryBambooLeaf);
            let inv_has_leaves = actor_inventory_json.contains("zl_item_dry_leaves");
            if !ground_has_leaves && !inv_has_leaves {
                return Err("腳邊與行囊皆無乾竹葉可供引火。".to_string());
            }
            // 伴生槽滿時自動消耗乾竹葉騰位（規格書 §4.1）
            if ground_has_leaves {
                rec.stack
                    .s0_companions
                    .retain(|c| *c != S0Companion::DryBambooLeaf);
            } else {
                deltas.push(("zl_item_dry_leaves", -1));
            }
            if rec.stack.s0_companions.len() >= 3 {
                rec.stack.s0_companions.pop();
            }
            if !rec.stack.add_companion(S0Companion::BambooCampfire) {
                return Err("地表伴生槽已滿，暫時騰不出空位生火。".to_string());
            }
            rec.stamp = clock.world_minute.max(1);
            rec.roll_day = clock.game_day.max(1);
            vit_delta = 12;
            let shelter_note = if field.weather_wetting && rec.stack.v1.provides_rain_cover() {
                "幸得頭頂濃密竹冠擋住雨絲，"
            } else {
                ""
            };
            let narrative = format!(
                "{shelter_note}你就地攏起乾竹葉與枯枝引火，火苗劈啪竄起，燃起一簇溫暖的「竹葉營火」！"
            );
            Ok((rec, deltas, narrative, vit_delta, require_positive_charge))
        }

        WorldIntent::ExtinguishCampfire => {
            if !rec.stack.has_campfire() {
                return Err("此處並無燃燒中的營火。".to_string());
            }
            rec.stack
                .s0_companions
                .retain(|c| *c != S0Companion::BambooCampfire);
            rec.stamp = 0;
            let narrative = "你撥土覆熄了竹葉營火，只餘一縷青煙與溫熱灰燼。".to_string();
            Ok((rec, deltas, narrative, vit_delta, require_positive_charge))
        }
    }
}

/// 玩家與 NPC 共用的四步仲裁統一入口（`Observe -> Declare Intent -> Resolve -> Update`）。
pub fn execute_world_intent(
    pool: &DbPool,
    actor_id: &str,
    gx: i32,
    gy: i32,
    clock: WorldClockSnapshot,
    intent: WorldIntent,
) -> ArbitrationOutcome {
    const MAX_RETRIES: usize = 3;
    let Some(rt) = runtime::global() else {
        return ArbitrationOutcome {
            success: false,
            narrative: "世界底板尚未就緒，暫時無法施作。".to_string(),
            yielded_items: Vec::new(),
            new_inventory: None,
            new_inventory_seq: None,
            vit_delta: 0,
        };
    };
    let l0 = rt.l0(gx, gy).unwrap_or_else(L0Cell::sea_nodata);
    let field = derive_cell_field(gx, gy, l0, clock);

    for _attempt in 0..MAX_RETRIES {
        // Step 1: Observe（含觀測現場惰性 settle）
        let settled = settle_cell_on_observe(pool, gx, gy, l0, clock, field);
        let expected_ver = settled.as_ref().map_or(0, |r| r.ver);
        let actor_inv = crate::db::get_entity(actor_id)
            .ok()
            .flatten()
            .map(|e| e.inventory)
            .unwrap_or_else(|| "[]".to_string());

        // Step 2 & 3: Declare Intent -> Resolve
        let (next_rec, deltas, narrative, vit_delta, req_pos) = match resolve_world_intent(
            settled.as_ref(),
            gx,
            gy,
            l0,
            runtime::WORLD_SEED,
            clock,
            field,
            &actor_inv,
            intent,
        ) {
            Ok(v) => v,
            Err(reason) => {
                return ArbitrationOutcome {
                    success: false,
                    narrative: reason,
                    yielded_items: Vec::new(),
                    new_inventory: None,
                    new_inventory_seq: None,
                    vit_delta: 0,
                };
            }
        };

        // Step 4: Update（cell_overlay CAS + entities.inventory FOR UPDATE 同交易原子提交）
        let delete_if_pristine = is_record_pristine(&next_rec, l0, runtime::WORLD_SEED);
        match commit_overlay_and_inventory_tx(
            pool,
            &next_rec,
            expected_ver,
            req_pos,
            delete_if_pristine,
            actor_id,
            &deltas,
        ) {
            Ok(OverlayInvTxResult::Committed {
                new_ver,
                new_inventory,
                new_inventory_seq,
            }) => {
                if delete_if_pristine {
                    remove_cached_overlay(gx, gy);
                } else {
                    let mut cached = next_rec;
                    cached.ver = new_ver;
                    put_cached_overlay(cached);
                }
                if let Some(store) = crate::store::get_store() {
                    let _ = store.write().mirror_inventory_and_seq_from_authority(
                        actor_id,
                        &new_inventory,
                        new_inventory_seq,
                    );
                }
                if vit_delta != 0
                    && let Ok(Some(ent)) = crate::db::get_entity(actor_id)
                {
                    let _ = crate::db::update_vit(actor_id, (ent.vit + vit_delta).clamp(1, 100));
                }
                return ArbitrationOutcome {
                    success: true,
                    narrative,
                    yielded_items: deltas,
                    new_inventory: Some(new_inventory),
                    new_inventory_seq: Some(new_inventory_seq),
                    vit_delta,
                };
            }
            Ok(OverlayInvTxResult::Conflict) => {
                invalidate_chunk(gx, gy);
                continue;
            }
            Err(e) => {
                return ArbitrationOutcome {
                    success: false,
                    narrative: format!("施作未能落盤：{e}"),
                    yielded_items: Vec::new(),
                    new_inventory: None,
                    new_inventory_seq: None,
                    vit_delta: 0,
                };
            }
        }
    }

    ArbitrationOutcome {
        success: false,
        narrative: "方才有人同時動過此處地物，請稍候再試。".to_string(),
        yielded_items: Vec::new(),
        new_inventory: None,
        new_inventory_seq: None,
        vit_delta: 0,
    }
}

/// 依 `CellSnapshot` 的 `SlotStack` 合成該格真實可互動之竹林／地表物件（單一權威投影，取代舊 `SquareGrid` 竹林物件）。
pub fn substrate_slot_objects(
    slots: &SlotStack,
    field: FieldState,
) -> Vec<crate::model::RoomObject> {
    let mut out = Vec::new();
    if slots.has_campfire() && !is_campfire_extinguished_by_rain(field.weather_wetting, slots.v1) {
        out.push(crate::model::RoomObject {
            id: "zl_bamboo_campfire".to_string(),
            name: "竹葉營火".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".into(),
                "WarmHands".into(),
                "CampfireDrinkWater".into(),
                "CampfireEatFood".into(),
                "FillBambooCanteen".into(),
                "CampfireRoastKnife".into(),
                "CampfireRoastShoot".into(),
                "CampfireRoastFungus".into(),
                "CampfireBoilSoup".into(),
                "CampfirePopCulm".into(),
                "ExtinguishCampfire".into(),
            ],
            responses: std::collections::HashMap::from([(
                "Look".into(),
                "一簇搖曳生暖的竹葉營火，炭灰紅亮，可烤刀、煨筍、烘烤木耳或就火取暖。".into(),
            )]),
            move_to_room_id: String::new(),
        });
    }
    if slots.v1 == V1Plugin::ThornyBambooClump {
        out.push(crate::model::RoomObject {
            id: "zl_bamboo_culm".to_string(),
            name: "合抱刺竹叢".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".into(),
                "Gather".into(),
                "ClearBambooTrail".into(),
                "CarveMark".into(),
                "FillBambooCanteen".into(),
            ],
            responses: std::collections::HashMap::from([(
                "Look".into(),
                "竹稈粗如合抱的原生刺竹叢，竹節密布堅硬彎刺，可伐取刺竹稈或以重器劈竹開路。".into(),
            )]),
            move_to_room_id: String::new(),
        });
    }
    if slots.exposes_bamboo_rhizome_socket() {
        out.push(crate::model::RoomObject {
            id: "zl_bamboo_rhizome".to_string(),
            name: "露土老竹鞭".to_string(),
            owner: String::new(),
            sockets: vec!["Look".into(), "UprootRhizome".into(), "Gather".into()],
            responses: std::collections::HashMap::from([(
                "Look".into(),
                "地上竹叢已除，盤錯如筋的老竹鞭裸露土面，可掘除取鞭並絕去竹潮。".into(),
            )]),
            move_to_room_id: String::new(),
        });
    }
    let has_cf = slots.has_campfire();
    for &c in &slots.s0_companions {
        let (id, name, sockets, desc) = match c {
            S0Companion::DryBambooLeaf => (
                "zl_dry_leaves",
                "乾竹葉",
                if has_cf {
                    vec!["Look".into(), "Gather".into()]
                } else {
                    vec!["Look".into(), "Gather".into(), "IgniteCampfire".into()]
                },
                "鋪滿林下的乾燥枯黃竹葉，踩上沙沙作響，可拾取或就地攏葉生火。",
            ),
            S0Companion::BambooSheath => (
                "zl_bamboo_sheath",
                "枯黃竹籜",
                vec!["Look".into(), "Gather".into()],
                "自竹節脫落的寬厚竹籜，纖維強韌，可撕成籜繩綁紮器物。",
            ),
            S0Companion::EmergingBambooShoot => (
                "zl_bamboo_shoot",
                "破土莿竹筍",
                vec!["Look".into(), "Gather".into()],
                "剛頂開濕潤紅土的飽滿莿竹筍，鮮嫩厚實。",
            ),
            S0Companion::BambooRootEar => (
                "zl_bamboo_root_ear",
                "竹根木耳",
                vec!["Look".into(), "Gather".into()],
                "貼生在潮濕老竹根上的深褐木耳，脆嫩可食。",
            ),
            _ => continue,
        };
        out.push(crate::model::RoomObject {
            id: id.to_string(),
            name: name.to_string(),
            owner: String::new(),
            sockets,
            responses: std::collections::HashMap::from([("Look".into(), desc.into())]),
            move_to_room_id: String::new(),
        });
    }
    let _ = CellFacade;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::substrate::hydrology::KaipuSeason;
    use crate::grid::substrate::l0_cell::{SubstrateMaterial, SurfaceWaterKind, landmass};
    use crate::grid::substrate::slots::U1Plugin;

    fn bamboo_land() -> L0Cell {
        L0Cell::new(
            Some(30.0),
            landmass::DAJIA_WU_TABLELAND,
            SubstrateMaterial::AlluvialSandyLoam,
            SurfaceWaterKind::None,
            20.0,
            90,
        )
    }

    fn dry_clock() -> WorldClockSnapshot {
        WorldClockSnapshot {
            game_day: 12,
            world_minute: 12 * 1440 + 600,
            minute_of_day: 600,
            season: KaipuSeason::SpringThaw,
            weather_wetting: false,
        }
    }

    #[test]
    fn clear_bamboo_exposes_rhizome_and_uproot_pops_symbionts() {
        let l0 = bamboo_land();
        let clock = dry_clock();
        let field = FieldState::default();

        // 徒手劈竹被拒
        assert!(
            resolve_world_intent(
                None,
                1,
                0,
                l0,
                runtime::WORLD_SEED,
                clock,
                field,
                "[]",
                WorldIntent::ClearBamboo
            )
            .is_err()
        );

        // 持石斧劈竹開路 -> V1 清空、產出 3 根刺竹稈、露出老竹鞭插座
        let axe_inv = r#"[{"item_id":"zl_item_bamboo_stone_axe","qty":1}]"#;
        let (after_clear, deltas1, _, _, _) = resolve_world_intent(
            None,
            1,
            0,
            l0,
            runtime::WORLD_SEED,
            clock,
            field,
            axe_inv,
            WorldIntent::ClearBamboo,
        )
        .expect("持斧劈竹應成功");
        assert_eq!(after_clear.stack.v1, V1Plugin::None);
        assert!(after_clear.stack.exposes_bamboo_rhizome_socket());
        assert_eq!(deltas1, vec![("zl_item_bamboo_culm", 3)]);

        // 掘除老竹鞭 -> U1 清空，連帶彈出破土莿竹筍（因 (1,0) 原生含筍）
        let (after_uproot, deltas2, _, _, _) = resolve_world_intent(
            Some(&after_clear),
            1,
            0,
            l0,
            runtime::WORLD_SEED,
            clock,
            field,
            axe_inv,
            WorldIntent::UprootRhizome,
        )
        .expect("掘鞭應成功");
        assert_eq!(after_uproot.stack.u1, U1Plugin::None);
        assert!(deltas2.contains(&("zl_item_bamboo_rhizome", 2)));
        assert!(deltas2.contains(&("zl_item_bamboo_shoot", 1)));
    }

    #[test]
    fn ignite_campfire_respects_rain_cover_and_flood_and_urban_ban() {
        let l0 = bamboo_land();
        let clock = dry_clock();
        // 高水期禁生野火
        let flood = FieldState {
            flood_active: true,
            ..FieldState::default()
        };
        assert!(
            resolve_world_intent(
                None,
                1,
                0,
                l0,
                runtime::WORLD_SEED,
                clock,
                flood,
                r#"[{"item_id":"zl_item_dry_leaves","qty":2}]"#,
                WorldIntent::IgniteCampfire
            )
            .is_err()
        );
        // 石板街禁生野火
        assert!(
            resolve_world_intent(
                None,
                -4,
                0,
                l0,
                runtime::WORLD_SEED,
                clock,
                FieldState::default(),
                r#"[{"item_id":"zl_item_dry_leaves","qty":2}]"#,
                WorldIntent::IgniteCampfire
            )
            .is_err()
        );
        // 密竹冠下（1,0）微雨仍可生火（v1.provides_rain_cover() == true）
        let rain = FieldState {
            weather_wetting: true,
            ..FieldState::default()
        };
        let (after_fire, _, _, vit, _) = resolve_world_intent(
            None,
            1,
            0,
            l0,
            runtime::WORLD_SEED,
            clock,
            rain,
            r#"[{"item_id":"zl_item_dry_leaves","qty":2}]"#,
            WorldIntent::IgniteCampfire,
        )
        .expect("濃密竹冠遮雨下應可生火");
        assert!(after_fire.stack.has_campfire());
        assert_eq!(vit, 12);
    }
}
