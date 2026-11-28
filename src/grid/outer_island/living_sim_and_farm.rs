//! 第四刀（上）：文字版模擬人生生活感（熱食暖胃／溫水沐浴／穿搭更換／居所佈置）與農田藥圃開墾播種採收閉環。
//!
//! 嚴守《設計關鍵字》原則：
//! 1. 一律單次決策推進過程，呈現真實前後因果，嚴禁重複連點勞作。
//! 2. 所有銅幣收付皆走 `delta.transfer_copper` 複式守恆，不憑空造錢；農產收成真實落入行囊可供交易或自炊。
//! 3. 前台只輸出白話體感與物候狀態，零原始數字外洩。

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::clock_and_lang::SolarClockSnapshot;
use super::live_land::ensure_live_land_seeded;
use super::mind::EventPublicity;
use super::movement::{add_carry_item, push_breath, take_clean_carry_items};
use super::persona::ValueAxis;
use super::sim_tick::broadcast_scene_event;
use super::types::{CarryItem, IslandWorldDelta, SpatialNodeDef, UnifiedMindBody, WorldStaticDoc};

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
pub struct OutfitSpec {
    pub slug: &'static str,
    pub name_zh: &'static str,
    pub appearance_zh: &'static str,
    pub price_copper: i64,
    pub vibe_zh: &'static str,
}

pub const OUTFIT_CATALOG: &[OutfitSpec] = &[
    OutfitSpec { slug: "cotton_daily", name_zh: "輕便棉麻日常裝", appearance_zh: "身著清爽透氣的米白棉麻襯衣與素色長褲", price_copper: 5, vibe_zh: "日常街坊起居最為舒爽輕快" },
    OutfitSpec { slug: "waxed_hunter", name_zh: "防風蠟布機能獵裝", appearance_zh: "穿著深橄欖色防風蠟布機能短外套與多袋獵褲", price_copper: 12, vibe_zh: "耐濕耐磨、多袋好掛隨身工具與樣本" },
    OutfitSpec { slug: "harbor_wool", name_zh: "港埠細呢體面長衣", appearance_zh: "披著剪裁俐落的深藍細呢長衣，領口整潔挺括", price_copper: 10, vibe_zh: "出入市集總商行與市政廳格外體面" },
    OutfitSpec { slug: "felt_winter", name_zh: "厚毛氈禦寒行裝", appearance_zh: "罩著厚實暖棕毛氈防寒大衣，腰束粗皮扣帶", price_copper: 9, vibe_zh: "海風寒夜與山嶺遠行皆能護住體溫" },
];

#[derive(Debug, Clone, Copy)]
pub struct FurnishingSpec {
    pub slug: &'static str,
    pub name_zh: &'static str,
    pub price_copper: i64,
    pub effect_zh: &'static str,
}

pub const FURNISHING_CATALOG: &[FurnishingSpec] = &[
    FurnishingSpec { slug: "pine_desk", name_zh: "海松木書案與燭臺", price_copper: 12, effect_zh: "屋內多了整潔書案，夜裡方便整理手札與行情筆記" },
    FurnishingSpec { slug: "copper_lamp", name_zh: "壁掛恆光導能銅燈", price_copper: 10, effect_zh: "暖黃燈火柔和映照四壁，入夜歸家格外安心" },
    FurnishingSpec { slug: "warm_brazier", name_zh: "鑄鐵烘衣炭火盆", price_copper: 14, effect_zh: "烘乾海風潮氣，安睡一覺更加解乏暖身" },
    FurnishingSpec { slug: "herb_rack", name_zh: "樟木草藥晾曬架", price_copper: 9, effect_zh: "滿室淡雅草木幽香，可懸掛採回的草藥與山貨" },
    FurnishingSpec { slug: "woven_rug", name_zh: "厚織海草暖腳地毯", price_copper: 8, effect_zh: "踏進屋門腳底柔軟溫厚，少了石板寒意" },
];

#[derive(Debug, Clone, Copy)]
struct CropSpec {
    kind: &'static str,
    name_zh: &'static str,
    yield_slug: &'static str,
    yield_name_zh: &'static str,
    yield_count: u32,
    weight_units: u16,
    grow_days: i64,
}

