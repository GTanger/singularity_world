//! 農田與藥圃閉環（拆自 `living_sim_and_farm.rs` 守住單檔體積預算）：
//! 1. 播種必耗真實種苗（`wheat_grain`／`herb_bundle`／`quarantine_wormwood`／`tough_vine`），嚴禁無中生有；
//! 2. 播種同步呼叫 `reclaim_arable_tiles_at_actor` 寫入 `delta.tile_diffs` 改寫真實地圖地貌，並經 `WorldExtrasBlob.farm_plots` 持久化；
//! 3. 播種／照料／收割皆觸發百工「山林採獵」藝相實算減耗，且玩家與 994 名 NPC 共用同一入口。

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::clock_and_lang::SolarClockSnapshot;
use super::island_substrate::compiled_island_substrate;
use super::live_land::ensure_live_land_seeded;
use super::live_land_tiles::{reclaim_arable_tiles_at_actor, resolve_actor_standing_xy};
use super::martial_art_and_manuals::invoke_craft_phase_effect;
use super::mind::EventPublicity;
use super::movement::{add_carry_item, push_breath, take_clean_carry_items};
use super::persona::ValueAxis;
use super::sim_tick::broadcast_scene_event;
use super::types::{CarryItem, IslandWorldDelta, SpatialNodeDef, UnifiedMindBody, WorldStaticDoc};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FarmPlotRecord {
    pub plot_id: String,
    pub node_id: String,
    pub node_name_zh: String,
    pub cell_id: u32,
    pub owner_id: String,
    pub owner_name: String,
    pub crop_kind: String,
    pub crop_name_zh: String,
    pub planted_day: i64,
    pub mature_day: i64,
    pub tended_day: i64,
}

#[derive(Debug, Clone, Copy)]
struct CropSpec {
    kind: &'static str,
    name_zh: &'static str,
    seed_slugs: &'static [&'static str],
    seed_hint_zh: &'static str,
    yield_slug: &'static str,
    yield_name_zh: &'static str,
    yield_count: u32,
    weight_units: u16,
    grow_days: i64,
}

const CROP_CATALOG: &[CropSpec] = &[
    CropSpec {
        kind: "dry_wheat",
        name_zh: "旱田黑麥",
        seed_slugs: &["wheat_grain"],
        seed_hint_zh: "飽滿黑麥穀袋 1 份作種",
        yield_slug: "wheat_grain",
        yield_name_zh: "飽滿黑麥穀袋",
        yield_count: 2,
        weight_units: 3,
        grow_days: 2,
    },
    CropSpec {
        kind: "paddy_grain",
        name_zh: "水田香穀",
        seed_slugs: &["wheat_grain"],
        seed_hint_zh: "飽滿黑麥穀袋 1 份作種",
        yield_slug: "wheat_grain",
        yield_name_zh: "飽滿黑麥穀袋",
        yield_count: 3,
        weight_units: 3,
        grow_days: 2,
    },
    CropSpec {
        kind: "herb_garden",
        name_zh: "山林止血藥圃",
        seed_slugs: &["herb_bundle", "tough_vine"],
        seed_hint_zh: "止血草束或韌藤草根 1 份作苗",
        yield_slug: "herb_bundle",
        yield_name_zh: "山林止血草束",
        yield_count: 2,
        weight_units: 1,
        grow_days: 2,
    },
    CropSpec {
        kind: "wormwood_bed",
        name_zh: "清冽苦艾藥圃",
        seed_slugs: &["quarantine_wormwood", "herb_bundle"],
        seed_hint_zh: "苦艾束或山林草苗 1 份作苗",
        yield_slug: "quarantine_wormwood",
        yield_name_zh: "南岸清冽苦艾束",
        yield_count: 2,
        weight_units: 1,
        grow_days: 2,
    },
];

fn lookup_crop(kind: &str) -> Option<CropSpec> {
    CROP_CATALOG.iter().copied().find(|c| c.kind == kind)
}

fn take_first_seed(backpack: &mut Vec<CarryItem>, slugs: &[&str]) -> Option<CarryItem> {
    for slug in slugs {
        if let Some(item) = take_clean_carry_items(backpack, slug, 1) {
            return Some(item);
        }
    }
    None
}

