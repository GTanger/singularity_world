//! 背景活史官（Living Chronicler）：九大高顯著度里程碑事實卡組裝與「單段就地方輿彙整」投射器。
//!
//! 嚴守：
//! 1. 純現代台灣正體白話散文（零「入太陽曆／莫不稱快／愈顯活絡」等半文半白套話，零「空屋自有屋契」系統詞）；
//! 2. 卷一〈方輿地區誌〉每鎮僅維持單一當代近況段（`deed:region_live:<town_slug>`），將該鎮所有新進展就地融合為一段，絕不往下堆疊重複罐頭；
//! 3. `forge_vessel` 僅記錄真正嵌有符文／異獸甲殼之銘器或匠人生平首件落款作，杜絕普通木杖洗版。

use std::sync::Arc;

use super::causal_arcs::solar_year_zh;
use super::clock_and_lang::{SolarClockSnapshot, solar_clock_for_settled_bell};
use super::harbor_lineage::{VacantCauseKind, WHITE_GULL_VACANT_HOUSES};
use super::living_chronicler::{
    atmosphere_clause, build_fact_packet, clean_house_door_zh, clean_place_zh, cooldown_ready,
    is_player_actor, make_row, num_zh, public_actor_label, push_bounded_actor_summary,
    refresh_town_live_region_note, town_slug_of_node,
};
use super::slow_think::mask_player_names_in_text;
use super::types::{IslandWorldDelta, WorldStaticDoc};

