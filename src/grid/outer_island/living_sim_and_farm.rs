//! 第四刀（上・拆骨重構版）：零無中生有之日常起居（熱食／溫水沐浴／實體穿搭／實材居所陳設）。
//! 農田藥圃邏輯已拆至 `farm_plots.rs` 以守單檔體積預算，並於本檔統一對外匯出與路由。

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::clock_and_lang::SolarClockSnapshot;
use super::live_commodities::{consume_shop_stock_one, quote_tavern_ration_copper};
use super::martial_art_and_manuals::invoke_craft_phase_effect;
use super::mind::EventPublicity;
use super::movement::{add_carry_item, push_breath, take_clean_carry_items};
use super::persona::ValueAxis;
use super::shop_leases::infer_shop_kind;
use super::sim_tick::broadcast_scene_event;
use super::types::{CarryItem, IslandWorldDelta, SpatialNodeDef, UnifiedMindBody, WorldStaticDoc};

pub use super::farm_plots::{
    FarmPlotRecord, append_farm_plot_scene_objects, apply_harvest_farm_plot, apply_sow_farm_plot,
    apply_tend_farm_plot, is_farmable_node, plot_growth_stage_zh,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LivingSimState {
    pub outfit_slug: String,
    pub outfit_name_zh: String,
    pub last_bath_day: i64,
    pub last_hot_meal_day: i64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub home_furnishings: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct OutfitSpec {
    pub slug: &'static str,
    pub item_slug: &'static str,
    pub name_zh: &'static str,
    pub appearance_zh: &'static str,
    pub tailor_fee: i64,
    pub mat_slugs: &'static [&'static str],
    pub mat_hint_zh: &'static str,
    pub vibe_zh: &'static str,
}

pub const OUTFIT_CATALOG: &[OutfitSpec] = &[
    OutfitSpec {
        slug: "cotton_daily",
        item_slug: "outfit_cotton_daily",
        name_zh: "輕便棉麻日常裝",
        appearance_zh: "身著清爽透氣的米白棉麻襯衣與素色長褲",
        tailor_fee: 4,
        mat_slugs: &["tough_vine", "herb_bundle"],
        mat_hint_zh: "韌藤草纖或草束 1 份",
        vibe_zh: "日常街坊起居舒爽輕快",
    },
    OutfitSpec {
        slug: "waxed_hunter",
        item_slug: "outfit_waxed_hunter",
        name_zh: "防風蠟布機能獵裝",
        appearance_zh: "穿著深橄欖色防風蠟布短外套與多袋獵褲",
        tailor_fee: 8,
        mat_slugs: &["beast_tallow", "lizard_scale", "spider_carapace"],
        mat_hint_zh: "野獸脂塊或異獸鱗甲 1 份",
        vibe_zh: "耐濕耐磨、適合出入山林荒野",
    },
    OutfitSpec {
        slug: "harbor_wool",
        item_slug: "outfit_harbor_wool",
        name_zh: "港埠細呢長衣",
        appearance_zh: "披著剪裁俐落的深藍細呢長衣，領口整潔挺括",
        tailor_fee: 6,
        mat_slugs: &["tough_vine", "beast_tallow"],
        mat_hint_zh: "韌藤草纖或獸脂 1 份",
        vibe_zh: "出入港埠商行與市政廳體面俐落",
    },
    OutfitSpec {
        slug: "felt_winter",
        item_slug: "outfit_felt_winter",
        name_zh: "厚毛氈禦寒行裝",
        appearance_zh: "罩著厚實暖棕毛氈防寒大衣，腰束粗皮扣帶",
        tailor_fee: 6,
        mat_slugs: &["beast_tallow", "tough_vine"],
        mat_hint_zh: "野獸脂塊或韌藤草纖 1 份",
        vibe_zh: "海風寒夜與山嶺遠行護住體溫",
    },
];

#[derive(Debug, Clone, Copy)]
pub struct FurnishingSpec {
    pub slug: &'static str,
    pub name_zh: &'static str,
    pub fee_copper: i64,
    pub mat_slugs: &'static [&'static str],
    pub mat_hint_zh: &'static str,
    pub effect_zh: &'static str,
}

