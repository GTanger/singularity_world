//! 第一刀「嘴動→身動」對話開場與世界後果裁決引擎（獨立自 `living_presence.rs` 以守單檔體積預算）。
//!
//! 嚴格遵守 `DESIGN_KEYWORDS.md:64`：LLM 僅負責語言潤飾，一切指路、邀約改路徑、酒館守恆請客、
//! 情報寫入關切與挑釁翻臉，皆由本模組以純 Rust 依真實性格、關係與銅幣守恆裁決落地。

use super::appraisal::ConcernCause;
use super::clock_and_lang::SolarClockSnapshot;
use super::dialogue_llm::NpcDialogueInput;
use super::mind::{ActiveIntention, EventPublicity, EvidenceTier};
use super::movement::shortest_walk_path;
use super::persona::{DialogueHardStance, ValueAxis, derive_innate_persona};
use super::sim_tick::broadcast_scene_event;
use super::types::{IslandWorldDelta, WorldStaticDoc};

fn short_node_label(name: &str) -> &str {
    name.split('・').next_back().unwrap_or(name)
}

fn sanitize_spoken_memory_snippet(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.contains('〔')
        || trimmed.contains('〕')
        || trimmed.contains("你只勉強")
        || trimmed.contains("低聲說了一串話")
    {
        return None;
    }
    let no_tag = trimmed
        .split("（暗記")
        .next()
        .unwrap_or(trimmed)
        .split("（牢記")
        .next()
        .unwrap_or(trimmed)
        .trim()
        .trim_end_matches('。');
    if no_tag.chars().count() < 6 || no_tag.chars().count() > 54 {
        return None;
    }
    Some(no_tag.to_string())
}

/// 取代早期 `dialogue_llm.rs` 裡 5 種職業固定模板：依人物此刻真實記憶、處境、家族與場地動態生成有血有肉的開場應答。
pub fn build_grounded_opening_reply(input: &NpcDialogueInput) -> String {
    match input.stance_verdict.stance {
        DialogueHardStance::RefuseColdly => {
            return format!(
                "「咱們先前話不投機，我手頭還有正事要忙，請自便吧。」{}（{}）頭也沒抬地冷淡應道。",
                input.npc.name, input.stance_verdict.posture_zh
            );
        }
        DialogueHardStance::HaggleFirm if input.used_pressure_tactic => {
            return format!(
                "「港裡做營生向來明碼實價，一文工本也少不得；手頭緊就去碼頭或爐邊扛活再來。」{}按住手邊的{}沉聲道。",
                input.npc.name, input.npc_tool_name
            );
        }
        _ => {}
    }

    let greet_head = if input.acquainted {
        format!("「又碰上了，{}！", input.player_name)
    } else {
        "「".to_string()
    };

    if input.stance_verdict.stance != DialogueHardStance::GuardedEvade
        && let Some(clean_mem) = input
            .relevant_memories_zh
            .iter()
            .find_map(|m| sanitize_spoken_memory_snippet(m))
    {
        return format!(
            "{}這會兒正值{}，剛才心裡還在惦記著：{}。你找我有什麼事，不妨直說。」{}（{}）停下手邊的{}望向你。",
            greet_head,
            input.bell_label,
            clean_mem,
            input.npc.name,
            input.stance_verdict.posture_zh,
            input.npc_tool_name.trim()
        );
    }

    if let Some((last_p, _)) = input.recent_talks_with_player.last() {
        let short_last: String = last_p.chars().take(18).collect();
        return format!(
            "{}剛才你提起「{}」，我這會兒還在{}這邊忙著{}，你可還有下文？」{}抬眼打量你。",
            greet_head,
            short_last,
            short_node_label(&input.node_name),
            input.npc.role,
            input.npc.name
        );
    }

    format!(
        "{}這會兒正值{}，{}裡大夥兒各顧各的生計；我正拿著{}料理手頭的{}活計。你是打聽行情、問路，還是有事要托付？」{}（{}）朝你點了點頭。",
        greet_head,
        input.bell_label,
        short_node_label(&input.node_name),
        input.npc_tool_name.trim(),
        input.npc.role,
        input.npc.name,
        input.stance_verdict.posture_zh
    )
}