const CROP_CATALOG: &[CropSpec] = &[
    CropSpec { kind: "dry_wheat", name_zh: "旱田黑麥", yield_slug: "wheat_grain", yield_name_zh: "飽滿黑麥穀袋", yield_count: 2, weight_units: 3, grow_days: 2 },
    CropSpec { kind: "paddy_grain", name_zh: "水田香穀", yield_slug: "wheat_grain", yield_name_zh: "飽滿黑麥穀袋", yield_count: 3, weight_units: 3, grow_days: 2 },
    CropSpec { kind: "herb_garden", name_zh: "山林止血藥圃", yield_slug: "herb_bundle", yield_name_zh: "山林止血草束", yield_count: 2, weight_units: 1, grow_days: 2 },
    CropSpec { kind: "wormwood_bed", name_zh: "清冽苦艾藥圃", yield_slug: "quarantine_wormwood", yield_name_zh: "南岸清冽苦艾束", yield_count: 2, weight_units: 1, grow_days: 2 },
];

fn lookup_outfit(slug: &str) -> Option<OutfitSpec> {
    OUTFIT_CATALOG.iter().copied().find(|o| o.slug == slug)
}

fn lookup_furnishing(slug: &str) -> Option<FurnishingSpec> {
    FURNISHING_CATALOG.iter().copied().find(|f| f.slug == slug)
}

fn lookup_crop(kind: &str) -> Option<CropSpec> {
    CROP_CATALOG.iter().copied().find(|c| c.kind == kind)
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
        "連日風塵奔波・衣角略沾海鹽微汗（可尋浴場或客房洗去風塵）"
    }
}

pub fn hot_meal_sense_zh(viewer: &UnifiedMindBody, game_day: i64) -> &'static str {
    if viewer.lifestyle.last_hot_meal_day == game_day {
        "今日已吃過溫熱熟食・胃裡踏實舒暢"
    } else if viewer.mind.stm.affect.hunger >= 45 {
        "腹中空響・很想喝碗熱湯暖胃"
    } else {
        "尚未用上熱餐・路過酒館食堂或茶棚可點份熱食"
    }
}