pub const FURNISHING_CATALOG: &[FurnishingSpec] = &[
    FurnishingSpec {
        slug: "pine_desk",
        name_zh: "海松木書案與燭臺",
        fee_copper: 6,
        mat_slugs: &["timber_log", "driftwood"],
        mat_hint_zh: "原木或漂流木 1 份",
        effect_zh: "屋內多了整潔書案，夜裡便於整理手札與見聞",
    },
    FurnishingSpec {
        slug: "copper_lamp",
        name_zh: "壁掛暖光銅燈",
        fee_copper: 6,
        mat_slugs: &["iron_ore", "quarry_stone"],
        mat_hint_zh: "粗鐵礦或方石料 1 份",
        effect_zh: "暖黃油燈柔和映照四壁，入夜歸家格外安心",
    },
    FurnishingSpec {
        slug: "warm_brazier",
        name_zh: "鑄鐵烘衣炭火盆",
        fee_copper: 8,
        mat_slugs: &["iron_ore", "black_coal", "charcoal_sack"],
        mat_hint_zh: "粗鐵礦或精煤木炭 1 份",
        effect_zh: "烘乾海風潮氣，安睡沐浴更加解乏暖身",
    },
    FurnishingSpec {
        slug: "herb_rack",
        name_zh: "樟木草藥晾曬架",
        fee_copper: 5,
        mat_slugs: &["timber_log", "driftwood", "dry_branch"],
        mat_hint_zh: "木料或枯枝 1 份",
        effect_zh: "滿室淡雅草木幽香，便於懸掛採回的草藥山貨",
    },
    FurnishingSpec {
        slug: "woven_rug",
        name_zh: "厚織海草暖腳地毯",
        fee_copper: 5,
        mat_slugs: &["tough_vine", "herb_bundle"],
        mat_hint_zh: "韌藤草纖或草束 1 份",
        effect_zh: "踏進屋門腳底柔軟溫厚，少了石板寒意",
    },
];

const HOT_MEAL_SLUGS: &[&str] = &[
    "fresh_fish_basket",
    "salt_fish_barrel",
    "shellfish_meat",
    "dairy_cheese",
    "wheat_grain",
    "rye_bread",
];

fn lookup_outfit(slug: &str) -> Option<OutfitSpec> {
    OUTFIT_CATALOG
        .iter()
        .copied()
        .find(|o| o.slug == slug || o.item_slug == slug)
}

fn lookup_furnishing(slug: &str) -> Option<FurnishingSpec> {
    FURNISHING_CATALOG.iter().copied().find(|f| f.slug == slug)
}

pub fn furnishing_display_name(slug: &str) -> &str {
    lookup_furnishing(slug).map_or(slug, |f| f.name_zh)
}

pub fn cleanliness_sense_zh(viewer: &UnifiedMindBody, game_day: i64) -> &'static str {
    let gap = game_day.saturating_sub(viewer.lifestyle.last_bath_day);
    if viewer.lifestyle.last_bath_day > 0 && gap <= 0 {
        "剛洗過溫水澡・衣領帶著淡雅松木暖香"
    } else if viewer.lifestyle.last_bath_day > 0 && gap <= 2 {
        "身面清爽整潔・神氣平和"
    } else {
        "連日風塵奔波・衣角略沾海鹽微汗"
    }
}

pub fn hot_meal_sense_zh(viewer: &UnifiedMindBody, game_day: i64) -> &'static str {
    if viewer.lifestyle.last_hot_meal_day == game_day {
        "今日已吃過溫熱熟食・胃裡踏實舒暢"
    } else if viewer.mind.stm.affect.hunger >= 45 {
        "腹中空響・很想喝碗熱湯暖胃"
    } else {
        "今日尚未進過熱食"
    }
}

fn find_present_staff(
    delta: &IslandWorldDelta,
    node_id: &str,
    actor_id: &str,
    roles: &[&str],
) -> Option<(String, String)> {
    let occ = delta.node_occupants.get(node_id)?;
    for oid in occ {
        if oid == actor_id {
            continue;
        }
        if let Some(a) = delta.actors.get(oid)
            && a.alive
            && a.controller_id.is_none()
            && roles.iter().any(|r| a.role == *r || a.role.contains(r))
        {
            return Some((oid.clone(), a.display_name.clone()));
        }
    }
    None
}

fn take_first_matching_material(
    backpack: &mut Vec<CarryItem>,
    slugs: &[&str],
) -> Option<CarryItem> {
    for slug in slugs {
        if let Some(item) = take_clean_carry_items(backpack, slug, 1) {
            return Some(item);
        }
    }
    None
}

fn is_tavern_node(node: &SpatialNodeDef) -> bool {
    node.kind == "shop" && (infer_shop_kind(node) == "tavern" || node.name.contains("酒館"))
}

