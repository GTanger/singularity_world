//! 潮汐商船有限班次、動態可否證場景描寫、在場排序與離線避風輔助（與 `live_tide` 協作嚴守單檔 `< 950` 行）。

use std::collections::BTreeSet;

use super::clock_and_lang::SolarClockSnapshot;
use super::household_economy::{count_box_home_rations_only, find_town_tavern_and_keeper};
use super::live_tide::{
    MAX_DAILY_DOCK_CARGO_SHIFTS, derive_tide_clock_details, dock_shift_window_key,
    forge_shift_window_key, is_dock_cargo_tide_open, plan_safe_offline_shelter_path,
    resolve_offline_player_night_target, touch_client_lease,
};
use super::mind::ActiveIntention;
use super::movement::push_breath;
use super::types::{IslandWorldDelta, SpatialNodeDef, UnifiedMindBody, WorldStaticDoc};

/// 組裝完全可被當下世界狀態否證之動態房間描寫（固定地貌幾何 ＋ 時辰光影天候 ＋ 在場營業／船班／存貨實況）。
pub fn compose_live_scene_desc(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    node: &SpatialNodeDef,
    clock: &SolarClockSnapshot,
    base_desc: &str,
) -> String {
    let td = derive_tide_clock_details(clock);
    if node.dungeon {
        return base_desc.to_string();
    }

    let present_count = delta.node_occupants.get(&node.id).map_or(0, |occ| {
        occ.iter()
            .filter(|&aid| {
                delta
                    .actors
                    .get(aid)
                    .is_some_and(|a| a.alive && a.setup_done)
            })
            .count()
    });

    if node.kind == "dock" {
        let geometry = "木石棧橋延伸入海，雙桅帆船與漁舟隨潮水起伏。";
        let weather_bit = match td.weather_zh {
            "微雨" => "細密海雨打濕了棧橋石板，",
            "薄霧" => "微涼海霧籠在防波堤外，",
            "海風" => "帶著鹹腥味的海風吹得纜繩作響，",
            _ => "",
        };
        if clock.is_night {
            return format!(
                "{geometry}{weather_bit}時值{}（{}），防風桅燈在暗潮上微晃，日間搬貨的苦力已散去歇息；棧橋邊港務潮汐木牌寫著：早班商船於晨鐘二響至日高四響靠岸卸貨，午班於午後初鐘至申時二鐘靠岸。",
                clock.bell_label, td.tide_name
            );
        }
        if is_dock_cargo_tide_open(clock) {
            return format!(
                "{geometry}{weather_bit}趁著{}潮水，雙桅商船正靠在棧橋邊放下跳板，待卸的鹽漬貨箱堆在石板上。",
                td.tide_name
            );
        }
        return format!(
            "{geometry}{weather_bit}上一班潮汐貨箱已清點入庫，港務木牌標示下一班商船將於午後初鐘乘潮靠岸。"
        );
    }

    if node.kind == "shop" {
        if node.name.contains("酒館") || node.name.contains("客棧") {
            let (_, keeper_opt) = find_town_tavern_and_keeper(doc, delta, node.cell_id);
            let stock = keeper_opt
                .as_deref()
                .and_then(|kid| delta.actors.get(kid))
                .map_or(0, |k| count_box_home_rations_only(k, &node.id));
            let stock_note = if stock == 0 {
                "今日黑麥熱食與乾糧剛被買空，得等明日農漁戶送來新糧。"
            } else if stock <= 3 {
                "櫃上竹籃裡的黑麥乾糧只剩最後幾份。"
            } else {
                "灶上麥湯與乾糧尚足。"
            };
            let tavern_staff_here = delta.node_occupants.get(&node.id).is_some_and(|occ| {
                occ.iter().any(|oid| {
                    delta.actors.get(oid).is_some_and(|o| {
                        o.alive
                            && o.setup_done
                            && o.node_id == node.id
                            && (o.role == "酒館掌櫃" || o.role == "酒館掌櫃學徒")
                    })
                })
            });
            let staff_note = if !tavern_staff_here && stock > 0 {
                " 櫃台後此刻暫無人看顧，案頭留著黑麥乾糧竹簍與誠實投幣木筒。"
            } else {
                ""
            };
            if td.hour < 6 {
                return format!(
                    "{base_desc} 殘夜大堂懸著一盞守夜風燈，壁爐餘炭微溫可供夜客避寒歇腳；{stock_note}{staff_note}"
                );
            }
            if clock.is_night {
                return format!("{base_desc} 入夜後大堂點起暖黃油燈，{stock_note}{staff_note}");
            }
            return format!("{base_desc} {stock_note}{staff_note}");
        }
        if clock.is_night {
            let shutter = if node.name.contains("鍛鐵") || node.name.contains("鐵匠") {
                "此刻夜深鋪門已落栓打烊，石爐炭火封灰停錘，唯餘屋簷風燈微亮（晨鐘二響開爐）。"
            } else {
                "此刻夜深鋪門木板已落栓打烊，師徒已退入後堂歇息（晨鐘二響開門營業）。"
            };
            return format!("{base_desc} {shutter}");
        }
        let craft_staff_here = delta.node_occupants.get(&node.id).is_some_and(|occ| {
            occ.iter().any(|oid| {
                delta.actors.get(oid).is_some_and(|o| {
                    o.alive
                        && o.setup_done
                        && o.node_id == node.id
                        && (o.work_node == node.id || o.home_node == node.id)
                })
            })
        });
        if !craft_staff_here {
            if node.name.contains("鍛鐵") || node.name.contains("鐵匠") {
                return format!(
                    "{base_desc} 石爐裡壓著微溫灰炭，木牌翻在「暫出片刻」，師徒此刻不在爐邊。"
                );
            }
            if node.name.contains("草藥") {
                return format!("{base_desc} 藥臼旁擱著剛搗碎的苦艾葉，堂內師徒此刻暫時出門片刻。");
            }
        }
    }

    if !node.indoor {
        if clock.is_night && present_count <= 1 {
            return format!(
                "{base_desc} 夜色深沉（{}），石板街巷空寂少人，唯聞遠處海潮與風聲。",
                td.tide_name
            );
        }
        if td.weather_zh == "微雨" {
            return format!("{base_desc} 天空飄著微涼細雨，屋簷滴水落入石縫。");
        }
    }

    base_desc.to_string()
}