/// 里程碑 1：購置待售空屋
pub fn record_vacant_house_bought(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    buyer_id: &str,
    house_node_id: &str,
    price_copper: i64,
) {
    let deed_id = format!("deed:buy_house:{house_node_id}");
    let back_opt = WHITE_GULL_VACANT_HOUSES
        .iter()
        .find(|b| b.house_node_id == house_node_id);
    let neighbor_id = back_opt.map_or("", |b| b.linked_shop_actor_id);
    let mut fact = build_fact_packet(
        doc,
        delta,
        clock,
        deed_id,
        "buy_vacant_house",
        house_node_id,
        buyer_id,
        neighbor_id,
    );
    fact.place_zh = doc
        .nodes
        .get(house_node_id)
        .map_or_else(|| fact.town_zh.clone(), |n| clean_house_door_zh(&n.name));
    fact.amount_copper = price_copper;
    let yr_zh = solar_year_zh(fact.solar_year);
    let price_zh = num_zh(price_copper);
    fact.allowed_numbers_zh = vec![yr_zh.clone(), price_zh.clone()];
    let atmos = atmosphere_clause(
        !fact.tide_zh.is_empty(),
        &fact.weather_zh,
        &fact.tide_zh,
        &fact.bell_zh,
    );

    let trace_clause = if let Some(b) = back_opt {
        let former_yr = solar_year_zh(b.solar_year);
        fact.allowed_numbers_zh.push(former_yr.clone());
        let cause_zh = match b.cause_kind {
            VacantCauseKind::SouthCoastPlague => "南岸船疫封門",
            VacantCauseKind::MainlandTariffBankruptcy => "海峽二成關稅斷貨",
        };
        fact.extra_context_zh = format!(
            "原屋主{}自太陽曆{}年{}後空置，屋內{}",
            b.former_head_zh,
            former_yr,
            cause_zh,
            b.room_trace_zh.trim_end_matches('。')
        );
        format!(
            "這間石木舊屋自太陽曆{former_yr}年{cause_zh}後便一直落栓空著，屋裡{}如今",
            b.room_trace_zh.trim_end_matches('。')
        )
    } else if fact.town_slug == "greypine_post" {
        fact.extra_context_zh = "早年開路石工下山後落鎖空置的舊石屋".to_string();
        "這間早年開路石工撤走後一直空著的舊石屋，如今".to_string()
    } else {
        fact.extra_context_zh = "巷內空置多時的石木舊屋".to_string();
        "這間空置多時的石木舊屋，如今".to_string()
    };

    fact.bullet_facts_zh = vec![
        format!(
            "買屋人：{}（{}）",
            fact.agent_public_label, fact.agent_role_zh
        ),
        format!("屋舍門牌：{}・{}", fact.town_zh, fact.place_zh),
        format!("繳付房契銅幣：{price_zh}枚銅幣"),
        format!("舊屋背景：{}", fact.extra_context_zh),
    ];

    if is_player_actor(delta, buyer_id) {
        if let Some(p) = delta.actors.get_mut(buyer_id) {
            let p_name = p.display_name.clone();
            push_bounded_actor_summary(
                p,
                format!(
                    "太陽曆{yr_zh}年{}，{p_name}繳清{price_zh}枚銅幣買下{}的舊石屋安頓門戶",
                    fact.season_zh, fact.place_zh
                ),
            );
        }
    } else {
        let prose = format!(
            "太陽曆{yr_zh}年{}，{atmos}。{trace_clause}{}向港務銀冊繳清{price_zh}枚銅幣領下{}的屋契，親手推開木門掃淨積塵，在石砌壁爐重新生起柴火，於{}穩穩立住自家門戶。",
            fact.season_zh, fact.agent_public_label, fact.place_zh, fact.town_zh
        );
        delta
            .living_chronicle
            .push_new_projection(make_row(&fact, "actor", buyer_id, "", "", "", "", prose));
        if let Some(a) = delta.actors.get_mut(buyer_id) {
            push_bounded_actor_summary(
                a,
                format!(
                    "太陽曆{yr_zh}年{}，以{price_zh}枚銅幣買下{}立起門戶",
                    fact.season_zh, fact.place_zh
                ),
            );
        }
    }

    if !neighbor_id.is_empty()
        && !is_player_actor(delta, neighbor_id)
        && delta.actors.contains_key(neighbor_id)
    {
        let mut nb_fact = fact.clone();
        nb_fact.kind = "buy_vacant_house_neighbor".to_string();
        nb_fact.bullet_facts_zh = vec![
            format!(
                "視角主角（街坊鄰居）：{}（{}）",
                fact.patient_public_label, fact.patient_role_zh
            ),
            format!(
                "隔壁重開門戶：{}由{}繳清{price_zh}枚銅幣買下屋契入住",
                fact.place_zh, fact.agent_public_label
            ),
        ];
        let nb_prose = format!(
            "太陽曆{yr_zh}年{}，{}見證了巷裡久違的新動靜：隔壁空置落鎖多時的{}由{}繳清{price_zh}枚銅幣買下屋契，冷透多年的煙囪重新冒出炊煙，整條巷子跟著熱絡起來。",
            fact.season_zh, fact.patient_public_label, fact.place_zh, fact.agent_public_label
        );
        delta.living_chronicle.push_new_projection(make_row(
            &nb_fact,
            "actor",
            neighbor_id,
            "",
            "",
            "",
            "",
            nb_prose,
        ));
    }

    refresh_town_live_region_note(doc, delta, clock, &fact.town_slug);

    if fact.town_slug == "white_gull_harbor" {
        let wgh_cnt = delta
            .sold_houses
            .keys()
            .filter(|nid| town_slug_of_node(nid) == "white_gull_harbor")
            .count();
        if let Some(&tier) = [6usize, 3, 1].iter().find(|&&t| wgh_cnt >= t) {
            let latch = format!("latch:white_gull_harbor:vacant_sold:{tier}");
            if !delta
                .living_chronicle
                .committed
                .fired_latches
                .contains(&latch)
            {
                {
                    let store = Arc::make_mut(&mut delta.living_chronicle.committed);
                    for lower in [1usize, 3] {
                        if lower < tier {
                            store
                                .fired_latches
                                .insert(format!("latch:white_gull_harbor:vacant_sold:{lower}"));
                        }
                    }
                }
                let cnt_zh = num_zh(wgh_cnt as i64);
                let mut afact = fact.clone();
                afact.deed_id = format!("deed:wgh_vacant_tier:{tier}");
                afact.allowed_numbers_zh.extend([
                    "三二三".to_string(),
                    "三二七".to_string(),
                    "二".to_string(),
                    "六".to_string(),
                    cnt_zh.clone(),
                ]);
                let ann_prose = format!(
                    "太陽曆三二三至三二七年間，白鷗港先後遇上蒼淵海峽二成防禦關稅與古加津南岸船疫，鐵砧巷、船帆巷、銅幣巷與鹹魚巷接連有六戶人家退租西遷或封門絕戶，石砌壁爐自此冷透多年。\n\n到了太陽曆{yr_zh}年{}，隨著{}買下{}並重新點起爐火，白鷗港昔日封門的舊宅已有{cnt_zh}戶恢復人煙。街坊老戶傍晚走過巷口，看見舊屋窗櫺重新透出暖黃燈光，才覺得那段封門避疫的陰霾正慢慢散去。",
                    fact.season_zh, fact.agent_public_label, fact.place_zh
                );
                delta.living_chronicle.push_new_projection(make_row(
                    &afact,
                    "annals",
                    "ann:live:wgh_vacant_rekindled",
                    &latch,
                    "白鷗舊巷空戶重燃燈火記",
                    "關稅與船疫封門舊宅重歸人煙之當代本末",
                    "白鷗港市政廳房契木冊與街坊見聞錄",
                    ann_prose,
                ));
            }
        }
    }
}