pub fn plot_growth_stage_zh(plot: &FarmPlotRecord, game_day: i64, flood_level: u8) -> String {
    if game_day >= plot.mature_day {
        format!("{}已飽滿熟成、隨風輕晃，隨時可下田收割", plot.crop_name_zh)
    } else if plot.mature_day - game_day <= 1 {
        if flood_level >= 2 && plot.crop_kind == "dry_wheat" {
            format!("{}已抽穗成形，田溝積水略深，宜趁晴疏水照料", plot.crop_name_zh)
        } else {
            format!("{}長勢茁壯、枝葉油亮，再過不久便能收成", plot.crop_name_zh)
        }
    } else {
        format!("{}剛播種入土、嫩芽初冒，土面透著濕潤地氣", plot.crop_name_zh)
    }
}

pub fn is_farmable_node(doc: &WorldStaticDoc, node: &SpatialNodeDef) -> bool {
    if node.indoor || node.dungeon {
        return false;
    }
    let Some(cell) = doc.cells.get(&node.cell_id.to_string()) else {
        return false;
    };
    if cell.kind == "settlement" || cell.height > 26 {
        return false;
    }
    matches!(
        node.kind.as_str(),
        "wild" | "village_gate" | "road" | "plaza" | "campfire_nook"
    )
}

pub fn apply_sow_farm_plot(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
    crop_kind: &str,
) -> Result<String, String> {
    ensure_live_land_seeded(doc, delta, clock.game_days);
    if !is_farmable_node(doc, node) {
        return Err("此處為城鎮石板街區或險峻岩坡，請移步城外鄉村空地或野外平緩坡地開墾播種。".to_string());
    }
    let plague_m = delta
        .live_land
        .cells
        .get(&node.cell_id)
        .map_or(0, |cl| cl.plague_miasmas);
    if plague_m >= 30 && crop_kind.trim() != "wormwood_bed" {
        return Err("此處疫氣未散，僅能先種植清冽苦艾藥圃或焚燻苦艾淨地。".to_string());
    }
    let Some(spec) = lookup_crop(if crop_kind.trim().is_empty() {
        "dry_wheat"
    } else {
        crop_kind.trim()
    }) else {
        return Err("請選擇作物種類（旱田黑麥／水田香穀／山林藥圃／苦艾藥圃）。".to_string());
    };
    let plot_id = format!("plot:{}:{}", node.id, aid);
    if delta.farm_plots.contains_key(&plot_id) {
        return Err("你在此處已有一片正在生長的田壟，請先照料或待熟成收割後再續種。".to_string());
    }
    if delta.actors.get(aid).is_some_and(|a| a.fatigue >= 86) {
        return Err("腰背酸乏，先吃份熱食或歇息片刻再下田起壟播種。".to_string());
    }

    let (owner_name, tool_bonus, seed_name, craft_prose, craft_save) = {
        let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
        let Some(seed) = take_first_seed(&mut a.backpack, spec.seed_slugs) else {
            return Err(format!(
                "播種【{}】須自備實物種苗（{}），行囊裡目前沒有種子。",
                spec.name_zh, spec.seed_hint_zh
            ));
        };
        let has_farm_tool = a.bound_weapon.slug.contains("hoe")
            || a.bound_weapon.slug.contains("shovel")
            || a.bound_weapon.slug.contains("axe")
            || a.bound_weapon.name.contains('鋤')
            || a.bound_weapon.name.contains('鎬')
            || a.bound_weapon.name.contains('斧');
        let (cp, save) = invoke_craft_phase_effect(a, "harvest");
        (a.display_name.clone(), has_farm_tool, seed.name, cp, save)
    };

    let sub = compiled_island_substrate(doc);
    super::map_walk::ensure_single_actor_grid_seeded(doc, sub, delta, aid);
    let mut carved_tiles = 0usize;
    if let Some((actor_gx, actor_gy)) = resolve_actor_standing_xy(delta, aid) {
        carved_tiles = reclaim_arable_tiles_at_actor(sub, delta, actor_gx, actor_gy, clock.game_days);
        super::map_memory::record_local_witness_at_tile(
            doc,
            delta,
            aid,
            actor_gx as f32 + 0.5,
            actor_gy as f32 + 0.5,
            clock.game_days,
        );
    }

    let flood_lvl = delta
        .live_land
        .cells
        .get(&node.cell_id)
        .map_or(0, |cl| cl.flood_level);
    let grow_days = if spec.kind == "paddy_grain" && flood_lvl >= 1 {
        (spec.grow_days - 1).max(1)
    } else if spec.kind == "dry_wheat" && flood_lvl >= 2 {
        spec.grow_days + 1
    } else {
        spec.grow_days
    };
    delta.farm_plots.insert(
        plot_id.clone(),
        FarmPlotRecord {
            plot_id,
            node_id: node.id.clone(),
            node_name_zh: node.name.clone(),
            cell_id: node.cell_id,
            owner_id: aid.to_string(),
            owner_name: owner_name.clone(),
            crop_kind: spec.kind.to_string(),
            crop_name_zh: spec.name_zh.to_string(),
            planted_day: clock.game_days,
            mature_day: clock.game_days + grow_days,
            tended_day: -1,
        },
    );
    if let Some(cl) = delta.live_land.cells.get_mut(&node.cell_id) {
        cl.reclaimed_plots = cl.reclaimed_plots.saturating_add(1).min(12);
        cl.fallow_days = 0;
        cl.forest_density = cl.forest_density.saturating_sub(3);
    }
    delta.extras_dirty = true;

    let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
    let fat_cost = (if tool_bonus { 8u8 } else { 12u8 })
        .saturating_sub(craft_save)
        .max(4);
    a.fatigue = a.fatigue.saturating_add(fat_cost).min(95);
    a.mind.stm.today_goal_progressed = true;
    a.mark_mutated();
    let land_tail = if carved_tiles > 0 {
        "腳下坡地已實地翻出新墾熟田"
    } else {
        "順著坡畔既有熟土起妥新壟"
    };
    let msg = format!(
        "你在【{}】翻鬆表土起出田壟（{craft_prose}），灑下隨身【{seed_name}】播成一片【{}】，{land_tail}。",
        node.name, spec.name_zh
    );
    push_breath(a, msg.clone());
    broadcast_scene_event(
        doc,
        delta,
        clock,
        &node.id,
        aid,
        &owner_name,
        EventPublicity::PublicLoud,
        None,
        40,
        25,
        ValueAxis::CraftPride,
        "sow_farm_plot",
        &format!("{owner_name}在【{}】灑種新墾起一片【{}】田壟", node.name, spec.name_zh),
    );
    Ok(msg)
}