pub fn plot_growth_stage_zh(plot: &FarmPlotRecord, game_day: i64, flood_level: u8) -> String {
    if game_day >= plot.mature_day {
        format!(
            "{}已飽滿熟成、隨風輕晃，隨時可下田收割",
            plot.crop_name_zh
        )
    } else if plot.mature_day - game_day <= 1 {
        if flood_level >= 2 && plot.crop_kind == "dry_wheat" {
            format!("{}已抽穗成形，田溝積水略深，宜趁晴趕緊疏水照料", plot.crop_name_zh)
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

fn is_bath_available_here(viewer: &UnifiedMindBody, node: &SpatialNodeDef) -> bool {
    node.kind == "inn_room"
        || viewer.owned_house_node.as_deref() == Some(node.id.as_str())
        || node.name.contains("酒館")
        || node.name.contains("臨河")
        || node.name.contains("溫泉")
        || node.name.contains("浴")
        || node.kind == "market"
}

fn is_hot_meal_available_here(viewer: &UnifiedMindBody, node: &SpatialNodeDef) -> bool {
    viewer.owned_house_node.as_deref() == Some(node.id.as_str())
        || node.name.contains("酒館")
        || node.name.contains("食堂")
        || node.kind == "tea_shed"
        || node.kind == "market"
        || node.kind == "plaza"
}

fn is_outfit_or_furniture_shop_here(viewer: &UnifiedMindBody, node: &SpatialNodeDef) -> bool {
    node.kind == "market"
        || node.kind == "plaza"
        || node.name.contains("市集")
        || node.name.contains("木匠")
        || node.name.contains("百工")
        || viewer.owned_house_node.as_deref() == Some(node.id.as_str())
}

fn find_local_merchant_or_treasury(
    delta: &IslandWorldDelta,
    node_id: &str,
    actor_id: &str,
) -> String {
    if let Some(occ) = delta.node_occupants.get(node_id) {
        for oid in occ {
            if oid == actor_id {
                continue;
            }
            if let Some(a) = delta.actors.get(oid)
                && a.alive
                && a.controller_id.is_none()
                && matches!(
                    a.role.as_str(),
                    "酒館掌櫃" | "酒館掌櫃學徒" | "行腳商旅" | "木匠" | "草藥師" | "鍛鐵匠"
                )
            {
                return oid.clone();
            }
        }
    }
    "municipal_treasury".to_string()
}

pub fn apply_take_warm_bath(
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
) -> Result<String, String> {
    let (in_owned, already_today, copper) = {
        let a = delta.actors.get(aid).ok_or("找不到旅人行蹤")?;
        if !is_bath_available_here(a, node) {
            return Err(
                "此處並無溫水浴桶，請前往酒館、二樓宿房、臨河街坊或自家屋舍沐浴洗塵。".to_string(),
            );
        }
        (
            a.owned_house_node.as_deref() == Some(node.id.as_str()),
            a.lifestyle.last_bath_day == clock.game_days && a.lifestyle.last_bath_day > 0,
            a.copper,
        )
    };
    if already_today {
        return Ok(
            "你今日已泡過一次舒坦的溫水澡，周身仍清爽帶著松香，不必反覆折騰。".to_string(),
        );
    }
    let fee = if in_owned { 0 } else { 2 };
    if fee > 0 {
        if copper < fee {
            return Err("手頭銅幣不足以支付溫水浴湯費（需 2 枚銅幣）。".to_string());
        }
        let payee = find_local_merchant_or_treasury(delta, &node.id, aid);
        if !delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "warm_bath",
            aid,
            &payee,
            fee,
            "溫水浴場沐浴洗塵費".to_string(),
        ) {
            return Err("結算浴湯費時帳目周轉受阻。".to_string());
        }
    }
    let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
    let has_brazier = a
        .lifestyle
        .home_furnishings
        .iter()
        .any(|f| f == "warm_brazier");
    let relief = if in_owned && has_brazier { 32 } else { 24 };
    a.fatigue = a.fatigue.saturating_sub(relief);
    a.mind.stm.affect.anger = a.mind.stm.affect.anger.saturating_sub(15);
    a.mind.stm.affect.fear = a.mind.stm.affect.fear.saturating_sub(12);
    a.lifestyle.last_bath_day = clock.game_days;
    a.mind.stm.today_goal_progressed = true;
    a.mark_mutated();
    let msg = if in_owned {
        "你在自家屋內燒暖一桶松葉溫水，一口氣洗去連日海風塵垢，換上乾爽內衫後肩背鬆快了許多。"
            .to_string()
    } else {
        "你付了 2 枚銅幣進泉香溫水浴間，熱氣氤氳中洗去一身風塵與海鹽黏膩，出來時神清氣爽、步伐輕快了不少。"
            .to_string()
    };
    push_breath(a, msg.clone());
    Ok(msg)
}

pub fn apply_enjoy_hot_meal(
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
) -> Result<String, String> {
    let (in_owned, already_today, copper) = {
        let a = delta.actors.get(aid).ok_or("找不到旅人行蹤")?;
        if !is_hot_meal_available_here(a, node) {
            return Err(
                "此處沒有熱食灶台，可到酒館食堂、市集熟食攤、官道茶棚或自家屋舍吃碗熱湯飯。"
                    .to_string(),
            );
        }
        (
            a.owned_house_node.as_deref() == Some(node.id.as_str()),
            a.lifestyle.last_hot_meal_day == clock.game_days && a.lifestyle.last_hot_meal_day > 0,
            a.copper,
        )
    };
    if already_today {
        return Ok(
            "你今日已吃過一頓飽足暖胃的熱食，此刻腹中踏實，留著肚子待下個時辰再吃吧。".to_string(),
        );
    }

    let cooked_from_bag = if in_owned {
        let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
        take_clean_carry_items(&mut a.backpack, "fresh_fish_basket", 1)
            .or_else(|| take_clean_carry_items(&mut a.backpack, "shellfish_meat", 1))
            .or_else(|| take_clean_carry_items(&mut a.backpack, "wheat_grain", 1))
            .or_else(|| take_clean_carry_items(&mut a.backpack, "rye_bread", 1))
            .map(|it| it.name)
    } else {
        None
    };

    if cooked_from_bag.is_none() {
        let price = if node.kind == "tea_shed" { 2 } else { 3 };
        if copper < price {
            return Err(format!(
                "手頭銅幣不足以點這份熱食（需 {} 枚銅幣，或攜帶食材回自家開伙）。",
                price
            ));
        }
        let payee = find_local_merchant_or_treasury(delta, &node.id, aid);
        if !delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "hot_meal",
            aid,
            &payee,
            price,
            "食堂酒館熱湯熟食費".to_string(),
        ) {
            return Err("結算餐費時帳目周轉受阻。".to_string());
        }
    }

    let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
    a.fatigue = a.fatigue.saturating_sub(26);
    a.mind.stm.affect.hunger = a.mind.stm.affect.hunger.saturating_sub(65);
    a.mind.stm.affect.anger = a.mind.stm.affect.anger.saturating_sub(8);
    a.lifestyle.last_hot_meal_day = clock.game_days;
    a.mind.ltm.mastery.craft_works = a.mind.ltm.mastery.craft_works.saturating_add(1);
    a.mind.stm.today_goal_progressed = true;
    a.mark_mutated();

    let msg = if let Some(ing) = cooked_from_bag {
        format!(
            "你在自家小灶將【{ing}】細火慢煮成一鍋熱騰騰的鮮湯炊飯，趁熱吃得額頭微汗，渾身暖意與體力都回來了。"
        )
    } else {
        format!(
            "你在【{}】要了一大碗滾燙的海魚黑麥濃湯配烤餅，一口口喝下暖透四肢，奔波的倦意與空腹感頓時一掃而空。",
            node.name
        )
    };
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
        return Err("請選擇欲更換的衣物款式（棉麻日常／蠟布獵裝／細呢長衣／毛氈行裝）。".to_string());
    };
    let (copper, already_wearing) = {
        let a = delta.actors.get(aid).ok_or("找不到旅人行蹤")?;
        if !is_outfit_or_furniture_shop_here(a, node) {
            return Err("請到白鷗市集館、市集廣場織造攤或自家屋舍更換衣物穿搭。".to_string());
        }
        (a.copper, a.lifestyle.outfit_slug == spec.slug)
    };
    if already_wearing {
        return Ok(format!("你身上正穿著【{}】，衣料合身俐落。", spec.name_zh));
    }
    if copper < spec.price_copper {
        return Err(format!(
            "訂製【{}】需 {} 枚銅幣，手頭盤纏尚不足。",
            spec.name_zh, spec.price_copper
        ));
    }
    let payee = find_local_merchant_or_treasury(delta, &node.id, aid);
    if !delta.transfer_copper(
        clock.game_days,
        clock.bell_index,
        "buy_outfit",
        aid,
        &payee,
        spec.price_copper,
        format!("織造攤置辦【{}】", spec.name_zh),
    ) {
        return Err("支付衣料費時帳目周轉受阻。".to_string());
    }
    let actor_name = {
        let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
        a.lifestyle.outfit_slug = spec.slug.to_string();
        a.lifestyle.outfit_name_zh = spec.name_zh.to_string();
        a.appearance = spec.appearance_zh.to_string();
        a.mind.stm.today_goal_progressed = true;
        a.mark_mutated();
        let msg = format!(
            "你付了 {} 枚銅幣換上一身【{}】（{}），照過銅鏡頓覺氣色一新。",
            spec.price_copper, spec.name_zh, spec.vibe_zh
        );
        push_breath(a, msg.clone());
        (a.display_name.clone(), msg)
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
        35,
        20,
        ValueAxis::CraftPride,
        "change_outfit",
        &format!("{}換穿了一身嶄新的【{}】", actor_name.0, spec.name_zh),
    );
    Ok(actor_name.1)
}