/// 里程碑 2：攢足木鐵料與銅幣自建新宅（`built_houses`）
pub fn record_homestead_built(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    builder_id: &str,
    lot_node_id: &str,
    lot_name: &str,
) {
    let deed_id = format!("deed:homestead:{builder_id}:{}", clock.game_days);
    let mut fact = build_fact_packet(
        doc,
        delta,
        clock,
        deed_id,
        "build_homestead",
        lot_node_id,
        builder_id,
        "",
    );
    fact.place_zh = clean_place_zh(lot_name);
    fact.amount_copper = 120;
    let yr_zh = solar_year_zh(fact.solar_year);
    fact.allowed_numbers_zh = vec![yr_zh.clone(), "一百二十".to_string()];
    fact.bullet_facts_zh = vec![
        format!(
            "起屋人：{}（{}）",
            fact.agent_public_label, fact.agent_role_zh
        ),
        format!("營建地點：{}・{}", fact.town_zh, fact.place_zh),
        "耗用工料：攢齊粗鐵料、木材與一百二十枚銅幣工本，親手立柱架樑成為自立門戶的屋主"
            .to_string(),
    ];
    let atmos = atmosphere_clause(
        !fact.tide_zh.is_empty(),
        &fact.weather_zh,
        &fact.tide_zh,
        &fact.bell_zh,
    );

    if is_player_actor(delta, builder_id) {
        if let Some(p) = delta.actors.get_mut(builder_id) {
            let p_name = p.display_name.clone();
            push_bounded_actor_summary(
                p,
                format!(
                    "太陽曆{yr_zh}年{}，{p_name}攢齊木鐵料與一百二十枚銅幣，在{}起樑築屋自立門戶",
                    fact.season_zh, fact.place_zh
                ),
            );
        }
    } else {
        let prose = format!(
            "太陽曆{yr_zh}年{}，{atmos}。{}攢足粗鐵料、木材與一百二十枚銅幣工本，在{}丈量地基、立柱架樑，親手蓋起一棟遮風擋雨的石木新宅，從此在{}自立門戶。",
            fact.season_zh, fact.agent_public_label, fact.place_zh, fact.town_zh
        );
        delta
            .living_chronicle
            .push_new_projection(make_row(&fact, "actor", builder_id, "", "", "", "", prose));
        if let Some(a) = delta.actors.get_mut(builder_id) {
            push_bounded_actor_summary(
                a,
                format!(
                    "太陽曆{yr_zh}年{}，在{}立柱築起新宅自立門戶",
                    fact.season_zh, fact.place_zh
                ),
            );
        }
    }

    refresh_town_live_region_note(doc, delta, clock, &fact.town_slug);
}

/// 里程碑 3：生計受阻轉行碼頭苦力重頭攢起
pub fn record_career_shift_dock(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_id: &str,
    dock_node_id: &str,
) {
    if is_player_actor(delta, actor_id)
        || !cooldown_ready(delta, actor_id, "career_shift_dock", clock.game_days, 7)
    {
        return;
    }
    let deed_id = format!("deed:career_dock:{actor_id}:{}", clock.game_days);
    let mut fact = build_fact_packet(
        doc,
        delta,
        clock,
        deed_id,
        "career_shift_dock",
        dock_node_id,
        actor_id,
        "",
    );
    let yr_zh = solar_year_zh(fact.solar_year);
    fact.allowed_numbers_zh = vec![yr_zh.clone()];
    fact.bullet_facts_zh = vec![
        format!("當事人：{}", fact.agent_public_label),
        format!("轉行地點：{}・{}", fact.town_zh, fact.place_zh),
        "緣由與行動：原本行當連日入不敷出、手頭緊絀，決定放下身段改去碼頭扛重貨箱，靠勞力掙銅幣重新攢起家計".to_string(),
    ];
    let atmos = atmosphere_clause(
        !fact.tide_zh.is_empty(),
        &fact.weather_zh,
        &fact.tide_zh,
        &fact.bell_zh,
    );
    let prose = format!(
        "太陽曆{yr_zh}年{}，{atmos}。{}因舊業連日入不敷出、手頭緊絀，索性捲起衣袖改去{}扛起沉甸甸的貨箱，靠一雙肩膀掙取銅幣重頭攢起家計。",
        fact.season_zh, fact.agent_public_label, fact.place_zh
    );
    delta
        .living_chronicle
        .push_new_projection(make_row(&fact, "actor", actor_id, "", "", "", "", prose));
    if let Some(a) = delta.actors.get_mut(actor_id) {
        push_bounded_actor_summary(
            a,
            format!(
                "太陽曆{yr_zh}年{}，改赴{}扛重活重頭攢起家計",
                fact.season_zh, fact.place_zh
            ),
        );
    }
}