pub fn apply_tend_farm_plot(
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
    param: &str,
) -> Result<String, String> {
    let target_key = if !param.trim().is_empty() && delta.farm_plots.contains_key(param.trim()) {
        param.trim().to_string()
    } else {
        let direct = format!("plot:{}:{}", node.id, aid);
        if delta.farm_plots.contains_key(&direct) {
            direct
        } else {
            delta
                .farm_plots
                .values()
                .find(|p| p.node_id == node.id && p.owner_id == aid)
                .map(|p| p.plot_id.clone())
                .ok_or("眼前並無由你照料的田壟藥圃。")?
        }
    };
    let Some(plot) = delta.farm_plots.get_mut(&target_key) else {
        return Err("找不到這片田壟紀錄。".to_string());
    };
    if plot.node_id != node.id {
        return Err("須站在田壟所在地塊旁方能澆水除草。".to_string());
    }
    if clock.game_days >= plot.mature_day {
        return Ok(format!("這片【{}】已飽滿熟成，直接下田收割即可！", plot.crop_name_zh));
    }
    if plot.tended_day == clock.game_days {
        return Ok(format!("今日已替【{}】澆水除草，靜候日照熟成即可。", plot.crop_name_zh));
    }
    plot.tended_day = clock.game_days;
    plot.mature_day = (plot.mature_day - 1).max(clock.game_days);
    let crop_name = plot.crop_name_zh.clone();
    let ready_now = clock.game_days >= plot.mature_day;
    delta.extras_dirty = true;

    let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
    let (craft_prose, save) = invoke_craft_phase_effect(a, "harvest");
    a.fatigue = a.fatigue.saturating_add(5u8.saturating_sub(save).max(2)).min(95);
    a.mind.stm.today_goal_progressed = true;
    a.mark_mutated();
    let msg = if ready_now {
        format!("你順著田溝引水清草（{craft_prose}），【{crop_name}】吸飽水氣後葉莖飽滿，此刻已可當場收割！")
    } else {
        format!("你順著田溝引水除草並培穩根土（{craft_prose}），【{crop_name}】長勢明顯加快。")
    };
    push_breath(a, msg.clone());
    Ok(msg)
}

