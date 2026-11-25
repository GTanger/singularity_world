//! 銀鷗島 93 地塊有限資源存量再生、潮汐採集、南岸封村遺址（0 刷怪）與三大太古巨物唯一討伐引擎。

use super::clock_and_lang::SolarClockSnapshot;
use super::live_tide::derive_tide_clock_details;
use super::movement::{add_carry_item, push_breath};
use super::types::{CarryItem, IslandWorldDelta, SkirmishState, SpatialNodeDef, WorldStaticDoc};

#[derive(Debug, Clone, Copy)]
pub struct ResourceSpec {
    pub slug: &'static str,
    pub name_zh: &'static str,
    pub weight_units: u16,
    pub sell_copper: i64,
    pub is_bulk_commodity: bool,
}

pub const RESOURCE_CATALOG: &[ResourceSpec] = &[
    ResourceSpec {
        slug: "black_coal",
        name_zh: "北林精煤塊",
        weight_units: 4,
        sell_copper: 4,
        is_bulk_commodity: true,
    },
    ResourceSpec {
        slug: "iron_ore",
        name_zh: "粗鐵礦石",
        weight_units: 4,
        sell_copper: 4,
        is_bulk_commodity: true,
    },
    ResourceSpec {
        slug: "timber_log",
        name_zh: "灰松厚原木",
        weight_units: 4,
        sell_copper: 4,
        is_bulk_commodity: true,
    },
    ResourceSpec {
        slug: "charcoal_sack",
        name_zh: "耐燒硬木炭袋",
        weight_units: 3,
        sell_copper: 4,
        is_bulk_commodity: true,
    },
    ResourceSpec {
        slug: "quarry_stone",
        name_zh: "青灰方石料",
        weight_units: 5,
        sell_copper: 4,
        is_bulk_commodity: true,
    },
    ResourceSpec {
        slug: "salt_fish_barrel",
        name_zh: "鹽漬海魚木桶",
        weight_units: 4,
        sell_copper: 5,
        is_bulk_commodity: true,
    },
    ResourceSpec {
        slug: "wheat_grain",
        name_zh: "飽滿黑麥穀袋",
        weight_units: 3,
        sell_copper: 3,
        is_bulk_commodity: true,
    },
    ResourceSpec {
        slug: "dairy_cheese",
        name_zh: "熟陳山羊乳酪輪",
        weight_units: 3,
        sell_copper: 5,
        is_bulk_commodity: true,
    },
    ResourceSpec {
        slug: "quartz_sand",
        name_zh: "晶瑩石英白沙袋",
        weight_units: 3,
        sell_copper: 6,
        is_bulk_commodity: true,
    },
    ResourceSpec {
        slug: "fresh_fish_basket",
        name_zh: "鮮活溪海魚簍",
        weight_units: 2,
        sell_copper: 3,
        is_bulk_commodity: false,
    },
    ResourceSpec {
        slug: "shellfish_meat",
        name_zh: "退潮肥美海貝",
        weight_units: 1,
        sell_copper: 3,
        is_bulk_commodity: false,
    },
    ResourceSpec {
        slug: "herb_bundle",
        name_zh: "山林止血草束",
        weight_units: 1,
        sell_copper: 3,
        is_bulk_commodity: false,
    },
    ResourceSpec {
        slug: "quarantine_wormwood",
        name_zh: "南岸清冽苦艾束",
        weight_units: 1,
        sell_copper: 4,
        is_bulk_commodity: false,
    },
    ResourceSpec {
        slug: "beast_tallow",
        name_zh: "精煉野獸脂塊",
        weight_units: 2,
        sell_copper: 5,
        is_bulk_commodity: false,
    },
    ResourceSpec {
        slug: "driftwood",
        name_zh: "異國刻紋漂流木",
        weight_units: 2,
        sell_copper: 3,
        is_bulk_commodity: false,
    },
    ResourceSpec {
        slug: "dawn_dew_herb",
        name_zh: "破曉凝露靈草",
        weight_units: 1,
        sell_copper: 6,
        is_bulk_commodity: false,
    },
    ResourceSpec {
        slug: "dock_cargo_crate",
        name_zh: "封蠟靠岸商貨木箱",
        weight_units: 6,
        sell_copper: 0,
        is_bulk_commodity: true,
    },
];