/// 里程碑 4：當眾見證武學成相／百工開相
#[expect(
    clippy::too_many_arguments,
    reason = "見證成相需完整傳遞角色、場所、隨身器物與相位名"
)]
pub fn record_witnessed_phase(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_id: &str,
    node_id: &str,
    weapon_raw: &str,
    phase_name: &str,
    is_martial: bool,
) {
    let kind = if is_martial {
        "martial_phase"
    } else {
        "craft_phase"
    };
    let latch = format!("latch:phase:{kind}:{actor_id}");
    if delta
        .living_chronicle
        .committed
        .fired_latches
        .contains(&latch)
    {
        return;
    }
    let deed_id = format!("deed:{kind}:{actor_id}");
    let mut fact = build_fact_packet(doc, delta, clock, deed_id, kind, node_id, actor_id, "");
    fact.vessel_zh = mask_player_names_in_text(delta, weapon_raw);
    fact.phase_zh = phase_name.to_string();
    let yr_zh = solar_year_zh(fact.solar_year);
    fact.allowed_numbers_zh = vec![yr_zh.clone()];
    let atmos = atmosphere_clause(
        !fact.tide_zh.is_empty(),
        &fact.weather_zh,
        &fact.tide_zh,
        &fact.bell_zh,
    );
    let wit_clause = if let Some(w0) = fact.witness_names.first() {
        format!("在{w0}等在場街坊親眼見證下，")
    } else {
        "在旁人親眼見證下，".to_string()
    };
    fact.bullet_facts_zh = vec![
        format!(
            "當事人：{}（{}）",
            fact.agent_public_label, fact.agent_role_zh
        ),
        format!("見證地點：{}・{}", fact.town_zh, fact.place_zh),
        format!("隨身器物：{}", fact.vessel_zh),
        format!("街坊傳開之名號：「{phase_name}」"),
    ];

    if is_player_actor(delta, actor_id) {
        if let Some(p) = delta.actors.get_mut(actor_id) {
            push_bounded_actor_summary(
                p,
                format!(
                    "太陽曆{yr_zh}年{}，於{}凝出「{phase_name}」名號",
                    fact.season_zh, fact.place_zh
                ),
            );
        }
        return;
    }
    let prose = if is_martial {
        format!(
            "太陽曆{yr_zh}年{}，{atmos}。{}{}在{}交鋒時揮動【{}】使出沉穩連貫的破敵勁道，「{}」這個身手名號自此在{}街坊間傳了開來。",
            fact.season_zh,
            wit_clause,
            fact.agent_public_label,
            fact.place_zh,
            fact.vessel_zh,
            phase_name,
            fact.town_zh
        )
    } else {
        format!(
            "太陽曆{yr_zh}年{}，{atmos}。{}{}在{}案台前運使【{}】成器俐落純熟，鎮裡同行自此稱道其「{}」的獨到火候。",
            fact.season_zh,
            wit_clause,
            fact.agent_public_label,
            fact.place_zh,
            fact.vessel_zh,
            phase_name
        )
    };
    delta.living_chronicle.push_new_projection(make_row(
        &fact, "actor", actor_id, &latch, "", "", "", prose,
    ));
    if let Some(a) = delta.actors.get_mut(actor_id) {
        push_bounded_actor_summary(
            a,
            format!(
                "太陽曆{yr_zh}年{}，在{}獲街坊公認「{phase_name}」名號",
                fact.season_zh, fact.place_zh
            ),
        );
    }
}

/// 里程碑 5：親手鍛造銘器（僅限嵌有古符文／異獸甲殼之真銘器，或匠人生平第一件署名代表作，杜絕普通木杖洗版！）
pub fn record_vessel_forged(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_id: &str,
    node_id: &str,
    vessel_raw: &str,
    rune_opt: Option<&str>,
) {
    let first_latch = format!("latch:first_vessel:{actor_id}");
    let is_first_ever = !delta
        .living_chronicle
        .committed
        .fired_latches
        .contains(&first_latch);
    if rune_opt.is_none() && !is_first_ever {
        return;
    }
    if !cooldown_ready(delta, actor_id, "forge_vessel", clock.game_days, 6) {
        return;
    }
    let deed_id = format!("deed:vessel:{actor_id}:{}", clock.game_days);
    let mut fact = build_fact_packet(
        doc,
        delta,
        clock,
        deed_id,
        "forge_vessel",
        node_id,
        actor_id,
        "",
    );
    fact.vessel_zh = clean_place_zh(&mask_player_names_in_text(delta, vessel_raw));
    let yr_zh = solar_year_zh(fact.solar_year);
    fact.allowed_numbers_zh = vec![yr_zh.clone()];
    let atmos = atmosphere_clause(
        !fact.tide_zh.is_empty(),
        &fact.weather_zh,
        &fact.tide_zh,
        &fact.bell_zh,
    );

    if is_player_actor(delta, actor_id) {
        if let Some(p) = delta.actors.get_mut(actor_id) {
            push_bounded_actor_summary(
                p,
                format!(
                    "太陽曆{yr_zh}年{}，在{}親手鍛成落款器物【{vessel_raw}】",
                    fact.season_zh, fact.place_zh
                ),
            );
        }
        return;
    }

    let (latch_to_use, prose, detail_bullet) = if let Some(r) = rune_opt {
        (
            if is_first_ever {
                first_latch.as_str()
            } else {
                ""
            },
            format!(
                "太陽曆{yr_zh}年{}，{atmos}。{}在{}爐台前反覆錘打淬火，將一枚{r}嵌合進器身，親手鍛成落有自家名號的【{}】，器脊透出沉實冷光，成了鋪裡難得的得意銘器。",
                fact.season_zh, fact.agent_public_label, fact.place_zh, fact.vessel_zh
            ),
            format!(
                "將稀有材料（{r}）嵌合器身，親手鍛成刻有自家名號的銘器【{}】",
                fact.vessel_zh
            ),
        )
    } else {
        (
            first_latch.as_str(),
            format!(
                "太陽曆{yr_zh}年{}，{atmos}。{}在{}案台前獨立選料開工，親手製成第一件刻上自家名號的【{}】，總算在同行與街坊面前亮出了自己的手藝招牌。",
                fact.season_zh, fact.agent_public_label, fact.place_zh, fact.vessel_zh
            ),
            format!(
                "在工坊獨立完成首件刻有自家名號的器物【{}】，立起手藝招牌",
                fact.vessel_zh
            ),
        )
    };
    fact.bullet_facts_zh = vec![
        format!(
            "匠人：{}（{}）",
            fact.agent_public_label, fact.agent_role_zh
        ),
        format!("工坊地點：{}・{}", fact.town_zh, fact.place_zh),
        detail_bullet,
    ];
    delta.living_chronicle.push_new_projection(make_row(
        &fact,
        "actor",
        actor_id,
        latch_to_use,
        "",
        "",
        "",
        prose,
    ));
    if let Some(a) = delta.actors.get_mut(actor_id) {
        push_bounded_actor_summary(
            a,
            format!(
                "太陽曆{yr_zh}年{}，在{}親手製成【{}】",
                fact.season_zh, fact.place_zh, fact.vessel_zh
            ),
        );
    }
}