pub fn apply_take_warm_bath(
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
) -> Result<String, String> {
    let (in_owned, in_paid_inn, already_today, copper) = {
        let a = delta.actors.get(aid).ok_or("找不到旅人行蹤")?;
        (
            a.owned_house_node.as_deref() == Some(node.id.as_str()),
            node.kind == "inn_room" && a.inn_nights_left > 0,
            a.lifestyle.last_bath_day == clock.game_days && a.lifestyle.last_bath_day > 0,
            a.copper,
        )
    };
    let tavern_keeper = if is_tavern_node(node) {
        find_present_staff(delta, &node.id, aid, &["酒館掌櫃", "酒館掌櫃學徒"])
    } else {
        None
    };
    if !in_owned && !in_paid_inn && tavern_keeper.is_none() {
        return Err("須在自家屋舍、已付租的二樓宿房，或有掌櫃值守的酒館方有熱水浴桶可用。".to_string());
    }
    if already_today {
        return Ok("今日已泡過溫水澡，周身仍清爽帶著松香。".to_string());
    }
    if !in_owned && !in_paid_inn {
        let Some((ref kid, ref kname)) = tavern_keeper else {
            return Err("酒館此刻無人看爐燒水。".to_string());
        };
        if copper < 2 {
            return Err("手頭盤纏不足 2 枚銅幣，請不動後堂燒桶熱水。".to_string());
        }
        if !delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "warm_bath",
            aid,
            kid,
            2,
            format!("向{kname}付後堂溫水浴桶費"),
        ) {
            return Err("結算浴湯費時帳目周轉受阻。".to_string());
        }
    }
    let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
    let has_brazier = a.lifestyle.home_furnishings.iter().any(|f| f == "warm_brazier");
    let relief = if in_owned && has_brazier { 32 } else { 24 };
    a.fatigue = a.fatigue.saturating_sub(relief);
    a.mind.stm.affect.anger = a.mind.stm.affect.anger.saturating_sub(15);
    a.mind.stm.affect.fear = a.mind.stm.affect.fear.saturating_sub(12);
    a.lifestyle.last_bath_day = clock.game_days;
    a.mind.stm.today_goal_progressed = true;
    a.mark_mutated();
    let msg = if in_owned || in_paid_inn {
        "你燒暖一桶松葉溫水，一口氣洗去連日風塵海鹽，換上乾爽內衫後肩背鬆快了許多。".to_string()
    } else {
        "你付了 2 枚銅幣借酒館後堂溫水浴桶洗去一身海鹽汗垢，出來時神清氣爽、步履輕快許多。".to_string()
    };
    push_breath(a, msg.clone());
    Ok(msg)
}

pub fn apply_enjoy_hot_meal(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
) -> Result<String, String> {
    let (in_owned, already_today, copper) = {
        let a = delta.actors.get(aid).ok_or("找不到旅人行蹤")?;
        (
            a.owned_house_node.as_deref() == Some(node.id.as_str()),
            a.lifestyle.last_hot_meal_day == clock.game_days && a.lifestyle.last_hot_meal_day > 0,
            a.copper,
        )
    };
    if already_today {
        return Ok("今日已吃過一頓飽足暖胃的熱食，此刻腹中踏實。".to_string());
    }

    if in_owned {
        let (ing_name, craft_prose) = {
            let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
            let Some(item) = take_first_matching_material(&mut a.backpack, HOT_MEAL_SLUGS) else {
                return Err("自家灶上沒有食材可下鍋（需自備魚簍、海貝、乳酪、黑麥穀袋或乾糧 1 份）。".to_string());
            };
            let (cp, bonus) = invoke_craft_phase_effect(a, "culinary");
            a.fatigue = a.fatigue.saturating_sub(24 + bonus * 2);
            a.mind.stm.affect.hunger = a.mind.stm.affect.hunger.saturating_sub(68);
            a.mind.stm.affect.anger = a.mind.stm.affect.anger.saturating_sub(8);
            a.lifestyle.last_hot_meal_day = clock.game_days;
            a.mind.stm.today_goal_progressed = true;
            a.mark_mutated();
            (item.name, cp)
        };
        let msg = format!(
            "你在自家小灶將【{ing_name}】細火慢煮成一鍋熱騰騰的鮮湯炊飯（{craft_prose}），趁熱喝下暖透四肢。"
        );
        if let Some(a) = delta.actors.get_mut(aid) {
            push_breath(a, msg.clone());
        }
        return Ok(msg);
    }

    if !is_tavern_node(node) {
        return Err("此處並無營業灶台，請前往有掌櫃當值的酒館或帶食材回自家屋舍煮熱食。".to_string());
    }
    let Some((keeper_id, keeper_name)) =
        find_present_staff(delta, &node.id, aid, &["酒館掌櫃", "酒館掌櫃學徒"])
    else {
        return Err("灶房火勢收小，酒館掌櫃與學徒此刻不在櫃台前。".to_string());
    };
    let price = quote_tavern_ration_copper(doc, delta, node.cell_id, &node.id).max(2);
    if copper < price {
        return Err(format!("此刻熱湯飯時價 {price} 枚銅幣，手頭盤纏尚不足。"));
    }
    let Some((holder_id, used_food)) =
        consume_shop_stock_one(delta, Some(&keeper_id), &node.id, HOT_MEAL_SLUGS)
    else {
        return Err(
            "掌櫃掀開陶鍋無奈搖頭：「今日酒館存的魚糧與黑麥都售罄見底了，得等鄉村農戶或漁船運貨進城才能再開灶！」"
                .to_string(),
        );
    };
    if !delta.transfer_copper(
        clock.game_days,
        clock.bell_index,
        "hot_meal",
        aid,
        &keeper_id,
        price,
        format!("向{keeper_name}點熱湯熟食（耗{}）", used_food.name),
    ) {
        super::live_commodities::restore_shop_stock_one(delta, &holder_id, &node.id, used_food);
        return Err("結算餐費時帳目周轉受阻。".to_string());
    }

    let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
    a.fatigue = a.fatigue.saturating_sub(26);
    a.mind.stm.affect.hunger = a.mind.stm.affect.hunger.saturating_sub(68);
    a.mind.stm.affect.anger = a.mind.stm.affect.anger.saturating_sub(8);
    a.lifestyle.last_hot_meal_day = clock.game_days;
    a.mind.stm.today_goal_progressed = true;
    a.mark_mutated();
    let msg = format!(
        "你付了 {price} 枚銅幣，{keeper_name}取灶房現存的【{}】現煮了一大碗滾燙濃湯配烤餅端上桌；喝下後四肢回暖、倦意大減。",
        used_food.name
    );
    push_breath(a, msg.clone());
    Ok(msg)
}