pub fn apply_harvest_farm_plot(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
    param: &str,
) -> Result<String, String> {
    let target_key = if !param.trim().is_empty() && delta.farm_plots.contains_key(param.trim()) {
        param.trim().to_string()
    } else {
        let direct = format!("plot:{}:{}", node.id, aid);
        if delta.farm_plots.contains_key(&direct) {
            direct
        } else {
            delta
                .farm_plots
                .values()
                .find(|p| p.node_id == node.id && p.owner_id == aid)
                .map(|p| p.plot_id.clone())
                .ok_or("眼前並無屬於你的熟成田壟可收割。")?
        }
    };
    let Some(plot) = delta.farm_plots.get(&target_key).cloned() else {
        return Err("找不到這片田壟紀錄。".to_string());
    };
    if plot.node_id != node.id {
        return Err("須親自走到田壟邊才能動手收割。".to_string());
    }
    if clock.game_days < plot.mature_day {
        return Err(format!("這片【{}】尚未飽滿熟成，可先順手照料或明日再來看。", plot.crop_name_zh));
    }
    let Some(spec) = lookup_crop(&plot.crop_kind) else {
        return Err("作物資料異常，暫時無法收割。".to_string());
    };
    delta.farm_plots.remove(&target_key);
    delta.extras_dirty = true;

    let (owner_name, msg) = {
        let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
        let (craft_prose, save) = invoke_craft_phase_effect(a, "harvest");
        a.fatigue = a.fatigue.saturating_add(7u8.saturating_sub(save).max(3)).min(95);
        a.mind.ltm.mastery.craft_works = a.mind.ltm.mastery.craft_works.saturating_add(1);
        a.mind.stm.today_goal_progressed = true;
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: spec.yield_slug.to_string(),
                name: spec.yield_name_zh.to_string(),
                count: spec.yield_count,
                weight_units: spec.weight_units,
                ..CarryItem::default()
            },
        );
        a.mark_mutated();
        let m = format!(
            "你收割完這片熟透的【{}】（{craft_prose}），捆妥 {} 份【{}】收入行囊，熟土空壟仍留待下回撒種。",
            plot.crop_name_zh, spec.yield_count, spec.yield_name_zh
        );
        push_breath(a, m.clone());
        (a.display_name.clone(), m)
    };
    broadcast_scene_event(
        doc,
        delta,
        clock,
        &node.id,
        aid,
        &owner_name,
        EventPublicity::PublicLoud,
        None,
        45,
        25,
        ValueAxis::CraftPride,
        "harvest_farm_plot",
        &format!("{owner_name}在【{}】收成了一茬【{}】", node.name, plot.crop_name_zh),
    );
    Ok(msg)
}