/// 里程碑 6：雙邊危難拔刀救援
#[expect(
    clippy::too_many_arguments,
    reason = "雙邊危難救援需完整綁定施救者、受救者、節點、兵器與敵手快照"
)]
pub fn record_crisis_rescue(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    helper_id: &str,
    ally_id: &str,
    node_id: &str,
    helper_wep_raw: &str,
    foe_name: &str,
) {
    if !cooldown_ready(delta, helper_id, "crisis_rescue", clock.game_days, 2)
        && !cooldown_ready(delta, ally_id, "crisis_rescue_patient", clock.game_days, 2)
    {
        return;
    }
    let deed_id = format!("deed:rescue:{}:{}:{}", clock.game_days, helper_id, ally_id);
    let mut fact = build_fact_packet(
        doc,
        delta,
        clock,
        deed_id,
        "crisis_rescue",
        node_id,
        helper_id,
        ally_id,
    );
    fact.vessel_zh = clean_place_zh(&mask_player_names_in_text(delta, helper_wep_raw));
    fact.extra_context_zh = foe_name.to_string();
    let yr_zh = solar_year_zh(fact.solar_year);
    fact.allowed_numbers_zh = vec![yr_zh.clone()];
    let atmos = atmosphere_clause(
        !fact.tide_zh.is_empty(),
        &fact.weather_zh,
        &fact.tide_zh,
        &fact.bell_zh,
    );

    if !is_player_actor(delta, helper_id)
        && cooldown_ready(delta, helper_id, "crisis_rescue", clock.game_days, 2)
    {
        fact.bullet_facts_zh = vec![
            format!(
                "本傳主角（出手救人方）：{}（{}）",
                fact.agent_public_label, fact.agent_role_zh
            ),
            format!("受救對象（遇險方）：{}", fact.patient_public_label),
            format!("交鋒地點與強敵：{}，敵手為{}", fact.place_zh, foe_name),
            format!(
                "經過：{}握緊【{}】從側翼切入戰圈震偏敵勢，與{}合力逼退強敵，結下患難交情",
                fact.agent_public_label, fact.vessel_zh, fact.patient_public_label
            ),
        ];
        let agent_prose = format!(
            "太陽曆{yr_zh}年{}，{atmos}。{}走過{}時撞見{}正被{}逼入險境，當即握緊【{}】從側翼切入戰圈震偏敵勢，兩人合力逼退強敵，也就此結下一份過命的患難交情。",
            fact.season_zh,
            fact.agent_public_label,
            fact.place_zh,
            fact.patient_public_label,
            foe_name,
            fact.vessel_zh
        );
        delta.living_chronicle.push_new_projection(make_row(
            &fact,
            "actor",
            helper_id,
            "",
            "",
            "",
            "",
            agent_prose,
        ));
        if let Some(h) = delta.actors.get_mut(helper_id) {
            push_bounded_actor_summary(
                h,
                format!(
                    "太陽曆{yr_zh}年{}，在{}拔刀助{}合力逼退{}",
                    fact.season_zh, fact.place_zh, fact.patient_public_label, foe_name
                ),
            );
        }
    }

    if !is_player_actor(delta, ally_id)
        && cooldown_ready(delta, ally_id, "crisis_rescue_patient", clock.game_days, 2)
    {
        let mut pat_fact = fact.clone();
        pat_fact.kind = "crisis_rescue_patient".to_string();
        pat_fact.bullet_facts_zh = vec![
            format!(
                "本傳主角（遇險受救方）：{}（{}）",
                fact.patient_public_label, fact.patient_role_zh
            ),
            format!("出手搭救恩人（施救方）：{}", fact.agent_public_label),
            format!(
                "遇險地點與敵手：{}，遭{}圍攻險象環生",
                fact.place_zh, foe_name
            ),
            format!(
                "經過：危急時刻幸得{}揮動【{}】從旁切入戰局合力解圍，{}將這份救命人情牢記在心",
                fact.agent_public_label, fact.vessel_zh, fact.patient_public_label
            ),
        ];
        let patient_prose = format!(
            "太陽曆{yr_zh}年{}，{atmos}。{}在{}迎戰{}時一度陷入險境，危急關頭幸得{}揮動【{}】從旁切入合擊解圍，這份救命恩情自此牢牢記在心裡。",
            fact.season_zh,
            fact.patient_public_label,
            fact.place_zh,
            foe_name,
            fact.agent_public_label,
            fact.vessel_zh
        );
        delta.living_chronicle.push_new_projection(make_row(
            &pat_fact,
            "actor",
            ally_id,
            "",
            "",
            "",
            "",
            patient_prose,
        ));
        if let Some(al) = delta.actors.get_mut(ally_id) {
            push_bounded_actor_summary(
                al,
                format!(
                    "太陽曆{yr_zh}年{}，在{}遇險時蒙{}出手合擊解圍",
                    fact.season_zh, fact.place_zh, fact.agent_public_label
                ),
            );
        }
    }
}