pub fn lookup_resource_spec(slug: &str) -> Option<ResourceSpec> {
    RESOURCE_CATALOG.iter().copied().find(|r| r.slug == slug)
}

/// 判斷物品是否屬於「大宗商貨」（禁止單人螞蟻搬家跨地塊長途步行，必須走站區 1~2 步短駁與車船託運）
pub fn is_bulk_commodity_slug(slug: &str) -> bool {
    lookup_resource_spec(slug).is_some_and(|r| r.is_bulk_commodity)
}

/// 確保全島 93 地塊之可再生資源池完成初次建帳，並隨遊戲日自動再生（上限 `stock_cap`）
pub fn ensure_and_regen_cell_resources(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    game_day: i64,
) {
    if delta.cell_stocks.is_empty() {
        for c in doc.cells.values() {
            if c.stock_cap > 0 {
                delta.cell_stocks.insert(c.cell_id, c.stock_cap);
            }
        }
        delta.cell_last_regen_day = game_day;
        delta.extras_dirty = true;
        return;
    }
    if game_day <= delta.cell_last_regen_day {
        return;
    }
    let elapsed = u16::try_from((game_day - delta.cell_last_regen_day).clamp(1, 30)).unwrap_or(1);
    for c in doc.cells.values() {
        if c.stock_cap > 0 && c.regen_per_day > 0 {
            let cur = delta.cell_stocks.get(&c.cell_id).copied().unwrap_or(0);
            let next = cur
                .saturating_add(c.regen_per_day.saturating_mul(elapsed))
                .min(c.stock_cap);
            delta.cell_stocks.insert(c.cell_id, next);
        }
    }
    delta.cell_last_regen_day = game_day;
    delta.extras_dirty = true;
}

pub fn is_low_tide_window(clock: &SolarClockSnapshot) -> bool {
    let td = derive_tide_clock_details(clock);
    // 每日早退潮（候 10..=21）與晚退潮（候 34..=45）開放下礁採貝
    (10..=21).contains(&td.tide_slot) || (34..=45).contains(&td.tide_slot)
}

pub fn is_apex_beast_slug(foe_slug: &str) -> bool {
    foe_slug.starts_with("apex:") || foe_slug.starts_with("apex_")
}