pub fn apply_change_outfit(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
    outfit_slug: &str,
) -> Result<String, String> {
    let Some(spec) = lookup_outfit(outfit_slug.trim()) else {
        return Err("請選擇欲換穿或訂製的衣物款式。".to_string());
    };
    let (already_wearing, in_bag) = {
        let a = delta.actors.get(aid).ok_or("找不到旅人行蹤")?;
        (
            a.lifestyle.outfit_slug == spec.slug,
            a.backpack.iter().any(|i| i.slug == spec.item_slug && i.count > 0),
        )
    };
    if already_wearing {
        return Ok(format!("你身上正穿著【{}】，衣料合身俐落。", spec.name_zh));
    }

    if in_bag {
        let actor_name = {
            let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
            if take_clean_carry_items(&mut a.backpack, spec.item_slug, 1).is_none() {
                return Err(format!("行囊裡的【{}】未能取出，無法換穿。", spec.name_zh));
            }
            if !a.lifestyle.outfit_slug.is_empty()
                && let Some(prev) = lookup_outfit(&a.lifestyle.outfit_slug)
            {
                add_carry_item(
                    &mut a.backpack,
                    CarryItem {
                        slug: prev.item_slug.to_string(),
                        name: prev.name_zh.to_string(),
                        count: 1,
                        weight_units: 1,
                        ..CarryItem::default()
                    },
                );
            }
            a.lifestyle.outfit_slug = spec.slug.to_string();
            a.lifestyle.outfit_name_zh = spec.name_zh.to_string();
            a.appearance = spec.appearance_zh.to_string();
            a.mark_mutated();
            let m = format!("你從行囊取出【{}】換上身，將原先衣衫摺妥收起。", spec.name_zh);
            push_breath(a, m.clone());
            (a.display_name.clone(), m)
        };
        broadcast_scene_event(
            doc,
            delta,
            clock,
            &node.id,
            aid,
            &actor_name.0,
            EventPublicity::PublicLoud,
            None,
            30,
            15,
            ValueAxis::CraftPride,
            "change_outfit",
            &format!("{}換穿了【{}】", actor_name.0, spec.name_zh),
        );
        return Ok(actor_name.1);
    }

    let in_owned = delta
        .actors
        .get(aid)
        .is_some_and(|a| a.owned_house_node.as_deref() == Some(node.id.as_str()));
    let tailor = find_present_staff(
        delta,
        &node.id,
        aid,
        &["行腳商旅", "木匠", "草藥師", "酒館掌櫃", "鍛鐵匠"],
    );
    if !in_owned && tailor.is_none() {
        return Err("此處並無商旅或手作匠人在場接單裁衣，請尋在場商販來料訂製或回自家裁縫。".to_string());
    }
    let fee = if in_owned { 0 } else { spec.tailor_fee };
    let copper = delta.actors.get(aid).map_or(0, |a| a.copper);
    if copper < fee {
        return Err(format!("裁製【{}】手工費需 {fee} 枚銅幣，手頭盤纏尚不足。", spec.name_zh));
    }
    let mut took_from_player: Option<CarryItem> = None;
    let mut took_from_tailor: Option<(String, CarryItem)> = None;
    if let Some(a) = delta.actors.get_mut(aid) {
        took_from_player = take_first_matching_material(&mut a.backpack, spec.mat_slugs);
    }
    if took_from_player.is_none()
        && let Some((ref tid, _)) = tailor
        && let Some(t_actor) = delta.actors.get_mut(tid)
        && let Some(it) = take_first_matching_material(&mut t_actor.backpack, spec.mat_slugs)
    {
        t_actor.mark_mutated();
        took_from_tailor = Some((tid.clone(), it));
    }
    if took_from_player.is_none() && took_from_tailor.is_none() {
        return Err(format!(
            "裁製【{}】須實有原料（{}），你與在場商販手邊目前都缺這批衣料。",
            spec.name_zh, spec.mat_hint_zh
        ));
    }
    if fee > 0
        && let Some((ref tid, ref tname)) = tailor
        && !delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "tailor_outfit",
            aid,
            tid,
            fee,
            format!("請{tname}以實料裁製【{}】", spec.name_zh),
        )
    {
        if let Some(it) = took_from_player
            && let Some(a) = delta.actors.get_mut(aid)
        {
            add_carry_item(&mut a.backpack, it);
        }
        if let Some((t_id, it)) = took_from_tailor
            && let Some(t_actor) = delta.actors.get_mut(&t_id)
        {
            add_carry_item(&mut t_actor.backpack, it);
        }
        return Err("結算手工費時帳目周轉受阻。".to_string());
    }

    let mat_name = took_from_player
        .as_ref()
        .map(|i| i.name.clone())
        .or_else(|| took_from_tailor.as_ref().map(|(_, i)| i.name.clone()))
        .unwrap_or_else(|| "實料".to_string());
    let (actor_name, msg) = {
        let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
        if !a.lifestyle.outfit_slug.is_empty()
            && let Some(prev) = lookup_outfit(&a.lifestyle.outfit_slug)
        {
            add_carry_item(
                &mut a.backpack,
                CarryItem {
                    slug: prev.item_slug.to_string(),
                    name: prev.name_zh.to_string(),
                    count: 1,
                    weight_units: 1,
                    ..CarryItem::default()
                },
            );
        }
        a.lifestyle.outfit_slug = spec.slug.to_string();
        a.lifestyle.outfit_name_zh = spec.name_zh.to_string();
        a.appearance = spec.appearance_zh.to_string();
        a.mind.ltm.mastery.craft_works = a.mind.ltm.mastery.craft_works.saturating_add(1);
        a.mind.stm.today_goal_progressed = true;
        a.mark_mutated();
        let m = if fee > 0 {
            format!(
                "以【{mat_name}】為底料、付了 {fee} 枚銅幣工資裁成【{}】換上身（{}），舊衫已摺入行囊。",
                spec.name_zh, spec.vibe_zh
            )
        } else {
            format!(
                "你在屋內以【{mat_name}】親手裁縫出【{}】換上身（{}），舊衫已摺入行囊。",
                spec.name_zh, spec.vibe_zh
            )
        };
        push_breath(a, m.clone());
        (a.display_name.clone(), m)
    };
    broadcast_scene_event(
        doc,
        delta,
        clock,
        &node.id,
        aid,
        &actor_name,
        EventPublicity::PublicLoud,
        None,
        35,
        20,
        ValueAxis::CraftPride,
        "change_outfit",
        &format!("{actor_name}換穿了一身實料裁製的【{}】", spec.name_zh),
    );
    Ok(msg)
}