/// 里程碑 7：雙邊契約違約失信
pub fn record_pact_default(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    contract_id: &str,
    creditor_id: &str,
    debtor_id: &str,
    amount_copper: i64,
) {
    let node_id = delta
        .actors
        .get(creditor_id)
        .or_else(|| delta.actors.get(debtor_id))
        .map_or_else(
            || "oi:white_gull_harbor:plaza".to_string(),
            |a| a.node_id.clone(),
        );
    let deed_id = format!("deed:default:{contract_id}:{}", clock.game_days);
    let mut fact = build_fact_packet(
        doc,
        delta,
        clock,
        deed_id,
        "pact_default",
        &node_id,
        debtor_id,
        creditor_id,
    );
    fact.amount_copper = amount_copper;
    let amt_zh = num_zh(amount_copper);
    let yr_zh = solar_year_zh(fact.solar_year);
    fact.allowed_numbers_zh = vec![yr_zh.clone(), amt_zh.clone()];
    let atmos = atmosphere_clause(
        !fact.tide_zh.is_empty(),
        &fact.weather_zh,
        &fact.tide_zh,
        &fact.bell_zh,
    );

    if !is_player_actor(delta, debtor_id)
        && cooldown_ready(delta, debtor_id, "pact_default", clock.game_days, 3)
    {
        fact.bullet_facts_zh = vec![
            format!("本傳主角（欠款違約方）：{}", fact.agent_public_label),
            format!("債權人（被拖欠方）：{}", fact.patient_public_label),
            format!(
                "地點與金額：{}，到期未能湊足{amt_zh}枚銅幣清償木契",
                fact.town_zh
            ),
        ];
        let deb_prose = format!(
            "太陽曆{yr_zh}年{}，{atmos}。{}因手頭周轉不開，到了約定日子仍湊不足{amt_zh}枚銅幣向{}清償木契，在{}落下一筆沉甸甸的逾期舊欠。",
            fact.season_zh, fact.agent_public_label, fact.patient_public_label, fact.town_zh
        );
        delta.living_chronicle.push_new_projection(make_row(
            &fact, "actor", debtor_id, "", "", "", "", deb_prose,
        ));
        if let Some(d) = delta.actors.get_mut(debtor_id) {
            push_bounded_actor_summary(
                d,
                format!(
                    "太陽曆{yr_zh}年{}，到期未能清償對{}的{amt_zh}枚銅幣木契",
                    fact.season_zh, fact.patient_public_label
                ),
            );
        }
    }

    if !is_player_actor(delta, creditor_id)
        && cooldown_ready(
            delta,
            creditor_id,
            "pact_default_creditor",
            clock.game_days,
            3,
        )
    {
        let mut cfact = fact.clone();
        cfact.kind = "pact_default_creditor".to_string();
        cfact.bullet_facts_zh = vec![
            format!("本傳主角（債權催討方）：{}", fact.patient_public_label),
            format!("逾期欠款方：{}", fact.agent_public_label),
            format!(
                "地點與金額：{}，對方過了約定鐘響仍未清償{amt_zh}枚銅幣木契，雙方情分轉趨緊繃",
                fact.town_zh
            ),
        ];
        let cred_prose = format!(
            "太陽曆{yr_zh}年{}，{atmos}。{}原與{}立有{amt_zh}枚銅幣的往來木契，不料過了約定時辰對方仍拿不出銅幣履約，只得沉著臉在帳冊記下這筆催討舊帳，兩家往來情分也跟著緊繃起來。",
            fact.season_zh, fact.patient_public_label, fact.agent_public_label
        );
        delta.living_chronicle.push_new_projection(make_row(
            &cfact,
            "actor",
            creditor_id,
            "",
            "",
            "",
            "",
            cred_prose,
        ));
        if let Some(c) = delta.actors.get_mut(creditor_id) {
            push_bounded_actor_summary(
                c,
                format!(
                    "太陽曆{yr_zh}年{}，遭{}拖欠{amt_zh}枚銅幣到期木契",
                    fact.season_zh, fact.agent_public_label
                ),
            );
        }
    }

    let recent_cnt = {
        let store = Arc::make_mut(&mut delta.living_chronicle.committed);
        let entries = store
            .town_default_days
            .entry(fact.town_slug.clone())
            .or_default();
        entries.retain(|(d, _)| clock.game_days.saturating_sub(*d) <= 10);
        entries.insert((clock.game_days, fact.deed_id.clone()));
        entries.len()
    };
    if recent_cnt >= 3 {
        refresh_town_live_region_note(doc, delta, clock, &fact.town_slug);
    }
}