/// 太古巨物永久討伐登載（全服唯一一次，死後永不重生，並入傳活史官！）
pub fn record_apex_beast_slain(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    slayer_id: &str,
    node_id: &str,
    foe_name: &str,
) -> Option<String> {
    let cell_id = doc.nodes.get(node_id).map_or(0, |n| n.cell_id);
    if cell_id == 0 || delta.slain_apex_beasts.contains_key(&cell_id) {
        return None;
    }
    let (slayer_name, is_player) = delta.actors.get(slayer_id).map_or_else(
        || ("外來旅人".to_string(), true),
        |a| {
            (
                a.display_name.clone(),
                a.controller_id.is_some() || slayer_id.starts_with("pl:"),
            )
        },
    );
    let public_slayer = if is_player {
        "外來旅人".to_string()
    } else {
        slayer_name.clone()
    };
    delta
        .slain_apex_beasts
        .insert(cell_id, (public_slayer.clone(), clock.game_days));
    delta.extras_dirty = true;

    let (rune_def, bone_def) = match cell_id {
        4070 => ("鎮潮古符文原石", "巨鰲千年背甲骨"),
        4326 => ("生機古符文原石", "石角古鹿冠骨"),
        5079 => ("裂岩古符文原石", "黑岩巨蟒逆鱗骨"),
        _ => ("古符文原石", "太古荒骨"),
    };
    let rune_name = rune_def.to_string();
    let bone_name = if cell_id == 4070 || cell_id == 4326 || cell_id == 5079 {
        bone_def.to_string()
    } else {
        format!("{}的{}", foe_name, bone_def)
    };

    let grant_true_rune = if delta.chaos_rune_pool > 0 {
        delta.chaos_rune_pool -= 1;
        true
    } else {
        let active = u16::from(super::phase_and_vessels::count_world_active_runes(delta));
        active < 12
    };
    let (r_slug, r_label) = if grant_true_rune {
        ("rune_stone", rune_name)
    } else {
        ("apex_bone_relic", format!("{}結晶殘片", rune_name))
    };

    if let Some(a) = delta.actors.get_mut(slayer_id) {
        a.active_skirmish = None;
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: r_slug.to_string(),
                name: r_label.clone(),
                count: 1,
                weight_units: 2,
                ..CarryItem::default()
            },
        );
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: "apex_bone_relic".to_string(),
                name: bone_name.clone(),
                count: 1,
                weight_units: 2,
                ..CarryItem::default()
            },
        );
        a.known_rumors.insert(format!("slain_apex:{cell_id}"));
        a.mark_mutated();
    }

    // 寫入背景活史官（接入 `Q4-ColdFusion-27B` 5D 事實卡三段式本末與角色列傳，嚴守 M4 玩家隱私遮罩）
    let raw_place = doc
        .nodes
        .get(node_id)
        .map_or("山海險境", |n| n.name.as_str());
    let base_place = raw_place.split('（').next().unwrap_or(raw_place).trim();
    let mut fact = super::living_chronicler::build_fact_packet(
        doc,
        delta,
        clock,
        format!("deed:apex_slain:{cell_id}"),
        "slay_apex_beast",
        node_id,
        slayer_id,
        "",
    );
    fact.place_zh = super::living_chronicler::clean_place_zh(base_place);
    let yr_zh = super::causal_arcs::solar_year_zh(fact.solar_year);
    fact.allowed_numbers_zh = vec![yr_zh.clone()];
    let atmos = super::living_chronicler::atmosphere_clause(
        !fact.tide_zh.is_empty(),
        &fact.weather_zh,
        &fact.tide_zh,
        &fact.bell_zh,
    );

    if !super::living_chronicler::is_player_actor(delta, slayer_id) {
        let mut afact = fact.clone();
        afact.bullet_facts_zh = vec![
            format!(
                "本傳主角：{}（{}）",
                fact.agent_public_label, fact.agent_role_zh
            ),
            format!(
                "討伐地點與太古巨物：{}，盤踞百年的{}",
                fact.place_zh, foe_name
            ),
            format!(
                "戰果：歷經苦戰擊倒{}，自背脊撬下{}與{}，平息當地百年荒氣威壓",
                foe_name, r_label, bone_name
            ),
        ];
        let actor_prose = format!(
            "太陽曆{yr_zh}年{}，{atmos}。{}深入{}迎戰盤踞此地百年的太古巨物{}，一番死鬥後終將其轟然擊倒，撬回珍貴的{}，自此平息了這片險境多年的荒氣威壓。",
            fact.season_zh, fact.agent_public_label, fact.place_zh, foe_name, bone_name
        );
        delta
            .living_chronicle
            .push_new_projection(super::living_chronicler::make_row(
                &afact,
                "actor",
                slayer_id,
                "",
                "",
                "",
                "",
                actor_prose,
            ));
        if let Some(a) = delta.actors.get_mut(slayer_id) {
            super::living_chronicler::push_bounded_actor_summary(
                a,
                format!(
                    "太陽曆{yr_zh}年{}，在{}誅除太古巨物{}",
                    fact.season_zh, fact.place_zh, foe_name
                ),
            );
        }
    }

    fact.bullet_facts_zh = vec![
        format!("前因：{}一帶長年受高濃度荒氣浸染，盤踞著體型龐大、甲骨堅硬的太古巨物{}，過往採集與行獵的島民皆視為畏途", fact.place_zh, foe_name),
        format!("當代破局：太陽曆{yr_zh}年{}，{public_slayer}踏入{}與{}正面交鋒，歷經苦戰將其徹底擊潰，取回{}與{}，這頭太古巨物自此永絕於世", fact.season_zh, fact.place_zh, foe_name, r_label, bone_name),
        "迴響：消息傳開後，鄰近村莊與五鎮酒館紛紛傳頌這場惡戰，原本受巨物威壓封鎖的山海資源地界終於重歸安寧".to_string(),
    ];
    let annals_prose = format!(
        "{}一帶長年受荒氣浸染，盤踞著太古巨物{}，百年來附近獵戶與採集人無不繞道而行。\n\n直到太陽曆{yr_zh}年{}，{atmos}。{public_slayer}深入{}與{}正面死鬥，歷經苦戰終將這頭百年巨物轟然擊倒，並取回珍稀的{}，盤踞此處的太古威壓自此徹底平息。\n\n消息隨過路商旅傳回各鎮酒館與市集廣場，附近村莊的獵戶與匠人再也不必提心吊膽，這樁壯舉也被鄭重補記進島上史冊。",
        fact.place_zh, foe_name, fact.season_zh, fact.place_zh, foe_name, bone_name
    );
    let latch_key = format!("latch:apex_slain:{cell_id}");
    let target_ann_id = format!("annals_apex_{cell_id}");
    let ann_title = format!("誅除{}本末", foe_name);
    let ann_sub = format!("{}・平息山海百年荒氣威壓始末", fact.place_zh);
    delta
        .living_chronicle
        .push_new_projection(super::living_chronicler::make_row(
            &fact,
            "annals",
            &target_ann_id,
            &latch_key,
            &ann_title,
            &ann_sub,
            "各鎮酒館傳抄與港務石碑補記",
            annals_prose,
        ));

    Some(format!(
        "盤踞此地百年的太古巨物【{}】轟然倒地，再也不會重現人間！你從牠背脊裂縫間撬下一塊溫潤的【{}】與【{}】。",
        foe_name, r_label, bone_name
    ))
}