/// 清除前一日之班次、步履節流與搜巡／切磋時段鍵，確保 `explored_windows` 跨千日仍維持 $O(1)$ 極小常數體積。
pub fn prune_stale_day_window_keys(set: &mut BTreeSet<String>, today: i64) {
    let day_tag = today.to_string();
    set.retain(|k| {
        if let Some(rest) = k
            .strip_prefix("shift:dock:")
            .or_else(|| k.strip_prefix("shift:forge:"))
            .or_else(|| k.strip_prefix("shift_cap:dock:"))
            .or_else(|| k.strip_prefix("foot_echo:"))
            .or_else(|| k.strip_prefix("foot_in:"))
        {
            return rest.split(':').next() == Some(day_tag.as_str());
        }
        if let Some((_, after_d)) = k.rsplit_once("@d") {
            return after_d.split(':').next() == Some(day_tag.as_str());
        }
        true
    });
}

/// 檢查離鎖工作副本是否真有實際狀態變更（無變更時跳過空轉 PG 交易提交，保留 100% 單一原子過帳權威）。
pub fn delta_has_pending_commit(working: &IslandWorldDelta, prev_settled_bell: i64) -> bool {
    working.last_settled_bell != prev_settled_bell
        || working.extras_dirty
        || !working.actors.dirty_keys().is_empty()
        || !working.dirty_contracts.is_empty()
        || !working.pending_ledger.is_empty()
        || !working.obsolete_actor_ids.is_empty()
        || !working.living_chronicle.pending_outbox.is_empty()
}

/// 玩家主動操作時立即續租並清除離線避風殘留路徑，毫秒級無縫收回身體控制權。
pub fn wake_player_on_explicit_input(delta: &mut IslandWorldDelta, pid: &str, now_unix: i64) {
    let _ = touch_client_lease(pid, now_unix, true);
    if let Some(aid) = delta.resolve_controller(pid)
        && let Some(a) = delta.actors.get_mut(&aid)
    {
        a.last_active_unix = now_unix;
        if !a.route_plan.is_empty() {
            a.route_plan.clear();
            a.mark_mutated();
        }
        if a.mind
            .stm
            .intentions
            .back()
            .is_some_and(|it| matches!(it.kind.as_str(), "shelter_hearth" | "go_home_sleep"))
        {
            a.mind.stm.intentions.pop_back();
            a.mark_mutated();
        }
    }
}