/// 里程碑 8：北林試煉深坑首殺吞石巨蛛
pub fn record_primordial_spider_slain(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    primary_id: &str,
    co_helper_id: Option<&str>,
    node_id: &str,
    weapon_raw: &str,
) {
    let latch = "latch:island:first_spider_slain";
    if delta
        .living_chronicle
        .committed
        .fired_latches
        .contains(latch)
    {
        return;
    }
    let helper_str = co_helper_id.unwrap_or("");
    let mut fact = build_fact_packet(
        doc,
        delta,
        clock,
        "deed:island:first_spider_slain".to_string(),
        "slay_stone_spider",
        node_id,
        primary_id,
        helper_str,
    );
    fact.town_slug = "greypine_post".to_string();
    fact.town_zh = "灰松林驛".to_string();
    fact.tide_zh.clear();
    fact.vessel_zh = clean_place_zh(&mask_player_names_in_text(delta, weapon_raw));
    let yr_zh = solar_year_zh(fact.solar_year);
    fact.allowed_numbers_zh = vec![yr_zh.clone(), "二三八".to_string(), "九十".to_string()];
    let atmos = atmosphere_clause(false, &fact.weather_zh, "", &fact.bell_zh);
    let hero_title = if fact.patient_public_label.is_empty()
        || fact.patient_public_label == fact.agent_public_label
    {
        fact.agent_public_label.clone()
    } else {
        format!("{}與{}", fact.agent_public_label, fact.patient_public_label)
    };

    for &uid in &[Some(primary_id), co_helper_id] {
        let Some(aid) = uid else { continue };
        if is_player_actor(delta, aid) {
            if let Some(p) = delta.actors.get_mut(aid) {
                let p_name = p.display_name.clone();
                push_bounded_actor_summary(
                    p,
                    format!(
                        "太陽曆{yr_zh}年{}，{p_name}深入{}擊潰盤踞多年的吞石巨蛛並撬下厚甲殼",
                        fact.season_zh, fact.place_zh
                    ),
                );
            }
        } else {
            let mut afact = fact.clone();
            let (my_lbl, _) = public_actor_label(delta, aid);
            afact.agent_public_label = my_lbl.clone();
            afact.patient_public_label.clear();
            afact.bullet_facts_zh = vec![
                format!("本傳主角：{my_lbl}"),
                format!("同行戰友：{hero_title}"),
                format!("地點與兵刃：北林斷橋後的{}，隨身兵刃為【{}】", fact.place_zh, fact.vessel_zh),
                "戰果：趁吞石巨蛛前螯撲空猛擊甲縫關節，碎裂重甲逼潰盤踞九十餘年的凶獸，撬下厚甲殼打通南面地脈凝石台石道".to_string(),
            ];
            let actor_prose = format!(
                "太陽曆{yr_zh}年{}，{atmos}。{my_lbl}持【{}】踏過北林斷橋深入{}，看準吞石巨蛛前螯撲空的空檔猛擊甲縫關節，一舉碎裂重甲擊潰這頭盤踞深坑多年的凶獸，撬下厚甲殼打通了通往南面地脈凝石台的石道。",
                fact.season_zh, fact.vessel_zh, fact.place_zh
            );
            delta.living_chronicle.push_new_projection(make_row(
                &afact,
                "actor",
                aid,
                "",
                "",
                "",
                "",
                actor_prose,
            ));
            if let Some(a) = delta.actors.get_mut(aid) {
                push_bounded_actor_summary(
                    a,
                    format!(
                        "太陽曆{yr_zh}年{}，深入{}擊潰吞石巨蛛打通地脈石台",
                        fact.season_zh, fact.place_zh
                    ),
                );
            }
        }
    }

    fact.bullet_facts_zh = vec![
        "前因：太陽曆二三八年北林老礦坑塌陷封坑、灰松木橋震斷後，斷橋岩縫後的試煉深坑被甲殼堅如生鐵的吞石巨蛛盤踞九十餘年，南面地脈凝石台淪為絕境".to_string(),
        format!("當代破局：太陽曆{yr_zh}年{}，{hero_title}踏過北林碎石坡深入{}，揮動【{}】重擊巨蛛甲縫關節將其擊潰，撬下吞石巨蛛厚甲殼，打通南面地脈凝石台石道", fact.season_zh, fact.place_zh, fact.vessel_zh),
        "迴響：消息傳回灰松林驛與白鷗港，山林獵戶與矮人老石匠舉杯慶賀，港鎮石工將此事補刻於廣場石碑背面，與〈北林深坑破隙與矮人封閘入城紀〉前後呼應".to_string(),
    ];
    let ann_prose = format!(
        "太陽曆二三八年北林老礦坑塌陷封坑、灰松木橋震斷之後，斷橋岩縫後的試煉深坑便被一頭甲殼堅如生鐵的吞石巨蛛盤踞。九十餘年間偶有獵戶探近，聽見坑底刮石嘶鳴無不變色退避，深處的地脈凝石台就此成了無人能近的絕境。\n\n直到太陽曆{yr_zh}年{}，{atmos}。{hero_title}踏過北林碎石坡深入{}，趁巨蛛前螯撲空的瞬間揮動【{}】猛擊甲縫關節，一舉劈裂厚甲將牠擊潰入深澗，當場撬下吞石巨蛛厚甲殼，封阻近百年的南面地脈石台石道自此重新貫通。\n\n消息傳回灰松林驛與白鷗港，山林獵戶與矮人老石匠紛紛舉杯相慶，港鎮石工也將這樁壯舉補刻在廣場石碑背面，與昔年〈北林深坑破隙與矮人封閘入城紀〉前後呼應。",
        fact.season_zh, fact.place_zh, fact.vessel_zh
    );
    delta.living_chronicle.push_new_projection(make_row(
        &fact,
        "annals",
        "ann:live:north_mine_spider_slain",
        latch,
        "北林試煉深坑誅除吞石巨蛛本末",
        "封坑九十載後重開地脈凝石台之當代壯舉",
        "灰松林驛守山木牌與白鷗港廣場補刻碑陰",
        ann_prose,
    ));

    refresh_town_live_region_note(doc, delta, clock, "greypine_post");
}

