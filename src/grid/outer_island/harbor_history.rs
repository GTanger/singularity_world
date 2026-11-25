//! 銀鷗島與白鷗港「現況逆推生肉 → 家族／師門史 → 銜接公史脈絡點」確定性雙邊歷史引擎。

use std::collections::BTreeMap;

pub use super::harbor_lineage::{
    CURRENT_SOLAR_YEAR, VacantCauseKind, VacantHouseBackstory, WHITE_GULL_VACANT_HOUSES,
    vacant_house_trace_zh,
};
use super::harbor_lineage::{
    EventLayer, PartyStance, concrete_member_craft_role, infer_household_birth_years_and_bonds,
    inject_event_into_actor, natural_age_appearance,
};
use super::types::{IslandWorldDelta, WorldStaticDoc};

fn parse_house_num_suffix(node_id: &str) -> u32 {
    node_id
        .rsplit_once(":h")
        .and_then(|(_, digits)| digits.parse::<u32>().ok())
        .unwrap_or(0)
}

/// 為銀鷗島全體常住居民逆推「年齡輩分、師門原生家系、器物來歷、≥80% 跨戶微歷史與 ≤20% 公史脈絡點」。
/// 嚴格冪等：重複呼叫不會翻倍關係計數或轉折點，不修改任何人的 `copper`，亦絕不觸碰玩家角色。
pub fn apply_island_history_backfill(doc: &WorldStaticDoc, delta: &mut IslandWorldDelta) -> usize {
    let mut hh_members: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (aid, a) in &delta.actors {
        if a.controller_id.is_none()
            && a.alive
            && !a.household_id.is_empty()
            && !a.household_id.contains(":homestead:")
            && a.household_bond != "自立門戶屋主"
        {
            hh_members
                .entry(a.household_id.clone())
                .or_default()
                .push(aid.clone());
        }
    }

    let mut touched_actors = 0usize;

    // 1. 第一步：以戶長為錨整戶逆推真實生年、單調世代輩分、具體行當與器物祖傳銘記
    for (hh_id, mem_ids) in &hh_members {
        let is_shop = hh_id.contains(":shop_");
        let is_porter = hh_id.ends_with(":porter");
        let fam_size = mem_ids.len();
        let Some((head_role, head_name, head_race, hh_seed)) = mem_ids
            .first()
            .and_then(|id| delta.actors.get(id))
            .map(|a| {
                (
                    a.role.clone(),
                    a.display_name.clone(),
                    a.race.clone(),
                    a.soul_seed.max(1),
                )
            })
        else {
            continue;
        };

        let schedule = infer_household_birth_years_and_bonds(
            &head_race, is_shop, is_porter, fam_size, hh_seed,
        );
        let mut member_profiles: Vec<(String, String, String)> = Vec::with_capacity(fam_size);

        for (idx, aid) in mem_ids.iter().enumerate() {
            let Some(ro) = delta.actors.get(aid) else {
                continue;
            };
            let race = ro.race.clone();
            let seed = ro.soul_seed.max(1);
            let (birth_year, bond) = schedule
                .get(idx)
                .copied()
                .unwrap_or((CURRENT_SOLAR_YEAR - 25, "同屋親眷"));
            let new_role = if !is_shop
                && !is_porter
                && idx > 0
                && (ro.role == "家眷" || ro.role == "年輕幫手")
            {
                concrete_member_craft_role(&head_role, idx, seed).to_string()
            } else {
                ro.role.clone()
            };
            let anc_note = if is_shop && idx > 0 {
                format!("拜在{}門下學藝，鋪裡規矩嚴實", head_name)
            } else if is_porter {
                "早年隨遠洋木船在風浪裡討生活，落腳港務碼頭多年".to_string()
            } else {
                match race.as_str() {
                    "精靈" => format!("{}一脈自開港早年便在島上林灣定居", head_name),
                    "矮人" => format!("{}家先輩自北林礦脈遷入石牆內傳下爐火手藝", head_name),
                    _ => format!("{}家在島上繁衍數代，靠雙手營生立住門戶", head_name),
                }
            };
            let default_tool_prov = if idx == 0 {
                match race.as_str() {
                    "矮人" => format!("北林封坑遷城後傳下的老傢伙，柄身刻著{}家記號", head_name),
                    "精靈" => format!("{}長年順手養出的舊器，木柄溫潤合手", head_name),
                    _ => format!("{}接手家業時重打整修的隨身吃飯傢伙", head_name),
                }
            } else if is_shop {
                format!("入鋪拜師時由師父{}親手交下的學藝傢伙", head_name)
            } else {
                format!("由家裡當家{}分派操持活計的隨身器具", head_name)
            };
            let is_inland = hh_id.starts_with("hh:greypine_post:")
                || doc
                    .households
                    .iter()
                    .find(|h| h.id == *hh_id)
                    .and_then(|h| doc.cells.get(&h.cell_id.to_string()))
                    .is_some_and(|c| !c.is_coast);

            if let Some(actor) = delta.actors.get_mut(aid) {
                actor.birth_year = birth_year;
                actor.household_bond = bond.to_string();
                actor.role = new_role.clone();
                if actor.appearance.starts_with("常年在島上從事") || actor.appearance.contains('・')
                {
                    actor.appearance =
                        natural_age_appearance(&race, &new_role, birth_year, seed, is_inland);
                } else if is_inland {
                    actor.appearance = actor
                        .appearance
                        .replace("海港濕鹹泥印", "山徑濕冷松泥")
                        .replace("百餘年潮汐", "百餘年山嵐雪線");
                }
                actor.ancestor_note_zh = anc_note;
                if actor.bound_weapon.provenance.is_empty()
                    || actor.bound_weapon.provenance.ends_with("慣用隨身傢伙")
                {
                    actor.bound_weapon.provenance = default_tool_prov;
                }
                actor.rev = actor.rev.saturating_add(1);
                actor.dirty = true;
                touched_actors += 1;
                member_profiles.push((aid.clone(), actor.display_name.clone(), bond.to_string()));
            }
        }

        for (aid_a, _, _) in &member_profiles {
            for (aid_b, name_b, bond_b) in &member_profiles {
                if aid_a == aid_b {
                    continue;
                }
                if let Some(actor_a) = delta.actors.get_mut(aid_a) {
                    let rel_desc = if is_shop {
                        format!("同鋪師門（對方為{}）", bond_b)
                    } else {
                        format!("同屋至親（對方為{}，共撐{}門戶）", bond_b, head_name)
                    };
                    actor_a
                        .mind
                        .seed_historical_social_tie(aid_b, name_b, &rel_desc);
                }
            }
        }
    }

    // 2. 將白鷗港主街 4 大店鋪之年輕學徒連回支巷同族原生家庭（打通「主街師門 ↔ 支巷血親」雙線網）
    let wgh_alley_heads: Vec<(String, String, String, String)> = hh_members
        .iter()
        .filter(|(hh_id, _)| {
            hh_id.starts_with("hh:white_gull_harbor:")
                && !hh_id.contains(":shop_")
                && !hh_id.ends_with(":porter")
        })
        .filter_map(|(hh_id, mems)| {
            let hid = mems.first()?;
            let a = delta.actors.get(hid)?;
            Some((
                hh_id.clone(),
                hid.clone(),
                a.display_name.clone(),
                a.race.clone(),
            ))
        })
        .collect();

    for shop_idx in 0..4usize {
        let app_id = format!("res:white_gull_harbor:shop_{shop_idx}_1");
        let master_id = format!("res:white_gull_harbor:shop_{shop_idx}_0");
        let (Some(app_ro), Some(mas_ro)) =
            (delta.actors.get(&app_id), delta.actors.get(&master_id))
        else {
            continue;
        };
        let app_race = app_ro.race.clone();
        let app_name = app_ro.display_name.clone();
        let master_name = mas_ro.display_name.clone();
        let shop_node = mas_ro.home_node.clone();

        let matching_natal: Vec<_> = wgh_alley_heads
            .iter()
            .filter(|(_, _, _, r)| r == &app_race)
            .collect();
        if let Some((natal_hh_id, natal_head_id, natal_head_name, _)) =
            matching_natal.get(shop_idx % matching_natal.len().max(1))
        {
            let ev_id = format!("hist:wgh:apprentice_kin:{shop_idx}");
            let ev_year = 327 + (shop_idx as i16 % 2);
            let f_cred = format!(
                "{}家託人說項，把晚輩{}送進鋪裡跟著{}學手藝，講好三年出師前照料鋪務",
                natal_head_name, app_name, master_name
            );
            let f_debt = format!(
                "家裡長輩{}湊了束脩禮，送我進{}的鋪子拜師學一門安身手藝",
                natal_head_name, master_name
            );
            inject_event_into_actor(
                delta,
                &master_id,
                &ev_id,
                ev_year,
                &shop_node,
                natal_head_id,
                natal_head_name,
                PartyStance::BenefactorOrCreditor,
                EventLayer::MicroBilateral,
                0xA100 + shop_idx as u64,
                &f_cred,
                &f_debt,
                &f_cred,
                false,
            );
            inject_event_into_actor(
                delta,
                &app_id,
                &ev_id,
                ev_year,
                &shop_node,
                natal_head_id,
                natal_head_name,
                PartyStance::BeneficiaryOrDebtor,
                EventLayer::MicroBilateral,
                0xA200 + shop_idx as u64,
                &f_cred,
                &f_debt,
                &f_debt,
                true,
            );
            inject_event_into_actor(
                delta,
                natal_head_id,
                &ev_id,
                ev_year,
                &shop_node,
                &master_id,
                &master_name,
                PartyStance::BeneficiaryOrDebtor,
                EventLayer::MicroBilateral,
                0xA300 + shop_idx as u64,
                &f_cred,
                &format!(
                    "我們把自家晚輩{}託付給主街的{}學藝，盼他在港裡站穩腳跟",
                    app_name, master_name
                ),
                &f_cred,
                false,
            );
            if let Some(natal_mems) = hh_members.get(natal_hh_id).cloned() {
                for nm_id in &natal_mems {
                    let nm_name = delta
                        .actors
                        .get(nm_id)
                        .map_or_else(String::new, |a| a.display_name.clone());
                    if let Some(nm_mut) = delta.actors.get_mut(nm_id) {
                        nm_mut.mind.seed_historical_social_tie(
                            &app_id,
                            &app_name,
                            &format!("自家送去主街{}鋪裡學藝的晚輩", master_name),
                        );
                    }
                    if !nm_name.is_empty()
                        && let Some(app_mut) = delta.actors.get_mut(&app_id)
                    {
                        app_mut.mind.seed_historical_social_tie(
                            nm_id,
                            &nm_name,
                            "住在支巷老家的同門血親",
                        );
                    }
                }
            }
        }
    }

    // 3. 第二步：逆推白鷗港 6 棟待售空屋之唯一前塵（船疫封門 4 棟 ＋ 二成關稅斷商 2 棟）
    for (v_idx, vh) in WHITE_GULL_VACANT_HOUSES.iter().enumerate() {
        let (shop_name, link_is_porter) = delta
            .actors
            .get(vh.linked_shop_actor_id)
            .map_or(("街口掌櫃".to_string(), false), |a| {
                (a.display_name.clone(), a.household_id.ends_with(":porter"))
            });
        let house_price = doc
            .households
            .iter()
            .find(|h| h.node_id == vh.house_node_id && h.price_copper > 0)
            .map_or(360, |h| h.price_copper);

        let (f_shop, f_neighbor) = match vh.cause_kind {
            VacantCauseKind::SouthCoastPlague => {
                let shop_side = if link_is_porter {
                    format!(
                        "南岸船疫漫進{}那陣子，{}高燒不退，我們幾個碼頭同鄉湊錢請草藥堂熬湯也沒能留住人，屋子封門後交由市政廳掛牌",
                        vh.house_label, vh.former_head_zh
                    )
                } else {
                    format!(
                        "南岸船疫漫進{}時，{}一家高燒不退，鋪裡連夜送去苦艾湯仍沒能留住人，屋子封門後交由市政廳掛牌",
                        vh.house_label, vh.former_head_zh
                    )
                };
                let nb_side = format!(
                    "隔壁{}的{}一家在那場南岸傳來的熱病裡沒熬過去，夜裡衛兵撒石灰封門的動靜整條巷子都聽見了，屋契後來由市政廳作價 {} 枚銅幣掛牌",
                    vh.house_label, vh.former_head_zh, house_price
                );
                (shop_side, nb_side)
            }
            VacantCauseKind::MainlandTariffBankruptcy => {
                if vh.house_node_id.contains(":cooper_ln:") {
                    (
                        format!(
                            "海峽開徵二成防禦關稅後，住{}的{}船桶訂單銳減，結清往來舊帳後把屋契交還市政廳，託驛馬車載著工具西遷離港",
                            vh.house_label, vh.former_head_zh
                        ),
                        format!(
                            "隔壁{}原本住著做{}的{}，因兩岸二成關稅風波斷了船桶大單，把屋契交市政廳作價 {} 枚銅幣掛牌後搭驛馬車西遷",
                            vh.house_label, vh.former_trade, vh.former_head_zh, house_price
                        ),
                    )
                } else {
                    (
                        format!(
                            "兩岸二成關稅風波卡斷舊商路後，住{}的{}貨船壓在港裡周轉不開，結清往來舊帳後把屋契抵給市政廳搭船離了島",
                            vh.house_label, vh.former_head_zh
                        ),
                        format!(
                            "隔壁{}原本住著做{}的{}，被兩岸二成關稅風波卡斷貨路，變賣家當把屋契交市政廳作價 {} 枚銅幣掛牌後遠走他鄉",
                            vh.house_label, vh.former_trade, vh.former_head_zh, house_price
                        ),
                    )
                }
            }
        };

        inject_event_into_actor(
            delta,
            vh.linked_shop_actor_id,
            vh.event_id,
            vh.solar_year,
            vh.alley_node_id,
            &format!("ghost:{}", vh.house_node_id),
            vh.former_head_zh,
            PartyStance::BenefactorOrCreditor,
            EventLayer::PublicSkeleton,
            0xB100 + v_idx as u64,
            &f_shop,
            &f_neighbor,
            &f_shop,
            v_idx == 0,
        );

        let alley_prefix = format!("{}:h", vh.alley_node_id);
        let mut same_alley_hhs: Vec<(u32, &Vec<String>)> = hh_members
            .values()
            .filter_map(|mems| {
                let first_id = mems.first()?;
                let a = delta.actors.get(first_id)?;
                if a.home_node.starts_with(&alley_prefix) {
                    Some((parse_house_num_suffix(&a.home_node), mems))
                } else {
                    None
                }
            })
            .collect();
        same_alley_hhs.sort_by_key(|(num, _)| std::cmp::Reverse(*num));

        for (_, mems) in same_alley_hhs.into_iter().take(2) {
            for nb_aid in mems {
                inject_event_into_actor(
                    delta,
                    nb_aid,
                    vh.event_id,
                    vh.solar_year,
                    vh.alley_node_id,
                    vh.linked_shop_actor_id,
                    &shop_name,
                    PartyStance::WitnessOrPeer,
                    EventLayer::PublicSkeleton,
                    0xB200 + v_idx as u64,
                    &f_shop,
                    &f_neighbor,
                    &f_neighbor,
                    false,
                );
            }
        }
    }

    // 4. 跨戶日常雙邊因果本末已全面升格由 `causal_arcs::apply_gazetteer_v3_upgrade`（`arc:v3:*` 五鎮生態自洽引擎）統一掌管

    // 5. 第四步：按種族壽命與行業地緣，精準銜接白鷗公史歷史脈絡點（避免自我指涉！）
    for (hh_id, mems) in &hh_members {
        if !hh_id.starts_with("hh:white_gull_harbor:") {
            continue;
        }
        let Some(head_id) = mems.first() else {
            continue;
        };
        let Some(head_ro) = delta.actors.get(head_id) else {
            continue;
        };
        let race = head_ro.race.clone();
        let role = head_ro.role.clone();
        let home_node = head_ro.home_node.clone();

        if race == "矮人" || home_node.contains("iron_anvil_ln") || home_node.contains("bal_forge")
        {
            for aid in mems {
                let stance = if race == "矮人" {
                    PartyStance::BeneficiaryOrDebtor
                } else {
                    PartyStance::WitnessOrPeer
                };
                let (cp_id, cp_name) = if aid == "res:white_gull_harbor:shop_0_0" {
                    ("ghost:north_mine", "北林封坑先輩")
                } else {
                    ("res:white_gull_harbor:shop_0_0", "鐵砧巴爾")
                };
                inject_event_into_actor(
                    delta,
                    aid,
                    "hist:wgh:epoch_238_mine_seal",
                    238,
                    "oi:white_gull_harbor:smithy_st",
                    cp_id,
                    cp_name,
                    stance,
                    EventLayer::PublicSkeleton,
                    0xE238,
                    "當年北林深坑挖穿荒氣地隙驚出吞石巨蛛，矮人匠師打入重鐵楔封死坑口，入城合築北段八角石牆",
                    "北林老礦坑被迫打入鐵楔封死那年，矮人先輩帶著最後幾爐精鐵退入白鷗港闢出鐵匠街，老坑暗門至今留在灰松斷橋下",
                    "鐵匠街與鐵砧巷的石基，正是太陽曆 238 年北林矮人封坑入城時與港方合龍築起的",
                    aid == "res:white_gull_harbor:shop_0_0",
                );
            }
        }

        if race != "矮人"
            && (race == "精靈"
                || hh_id.ends_with(":porter")
                || home_node.contains("old_well_ln")
                || home_node.contains("south_wall_ln"))
        {
            for aid in mems {
                let (stance, cp_id, cp_name) = if race == "獸人" {
                    (
                        PartyStance::BeneficiaryOrDebtor,
                        "ghost:first_settlers",
                        "東灣開港船隊",
                    )
                } else if race == "精靈" {
                    (
                        PartyStance::WitnessOrPeer,
                        "ghost:wild_clan",
                        "灰松林原住散部",
                    )
                } else {
                    (
                        PartyStance::BenefactorOrCreditor,
                        "ghost:wild_clan",
                        "灰松林原住散部",
                    )
                };
                inject_event_into_actor(
                    delta,
                    aid,
                    "hist:wgh:epoch_123_founding",
                    123,
                    "oi:white_gull_harbor:dock",
                    cp_id,
                    cp_name,
                    stance,
                    EventLayer::PublicSkeleton,
                    0xE123,
                    "先遣船隊在東灣拋錨平荒立寨，靠幾代人聚起百戶煙火才鎮住海岬荒氣",
                    "當年開港征荒把不肯入城的原住散部逼進北面灰松林深谷，如今林裡的灰耳部族便是那時退入山林的遠支",
                    "白鷗港最初只是東灣亂石灘上的木埠哨寨，歷經兩百餘年拋石築堤才成今日規模",
                    false,
                );
            }
        }

        if home_node.contains("sail_ln")
            || home_node.contains("copper_ln")
            || home_node.contains("cooper_ln")
            || role.contains("酒館")
            || role.contains("車行")
        {
            for aid in mems {
                let (cp_id, cp_name) = if aid == "res:white_gull_harbor:shop_1_0" {
                    ("ghost:harbor_customs", "港務稅司")
                } else {
                    ("res:white_gull_harbor:shop_1_0", "瑪莎")
                };
                inject_event_into_actor(
                    delta,
                    aid,
                    "hist:wgh:epoch_296_tariff",
                    296,
                    "oi:white_gull_harbor:plaza",
                    cp_id,
                    cp_name,
                    PartyStance::WitnessOrPeer,
                    EventLayer::PublicSkeleton,
                    0xE296,
                    "自從對主陸九邦反課二成關稅、改與羅泉等七邦免稅通商後，港裡黑煤與鹽漬魚全改走熟客航線",
                    "海峽那頭九國加稅、本港反課二成關稅後，跑舊線的小船商撐不住，不少改跑島內馬車貨運或私下摸黑走水路",
                    "港務公庫靠關稅與路權標金養護防波石堤，只是苦了夾在兩岸稅單中間的小行商",
                    false,
                );
            }
        }
    }

    touched_actors
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::grid::outer_island::dialogue_llm::validate_clean_in_world_reply;
    use crate::grid::outer_island::engine::load_static_world_doc;
    use crate::grid::outer_island::mind::EvidenceTier;
    use crate::grid::outer_island::sim_tick::seed_genesis_actors_once;

    #[test]
    fn white_gull_harbor_history_backfill_is_idempotent_monotonic_and_refracts_views() {
        let doc = load_static_world_doc();
        let mut delta = IslandWorldDelta {
            genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
            municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
            silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
            ..IslandWorldDelta::default()
        };
        seed_genesis_actors_once(&doc, &mut delta, false);

        let ep_count_1: usize = delta
            .actors
            .values()
            .map(|a| a.mind.ltm.episodic.len())
            .sum();
        let tp_count_1: usize = delta
            .actors
            .values()
            .map(|a| a.mind.ltm.turning_points.len())
            .sum();

        // 重複執行第 2 次：驗證完全冪等（不翻倍轉折點與記憶，不污染 broken_pacts）
        apply_island_history_backfill(&doc, &mut delta);
        let ep_count_2: usize = delta
            .actors
            .values()
            .map(|a| a.mind.ltm.episodic.len())
            .sum();
        let tp_count_2: usize = delta
            .actors
            .values()
            .map(|a| a.mind.ltm.turning_points.len())
            .sum();
        assert_eq!(ep_count_1, ep_count_2);
        assert_eq!(tp_count_1, tp_count_2);
        assert_eq!(delta.conservation_diff(&doc), 0);

        let mut wgh_residents = 0usize;
        let mut total_episodes = 0usize;
        let mut skeleton_episodes = 0usize;
        let mut by_hh: BTreeMap<String, Vec<(i16, String)>> = BTreeMap::new();

        for a in delta.actors.values() {
            if a.controller_id.is_none() {
                assert!(a.role != "家眷" && a.role != "年輕幫手");
                assert!(!a.appearance.contains("歲"));
                assert!(a.birth_year > 0 && a.birth_year < CURRENT_SOLAR_YEAR);
                // 零自我指涉社交條目、零誤判背約懲罰
                assert!(!a.mind.ltm.social.contains_key(&a.actor_id));
                assert!(
                    validate_clean_in_world_reply(&a.household_bond).is_some(),
                    "unclean bond: {}",
                    a.household_bond
                );
                assert!(
                    validate_clean_in_world_reply(&a.ancestor_note_zh).is_some(),
                    "unclean ancestor_note: {}",
                    a.ancestor_note_zh
                );
                assert!(
                    validate_clean_in_world_reply(&a.bound_weapon.provenance).is_some(),
                    "unclean provenance: {}",
                    a.bound_weapon.provenance
                );
                for soc in a.mind.ltm.social.values() {
                    assert_eq!(soc.broken_pacts, 0);
                    if !soc.notable_note_zh.is_empty() {
                        assert!(
                            validate_clean_in_world_reply(&soc.notable_note_zh).is_some(),
                            "unclean social note: {}",
                            soc.notable_note_zh
                        );
                    }
                }
                by_hh
                    .entry(a.household_id.clone())
                    .or_default()
                    .push((a.birth_year, a.household_bond.clone()));
            }
            if a.household_id.starts_with("hh:white_gull_harbor:") {
                wgh_residents += 1;
                assert!(!a.mind.ltm.episodic.is_empty());
                for ep in &a.mind.ltm.episodic {
                    assert!(!ep.subject_actor_id.starts_with("ghost:"));
                    assert!(
                        validate_clean_in_world_reply(&ep.subjective_zh).is_some(),
                        "unclean episode text: {}",
                        ep.subjective_zh
                    );
                    total_episodes += 1;
                    if ep.event_id.starts_with("hist:wgh:epoch_")
                        || ep.event_id.starts_with("hist:wgh:vacant:")
                    {
                        skeleton_episodes += 1;
                    }
                    if ep.tier == EvidenceTier::FamilyOral {
                        assert!(ep.credence >= 85 && ep.fidelity < ep.credence);
                    }
                }
            }
        }
        assert_eq!(wgh_residents, 263);
        assert!(skeleton_episodes * 5 <= total_episodes);

        // 驗證每戶世代年齡嚴格單調（長老/老輩出生年 < 當家出生年；當家出生年 <= 大輩出生年 - 18；大輩出生年 < 么輩出生年）
        for (hh_id, list) in &by_hh {
            if hh_id.contains(":shop_") || hh_id.ends_with(":porter") {
                continue;
            }
            let head_by = list[0].0;
            for (by, bond) in &list[1..] {
                if bond.contains("長老") || bond.contains("老輩") {
                    assert!(*by <= head_by - 20, "elder not older in {hh_id}");
                } else if bond == "隨業大輩" {
                    assert!(*by >= head_by + 18, "parent-child gap too small in {hh_id}");
                }
            }
            if list.len() >= 4 {
                assert!(
                    list[3].0 > list[2].0,
                    "younger sibling older than elder in {hh_id}"
                );
            }
        }

        let mut views_by_event: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for a in delta.actors.values() {
            for ep in &a.mind.ltm.episodic {
                if ep.event_id.starts_with("arc:v3:white_gull_harbor:") {
                    views_by_event
                        .entry(ep.event_id.clone())
                        .or_default()
                        .insert(ep.subjective_zh.clone());
                }
            }
        }
        let multi_perspective_cnt = views_by_event.values().filter(|s| s.len() >= 2).count();
        assert!(multi_perspective_cnt >= 50);
    }
}