/// 「嘴動→身動」：把玩家說出口的話由 Rust 依真實性格、關係與守恆規則裁決為具體世界後果。
pub fn apply_dialogue_world_consequence(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    speaker_aid: &str,
    target_aid: &str,
    node_id: &str,
    player_said: &str,
) -> Option<String> {
    let said = player_said.trim();
    if said.is_empty() {
        return None;
    }
    let abs_bell = clock.game_days * 12 + i64::from(clock.bell_index);
    let cur_cell = doc.nodes.get(node_id).map_or(4616, |n| n.cell_id);
    let (sp_name, sp_copper) = {
        let sp = delta.actors.get(speaker_aid)?;
        (sp.display_name.clone(), sp.copper)
    };
    let (tg_name, tg_warmth, tg_known_nodes, tg_innate) = {
        let tg = delta.actors.get(target_aid)?;
        let w = tg
            .mind
            .ltm
            .social
            .get(speaker_aid)
            .map_or(0, |s| s.warmth);
        (
            tg.display_name.clone(),
            w,
            tg.mind.ltm.known_nodes.clone(),
            derive_innate_persona(tg.soul_seed.max(1)),
        )
    };

    // 1. 挑釁／威脅：真實降好感、拉升怒氣並收攤結束交談租約
    if ["滾開", "滾蛋", "廢物", "揍你", "殺了", "搶你"]
        .iter()
        .any(|k| said.contains(k))
    {
        if let Some(tg) = delta.actors.get_mut(target_aid) {
            let s = tg.mind.ltm.social.entry(speaker_aid.to_string()).or_default();
            s.warmth = (s.warmth - 18).clamp(-100, 100);
            tg.mind.stm.affect.anger = tg.mind.stm.affect.anger.saturating_add(28).min(100);
            tg.conversing_until_ms = 0;
            let _ = tg.mind.appraise_event(
                &tg_innate,
                ConcernCause::RivalEncounter,
                Some(speaker_aid),
                Some(ValueAxis::Autonomy),
                52,
                abs_bell,
            );
            tg.mark_mutated();
        }
        let note = format!("【因果】{}臉色鐵青、轉身收起話頭，將這番羞辱記上了心頭。", tg_name);
        broadcast_scene_event(
            doc,
            delta,
            clock,
            node_id,
            speaker_aid,
            &sp_name,
            EventPublicity::PublicLoud,
            Some("harbor_trade"),
            40,
            18,
            ValueAxis::Autonomy,
            "dialogue_fallout",
            &format!("{}出言衝撞{}，兩人當街不歡而散", sp_name, tg_name),
        );
        return Some(note);
    }

    // 2. 請客／致歉／結交：在酒館真實扣 2 銅付給掌櫃請對方喝一杯，或真誠致歉緩和關係
    if ["請你喝", "請吃", "這杯算我的", "交個朋友", "抱歉", "對不住", "失禮"]
        .iter()
        .any(|k| said.contains(k))
    {
        let in_tavern = doc
            .nodes
            .get(node_id)
            .is_some_and(|n| n.kind == "shop" && n.name.contains("酒館"));
        let wants_treat =
            said.contains("請你喝") || said.contains("請吃") || said.contains("這杯算我的");
        if wants_treat && in_tavern && sp_copper >= 2 {
            let keeper_opt = delta
                .node_occupants
                .get(node_id)
                .and_then(|occ| {
                    occ.iter().find(|&oid| {
                        oid != speaker_aid
                            && delta.actors.get(oid).is_some_and(|a| {
                                a.alive && a.controller_id.is_none() && a.role.contains("酒館")
                            })
                    })
                })
                .cloned();
            if let Some(kid) = keeper_opt
                && delta.transfer_copper(
                    clock.game_days,
                    clock.bell_index,
                    "dialogue_treat",
                    speaker_aid,
                    &kid,
                    2,
                    format!("{}請{}喝一杯", sp_name, tg_name),
                )
            {
                if let Some(tg) = delta.actors.get_mut(target_aid) {
                    let s = tg.mind.ltm.social.entry(speaker_aid.to_string()).or_default();
                    s.acquainted = true;
                    s.familiarity = s.familiarity.saturating_add(14).min(100);
                    s.warmth = (s.warmth + 14).clamp(-100, 100);
                    tg.mind.ingest_historical_episode(
                        &format!("treat:{}:{}:{}", clock.game_days, speaker_aid, target_aid),
                        clock.game_days,
                        node_id,
                        speaker_aid,
                        &sp_name,
                        EvidenceTier::DirectWitness,
                        98,
                        95,
                        26,
                        1,
                        format!("{}在酒館掏錢請自己喝了一杯熱麥酒", sp_name),
                    );
                    tg.mark_mutated();
                }
                if let Some(sp) = delta.actors.get_mut(speaker_aid) {
                    let s = sp.mind.ltm.social.entry(target_aid.to_string()).or_default();
                    s.acquainted = true;
                    s.familiarity = s.familiarity.saturating_add(14).min(100);
                    s.warmth = (s.warmth + 14).clamp(-100, 100);
                    sp.mark_mutated();
                }
                return Some(format!(
                    "【因果】你掏出 2 枚銅幣請{}喝了杯溫麥酒，對方神色明顯熱絡起來，兩人已算熟識。",
                    tg_name
                ));
            }
        }
        if let Some(tg) = delta.actors.get_mut(target_aid) {
            let s = tg.mind.ltm.social.entry(speaker_aid.to_string()).or_default();
            s.acquainted = true;
            s.familiarity = s.familiarity.saturating_add(8).min(100);
            s.warmth = (s.warmth + 8).clamp(-100, 100);
            tg.mind.stm.affect.anger = tg.mind.stm.affect.anger.saturating_sub(15);
            tg.mark_mutated();
        }
        if let Some(sp) = delta.actors.get_mut(speaker_aid) {
            let s = sp.mind.ltm.social.entry(target_aid.to_string()).or_default();
            s.acquainted = true;
            s.familiarity = s.familiarity.saturating_add(8).min(100);
            s.warmth = (s.warmth + 8).clamp(-100, 100);
            sp.mark_mutated();
        }
        return Some(format!("【因果】{}收下你的善意，雙方語氣緩和了幾分，已互記名號。", tg_name));
    }

    // 3. 問路／請對方指點地標：優先匹配同城提到且玩家尚未踏足之真實 `known_nodes`
    if ["怎麼走", "在哪", "帶路", "往哪", "路線", "指引"]
        .iter()
        .any(|k| said.contains(k))
    {
        if tg_warmth <= -18 {
            return Some(format!("【因果】{}與你本有芥蒂，只冷冷擺手不肯替你指路。", tg_name));
        }
        let sp_known = delta
            .actors
            .get(speaker_aid)
            .map(|s| s.mind.ltm.known_nodes.clone())
            .unwrap_or_default();
        let name_matches = |nd: &super::types::SpatialNodeDef| {
            let lbl = short_node_label(&nd.name);
            !nd.indoor && !nd.dungeon && lbl.chars().count() >= 2 && said.contains(lbl)
        };
        let matched = tg_known_nodes
            .iter()
            .filter_map(|nid| doc.nodes.get(nid))
            .find(|nd| nd.cell_id == cur_cell && !sp_known.contains(&nd.id) && name_matches(nd))
            .or_else(|| {
                tg_known_nodes
                    .iter()
                    .filter_map(|nid| doc.nodes.get(nid))
                    .find(|nd| nd.cell_id == cur_cell && name_matches(nd))
            })
            .or_else(|| {
                tg_known_nodes
                    .iter()
                    .filter_map(|nid| doc.nodes.get(nid))
                    .find(|nd| name_matches(nd))
            })
            .or_else(|| {
                tg_known_nodes
                    .iter()
                    .filter_map(|nid| doc.nodes.get(nid))
                    .find(|nd| {
                        !nd.indoor
                            && !nd.dungeon
                            && !sp_known.contains(&nd.id)
                            && matches!(nd.kind.as_str(), "plaza" | "dock" | "gate" | "shop" | "bridge")
                    })
            });
        if let Some(target_nd) = matched {
            let shared_id = target_nd.id.clone();
            let shared_name = target_nd.name.clone();
            let shared_cell = target_nd.cell_id;
            if let Some(sp) = delta.actors.get_mut(speaker_aid) {
                sp.mind.ltm.known_nodes.insert(shared_id.clone());
                sp.mind.ltm.known_cells.insert(shared_cell);
                sp.mind.ingest_historical_episode(
                    &format!("guide:{}:{}:{}", clock.game_days, target_aid, shared_id),
                    clock.game_days,
                    node_id,
                    target_aid,
                    &tg_name,
                    EvidenceTier::TavernHearsay,
                    72,
                    82,
                    20,
                    1,
                    format!("{}指點了前往【{}】的門徑方向", tg_name, shared_name),
                );
                sp.mark_mutated();
            }
            return Some(format!(
                "【因果】{}憑親身走過的路徑，替你指明了【{}】的去路，已記進你的腦海地圖。",
                tg_name, shared_name
            ));
        }
    }

    // 4. 邀約對方動身：依交情真實改寫對方 `route_plan`（優先同城提到地標）
    if ["跟我來", "一起去", "同去", "走一趟", "去看看"]
        .iter()
        .any(|k| said.contains(k))
    {
        if tg_warmth < 10 {
            return Some(format!(
                "【因果】{}婉拒了你的邀約——你們交情尚淺，他暫時不願放下手邊活計跟你走。",
                tg_name
            ));
        }
        let match_dst = |nd: &super::types::SpatialNodeDef| {
            let lbl = short_node_label(&nd.name);
            nd.id != node_id && lbl.chars().count() >= 2 && said.contains(lbl)
        };
        let dest_opt = tg_known_nodes
            .iter()
            .filter_map(|nid| doc.nodes.get(nid))
            .find(|nd| nd.cell_id == cur_cell && match_dst(nd))
            .or_else(|| {
                tg_known_nodes
                    .iter()
                    .filter_map(|nid| doc.nodes.get(nid))
                    .find(|nd| match_dst(nd))
            })
            .map(|nd| (nd.id.clone(), nd.name.clone()));
        if let Some((dst_id, dst_name)) = dest_opt
            && let Some(path) = shortest_walk_path(doc, node_id, &dst_id, &tg_known_nodes)
            && path.len() >= 2
        {
            if let Some(tg) = delta.actors.get_mut(target_aid) {
                tg.route_plan = path.into_iter().skip(1).collect();
                tg.conversing_until_ms = 0;
                if tg.mind.stm.intentions.len() >= 4 {
                    tg.mind.stm.intentions.pop_front();
                }
                tg.mind.stm.intentions.push_back(ActiveIntention {
                    kind: "visit_confidant".to_string(),
                    target_node: dst_id,
                    reason_zh: format!("應{}之邀前往{}", sp_name, short_node_label(&dst_name)),
                    created_bell: abs_bell,
                });
                tg.mark_mutated();
            }
            return Some(format!(
                "【因果】{}點頭答應了你的邀約，收拾妥當轉身先往【{}】邁步而去！",
                tg_name,
                short_node_label(&dst_name)
            ));
        }
    }

    // 5. 通報真實變故／商情：寫入對方記憶與關切
    let intel_cause = if said.contains("缺煤") || said.contains("熄火") || said.contains("缺藥") {
        Some((ConcernCause::LivelihoodShortfall, "聽聞鋪倉缺料行情"))
    } else if said.contains("斷橋")
        || said.contains("哥布林")
        || said.contains("巨蛛")
        || said.contains("試煉坑")
    {
        Some((ConcernCause::CrossTownYearning, "聽聞鎮外險情與異聞"))
    } else if said.contains("欠債") || said.contains("欠帳") || said.contains("賴帳") {
        Some((ConcernCause::DebtPressured, "聽聞街坊債帳風聲"))
    } else {
        None
    };
    if let Some((cause, tag)) = intel_cause {
        let short_said: String = said.chars().take(36).collect();
        if let Some(tg) = delta.actors.get_mut(target_aid) {
            tg.mind.ingest_historical_episode(
                &format!("intel:{}:{}:{}", abs_bell, speaker_aid, target_aid),
                clock.game_days,
                node_id,
                speaker_aid,
                &sp_name,
                EvidenceTier::TavernHearsay,
                68,
                75,
                28,
                1,
                format!("{}當面告知：{}", sp_name, short_said),
            );
            let _ = tg
                .mind
                .appraise_event(&tg_innate, cause, Some(speaker_aid), None, 38, abs_bell);
            let s = tg.mind.ltm.social.entry(speaker_aid.to_string()).or_default();
            s.acquainted = true;
            s.warmth = (s.warmth + 4).clamp(-100, 100);
            tg.mark_mutated();
        }
        return Some(format!(
            "【因果】{}神色一動，將你剛通報的{}記進心裡，後續盤算恐會因此生變。",
            tg_name, tag
        ));
    }

    None
}