/// 里程碑 9：跨海新移民登島入籍（`spawn_dynamic_actors`）
pub fn record_cross_sea_arrival(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    game_day: i64,
    bell_idx: u8,
    actor_id: &str,
) {
    if is_player_actor(delta, actor_id) {
        return;
    }
    let Some((node_id, role, race)) = delta
        .actors
        .get(actor_id)
        .map(|a| (a.node_id.clone(), a.role.clone(), a.race.clone()))
    else {
        return;
    };
    let clock = solar_clock_for_settled_bell(game_day * 12 + i64::from(bell_idx), 0);
    let deed_id = format!("deed:arrival:{actor_id}");
    let mut fact = build_fact_packet(
        doc,
        delta,
        &clock,
        deed_id,
        "cross_sea_migrant",
        &node_id,
        actor_id,
        "",
    );
    let yr_zh = solar_year_zh(fact.solar_year);
    fact.allowed_numbers_zh = vec![yr_zh.clone()];
    fact.bullet_facts_zh = vec![
        format!("新入籍居民：{}（{race}{role}）", fact.agent_public_label),
        format!("登岸落腳處：{}・{}", fact.town_zh, fact.place_zh),
        "背景：自蒼淵主陸乘船渡海避亂抵達銀鷗島，辦妥入籍落腳手續展開新生活".to_string(),
    ];
    let atmos = atmosphere_clause(
        !fact.tide_zh.is_empty(),
        &fact.weather_zh,
        &fact.tide_zh,
        &fact.bell_zh,
    );
    let prose = format!(
        "太陽曆{yr_zh}年{}，{atmos}。身為{race}{role}的{}從蒼淵主陸乘船渡海抵達{}，在{}辦妥入籍落腳手續，帶著一身手藝與行囊在銀鷗島展開新生活。",
        fact.season_zh, fact.agent_public_label, fact.place_zh, fact.town_zh
    );
    delta
        .living_chronicle
        .push_new_projection(make_row(&fact, "actor", actor_id, "", "", "", "", prose));
}