/// 依據真實地塊生態、潮汐、南岸封村遺址（0 刷怪）與有限資源池結算搜巡／採集
pub fn apply_cell_ecology_explore(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
    win_key: String,
) -> Result<String, String> {
    ensure_and_regen_cell_resources(doc, delta, clock.game_days);
    let cell_opt = doc.cells.get(&node.cell_id.to_string());
    let cell_kind = cell_opt.map_or("wilderness", |c| c.kind.as_str());
    let res_slug = cell_opt.map_or("", |c| c.resource_slug.as_str());
    let plague_sealed = cell_opt.is_some_and(|c| c.plague_sealed);

    // 1. 南岸船疫 5 處封村遺址：鐵律 0 刷怪！可尋獲三二六年未寄家書與野生清冽苦艾
    if node.kind == "plague_ruin" || cell_kind == "plague_ruin" {
        let letter_key = format!("plague_letter_found:{}", node.cell_id);
        let already_found_letter = delta
            .actors
            .get(aid)
            .is_some_and(|a| a.known_rumors.contains(&letter_key));
        if node.id.ends_with(":ruin_hall") && !already_found_letter {
            let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
            super::live_tide::prune_stale_day_window_keys(&mut a.explored_windows, clock.game_days);
            a.explored_windows.insert(win_key);
            a.known_rumors.insert(letter_key);
            a.last_active_unix = clock.real_unix;
            add_carry_item(
                &mut a.backpack,
                CarryItem {
                    slug: "plague_family_letter".to_string(),
                    name: "三二六年封村未寄家書".to_string(),
                    count: 1,
                    weight_units: 1,
                    ..CarryItem::default()
                },
            );
            a.mark_mutated();
            let msg = format!(
                "你在【{}】供桌石磚下尋獲一封油紙裹緊的【三二六年封村未寄家書】，信封上寫著白鷗港與鷗尾漁港親族的名姓，空寂舊堂外唯有海風吹拂苦艾。",
                node.name
            );
            push_breath(a, msg.clone());
            return Ok(msg);
        }

        let cur_stock = delta.cell_stocks.get(&node.cell_id).copied().unwrap_or(0);
        let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
        super::live_tide::prune_stale_day_window_keys(&mut a.explored_windows, clock.game_days);
        a.explored_windows.insert(win_key);
        a.last_active_unix = clock.real_unix;
        if cur_stock > 0 {
            delta.cell_stocks.insert(node.cell_id, cur_stock - 1);
            delta.extras_dirty = true;
            add_carry_item(
                &mut a.backpack,
                CarryItem {
                    slug: "quarantine_wormwood".to_string(),
                    name: "南岸清冽苦艾束".to_string(),
                    count: 1,
                    weight_units: 1,
                    ..CarryItem::default()
                },
            );
            a.mark_mutated();
            let msg = format!(
                "你在寂靜無人的【{}】釘封石牆根採下一束藥氣清冽的【南岸清冽苦艾束】（此地存量餘 {}）。",
                node.name,
                cur_stock - 1
            );
            push_breath(a, msg.clone());
            return Ok(msg);
        }
        a.mark_mutated();
        let msg = format!(
            "【{}】空巷寂然無聲，牆根熟成的野生苦艾今日已被採盡，待明日晨露再生。",
            node.name
        );
        push_breath(a, msg.clone());
        return Ok(msg);
    }

    // 2. 三大太古巨物棲地（若尚未被全服討伐，踏入搜巡即驚動唯一巨物！）
    if let Some(ab) = cell_opt.and_then(|c| c.apex_beast.as_ref()) {
        if let Some((slayer, sday)) = delta.slain_apex_beasts.get(&node.cell_id).cloned() {
            let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
            a.explored_windows.insert(win_key.clone());
            let echo = format!(
                "岩壁下仍留著【{}】昔日（第 {} 日由{}討伐）伏亡後的風化巨骨痕跡。",
                ab.name_zh, sday, slayer
            );
            push_breath(a, echo);
        } else {
            let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
            super::live_tide::prune_stale_day_window_keys(&mut a.explored_windows, clock.game_days);
            a.explored_windows.insert(win_key);
            a.last_active_unix = clock.real_unix;
            a.active_skirmish = Some(SkirmishState {
                foe_slug: ab.slug.clone(),
                foe_name: ab.name_zh.clone(),
                can_speak_goblin: false,
                foe_stagger: 0,
                player_guard: false,
                round_idx: 1,
            });
            a.mark_mutated();
            let msg = format!(
                "地脈碎石劇烈震顫——{}【{}】自深谷岩影中昂首現身，沉重威壓封住了退路！",
                ab.title_zh, ab.name_zh
            );
            push_breath(a, msg.clone());
            return Ok(msg);
        }
    }

    // 3. 城鎮街巷室內等非產地節點：純觀察環境
    let harvestable_node = matches!(
        node.kind.as_str(),
        "wild" | "road" | "village_gate" | "plaza"
    ) && !res_slug.is_empty()
        && cell_kind != "settlement";
    if !harvestable_node {
        let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
        super::live_tide::prune_stale_day_window_keys(&mut a.explored_windows, clock.game_days);
        a.explored_windows.insert(win_key);
        a.last_active_unix = clock.real_unix;
        a.mark_mutated();
        let msg = format!(
            "你留心觀察【{}】四周往來行色：門廊木牌字跡清晰，地面石板被車馬磨得光滑。",
            node.name
        );
        push_breath(a, msg.clone());
        return Ok(msg);
    }

    // 4. 非疫區深山野林偶遇灰耳哥布林斥候（道路走廊與鄉鎮村內絕不亂刷怪！）
    let roll = ((u64::from(node.cell_id) * 31)
        .wrapping_add(clock.game_days.unsigned_abs() * 17)
        .wrapping_add(u64::from(clock.hour)))
        % 5;
    if node.kind == "wild"
        && cell_kind == "wilderness"
        && !plague_sealed
        && cell_opt.is_some_and(|c| !c.is_coast)
        && roll == 0
    {
        let boss_prefix = delta
            .dropped_loot_by_node
            .get(&node.id)
            .and_then(|v| v.last())
            .and_then(|b| b.promoted_boss_name.clone());
        let foe_name = boss_prefix.unwrap_or_else(|| "灰耳哥布林斥候".to_string());
        let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
        super::live_tide::prune_stale_day_window_keys(&mut a.explored_windows, clock.game_days);
        a.explored_windows.insert(win_key);
        a.last_active_unix = clock.real_unix;
        let can_gob = a.languages.get("goblin").copied().unwrap_or(0) >= 25;
        a.active_skirmish = Some(SkirmishState {
            foe_slug: "grey_ear_goblin".to_string(),
            foe_name: foe_name.clone(),
            can_speak_goblin: can_gob,
            foe_stagger: 0,
            player_guard: false,
            round_idx: 1,
        });
        a.mark_mutated();
        let msg = format!(
            "灌木叢驟然分開，一名手持骨柄短刀的【{}】伏低身子竄出，緊盯著你腰間行囊！",
            foe_name
        );
        push_breath(a, msg.clone());
        return Ok(msg);
    }

    // 5. 退潮限定海貝檢定：漲潮時礁盤沒入白浪，改拾漂流木或回報漲潮
    let effective_slug = if res_slug == "shellfish_meat" && !is_low_tide_window(clock) {
        "driftwood"
    } else {
        res_slug
    };
    let Some(spec) = lookup_resource_spec(effective_slug) else {
        let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
        a.explored_windows.insert(win_key);
        a.mark_mutated();
        return Ok(format!("你沿著【{}】巡看一圈，四周風物安穩。", node.name));
    };

    let cur_stock = delta.cell_stocks.get(&node.cell_id).copied().unwrap_or(0);
    let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
    super::live_tide::prune_stale_day_window_keys(&mut a.explored_windows, clock.game_days);
    a.explored_windows.insert(win_key);
    a.last_active_unix = clock.real_unix;
    if cur_stock == 0 {
        a.mark_mutated();
        let msg = format!(
            "【{}】一帶今日可採收的【{}】已被鄉民與行旅採集見底，須待明日地氣潮水回養。",
            node.name, spec.name_zh
        );
        push_breath(a, msg.clone());
        return Ok(msg);
    }

    if a.controller_id.is_none()
        && spec.is_bulk_commodity
        && super::transit_and_cargo::count_backpack_bulk_units(&a.backpack) >= 2
    {
        a.mark_mutated();
        return Ok(format!(
            "【{}】行囊樣品已滿兩件，收手返程。",
            a.display_name
        ));
    }
    delta.cell_stocks.insert(node.cell_id, cur_stock - 1);
    delta.extras_dirty = true;
    add_carry_item(
        &mut a.backpack,
        CarryItem {
            slug: spec.slug.to_string(),
            name: spec.name_zh.to_string(),
            count: 1,
            weight_units: spec.weight_units,
            ..CarryItem::default()
        },
    );
    a.mark_mutated();
    let tide_note = if res_slug == "shellfish_meat" && effective_slug == "driftwood" {
        "此刻正逢漲潮，暗礁海貝沒入浪底，你在高潮線碎石間改拾獲一截【異國刻紋漂流木】"
    } else {
        ""
    };
    let msg = if !tide_note.is_empty() {
        format!("{}（地塊存量餘 {}）。", tide_note, cur_stock - 1)
    } else {
        format!(
            "你在【{}】悉心尋覓採得一份【{}】收進背囊（此地今日存量餘 {}）。",
            node.name,
            spec.name_zh,
            cur_stock - 1
        )
    };
    push_breath(a, msg.clone());
    Ok(msg)
}