pub fn append_farm_plot_scene_objects(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    viewer: &UnifiedMindBody,
    node: &SpatialNodeDef,
    clock: &SolarClockSnapshot,
    objects: &mut Vec<Value>,
    push_audit: &mut impl FnMut(&str, &str, &str, &str, &str, &str),
) {
    if let Some(plot) = delta
        .farm_plots
        .values()
        .find(|p| p.node_id == node.id && p.owner_id == viewer.actor_id)
    {
        let fl = delta.live_land.cells.get(&plot.cell_id).map_or(0, |cl| cl.flood_level);
        let stage_zh = plot_growth_stage_zh(plot, clock.game_days, fl);
        let ready = clock.game_days >= plot.mature_day;
        let opts = if ready {
            push_audit("harvest_farm_plot", &plot.plot_id, "下田收割熟成作物", "收割入囊", "object", "obj:farm_plot");
            vec![json!({
                "id": "harvest_farm_plot",
                "param": plot.plot_id,
                "label": format!("收割【{}】入囊", plot.crop_name_zh),
                "sub": "整壟收成入囊，熟土留待下茬",
            })]
        } else {
            push_audit("tend_farm_plot", &plot.plot_id, "澆水除草照料田壟", "引水催熟", "object", "obj:farm_plot");
            vec![json!({
                "id": "tend_farm_plot",
                "param": plot.plot_id,
                "label": format!("引水除草照料【{}】", plot.crop_name_zh),
                "sub": "清草疏水加速抽穗熟成",
            })]
        };
        objects.push(json!({
            "object_id": "obj:farm_plot",
            "name": format!("你的【{}】田壟", plot.crop_name_zh),
            "cue": stage_zh,
            "desc": format!("你在【{}】親手開墾的田壟，土色濕潤深厚。", node.name),
            "direct_action": Value::Null,
            "options": opts,
        }));
    } else if is_farmable_node(doc, node) {
        let mut sow_opts: Vec<Value> = Vec::new();
        for c in CROP_CATALOG {
            let has_seed = viewer
                .backpack
                .iter()
                .any(|i| i.count > 0 && c.seed_slugs.contains(&i.slug.as_str()));
            if !has_seed {
                continue;
            }
            let lbl = format!("播種開墾【{}】", c.name_zh);
            sow_opts.push(json!({
                "id": "sow_farm_plot",
                "param": c.kind,
                "label": lbl,
                "sub": format!("耗{}・熟成收【{}】", c.seed_hint_zh, c.yield_name_zh),
            }));
            push_audit("sow_farm_plot", c.kind, &lbl, "開墾播種", "object", "obj:arable_slope");
        }
        let (cue, desc) = if sow_opts.is_empty() {
            (
                "可開墾平緩沃土坡地（須備穀種、草束、韌藤或避瘴艾草方能播種）",
                "這片平緩坡地土層鬆軟；攜帶【白鷗麥種／草束／韌藤纖維／避瘴艾草】任一種苗前來，即可就地起壟開墾成旱田、水田或藥圃。",
            )
        } else {
            (
                "行囊備有種苗，可就地起壟耕作",
                "這片平緩坡地土層鬆軟，灑下隨身穀種或草苗即可就地起壟成田。",
            )
        };
        objects.push(json!({
            "object_id": "obj:arable_slope",
            "name": "可開墾沃土坡地",
            "cue": cue,
            "desc": desc,
            "direct_action": Value::Null,
            "options": sow_opts,
        }));
    }
}