pub fn apply_buy_home_furnishing(
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
    furn_slug: &str,
) -> Result<String, String> {
    let Some(spec) = lookup_furnishing(furn_slug.trim()) else {
        return Err("請選擇要為居所添購的木作傢俱或燈火陳設。".to_string());
    };
    let (copper, has_home_or_room, already_owned) = {
        let a = delta.actors.get(aid).ok_or("找不到旅人行蹤")?;
        if !is_outfit_or_furniture_shop_here(a, node) && node.kind != "inn_room" {
            return Err("請在市集館、市集廣場木作坊、宿房或自家屋舍添置居所陳設。".to_string());
        }
        (
            a.copper,
            a.owned_house_node.is_some() || a.inn_nights_left > 0 || node.kind == "inn_room",
            a.lifestyle
                .home_furnishings
                .iter()
                .any(|f| f == spec.slug),
        )
    };
    if !has_home_or_room {
        return Err("你目前尚未投宿客房或購置自有宅院，還沒有落腳屋舍可擺放傢俱。".to_string());
    }
    if already_owned {
        return Ok(format!(
            "你的居所早已擺妥【{}】，不必重複添置。",
            spec.name_zh
        ));
    }
    if copper < spec.price_copper {
        return Err(format!(
            "添置【{}】需 {} 枚銅幣，手頭盤纏尚不足。",
            spec.name_zh, spec.price_copper
        ));
    }
    let payee = find_local_merchant_or_treasury(delta, &node.id, aid);
    if !delta.transfer_copper(
        clock.game_days,
        clock.bell_index,
        "buy_furnishing",
        aid,
        &payee,
        spec.price_copper,
        format!("居所添置【{}】", spec.name_zh),
    ) {
        return Err("支付木作工料費時帳目周轉受阻。".to_string());
    }
    let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
    a.lifestyle.home_furnishings.push(spec.slug.to_string());
    a.mind.stm.today_goal_progressed = true;
    a.mark_mutated();
    let msg = format!(
        "你付了 {} 枚銅幣請木匠送來並擺妥【{}】——{}。",
        spec.price_copper, spec.name_zh, spec.effect_zh
    );
    push_breath(a, msg.clone());
    Ok(msg)
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
        return Err("此處疫氣未散，僅能先種植淨瘴苦艾圃或焚燻苦艾淨地。".to_string());
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
        return Err("你在此處已有一片正在生長的田壟，請先照料或待熟成收割後再重種。".to_string());
    }
    let (owner_name, tool_bonus) = {
        let a = delta.actors.get(aid).ok_or("找不到旅人行蹤")?;
        if a.fatigue >= 86 {
            return Err("腰背酸乏，先吃份熱食或歇息一會再下田起壟播種。".to_string());
        }
        let has_farm_tool = a.bound_weapon.slug.contains("hoe")
            || a.bound_weapon.slug.contains("shovel")
            || a.bound_weapon.slug.contains("axe")
            || a.bound_weapon.name.contains('鋤')
            || a.bound_weapon.name.contains('鎬')
            || a.bound_weapon.name.contains('斧');
        (a.display_name.clone(), has_farm_tool)
    };
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
    let plot = FarmPlotRecord {
        plot_id: plot_id.clone(),
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
    };
    delta.farm_plots.insert(plot_id, plot);
    if let Some(cl) = delta.live_land.cells.get_mut(&node.cell_id) {
        cl.reclaimed_plots = cl.reclaimed_plots.saturating_add(1).min(12);
        cl.fallow_days = 0;
    }
    delta.extras_dirty = true;

    let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
    let fat_cost = if tool_bonus { 8 } else { 12 };
    a.fatigue = a.fatigue.saturating_add(fat_cost).min(95);
    a.mind.ltm.mastery.craft_works = a.mind.ltm.mastery.craft_works.saturating_add(1);
    a.mind.stm.today_goal_progressed = true;
    a.mark_mutated();
    let msg = format!(
        "你在【{}】一氣呵成清草起壟、翻鬆泥土並播下【{}】種子；接下來隨地氣與日照自然抽芽，平日順手照料一次即可提早熟成採收。",
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
        &format!("{}在【{}】新墾起一片【{}】田壟", owner_name, node.name, spec.name_zh),
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
        return Ok(format!(
            "這片【{}】已飽滿熟成，直接下田收割即可！",
            plot.crop_name_zh
        ));
    }
    if plot.tended_day == clock.game_days {
        return Ok(format!(
            "今日你已替【{}】澆透水並清過雜草，靜候地氣日照催熟即可。",
            plot.crop_name_zh
        ));
    }
    plot.tended_day = clock.game_days;
    plot.mature_day = (plot.mature_day - 1).max(clock.game_days);
    let crop_name = plot.crop_name_zh.clone();
    let ready_now = clock.game_days >= plot.mature_day;
    delta.extras_dirty = true;

    let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
    a.fatigue = a.fatigue.saturating_add(5).min(95);
    a.mind.ltm.mastery.craft_works = a.mind.ltm.mastery.craft_works.saturating_add(1);
    a.mind.stm.today_goal_progressed = true;
    a.mark_mutated();
    let msg = if ready_now {
        format!(
            "你順著田溝引水清草，【{crop_name}】吸飽水氣後葉莖挺立飽滿，此刻已可當場收割入囊！"
        )
    } else {
        format!(
            "你順著田溝引水除草並培穩根土，【{crop_name}】長勢明顯加快，再過不久便能迎來收成。"
        )
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
        return Err(format!(
            "這片【{}】尚未飽滿熟成，可先順手照料引水催熟或明日再來看。",
            plot.crop_name_zh
        ));
    }
    let Some(spec) = lookup_crop(&plot.crop_kind) else {
        return Err("作物資料異常，暫時無法收割。".to_string());
    };
    delta.farm_plots.remove(&target_key);
    delta.extras_dirty = true;

    let (owner_name, msg) = {
        let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
        a.fatigue = a.fatigue.saturating_add(7).min(95);
        a.mind.ltm.mastery.craft_works = a.mind.ltm.mastery.craft_works.saturating_add(2);
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
            "你一口氣收割完這片熟透的【{}】，捆妥 {} 份【{}】收入行囊（可帶回城鎮商鋪出售、交給藥堂或留作自家炊煮），熟土空壟隨時可再續種下一茬。",
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
        &format!(
            "{}在【{}】歡喜收成了一茬【{}】",
            owner_name, node.name, plot.crop_name_zh
        ),
    );
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
        "enjoy_hot_meal" => Some(apply_enjoy_hot_meal(delta, clock, aid, node)),
        "change_outfit" => Some(apply_change_outfit(doc, delta, clock, aid, node, param)),
        "buy_home_furnishing" => Some(apply_buy_home_furnishing(delta, clock, aid, node, param)),
        "sow_farm_plot" => Some(apply_sow_farm_plot(doc, delta, clock, aid, node, param)),
        "tend_farm_plot" => Some(apply_tend_farm_plot(delta, clock, aid, node, param)),
        "harvest_farm_plot" => Some(apply_harvest_farm_plot(doc, delta, clock, aid, node, param)),
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
            let fl = delta
                .live_land
                .cells
                .get(&p.cell_id)
                .map_or(0, |cl| cl.flood_level);
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
    // 1. 模擬人生起居插座（熱食暖胃 × 溫水沐浴）
    let mut care_opts: Vec<Value> = Vec::new();
    if is_hot_meal_available_here(viewer, node)
        && viewer.lifestyle.last_hot_meal_day != clock.game_days
    {
        let in_owned = viewer.owned_house_node.as_deref() == Some(node.id.as_str());
        let lbl = if in_owned {
            "自家開伙煮碗熱湯飯"
        } else if node.kind == "tea_shed" {
            "點份熱湯烤餅暖胃 · 2 枚銅幣"
        } else {
            "點份海魚黑麥熱湯飯 · 3 枚銅幣"
        };
        care_opts.push(json!({
            "id": "enjoy_hot_meal",
            "param": "",
            "label": lbl,
            "sub": "單次決策解飢回氣、舒緩奔波倦意",
        }));
        push_audit(
            "enjoy_hot_meal",
            "",
            lbl,
            "熱食暖胃",
            "object",
            "obj:living_care",
        );
    }
    if is_bath_available_here(viewer, node) && viewer.lifestyle.last_bath_day != clock.game_days {
        let in_owned = viewer.owned_house_node.as_deref() == Some(node.id.as_str());
        let lbl = if in_owned {
            "自家燒水泡個松葉溫水澡"
        } else {
            "泉香溫水浴間洗去風塵 · 2 枚銅幣"
        };
        care_opts.push(json!({
            "id": "take_warm_bath",
            "param": "",
            "label": lbl,
            "sub": "洗淨海鹽汗垢、大解疲乏並平復心氣",
        }));
        push_audit(
            "take_warm_bath",
            "",
            lbl,
            "溫水沐浴",
            "object",
            "obj:living_care",
        );
    }
    if !care_opts.is_empty() {
        objects.push(json!({
            "object_id": "obj:living_care",
            "name": "食堂熱灶與泉香浴間",
            "cue": "冒著鮮湯熱氣與松木溫湯水霧，可暖胃洗塵",
            "desc": "日常起居最實在的享受——喝碗熱湯暖胃，或泡進溫水浴桶洗去一身風塵海鹽，單次安排即可舒坦回氣。",
            "direct_action": Value::Null,
            "options": care_opts,
        }));
    }

    // 2. 織造穿搭與木作居所佈置插座
    if is_outfit_or_furniture_shop_here(viewer, node) {
        let mut style_opts: Vec<Value> = Vec::new();
        for o in OUTFIT_CATALOG {
            if viewer.lifestyle.outfit_slug == o.slug {
                continue;
            }
            let lbl = format!("換穿【{}】· {} 枚銅幣", o.name_zh, o.price_copper);
            style_opts.push(json!({
                "id": "change_outfit",
                "param": o.slug,
                "label": lbl,
                "sub": o.vibe_zh,
            }));
            push_audit(
                "change_outfit",
                o.slug,
                &lbl,
                "更換日常穿搭",
                "object",
                "obj:tailor_carpenter",
            );
        }
        let has_home_or_room =
            viewer.owned_house_node.is_some() || viewer.inn_nights_left > 0;
        if has_home_or_room {
            for f in FURNISHING_CATALOG {
                if viewer.lifestyle.home_furnishings.iter().any(|x| x == f.slug) {
                    continue;
                }
                let lbl = format!("添置居所【{}】· {} 枚銅幣", f.name_zh, f.price_copper);
                style_opts.push(json!({
                    "id": "buy_home_furnishing",
                    "param": f.slug,
                    "label": lbl,
                    "sub": f.effect_zh,
                }));
                push_audit(
                    "buy_home_furnishing",
                    f.slug,
                    &lbl,
                    "佈置居所陳設",
                    "object",
                    "obj:tailor_carpenter",
                );
            }
        }
        if !style_opts.is_empty() {
            objects.push(json!({
                "object_id": "obj:tailor_carpenter",
                "name": "織造服飾攤與木作傢俱坊",
                "cue": "掛滿棉麻蠟布成衣，一側陳列海松書案與恆光銅燈",
                "desc": "可依天候與出門需要更換合身穿搭，若已有宿房或自有宅院，還能訂購書案、火盆與銅燈佈置居所。",
                "direct_action": Value::Null,
                "options": style_opts,
            }));
        }
    }

    // 3. 農田／藥圃開墾播種與照料收成插座
    let my_plot_here = delta
        .farm_plots
        .values()
        .find(|p| p.node_id == node.id && p.owner_id == viewer.actor_id);
    if let Some(plot) = my_plot_here {
        let fl = delta
            .live_land
            .cells
            .get(&plot.cell_id)
            .map_or(0, |cl| cl.flood_level);
        let stage_zh = plot_growth_stage_zh(plot, clock.game_days, fl);
        let ready = clock.game_days >= plot.mature_day;
        let opts = if ready {
            push_audit(
                "harvest_farm_plot",
                &plot.plot_id,
                "下田收割熟成作物",
                "收入行囊可供販售或自炊",
                "object",
                "obj:farm_plot",
            );
            vec![json!({
                "id": "harvest_farm_plot",
                "param": plot.plot_id,
                "label": format!("收割【{}】入囊", plot.crop_name_zh),
                "sub": "單次整壟收成，熟土隨時可再播下一茬",
            })]
        } else {
            push_audit(
                "tend_farm_plot",
                &plot.plot_id,
                "澆水除草照料田壟",
                "順手照料提早熟成",
                "object",
                "obj:farm_plot",
            );
            vec![json!({
                "id": "tend_farm_plot",
                "param": plot.plot_id,
                "label": format!("引水除草照料【{}】", plot.crop_name_zh),
                "sub": "每日照料一次可加速抽穗熟成",
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
            let lbl = format!("開墾起壟播種【{}】", c.name_zh);
            sow_opts.push(json!({
                "id": "sow_farm_plot",
                "param": c.kind,
                "label": lbl,
                "sub": format!("熟成收成【{}】", c.yield_name_zh),
            }));
            push_audit(
                "sow_farm_plot",
                c.kind,
                &lbl,
                "開墾農田藥圃",
                "object",
                "obj:arable_slope",
            );
        }
        objects.push(json!({
            "object_id": "obj:arable_slope",
            "name": "可開墾沃土坡地",
            "cue": "地氣充沛、土層鬆軟，適合翻墾成農田或藥圃",
            "desc": "這片平緩坡地受海島充沛地氣滋養，單次下令即可完成清草起壟與播種，靜候熟成採收。",
            "direct_action": Value::Null,
            "options": sow_opts,
        }));
    }
}