#[expect(clippy::too_many_arguments, reason = "explicit specialty sale context")]
pub const SPECIALTY_SELL_TABLE: &[(&str, i64)] = &[
    ("rune_stone", 15),
    ("apex_bone_relic", 18),
    ("spider_carapace", 12),
    ("lizard_scale", 8),
    ("quartz_sand", 6),
    ("iron_ore", 5),
    ("salt_fish_barrel", 5),
    ("dairy_cheese", 5),
    ("beast_tallow", 5),
    ("black_coal", 4),
    ("timber_log", 4),
    ("charcoal_sack", 4),
    ("quarry_stone", 4),
    ("quarantine_wormwood", 4),
    ("driftwood", 4),
    ("herb_bundle", 4),
    ("wheat_grain", 3),
    ("fresh_fish_basket", 3),
    ("shellfish_meat", 3),
];

pub fn specialty_sell_unit_price(item: &super::types::CarryItem) -> Option<i64> {
    if item.count == 0 || item.bound_owner.is_some() {
        return None;
    }
    let base = SPECIALTY_SELL_TABLE
        .iter()
        .find_map(|(s, p)| (*s == item.slug).then_some(*p))
        .or_else(|| {
            item.vessel_meta
                .is_some()
                .then(|| super::phase_and_vessels::appraise_item_fair_copper(item))
        })?;
    Some(if item.stolen_from.is_some() {
        (base / 2).max(1)
    } else {
        base
    })
}