/// 身故遺澤完整承接（`DESIGN_KEYWORDS.md` L209：農田田壟、店鋪租契、全身多部位護具、居所陳設與個人成相《遺稿手札》真實留存於世並交予繼承人）
pub fn transfer_wave4_legacy_estate_to_heir(
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    deceased_id: &str,
    deceased_name: &str,
    heir_id: &str,
) -> Vec<String> {
    let mut notes: Vec<String> = Vec::new();
    let heir_name = delta
        .actors
        .get(heir_id)
        .map_or_else(|| "至親".to_string(), |h| h.display_name.clone());

    // 1. 農田與藥圃田壟繼承（重新歸戶至繼承人名下，不隨身故蒸發）
    let dec_plot_keys: Vec<String> = delta
        .farm_plots
        .iter()
        .filter(|(_, p)| p.owner_id == deceased_id)
        .map(|(k, _)| k.clone())
        .collect();
    let mut transferred_plots = 0usize;
    for old_key in dec_plot_keys {
        if let Some(mut plot) = delta.farm_plots.remove(&old_key) {
            let new_key = format!("plot:{}:{heir_id}", plot.node_id);
            plot.plot_id = new_key.clone();
            plot.owner_id = heir_id.to_string();
            plot.owner_name = heir_name.clone();
            if !delta.farm_plots.contains_key(&new_key) {
                delta.farm_plots.insert(new_key, plot);
                transferred_plots += 1;
            } else {
                // 若繼承人在同格已有田壟，保留該片已翻熟地留歸原地
                delta.farm_plots.insert(old_key, plot);
            }
        }
    }
    if transferred_plots > 0 {
        delta.extras_dirty = true;
        notes.push(format!("{transferred_plots} 處熟地田壟歸戶予{heir_name}"));
    }

    // 2. 店鋪所有權與營運租契承接
    let mut transferred_leases = 0usize;
    for lease in delta.shop_leases.values_mut() {
        let mut changed = false;
        if lease.owner_actor_id == deceased_id {
            lease.owner_actor_id = heir_id.to_string();
            changed = true;
        }
        if lease.operator_actor_id.as_deref() == Some(deceased_id) {
            lease.operator_actor_id = Some(heir_id.to_string());
            changed = true;
        }
        if changed {
            transferred_leases += 1;
        }
    }
    if transferred_leases > 0 {
        delta.extras_dirty = true;
        notes.push(format!("{transferred_leases} 紙鋪面租契由{heir_name}接手經營"));
    }

    // 3. 穿戴中多部位護具、居所陳設與成相遺稿手札承接
    let (legacy_gear, legacy_furnishings, posthumous_manual) = {
        let Some(dec) = delta.actors.get_mut(deceased_id) else {
            return notes;
        };
        let mut gear_items: Vec<CarryItem> = Vec::new();
        for piece_opt in [
            dec.body_gear.arms.take(),
            dec.body_gear.feet.take(),
            dec.body_gear.torso.take(),
        ] {
            if let Some(mut piece) = piece_opt {
                piece.provenance = if piece.provenance.is_empty() {
                    format!("{deceased_name}生前遺留護具")
                } else {
                    format!("{}・{deceased_name}遺留", piece.provenance)
                };
                gear_items.push(CarryItem {
                    slug: piece.slug.clone(),
                    name: piece.name.clone(),
                    count: 1,
                    weight_units: 1,
                    gear_meta: Some(piece),
                    ..CarryItem::default()
                });
            }
        }
        let furns = std::mem::take(&mut dec.lifestyle.home_furnishings);
        let manual_opt = if dec.mind.ltm.mastery.martial_bouts >= 2
            || dec.mind.ltm.mastery.private_martial_phase.is_some()
        {
            let bone = super::phase_and_vessels::actor_bone_phase(dec);
            let carrier = super::phase_and_vessels::tool_carrier_of(
                &dec.bound_weapon.slug,
                &dec.bound_weapon.name,
            );
            let prac = u32::from(dec.mind.ltm.mastery.martial_bouts)
                + u32::from(dec.bound_weapon.grip_familiarity / 2);
            let r = crate::combat::human_phase::derive_personal_martial_routine(
                &bone,
                crate::combat::wordplate::TacticalSlot::Finisher,
                carrier,
                &dec.bound_weapon.name,
                super::martial_art_and_manuals::actor_learned_bonus_for_slot(
                    dec,
                    crate::combat::wordplate::TacticalSlot::Finisher,
                ),
                prac,
                crate::combat::wordplate::WordRegistry::global(),
            );
            let bname = format!("《{}・遺稿手札》", r.formal_name);
            Some(CarryItem {
                slug: format!("manual_legacy_{}_{}", deceased_id, clock.game_days),
                name: bname,
                count: 1,
                weight_units: 1,
                manual_meta: Some(super::martial_art_and_manuals::MartialManualMeta {
                    author_name: deceased_name.to_string(),
                    slot_code: "finisher".to_string(),
                    carrier_zh: dec.bound_weapon.name.clone(),
                    formal_name: r.formal_name,
                    motions: r.main_motions.to_vec(),
                    turn_branch_zh: format!(
                        "遇{}則轉「{}」",
                        r.turn_counter_target_zh, r.turn_branch_motion
                    ),
                    origin_seed: dec.soul_seed,
                }),
                ..CarryItem::default()
            })
        } else {
            None
        };
        dec.mark_mutated();
        (gear_items, furns, manual_opt)
    };

    if let Some(heir) = delta.actors.get_mut(heir_id) {
        let gear_cnt = legacy_gear.len();
        for g in legacy_gear {
            add_carry_item(&mut heir.backpack, g);
        }
        if gear_cnt > 0 {
            notes.push(format!("{gear_cnt} 件身披護具轉交{heir_name}"));
        }
        let mut furn_added = 0usize;
        for f in legacy_furnishings {
            if !heir.lifestyle.home_furnishings.contains(&f) {
                heir.lifestyle.home_furnishings.push(f);
                furn_added += 1;
            }
        }
        if furn_added > 0 {
            notes.push(format!("{furn_added} 件居所傢俱陳設移交{heir_name}"));
        }
        if let Some(m_item) = posthumous_manual {
            let mname = m_item.name.clone();
            add_carry_item(&mut heir.backpack, m_item);
            notes.push(format!("生前絕學整理為{mname}留傳{heir_name}"));
        }
        if !notes.is_empty() {
            push_breath(
                heir,
                format!("承接了{deceased_name}生前遺澤：{}。", notes.join("、")),
            );
            heir.mark_mutated();
        }
    }
    notes
}