/// 為離線真人角色啟動安全夜間避風走步（零耗費、避開嫌隙仇家）。
pub fn assign_offline_shelter_route_if_needed(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    aid: &str,
    abs_bell: i64,
) {
    let resolved = delta.actors.get(aid).and_then(|a| {
        if a.route_plan.is_empty() {
            resolve_offline_player_night_target(doc, delta, a).and_then(|(k, dst, r)| {
                plan_safe_offline_shelter_path(doc, delta, a, &dst).map(|p| (k, dst, r, p))
            })
        } else {
            None
        }
    });
    if let Some((k, dst, r, full_path)) = resolved
        && full_path.len() > 1
        && let Some(a) = delta.actors.get_mut(aid)
    {
        a.mind.ltm.known_nodes.insert(dst.clone());
        a.route_plan = full_path.into_iter().skip(1).collect();
        if a.mind.stm.intentions.len() >= 4 {
            a.mind.stm.intentions.pop_front();
        }
        a.mind.stm.intentions.push_back(ActiveIntention {
            kind: k.to_string(),
            target_node: dst,
            reason_zh: r,
            created_bell: abs_bell,
        });
        a.mark_mutated();
    }
}

/// 依真實登岸時辰生成純感官初抵敘事（拔除全知透視與跨時辰鐵砧爐火幻覺）。
pub fn compose_arrival_breaths(clock: &SolarClockSnapshot, valid_pack: &str) -> (String, String) {
    let td = derive_tide_clock_details(clock);
    let stamp = format!("{:02}:{:02}", td.hour, td.minute);
    let arrival = if clock.is_night {
        format!(
            "〔{stamp}〕夜霧微涼，渡海商船收帆靠妥白鷗港石棧橋，防風桅燈在暗潮水面上輕晃，四周靜聞海浪拍岸聲。"
        )
    } else {
        format!(
            "〔{stamp}〕海霧散開，雙桅商船乘潮靠上白鷗港石棧橋，潮濕海風裡夾著海鷗鳴叫與港邊隱約的人聲。"
        )
    };
    let pack_line = match valid_pack {
        "forest_hunter" => {
            format!("〔{stamp}〕你緊了緊腰間的舊鹿角獵刀與兩包黑麥乾糧，踏上白鷗港濕潤的石棧橋。")
        }
        "fallen_peddler" => format!(
            "〔{stamp}〕你揣好懷裡寫著你名字的船東抵宿券，拄著包鐵行商手杖走下跳板，券背印有鎮內鏽錨酒館店記。"
        ),
        "deck_sailor" => {
            if clock.is_night {
                format!(
                    "〔{stamp}〕你扛起一卷結實粗麻繩踏上白鷗港碼頭，日間搬貨苦力已散去歇息，棧橋邊立著潮汐船班木牌。"
                )
            } else {
                format!(
                    "〔{stamp}〕你扛起一卷結實粗麻繩踏上白鷗港碼頭，棧橋邊立著港務潮汐船班木牌。"
                )
            }
        }
        _ => format!(
            "〔{stamp}〕你拎著港務舊鍛錘與一張印有鏽錨酒館店記的船東抵宿券，踏上白鷗港石棧橋。"
        ),
    };
    (arrival, pack_line)
}

fn canonical_port_porter_id(delta: &IslandWorldDelta, node_id: &str) -> Option<String> {
    let fixed_id = match node_id {
        "oi:white_gull_harbor:dock" => Some("res:white_gull_harbor:dock_porter"),
        "oi:west_strait_ferry:dock" => Some("res:west_strait_ferry:dock_porter"),
        "oi:gull_tail_port:dock" => Some("res:gull_tail_port:dock_porter"),
        "oi:north_cape_beacon:dock" => Some("res:north_cape_beacon:dock_porter"),
        _ => None,
    };
    if let Some(fid) = fixed_id
        && delta.actors.contains_key(fid)
    {
        return Some(fid.to_string());
    }
    delta
        .actors
        .values()
        .find(|a| a.controller_id.is_none() && a.work_node == node_id && a.role == "碼頭苦力")
        .map(|a| a.actor_id.clone())
}