pub fn is_specialty_buyer_role(role: &str) -> bool {
    matches!(
        role,
        "鍛鐵匠"
            | "鍛鐵匠學徒"
            | "木匠"
            | "草藥師"
            | "草藥師學徒"
            | "酒館掌櫃"
            | "酒館掌櫃學徒"
            | "碼頭苦力"
            | "港務記籌手"
            | "行腳商旅"
            | "車行掌櫃"
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "atomic same-node counterparty, item slug, and quote verification"
)]
pub fn settle_sell_specialty_action(
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
    param: &str,
    smith_id: Option<&str>,
    herb_id: Option<&str>,
    porter_id: Option<&str>,
    record_trade_fn: fn(&mut IslandWorldDelta, i64, &str, &str, &str),
) -> Result<String, String> {
    if !matches!(
        node.kind.as_str(),
        "shop" | "dock" | "plaza" | "village_gate"
    ) {
        return Err("需在商鋪、市集廣場或港務碼頭向在場商販當面出售貨品。".to_string());
    }

    // 解析 param：`"<buyer_token>|<item_slug>|<quoted_copper>"` 或 `"<item_slug>"` 或 `""`
    let mut req_buyer_tok: Option<&str> = None;
    let mut req_slug: Option<&str> = None;
    let mut req_quote: Option<i64> = None;
    let trimmed = param.trim();
    if !trimmed.is_empty() {
        let parts: Vec<&str> = trimmed.split('|').collect();
        match parts.as_slice() {
            [btok, islg, qstr] => {
                if !btok.is_empty() {
                    req_buyer_tok = Some(btok);
                }
                if !islg.is_empty() {
                    req_slug = Some(islg);
                }
                req_quote = qstr.parse::<i64>().ok();
            }
            [btok, islg] => {
                if !btok.is_empty() {
                    req_buyer_tok = Some(btok);
                }
                if !islg.is_empty() {
                    req_slug = Some(islg);
                }
            }
            [single] => {
                req_slug = Some(single);
            }
            _ => {}
        }
    }

    let buyer_aid = if let Some(btok) = req_buyer_tok {
        delta
            .node_occupants
            .get(&node.id)
            .and_then(|occ| {
                occ.iter().find(|&oid| {
                    oid != aid
                        && delta.actors.get(oid).is_some_and(|o| {
                            o.alive
                                && o.setup_done
                                && o.controller_id.is_none()
                                && o.node_id == node.id
                                && (o.public_actor_token() == btok || o.actor_id == btok)
                                && is_specialty_buyer_role(&o.role)
                        })
                })
            })
            .cloned()
            .ok_or_else(|| "收貨對象剛離開此處，請重新找在場鋪主或行商議價。".to_string())?
    } else {
        let fallback_role_match = if node.name.contains("草藥") {
            herb_id.or(smith_id)
        } else {
            smith_id.or(herb_id).or(porter_id)
        };
        if let Some(fid) = fallback_role_match {
            fid.to_string()
        } else {
            delta
                .node_occupants
                .get(&node.id)
                .and_then(|occ| {
                    occ.iter().find(|&oid| {
                        oid != aid
                            && delta.actors.get(oid).is_some_and(|o| {
                                o.alive
                                    && o.setup_done
                                    && o.controller_id.is_none()
                                    && o.node_id == node.id
                                    && is_specialty_buyer_role(&o.role)
                            })
                    })
                })
                .cloned()
                .ok_or_else(|| "眼前暫無收購山海特產的鋪主或行商在場。".to_string())?
        }
    };

    let chosen = {
        let a = delta.actors.get(aid).ok_or("找不到旅人行蹤")?;
        if let Some(target_slug) = req_slug {
            a.backpack
                .iter()
                .find(|i| i.slug == target_slug && i.count > 0 && i.bound_owner.is_none())
                .and_then(|i| {
                    specialty_sell_unit_price(i)
                        .map(|fp| (i.slug.clone(), i.name.clone(), fp, i.stolen_from.is_some()))
                })
        } else {
            SPECIALTY_SELL_TABLE
                .iter()
                .find_map(|(s, _)| {
                    a.backpack
                        .iter()
                        .find(|i| i.slug == *s && i.count > 0 && i.bound_owner.is_none())
                        .and_then(|i| {
                            specialty_sell_unit_price(i).map(|fp| {
                                (i.slug.clone(), i.name.clone(), fp, i.stolen_from.is_some())
                            })
                        })
                })
                .or_else(|| {
                    a.backpack
                        .iter()
                        .find(|i| i.vessel_meta.is_some() && i.count > 0 && i.bound_owner.is_none())
                        .and_then(|i| {
                            specialty_sell_unit_price(i).map(|fp| {
                                (i.slug.clone(), i.name.clone(), fp, i.stolen_from.is_some())
                            })
                        })
                })
        }
    };
    let Some((slug, iname, final_price, is_unclaimed)) = chosen else {
        return Err("背囊裡目前沒有該件可出售的山海貨品或落款器物。".to_string());
    };
    if let Some(q) = req_quote
        && q != final_price
    {
        return Err(format!(
            "【{}】當面過秤現價為 {} 枚銅幣，請重新確認報價。",
            iname, final_price
        ));
    }
    let summary = format!("收購山海特產【{}】", iname);
    let can_buyer_self_pay = delta
        .actors
        .get(&buyer_aid)
        .is_some_and(|b| b.copper >= final_price + 4);
    let paid_ok = if can_buyer_self_pay {
        delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "sell_specialty",
            &buyer_aid,
            aid,
            final_price,
            summary.clone(),
        )
    } else {
        delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "sell_specialty",
            "municipal_treasury",
            aid,
            final_price,
            summary.clone(),
        )
    };
    if !paid_ok {
        return Err("收貨商販手頭銅幣暫時周轉不開。".to_string());
    }
    record_trade_fn(delta, clock.game_days, aid, &buyer_aid, &summary);
    let taken = {
        let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
        let pos_opt = a.backpack.iter().position(|i| {
            i.slug == slug
                && i.count > 0
                && i.bound_owner.is_none()
                && i.stolen_from.is_some() == is_unclaimed
        });
        let t = pos_opt.map(|pos| {
            let mut one = a.backpack[pos].clone();
            one.count = 1;
            if a.backpack[pos].count == 1 {
                a.backpack.remove(pos);
            } else {
                a.backpack[pos].count -= 1;
            }
            one
        });
        a.mind.stm.today_goal_progressed = true;
        a.mark_mutated();
        t
    };
    if let Some(mut item) = taken
        && let Some(ba) = delta.actors.get_mut(&buyer_aid)
    {
        item.stolen_from = None;
        if item.vessel_meta.is_some() && ba.backpack.len() < 6 {
            add_carry_item(&mut ba.backpack, item);
        } else {
            let wbox = ba.work_node.clone();
            add_carry_item(ba.inn_lockers.entry(wbox).or_default(), item);
        }
        ba.mark_mutated();
    }
    let buyer_name = delta
        .actors
        .get(&buyer_aid)
        .map_or_else(|| "收貨行商".to_string(), |b| b.display_name.clone());
    let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
    let msg = format!(
        "你將【{}】過秤交予{}，當面點收 {} 枚銅幣。",
        iname, buyer_name, final_price
    );
    push_breath(a, msg.clone());
    Ok(msg)
}