pub fn apply_buy_home_furnishing(
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
    furn_slug: &str,
) -> Result<String, String> {
    let Some(spec) = lookup_furnishing(furn_slug.trim()) else {
        return Err("請選擇要為居所打造的木作傢俱或燈火陳設。".to_string());
    };
    let (copper, in_owned, has_home_or_room, already_owned) = {
        let a = delta.actors.get(aid).ok_or("找不到旅人行蹤")?;
        (
            a.copper,
            a.owned_house_node.as_deref() == Some(node.id.as_str()),
            a.owned_house_node.is_some() || a.inn_nights_left > 0 || node.kind == "inn_room",
            a.lifestyle.home_furnishings.iter().any(|f| f == spec.slug),
        )
    };
    if !has_home_or_room {
        return Err("你目前尚未投宿宿房或購置自有宅院，還沒有落腳屋舍可擺放陳設。".to_string());
    }
    if already_owned {
        return Ok(format!("你的居所早已擺妥【{}】，不必重複添置。", spec.name_zh));
    }
    let crafter = find_present_staff(delta, &node.id, aid, &["木匠", "鍛鐵匠", "行腳商旅"]);
    if !in_owned && crafter.is_none() {
        return Err("打造傢俱陳設須尋在場木匠／鍛鐵匠，或帶齊原材回自家動手製作。".to_string());
    }
    let fee = if in_owned { 0 } else { spec.fee_copper };
    if copper < fee {
        return Err(format!("請匠人打造【{}】工錢需 {fee} 枚銅幣，盤纏尚不足。", spec.name_zh));
    }
    let mat_item = {
        let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
        let Some(it) = take_first_matching_material(&mut a.backpack, spec.mat_slugs) else {
            return Err(format!(
                "製作【{}】須自備實料（{}），行囊目前缺料無法開工。",
                spec.name_zh, spec.mat_hint_zh
            ));
        };
        it
    };
    if fee > 0
        && let Some((ref cid, ref cname)) = crafter
        && !delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "craft_furnishing",
            aid,
            cid,
            fee,
            format!("請{cname}以【{}】打造居所【{}】", mat_item.name, spec.name_zh),
        )
    {
        if let Some(a) = delta.actors.get_mut(aid) {
            add_carry_item(&mut a.backpack, mat_item);
        }
        return Err("支付木作工錢時帳目周轉受阻。".to_string());
    }

    let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
    let (craft_prose, _) = invoke_craft_phase_effect(a, "forge");
    a.lifestyle.home_furnishings.push(spec.slug.to_string());
    a.mind.ltm.mastery.craft_works = a.mind.ltm.mastery.craft_works.saturating_add(1);
    a.mind.stm.today_goal_progressed = true;
    a.mark_mutated();
    let msg = if fee > 0 {
        format!(
            "你交出【{}】並付 {fee} 枚銅幣工資請匠人做好【{}】擺進屋內——{}。",
            mat_item.name, spec.name_zh, spec.effect_zh
        )
    } else {
        format!(
            "你以【{}】親手在屋內做好【{}】（{craft_prose}）——{}。",
            mat_item.name, spec.name_zh, spec.effect_zh
        )
    };
    push_breath(a, msg.clone());
    Ok(msg)
}