/// 唯讀檢驗當班潮汐卸貨資格（轉帳成功後方呼叫 `record_dock_shift_claimed` 扣減名額）。
pub fn check_dock_shift_eligible(
    delta: &IslandWorldDelta,
    clock: &SolarClockSnapshot,
    node_id: &str,
    aid: &str,
) -> Result<(), String> {
    if !is_dock_cargo_tide_open(clock) {
        return Err(
            "此時並非商船乘潮靠岸時段，棧橋邊暫無貨箱可卸（早潮為晨鐘二響至日高四響，午潮為午後初鐘至申時二鐘）。"
                .to_string(),
        );
    }
    let shift_key = dock_shift_window_key(clock.game_days, clock.bell_index);
    if let Some(a) = delta.actors.get(aid) {
        if a.fatigue >= 85 {
            return Err("你雙肩痠沉氣喘吁吁，先吃口乾糧或回宿房歇息再扛重活吧。".to_string());
        }
        if a.explored_windows.contains(&shift_key) {
            return Err(
                "這一班靠岸商船的貨箱已搬卸完畢，等下一班潮汐船靠岸再來搭手吧。".to_string(),
            );
        }
    }
    let cap_prefix = format!("shift_cap:dock:{}:{}:", clock.game_days, node_id);
    if let Some(canon_pid) = canonical_port_porter_id(delta, node_id)
        && let Some(p) = delta.actors.get(&canon_pid)
    {
        let used = p
            .explored_windows
            .iter()
            .filter(|k| k.starts_with(&cap_prefix))
            .count();
        if used >= MAX_DAILY_DOCK_CARGO_SHIFTS {
            return Err("今日靠港商船貨艙已全數卸清入庫，待明日早潮商船入港再來吧。".to_string());
        }
    }
    Ok(())
}

pub fn record_dock_shift_claimed(
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    node_id: &str,
    aid: &str,
) {
    let shift_key = dock_shift_window_key(clock.game_days, clock.bell_index);
    let cap_prefix = format!("shift_cap:dock:{}:{}:", clock.game_days, node_id);
    if let Some(canon_pid) = canonical_port_porter_id(delta, node_id)
        && let Some(p) = delta.actors.get_mut(&canon_pid)
    {
        prune_stale_day_window_keys(&mut p.explored_windows, clock.game_days);
        let next_idx = p
            .explored_windows
            .iter()
            .filter(|k| k.starts_with(&cap_prefix))
            .count();
        p.explored_windows.insert(format!("{cap_prefix}{next_idx}"));
        p.mark_mutated();
    }
    if let Some(a) = delta.actors.get_mut(aid) {
        prune_stale_day_window_keys(&mut a.explored_windows, clock.game_days);
        a.explored_windows.insert(shift_key);
        a.mark_mutated();
    }
}

/// 唯讀檢驗打鐵拉風箱資格（轉帳成功後方呼叫 `record_forge_shift_claimed`）。
pub fn check_forge_shift_eligible(
    delta: &IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
) -> Result<(), String> {
    if clock.is_night {
        return Err("夜深鐵匠鋪已封爐落栓打烊，待破曉後晨鐘二響再開爐吧。".to_string());
    }
    let shift_key = forge_shift_window_key(clock.game_days, clock.bell_index);
    if let Some(a) = delta.actors.get(aid) {
        if a.fatigue >= 85 {
            return Err("你手臂發軟，歇口氣再拉風箱才不會失手燙傷。".to_string());
        }
        if a.explored_windows.contains(&shift_key) {
            return Err("這一爐粗鐵胚剛敲打完畢，待師傅添炭燒紅下一爐再搭手吧。".to_string());
        }
    }
    Ok(())
}

pub fn record_forge_shift_claimed(
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
) {
    let shift_key = forge_shift_window_key(clock.game_days, clock.bell_index);
    if let Some(a) = delta.actors.get_mut(aid) {
        prune_stale_day_window_keys(&mut a.explored_windows, clock.game_days);
        a.explored_windows.insert(shift_key);
        a.mark_mutated();
    }
}

/// 檢查酒館櫃上乾糧實存（無掌櫃之純合成測試環境回退為允許）。
pub fn tavern_has_ration_stock(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    cell_id: u32,
    tavern_nid: &str,
) -> bool {
    let (_, keeper_opt) = find_town_tavern_and_keeper(doc, delta, cell_id);
    let Some(kid) = keeper_opt else {
        return true;
    };
    delta
        .actors
        .get(&kid)
        .is_none_or(|k| count_box_home_rations_only(k, tavern_nid) > 0)
}

pub fn check_speaker_ready_to_talk(sp: &UnifiedMindBody) -> Result<(), String> {
    if !sp.setup_done {
        return Err("請先在港務登岸木冊留下你的名號與來歷。".to_string());
    }
    if sp.active_skirmish.is_some() {
        return Err("眼前正與敵手短兵相接，無暇分心攀談！".to_string());
    }
    if sp.active_transit.is_some() {
        return Err("雙馬篷車正在大路上奔馳，需下車後才能與街坊交談。".to_string());
    }
    Ok(())
}