/// 修補受損之全身多部位護具（開爐鍛鐵鋪 2 銅＋1 燃料，或在任意處消耗 1 份韌藤／粗鐵礦現場修補，絕不無中生有）
pub fn apply_repair_body_gear(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    slot: &str,
) -> Result<String, String> {
    let aid = delta
        .resolve_actor_id(actor_key)
        .ok_or("請先向港務官核對通行身分。")?;
    let slot_key = slot.trim();
    let (node_id, gname, wear_now) = {
        let a = delta.actors.get(&aid).ok_or("找不到旅人行蹤")?;
        let piece = match slot_key {
            "arms" => a.body_gear.arms.as_ref(),
            "feet" => a.body_gear.feet.as_ref(),
            "torso" => a.body_gear.torso.as_ref(),
            _ => None,
        }
        .ok_or("該部位目前並未穿戴護具。")?;
        (a.node_id.clone(), piece.name.clone(), piece.wear_stage)
    };
    if wear_now == 0 {
        return Ok(format!("【{gname}】目前完好貼合，尚無需修補。"));
    }
    let node = doc.nodes.get(&node_id).ok_or("找不到所在處")?;
    let at_open_smithy = !clock.is_night
        && node.kind == "shop"
        && (super::shop_leases::infer_shop_kind(node) == "smithy"
            || node.name.contains("鍛鐵")
            || node.name.contains("鐵匠")
            || node.name.contains("集坊"))
        && !super::live_commodities::is_smithy_forge_cold(delta, &node_id);

    let self_operates = super::shop_leases::actor_operates_shop(delta, &aid, &node_id);
    let smith_opt = if at_open_smithy {
        find_present_staff(delta, &node_id, &aid, &["鍛鐵匠", "木匠"]).map(|(id, _)| id)
    } else {
        None
    };

    let used_note = if at_open_smithy && (self_operates || smith_opt.is_some()) {
        let fee = if self_operates { 0i64 } else { 2i64 };
        if fee > 0
            && let Some(ref sid) = smith_opt
            && !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "repair_body_gear",
                &aid,
                sid,
                fee,
                format!("請工坊修補【{gname}】"),
            )
        {
            return Err("手頭銅幣不足 2 枚，無法付工坊修補費；亦可自備韌藤或粗鐵礦就地修補。".to_string());
        }
        if super::live_commodities::consume_smithy_fuel_one(delta, smith_opt.as_deref(), &node_id)
            .is_none()
        {
            if fee > 0
                && let Some(ref sid) = smith_opt
            {
                delta.transfer_copper(
                    clock.game_days,
                    clock.bell_index,
                    "repair_body_gear_refund",
                    sid,
                    &aid,
                    fee,
                    "鍛爐炭火不足，退還護具修補工錢".to_string(),
                );
            }
            return Err("鍛爐炭火已盡，暫無法在爐邊修補護具。".to_string());
        }
        if fee > 0 {
            format!("於工坊付妥 {fee} 枚銅幣借爐整修")
        } else {
            "在自家開爐工坊整修".to_string()
        }
    } else {
        let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
        if take_clean_carry_items(&mut a.backpack, "tough_vine", 1).is_some() {
            "取行囊裡 1 份【韌藤纖維】重新纏緊加固".to_string()
        } else if take_clean_carry_items(&mut a.backpack, "iron_ore", 1).is_some() {
            "取行囊裡 1 份【粗鐵礦石】敲補鉚緊".to_string()
        } else {
            return Err("修補護具須回開爐鍛鐵鋪（2 枚銅幣），或自備 1 份【韌藤纖維】／【粗鐵礦石】現場修補。".to_string());
        }
    };

    let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
    let piece = match slot_key {
        "arms" => a.body_gear.arms.as_mut(),
        "feet" => a.body_gear.feet.as_mut(),
        "torso" => a.body_gear.torso.as_mut(),
        _ => None,
    }
    .ok_or("找不到該部位護具")?;
    piece.wear_stage = 0;
    let (craft_prose, _) = invoke_craft_phase_effect(a, "forge");
    a.fatigue = a.fatigue.saturating_add(4).min(95);
    a.last_active_unix = clock.real_unix;
    let msg = format!("你{used_note}（{craft_prose}），【{gname}】已重新回復完好貼合。");
    push_breath(a, msg.clone());
    a.mark_mutated();
    Ok(msg)
}