/// 執行潮汐商船搬貨工資結算與在場見證廣播。
pub fn settle_work_dock_action(
    _doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    node_id: &str,
    aid: &str,
    porter_id: Option<&str>,
) -> Result<String, String> {
    check_dock_shift_eligible(delta, clock, node_id, aid)?;
    let paid_by_porter = porter_id.is_some_and(|pid_npc| {
        if delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "work_dock",
            pid_npc,
            aid,
            4,
            "港務碼頭扛貨工資".to_string(),
        ) {
            let na = delta.actors.get(aid).map(|a| a.display_name.clone());
            let nb = delta.actors.get(pid_npc).map(|b| b.display_name.clone());
            if let (Some(na), Some(nb)) = (na, nb) {
                if let Some(a) = delta.actors.get_mut(aid) {
                    a.mind.record_pact_or_trade_evidence(
                        pid_npc,
                        &nb,
                        clock.game_days,
                        true,
                        false,
                        "港務碼頭扛貨工資",
                    );
                    a.mark_mutated();
                }
                if let Some(b) = delta.actors.get_mut(pid_npc) {
                    b.mind.record_pact_or_trade_evidence(
                        aid,
                        &na,
                        clock.game_days,
                        true,
                        false,
                        "港務碼頭扛貨工資",
                    );
                    b.mark_mutated();
                }
            }
            true
        } else {
            false
        }
    });
    if !paid_by_porter
        && !delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "work_dock",
            "municipal_treasury",
            aid,
            4,
            "港務碼頭扛貨工資".to_string(),
        )
    {
        return Err("今日碼頭貨船已卸完，暫無零工可派。".to_string());
    }
    record_dock_shift_claimed(delta, clock, node_id, aid);
    let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
    a.fatigue = (a.fatigue + 12).min(95);
    a.mind.stm.today_goal_progressed = true;
    a.mark_mutated();
    let td = derive_tide_clock_details(clock);
    let msg = "你捲起衣袖趁潮水靠船幫著把幾隻鹽漬海獸皮箱扛上棧橋，領工點給你 4 枚沉甸甸的銅幣。"
        .to_string();
    push_breath(a, format!("〔{:02}:{:02}〕{msg}", td.hour, td.minute));
    Ok(msg)
}

/// 依 $O(1)$ `node_occupants` 空間索引與釘選優先序回傳同節點全體在場人物（零 `doc.residents` 依賴、零 `.take(8)` 截斷！）
pub fn ordered_actors_at_node<'a>(
    delta: &'a IslandWorldDelta,
    viewer: &UnifiedMindBody,
    node: &SpatialNodeDef,
    now_ms: u64,
) -> Vec<&'a UnifiedMindBody> {
    let Some(occ) = delta.node_occupants.get(&node.id) else {
        return Vec::new();
    };

    let mut list: Vec<(u8, &'a UnifiedMindBody)> = occ
        .iter()
        .filter(|&aid| aid != &viewer.actor_id)
        .filter_map(|aid| delta.actors.get(aid))
        .filter(|other| other.alive && other.setup_done && other.node_id == node.id)
        .map(|other| {
            let conversing = now_ms > 0 && other.conversing_until_ms > now_ms;
            let is_duty_role = matches!(
                other.role.as_str(),
                "港務記籌手"
                    | "碼頭苦力"
                    | "酒館掌櫃"
                    | "酒館掌櫃學徒"
                    | "鍛鐵匠"
                    | "鍛鐵匠學徒"
                    | "草藥師"
                    | "草藥師學徒"
                    | "車行掌櫃"
                    | "行腳商旅"
                    | "巡城衛兵"
                    | "關哨衛兵"
                    | "檢疫衛兵"
                    | "木匠"
            );
            let at_post = other.controller_id.is_none()
                && ((other.work_node == node.id
                    || (node.kind == "shop" && other.home_node == node.id))
                    && (matches!(
                        node.kind.as_str(),
                        "shop" | "dock" | "gate" | "village_gate" | "plaza"
                    ) || is_duty_role));
            let acquainted = viewer.is_acquainted_with(&other.actor_id)
                || other.is_acquainted_with(&viewer.actor_id);
            let urgent_peer = conversing
                || other.active_skirmish.is_some()
                || viewer
                    .mind
                    .stm
                    .pending_trade_offer
                    .as_ref()
                    .is_some_and(|o| o.seller_id == other.actor_id)
                || viewer
                    .mind
                    .stm
                    .pending_spar_from
                    .as_ref()
                    .is_some_and(|(fid, _, _)| fid == &other.actor_id);
            let rank = if urgent_peer {
                0
            } else if at_post {
                1
            } else if acquainted || other.controller_id.is_some() {
                2
            } else {
                3
            };
            (rank, other)
        })
        .collect();

    list.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.actor_id.cmp(&b.1.actor_id)));
    list.into_iter().map(|(_, other)| other).collect()
}