pub fn try_apply_living_sim_farm_action(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
    action: &str,
    param: &str,
) -> Option<Result<String, String>> {
    match action {
        "take_warm_bath" => Some(apply_take_warm_bath(delta, clock, aid, node)),
        "enjoy_hot_meal" => Some(apply_enjoy_hot_meal(doc, delta, clock, aid, node)),
        "change_outfit" => Some(apply_change_outfit(doc, delta, clock, aid, node, param)),
        "buy_home_furnishing" => Some(apply_buy_home_furnishing(delta, clock, aid, node, param)),
        "sow_farm_plot" => Some(apply_sow_farm_plot(doc, delta, clock, aid, node, param)),
        "tend_farm_plot" => Some(apply_tend_farm_plot(delta, clock, aid, node, param)),
        "harvest_farm_plot" => Some(apply_harvest_farm_plot(doc, delta, clock, aid, node, param)),
        "repair_body_gear" => Some(apply_repair_body_gear(doc, delta, clock, aid, param)),
        _ => None,
    }
}

pub fn build_lifestyle_and_farm_view_json(
    delta: &IslandWorldDelta,
    viewer: &UnifiedMindBody,
    clock: &SolarClockSnapshot,
) -> Value {
    let outfit_name = if viewer.lifestyle.outfit_name_zh.is_empty() {
        "海風舊斗篷與粗麻行裝"
    } else {
        viewer.lifestyle.outfit_name_zh.as_str()
    };
    let furn_names: Vec<&str> = viewer
        .lifestyle
        .home_furnishings
        .iter()
        .map(|s| furnishing_display_name(s))
        .collect();
    let my_plots: Vec<Value> = delta
        .farm_plots
        .values()
        .filter(|p| p.owner_id == viewer.actor_id)
        .map(|p| {
            let fl = delta.live_land.cells.get(&p.cell_id).map_or(0, |cl| cl.flood_level);
            json!({
                "plot_id": p.plot_id,
                "node_name": p.node_name_zh,
                "crop_name": p.crop_name_zh,
                "stage_sense": plot_growth_stage_zh(p, clock.game_days, fl),
                "ready": clock.game_days >= p.mature_day,
            })
        })
        .collect();
    json!({
        "outfit_name": outfit_name,
        "cleanliness_sense": cleanliness_sense_zh(viewer, clock.game_days),
        "meal_sense": hot_meal_sense_zh(viewer, clock.game_days),
        "furnishings": furn_names,
        "my_plots": my_plots,
    })
}

pub fn append_lifestyle_and_farm_scene_objects(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    viewer: &UnifiedMindBody,
    node: &SpatialNodeDef,
    clock: &SolarClockSnapshot,
    objects: &mut Vec<Value>,
    push_audit: &mut impl FnMut(&str, &str, &str, &str, &str, &str),
) {
    let in_owned = viewer.owned_house_node.as_deref() == Some(node.id.as_str());
    let in_paid_inn = node.kind == "inn_room" && viewer.inn_nights_left > 0;
    let tavern_staff = if is_tavern_node(node) {
        find_present_staff(delta, &node.id, &viewer.actor_id, &["酒館掌櫃", "酒館掌櫃學徒"])
    } else {
        None
    };

    let mut care_opts: Vec<Value> = Vec::new();
    if viewer.lifestyle.last_hot_meal_day != clock.game_days {
        if in_owned
            && viewer
                .backpack
                .iter()
                .any(|i| i.count > 0 && HOT_MEAL_SLUGS.contains(&i.slug.as_str()))
        {
            care_opts.push(json!({
                "id": "enjoy_hot_meal",
                "param": "",
                "label": "自家開伙煮碗熱湯飯",
                "sub": "取行囊現有魚糧現煮暖胃",
            }));
            push_audit("enjoy_hot_meal", "", "自家開伙煮碗熱湯飯", "自家熱灶", "object", "obj:living_care");
        } else if tavern_staff.is_some() {
            let price = quote_tavern_ration_copper(doc, delta, node.cell_id, &node.id).max(2);
            let lbl = format!("點份灶房熱湯烤餅 · {price} 枚銅幣");
            care_opts.push(json!({
                "id": "enjoy_hot_meal",
                "param": "",
                "label": lbl,
                "sub": "現用酒館存糧烹煮，暖胃解乏",
            }));
            push_audit("enjoy_hot_meal", "", &lbl, "酒館熱食", "object", "obj:living_care");
        }
    }
    if viewer.lifestyle.last_bath_day != clock.game_days
        && (in_owned || in_paid_inn || tavern_staff.is_some())
    {
        let lbl = if in_owned || in_paid_inn {
            "燒暖松葉溫水沐浴洗塵"
        } else {
            "借後堂溫水浴桶洗塵 · 2 枚銅幣"
        };
        care_opts.push(json!({
            "id": "take_warm_bath",
            "param": "",
            "label": lbl,
            "sub": "洗去一身海鹽塵垢、舒緩肩背倦意",
        }));
        push_audit("take_warm_bath", "", lbl, "溫水沐浴", "object", "obj:living_care");
    }
    if !care_opts.is_empty() {
        objects.push(json!({
            "object_id": "obj:living_care",
            "name": if in_owned { "自家小灶與松木浴桶" } else { "灶房熱鍋與後堂浴間" },
            "cue": "冒著鮮湯熱氣與溫湯水霧",
            "desc": "灶上正溫著大鍋熱水與鮮湯，可吃碗熱食暖胃，或洗去奔波一身風塵。",
            "direct_action": Value::Null,
            "options": care_opts,
        }));
    }

    let crafter = find_present_staff(
        delta,
        &node.id,
        &viewer.actor_id,
        &["木匠", "鍛鐵匠", "行腳商旅", "草藥師", "酒館掌櫃"],
    );
    if in_owned || crafter.is_some() {
        let mut style_opts: Vec<Value> = Vec::new();
        for o in OUTFIT_CATALOG {
            if viewer.lifestyle.outfit_slug == o.slug {
                continue;
            }
            let has_in_bag = viewer.backpack.iter().any(|i| i.slug == o.item_slug && i.count > 0);
            let has_mat = viewer
                .backpack
                .iter()
                .any(|i| i.count > 0 && o.mat_slugs.contains(&i.slug.as_str()));
            if has_in_bag {
                let lbl = format!("換穿行囊裡的【{}】", o.name_zh);
                style_opts.push(json!({
                    "id": "change_outfit",
                    "param": o.slug,
                    "label": lbl,
                    "sub": o.vibe_zh,
                }));
                push_audit("change_outfit", o.slug, &lbl, "換穿實物成衣", "object", "obj:tailor_carpenter");
            } else if has_mat {
                let lbl = if in_owned {
                    format!("以實料自製【{}】", o.name_zh)
                } else {
                    format!("來料裁製【{}】· {} 枚銅幣", o.name_zh, o.tailor_fee)
                };
                style_opts.push(json!({
                    "id": "change_outfit",
                    "param": o.slug,
                    "label": lbl,
                    "sub": format!("耗{}・{}", o.mat_hint_zh, o.vibe_zh),
                }));
                push_audit("change_outfit", o.slug, &lbl, "來料裁衣", "object", "obj:tailor_carpenter");
            }
        }
        let has_home_or_room = viewer.owned_house_node.is_some() || viewer.inn_nights_left > 0;
        if has_home_or_room {
            for f in FURNISHING_CATALOG {
                if viewer.lifestyle.home_furnishings.iter().any(|x| x == f.slug) {
                    continue;
                }
                let has_mat = viewer
                    .backpack
                    .iter()
                    .any(|i| i.count > 0 && f.mat_slugs.contains(&i.slug.as_str()));
                if !has_mat {
                    continue;
                }
                let lbl = if in_owned {
                    format!("以實料自製居所【{}】", f.name_zh)
                } else {
                    format!("來料打造居所【{}】· {} 枚銅幣", f.name_zh, f.fee_copper)
                };
                style_opts.push(json!({
                    "id": "buy_home_furnishing",
                    "param": f.slug,
                    "label": lbl,
                    "sub": format!("耗{}・{}", f.mat_hint_zh, f.effect_zh),
                }));
                push_audit("buy_home_furnishing", f.slug, &lbl, "實料打造居所陳設", "object", "obj:tailor_carpenter");
            }
        }
        let (cue, desc) = if style_opts.is_empty() {
            (
                "可來料裁衣與打造居所陳設（目前行囊缺料，點開查看所需實料）",
                "裁製日常成衣須自備【韌藤纖維／草束／野獸脂塊／異獸鱗甲】其一；打造居所書案、銅燈、烘衣火盆、實木床榻或毛氈地毯須備【原木／漂流木／粗鐵礦／方石料／精煤／韌藤】其一並先有宿房或自家宅院。",
            )
        } else {
            (
                "行囊已備齊部分原材，可請在場匠人或親手裁衣與打造居所陳設",
                "以實有草纖、獸脂、木料或鐵礦親手或委託匠人裁製成衣與屋內陳設。",
            )
        };
        objects.push(json!({
            "object_id": "obj:tailor_carpenter",
            "name": if in_owned { "自家針線木工案" } else { "來料裁衣與木作案台" },
            "cue": cue,
            "desc": desc,
            "direct_action": Value::Null,
            "options": style_opts,
        }));
    }

    append_farm_plot_scene_objects(doc, delta, viewer, node, clock, objects, push_audit);
}
