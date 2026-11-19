//! Route B 活態過招引擎：真實 LLM 角色扮演、街坊公開目擊共振、四大人情槓桿與動態機鋒選項。
//!
//! 鐵律：
//! 1. 所有角色對話事實（執念、稜角、器物、債權、熟客契、目擊八卦）100% 鎖定 `SocialWorldState` 真實帳本，嚴禁捏造未發生事實。
//! 2. 嚴守鎂幣零和守恆：所有周旋／清帳／請食動作絕不憑空增減 `resident.magnesium`，債務化解透過債權展延／代保／履歷移轉落盤。
//! 3. LLM 優先走玩家個人 OpenRouter Key 或伺服器 `PLAYER_TALK_API_*`，8 秒逾時自動平滑回退至語境化確定性口語。

use std::collections::HashMap;
use std::sync::LazyLock;

use parking_lot::RwLock;
use serde::Serialize;

use super::person::{BeliefKind, ConductEdge, CoreObsession, SocialResident, SubjectiveBelief};
use super::relations::{
    GrantedRight, RelationSocket, SharedHistoryEntry, SharedHistoryKind, canonical_pair,
};
use super::sim::SocialWorldState;
use crate::grid::zhulin::spatial_nodes::resolve_spatial_node_meta;

/// 角色出場戲劇鉤子（最多 2 條，完全去數值化）。
#[derive(Debug, Clone, Serialize)]
pub struct ResidentHook {
    pub label: String,
    /// `info` | `warn` | `warm`
    pub tone: &'static str,
}

/// 對話舞台上方角色隨身檔案條。
#[derive(Debug, Clone, Serialize)]
pub struct ResidentDossierView {
    pub obsession_label: &'static str,
    pub edge_label: &'static str,
    pub body_vibe: &'static str,
    pub vessel_summary: String,
    pub relation_summary: String,
}

/// 過招結果擴充欄位（附加回傳至 `SocialInteractResult`）。
#[derive(Debug, Clone)]
pub struct DialogueOutcome {
    pub narrative: String,
    pub witness_line: Option<String>,
    pub tactical_choices: Vec<String>,
    pub actions: Vec<String>,
    pub suggested_action: String,
    pub mutated_world: bool,
}

/// 每位 `(player_id, resident_id)` 的近期對話記憶（保留最近 6 輪，讓多輪過招有上下文連貫）。
static DIALOGUE_MEMORY: LazyLock<RwLock<HashMap<(String, String), Vec<(String, String)>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

fn push_dialogue_memory(player_id: &str, resident_id: &str, player_line: &str, npc_line: &str) {
    let key = (player_id.to_string(), resident_id.to_string());
    let mut guard = DIALOGUE_MEMORY.write();
    let hist = guard.entry(key).or_default();
    hist.push((player_line.to_string(), npc_line.to_string()));
    if hist.len() > 6 {
        let excess = hist.len() - 6;
        hist.drain(0..excess);
    }
}

fn recent_dialogue_memory(player_id: &str, resident_id: &str) -> Vec<(String, String)> {
    let key = (player_id.to_string(), resident_id.to_string());
    DIALOGUE_MEMORY
        .read()
        .get(&key)
        .cloned()
        .unwrap_or_default()
}

/// 為同格居民計算最多 2 條真實戲劇鉤子（供 UI 出場卡片顯示）。
pub fn build_resident_hooks(
    state: &SocialWorldState,
    r: &SocialResident,
    viewer_player_id: &str,
) -> Vec<ResidentHook> {
    let mut hooks = Vec::new();
    let player_edge = state.ledger.get(viewer_player_id, &r.id);

    if let Some(edge) = player_edge {
        if edge
            .sockets
            .iter()
            .any(|s| matches!(s, RelationSocket::ExpeditionBond { .. }))
        {
            hooks.push(ResidentHook {
                label: "欠你人情".to_string(),
                tone: "warm",
            });
        } else if edge
            .sockets
            .iter()
            .any(|s| matches!(s, RelationSocket::SupplyPact { .. }))
        {
            hooks.push(ResidentHook {
                label: "熟客・記你手路".to_string(),
                tone: "warm",
            });
        }
    }

    let (owed_out, owed_in) = state.ledger.debt_summary_of(&r.id);
    if owed_out > 0 && hooks.len() < 2 {
        hooks.push(ResidentHook {
            label: "欠單在身".to_string(),
            tone: "warn",
        });
    } else if owed_in > 0 && hooks.len() < 2 {
        hooks.push(ResidentHook {
            label: "街坊債主".to_string(),
            tone: "info",
        });
    }

    if hooks.len() < 2 {
        let has_rivalry = state.ledger.edges.values().any(|e| {
            (e.a == r.id || e.b == r.id)
                && e.sockets
                    .iter()
                    .any(|s| matches!(s, RelationSocket::TradeRivalry { .. }))
        });
        if has_rivalry {
            hooks.push(ResidentHook {
                label: "同行有隙".to_string(),
                tone: "warn",
            });
        }
    }

    if hooks.len() < 2 && r.deficit.hunger >= 0.58 {
        hooks.push(ResidentHook {
            label: "略顯飢乏".to_string(),
            tone: "info",
        });
    }

    if hooks.len() < 2 && matches!(r.temperament.edge, ConductEdge::LooseTongued) {
        hooks.push(ResidentHook {
            label: "消息靈通".to_string(),
            tone: "info",
        });
    }

    hooks
}

/// 組合角色對話舞台上方檔案條。
pub fn build_resident_dossier(
    state: &SocialWorldState,
    r: &SocialResident,
    viewer_player_id: &str,
) -> ResidentDossierView {
    let vessel_summary = r
        .vessels
        .first()
        .map(|v| v.inspect_line())
        .unwrap_or_else(|| "身無長物".to_string());

    let relation_summary = if let Some(edge) = state.ledger.get(viewer_player_id, &r.id) {
        if edge
            .sockets
            .iter()
            .any(|s| matches!(s, RelationSocket::ExpeditionBond { .. }))
        {
            "曾受你周旋相助，對你另眼相看".to_string()
        } else if edge
            .sockets
            .iter()
            .any(|s| matches!(s, RelationSocket::SupplyPact { .. }))
        {
            "與你議過熟客價，願先留貨報訊".to_string()
        } else if !edge.ledger.is_empty() {
            "先前與你打過交道，留有印象".to_string()
        } else {
            "初見街坊，彼此尚在試探".to_string()
        }
    } else {
        "初見街坊，彼此尚在試探".to_string()
    };

    ResidentDossierView {
        obsession_label: r.temperament.obsession.label_zh(),
        edge_label: r.temperament.edge.label_zh(),
        body_vibe: r.deficit.body_vibe_zh(),
        vessel_summary,
        relation_summary,
    }
}

/// 依據居民當下真實狀態與所處樓層，動態生成 4 個機鋒選項（格式：`action_code|meta|label`）。
pub fn build_tactical_choices(
    state: &SocialWorldState,
    r: &SocialResident,
    viewer_player_id: &str,
    player_gz: i16,
) -> Vec<String> {
    let mut choices = Vec::with_capacity(4);
    let has_pact = state
        .ledger
        .get(viewer_player_id, &r.id)
        .is_some_and(|e| {
            e.sockets
                .iter()
                .any(|s| matches!(s, RelationSocket::SupplyPact { .. }))
        });
    let (owed_out, _) = state.ledger.debt_summary_of(&r.id);

    // 1. 易物議價 / 熟客詢價
    if has_pact {
        choices.push(
            "SocialBargain|熟客詢價|「老交情了，這批貨今天開個實在價來聽聽」".to_string(),
        );
    } else {
        choices.push(
            "SocialBargain|易物議價|「我手頭有鮮貨，先開個熟客價來聽聽」".to_string(),
        );
    }

    // 2. 請食套話（飢乏或愛傳話時特別有效）
    if r.deficit.hunger >= 0.45 {
        choices.push("SocialTreat|請食套話|「熱湯算我的，坐下來把這條街的底細細說來聽」".to_string());
    } else {
        choices.push("SocialTreat|請茶套話|「請你喝盞涼茶，近來街坊間可有什麼暗流？」".to_string());
    }

    // 3. 債務槓桿（若自身背債可周旋代保，否則打聽街坊債權）
    if owed_out > 0 {
        choices.push(
            "SocialSettleDebt|周旋清帳|「聽說你腰間壓著一筆舊帳？說不定我能替你周旋」".to_string(),
        );
    } else {
        choices.push(
            "SocialDebtInquiry|打聽債權|「這條街上最近誰手頭緊、欠了哪家的帳？」".to_string(),
        );
    }

    // 4. 公開街面避人私談 vs 室內深入問底
    if player_gz == 0 {
        choices.push(
            "SocialPrivateTalk|避人私談|「街面耳目雜，進內室避開旁人再細談」".to_string(),
        );
    } else {
        choices.push(
            "Talk|趁靜問底|「這會兒沒外人旁聽，你把真正顧忌的事直說吧」".to_string(),
        );
    }

    choices
}

/// 供解鎖後執行外部 LLM 請求之唯讀上下文包（避免在 HTTP 期間長時間持有全域寫鎖）。
#[derive(Debug, Clone)]
pub struct PreparedDialogueTurn {
    pub resident_id: String,
    pub resident_name: String,
    pub viewer_player_id: String,
    pub intro: String,
    pub witness_line: Option<String>,
    pub tactical_choices: Vec<String>,
    pub actions: Vec<String>,
    pub suggested_action: String,
    pub mutated_world: bool,
    pub fallback_speech: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub player_utterance: String,
}

/// 步驟 A（持鎖 <0.1ms）：執行確定性人情規則仲裁、公開目擊共振並封裝 LLM 提示詞。
pub fn prepare_resident_dialogue_turn(
    state: &mut SocialWorldState,
    r_idx: usize,
    viewer_player_id: &str,
    pos: (i32, i32, i16),
    action: &str,
    player_input: &str,
    unlocked_name: bool,
) -> PreparedDialogueTurn {
    let (player_gx, player_gy, player_gz) = pos;
    let resident_id = state.residents[r_idx].id.clone();
    let resident_name = state.residents[r_idx].true_name.clone();
    let game_day = state.game_day;
    let space_meta = resolve_spatial_node_meta(player_gx, player_gy, player_gz, "");

    let mut mutated_world = unlocked_name;
    let mut suggested_action = String::new();
    let mut lever_fact_summary = String::new();
    let mut sensitive_topic_triggered = false;

    // ── 1. 先執行確定性世界規則仲裁（Concordia: Declare -> Resolve -> Update）──
    match action {
        "SocialBargain" => {
            sensitive_topic_triggered = true;
            mutated_world = true;
            let unit_mg = state.valves.shoot_unit_mg.max(2);
            let edge = state.ledger.get_or_create_mut(viewer_player_id, &resident_id);
            let mut updated_existing = false;
            for sock in &mut edge.sockets {
                if let RelationSocket::SupplyPact {
                    deliveries,
                    agreed_unit_mg,
                    ..
                } = sock
                {
                    *deliveries = deliveries.saturating_add(1);
                    *agreed_unit_mg = unit_mg;
                    updated_existing = true;
                    break;
                }
            }
            if !updated_existing {
                edge.sockets.push(RelationSocket::SupplyPact {
                    supplier_id: viewer_player_id.to_string(),
                    buyer_id: resident_id.clone(),
                    item_id: "bamboo_shoot".to_string(),
                    agreed_unit_mg: unit_mg,
                    deliveries: 1,
                });
            }
            edge.grant_right(GrantedRight::PriorityReserve {
                holder_id: viewer_player_id.to_string(),
                item_id: "bamboo_shoot".to_string(),
            });
            edge.append_history(SharedHistoryEntry {
                game_day,
                gx: player_gx,
                gy: player_gy,
                kind: SharedHistoryKind::SupplyDelivered {
                    item_id: "bamboo_shoot".to_string(),
                    qty: 1,
                    total_mg: unit_mg,
                },
            });
            state.push_echo(
                game_day,
                player_gx,
                player_gy,
                format!("{resident_name}在{}與來客議定熟客交貨手路。", space_meta.name),
            );
            lever_fact_summary = if updated_existing {
                "雙方剛再次敲定熟客留貨約定，你對這位來客手路更放心。".to_string()
            } else {
                "雙方剛立下鮮貨熟客約定，你答應日後先替對方留貨報實在價。".to_string()
            };
        }
        "SocialTreat" => {
            mutated_world = true;
            let r = &mut state.residents[r_idx];
            let was_hungry = r.deficit.hunger >= 0.45;
            r.deficit.hunger = (r.deficit.hunger - 0.35).max(0.0);
            r.deficit.fatigue = (r.deficit.fatigue - 0.12).max(0.0);
            let edge = state.ledger.get_or_create_mut(viewer_player_id, &resident_id);
            edge.grant_right(GrantedRight::StreetWitnessGuarantee {
                guarantor_id: resident_id.clone(),
                beneficiary_id: viewer_player_id.to_string(),
            });
            let secret_lead = extract_real_secret_lead(state, &resident_id, player_gx, player_gy);
            lever_fact_summary = if was_hungry {
                format!("對方請你吃了一碗熱騰騰的鮮筍湯，你飢意大減、話匣子打開，決定透露一條真實內幕：{secret_lead}")
            } else {
                format!("對方請你喝了一盞涼茶示好，你賣個面子，順口透露一條真實街坊消息：{secret_lead}")
            };
        }
        "SocialDebtInquiry" => {
            sensitive_topic_triggered = true;
            let lead = extract_real_debt_lead(state, &resident_id, player_gx, player_gy);
            lever_fact_summary = format!("對方主動打聽這條街的欠帳與債權，你知道的真實情況是：{lead}");
        }
        "SocialSettleDebt" => {
            sensitive_topic_triggered = true;
            let active_debt = find_primary_debt_of(state, &resident_id);
            if let Some((creditor_id, creditor_name, amount_mg)) = active_debt {
                mutated_world = true;
                // 嚴守鎂幣零和守恆：由玩家出面作保／展延抵銷債務，不憑空增減任何居民現金
                let (a, b) = canonical_pair(&creditor_id, &resident_id);
                let edge_key = format!("{a}::{b}");
                if let Some(debt_edge) = state.ledger.edges.get_mut(&edge_key) {
                    let settle_part = (amount_mg / 2).max(1);
                    for sock in &mut debt_edge.sockets {
                        if let RelationSocket::Debt {
                            creditor_id: c,
                            debtor_id: d,
                            amount_mg: amt,
                            due_day,
                            ..
                        } = sock
                            && c == &creditor_id
                            && d == &resident_id
                        {
                            *amt = (*amt - settle_part).max(0);
                            *due_day = game_day + 7;
                        }
                    }
                    debt_edge.sockets.retain(|s| {
                        !matches!(s, RelationSocket::Debt { amount_mg: amt, .. } if *amt <= 0)
                    });
                    debt_edge.append_history(SharedHistoryEntry {
                        game_day,
                        gx: player_gx,
                        gy: player_gy,
                        kind: SharedHistoryKind::PaidDebtForOther {
                            payer_id: viewer_player_id.to_string(),
                            amount_mg: settle_part,
                        },
                    });
                }
                let p_edge = state.ledger.get_or_create_mut(viewer_player_id, &resident_id);
                p_edge.grant_right(GrantedRight::CreditExtension {
                    holder_id: viewer_player_id.to_string(),
                    extra_days: 7,
                });
                if !p_edge
                    .sockets
                    .iter()
                    .any(|s| matches!(s, RelationSocket::ExpeditionBond { .. }))
                {
                    p_edge.sockets.push(RelationSocket::ExpeditionBond {
                        trips: 0,
                        saved_count: 1,
                        betrayal_count: 0,
                    });
                }
                p_edge.append_history(SharedHistoryEntry {
                    game_day,
                    gx: player_gx,
                    gy: player_gy,
                    kind: SharedHistoryKind::PaidDebtForOther {
                        payer_id: viewer_player_id.to_string(),
                        amount_mg: (amount_mg / 2).max(1),
                    },
                });
                state.push_echo(
                    game_day,
                    player_gx,
                    player_gy,
                    format!(
                        "有來客在{}替{resident_name}向{creditor_name}出面擔保展期，壓了許久的欠帳鬆了一大口氣。",
                        space_meta.name
                    ),
                );
                lever_fact_summary = format!(
                    "來客剛出面替你向債主{creditor_name}擔保展期並分擔掉大半舊帳壓力，你心裡記下這份實打實的人情。"
                );
            } else {
                lever_fact_summary = "你目前自己手頭並未背著急債，但你確實知道附近有幾戶街坊手頭很緊。".to_string();
            }
        }
        "SocialPrivateTalk" => {
            if player_gz == 0 && (player_gy == 1 || player_gy == -1 || player_gy == -2) {
                suggested_action = "enter_indoor".to_string();
                lever_fact_summary = format!(
                    "來客提議避開街面耳目進內室談；你點頭壓低聲音，示意對方直接推開{}的內門步入裡間。",
                    space_meta.name
                );
            } else if player_gz == 0 {
                lever_fact_summary = "這裡是公開石板街面，你壓低聲音提醒對方：往北側或南側騎樓店面推門進內室，才不怕街坊聽見。".to_string();
            } else {
                lever_fact_summary = "這裡已是室內隔間，四下沒有外人旁聽，你可以放開說話。".to_string();
            }
        }
        _ => {
            let trimmed = player_input.trim();
            if trimmed.contains('欠')
                || trimmed.contains('帳')
                || trimmed.contains('債')
                || trimmed.contains('價')
                || trimmed.contains('吵')
                || trimmed.contains('貨')
                || trimmed.contains('私')
                || trimmed.contains('偷')
            {
                sensitive_topic_triggered = true;
            }
        }
    }

    // ── 2. 公開層（z == 0）真實街坊目擊共振 ──
    let effective_player_utterance = if !player_input.trim().is_empty() {
        player_input.trim().to_string()
    } else {
        default_utterance_for_action(action).to_string()
    };

    let witness_line = if player_gz == 0 && sensitive_topic_triggered {
        maybe_bystander_witness(
            state,
            &resident_id,
            &resident_name,
            player_gx,
            player_gy,
            action,
            &effective_player_utterance,
        )
    } else {
        None
    };

    if witness_line.is_some() {
        mutated_world = true;
    }

    let intro = if unlocked_name {
        format!("對方抬眼打量了你一番，報了名號：「叫我{resident_name}便成。」")
    } else {
        String::new()
    };

    let (fallback_speech, system_prompt, user_prompt) = build_reply_prompts(
        state,
        r_idx,
        viewer_player_id,
        (&space_meta.name, space_meta.privacy_zh),
        &effective_player_utterance,
        &lever_fact_summary,
        action,
    );

    let tactical_choices =
        build_tactical_choices(state, &state.residents[r_idx], viewer_player_id, player_gz);

    PreparedDialogueTurn {
        resident_id,
        resident_name,
        viewer_player_id: viewer_player_id.to_string(),
        intro,
        witness_line,
        tactical_choices,
        actions: vec![
            "Look".to_string(),
            "Talk".to_string(),
            "SocialBargain".to_string(),
            "SocialTreat".to_string(),
            "SocialDebtInquiry".to_string(),
        ],
        suggested_action,
        mutated_world,
        fallback_speech,
        system_prompt,
        user_prompt,
        player_utterance: effective_player_utterance,
    }
}

/// 步驟 B（無鎖執行）：呼叫真實 LLM（逾時／離線自動平滑回退確定性口語）並寫入近期對話記憶。
pub fn finalize_resident_dialogue_unlocked(prepared: PreparedDialogueTurn) -> DialogueOutcome {
    let speech = if cfg!(test) {
        prepared.fallback_speech.clone()
    } else if let Some((endpoint, api_key, model)) =
        resolve_social_llm_target(&prepared.viewer_player_id)
    {
        match crate::ai::talk::call_openai_compatible_completions(
            &endpoint,
            &api_key,
            &model,
            &prepared.system_prompt,
            &prepared.user_prompt,
            8,
            220,
        ) {
            Ok(reply) if !reply.trim().is_empty() => reply.trim().to_string(),
            Ok(_) => prepared.fallback_speech.clone(),
            Err(e) => {
                tracing::debug!(
                    "zhulin::social LLM reply fallback ({}): {e}",
                    prepared.resident_id
                );
                prepared.fallback_speech.clone()
            }
        }
    } else {
        prepared.fallback_speech.clone()
    };

    push_dialogue_memory(
        &prepared.viewer_player_id,
        &prepared.resident_id,
        &prepared.player_utterance,
        &speech,
    );

    let mut narrative = if prepared.intro.is_empty() {
        speech
    } else {
        format!("{}\n{}", prepared.intro, speech)
    };
    if let Some(ref w) = prepared.witness_line {
        narrative.push('\n');
        narrative.push_str(w);
    }

    DialogueOutcome {
        narrative,
        witness_line: prepared.witness_line,
        tactical_choices: prepared.tactical_choices,
        actions: prepared.actions,
        suggested_action: prepared.suggested_action,
        mutated_world: prepared.mutated_world,
    }
}

fn default_utterance_for_action(action: &str) -> &'static str {
    match action {
        "SocialBargain" => "我手頭有鮮貨，先開個熟客價來聽聽。",
        "SocialTreat" => "熱湯算我的，坐下來把這條街的底細細說來聽。",
        "SocialDebtInquiry" => "這條街上最近誰手頭緊、欠了哪家的帳？",
        "SocialSettleDebt" => "聽說你腰間壓著一筆舊帳？說不定我能替你周旋。",
        "SocialPrivateTalk" => "這裡耳目雜，我們進內室避開旁人再細談。",
        _ => "打聲招呼，聊聊近況。",
    }
}

fn build_reply_prompts(
    state: &SocialWorldState,
    r_idx: usize,
    viewer_player_id: &str,
    space_info: (&str, &str),
    player_utterance: &str,
    lever_fact_summary: &str,
    action: &str,
) -> (String, String, String) {
    let (space_name, privacy_zh) = space_info;
    let r = &state.residents[r_idx];
    let fallback = compose_deterministic_speech(state, r, action, player_utterance, lever_fact_summary);

    let (owed_out, owed_in) = state.ledger.debt_summary_of(&r.id);
    let debt_fact = if owed_out > 0 {
        if let Some((_, c_name, _)) = find_primary_debt_of(state, &r.id) {
            format!("你目前欠街坊{c_name}一筆尚未結清的舊帳，心裡頗有壓力。")
        } else {
            "你身上背著一筆舊帳。".to_string()
        }
    } else if owed_in > 0 {
        "你在街坊間有幾筆放出去還沒收回來的欠帳。".to_string()
    } else {
        "你目前手頭帳目還算清白。".to_string()
    };

    let vessel_fact = r
        .vessels
        .first()
        .map(|v| v.inspect_line())
        .unwrap_or_else(|| "隨身沒帶特別器物".to_string());

    let beliefs_fact: Vec<String> = r
        .beliefs
        .iter()
        .rev()
        .take(3)
        .map(|b| render_belief_fact(state, b))
        .collect();
    let beliefs_block = if beliefs_fact.is_empty() {
        "近期沒聽見特別風聲。".to_string()
    } else {
        beliefs_fact.join("；")
    };

    let history_block = recent_dialogue_memory(viewer_player_id, &r.id)
        .into_iter()
        .rev()
        .take(4)
        .rev()
        .map(|(p, n)| format!("來客：{p}\n你：{n}"))
        .collect::<Vec<_>>()
        .join("\n");

    let system_prompt = "你是《奇點世界》沙鹿西麓街鎮裡一位有血有肉的在地居民。\
請嚴格使用自然流暢的台灣正體中文口語直接以第一人稱回覆（2～4 句，最多 120 字）。\
鐵律：1) 絕不跳戲、絕不說自己是 AI、絕不出現系統名詞、屬性數字、座標或簡體字；\
2) 說話語氣必須鮮明貼合你的「核心執念」與「處事稜角」（例如算帳入骨就愛計較價碼、耳根軟愛傳話就忍不住爆街坊八卦、嘴硬心軟就語氣沖但給提示）；\
3) 只引用提供給你的真實街坊姓名與事實，嚴禁捏造未提供的債主或事件；\
4) 直接輸出你的動作微描寫與口語對白（用「」括起說話內容）。".to_string();

    let user_prompt = format!(
        "【你的真實身分】\n姓名：{name}\n所在空間：{space_name}（{privacy_zh}）\n正在做的事：{act}\n身體感受：{body}\n核心執念：{obs}\n處事稜角：{edge}\n隨身器物：{vessel}\n債務實況：{debt}\n你知道的街坊耳聞：{beliefs}\n{lever}{hist}【來客剛剛對你說】\n「{utterance}」\n請直接以{name}的口吻回應：",
        name = r.true_name,
        act = r.current_activity.activity_phrase_zh(),
        body = r.deficit.body_vibe_zh(),
        obs = r.temperament.obsession.label_zh(),
        edge = r.temperament.edge.label_zh(),
        vessel = vessel_fact,
        debt = debt_fact,
        beliefs = beliefs_block,
        lever = if lever_fact_summary.is_empty() {
            String::new()
        } else {
            format!("【剛發生的真實互動結果（必須順著這件事回應）】\n{lever_fact_summary}\n")
        },
        hist = if history_block.is_empty() {
            String::new()
        } else {
            format!("【先前對話紀錄】\n{history_block}\n")
        },
        utterance = player_utterance,
    );

    (fallback, system_prompt, user_prompt)
}

fn resolve_social_llm_target(viewer_player_id: &str) -> Option<(String, String, String)> {
    if let Some(or_key) = crate::ai::get_player_openrouter_key(viewer_player_id) {
        return Some((
            crate::ai::DEFAULT_OPENROUTER_BASE_URL.to_string(),
            or_key,
            crate::ai::DEFAULT_OPENROUTER_TALK_MODEL.to_string(),
        ));
    }
    let base = std::env::var("PLAYER_TALK_API_BASE_URL").ok()?;
    let model = std::env::var("PLAYER_TALK_API_MODEL").ok()?;
    let key = std::env::var("PLAYER_TALK_API_KEY").unwrap_or_default();
    if base.trim().is_empty() || model.trim().is_empty() {
        return None;
    }
    Some((base.trim().to_string(), key.trim().to_string(), model.trim().to_string()))
}

fn maybe_bystander_witness(
    state: &mut SocialWorldState,
    speaker_id: &str,
    speaker_name: &str,
    gx: i32,
    gy: i32,
    action: &str,
    utterance: &str,
) -> Option<String> {
    let bystander_idx = state
        .residents
        .iter()
        .position(|r| r.id != speaker_id && r.gx == gx && r.gy == gy && r.gz == 0)?;
    let b_name = state.residents[bystander_idx].true_name.clone();
    let keyword = if action == "SocialSettleDebt" || utterance.contains('欠') || utterance.contains('帳') || utterance.contains('債') {
        "舊帳"
    } else if action == "SocialBargain" || utterance.contains('價') || utterance.contains('貨') {
        "熟客價"
    } else {
        "風聲"
    };

    let game_day = state.game_day;
    let rumor_text = format!("在街面聽見外來客與{speaker_name}低聲談起{keyword}");
    state.residents[bystander_idx].push_belief(SubjectiveBelief {
        game_day,
        source_actor_id: speaker_id.to_string(),
        kind: BeliefKind::StreetRumor {
            summary: rumor_text,
        },
        confidence: 70,
    });

    Some(format!(
        "〔街坊目擊〕一旁的{b_name}聽見「{keyword}」二字，手頭動作微微一頓，悄悄朝你們這邊瞥了一眼。"
    ))
}

fn find_primary_debt_of(state: &SocialWorldState, debtor_id: &str) -> Option<(String, String, i32)> {
    for edge in state.ledger.edges.values() {
        for sock in &edge.sockets {
            if let RelationSocket::Debt {
                creditor_id,
                debtor_id: d,
                amount_mg,
                ..
            } = sock
                && d == debtor_id
                && *amount_mg > 0
            {
                let c_name = resident_name_or(state, creditor_id, "騎樓掌櫃");
                return Some((creditor_id.clone(), c_name, *amount_mg));
            }
        }
    }
    None
}

fn extract_real_debt_lead(
    state: &SocialWorldState,
    speaker_id: &str,
    gx: i32,
    gy: i32,
) -> String {
    if let Some((_, c_name, _)) = find_primary_debt_of(state, speaker_id) {
        return format!("我自己就還欠著{c_name}一筆舊帳還沒結清");
    }
    // 找同街或鄰近有實際欠帳的真實街坊
    for r in &state.residents {
        if r.id != speaker_id && (r.gx - gx).abs() <= 2 && (r.gy - gy).abs() <= 2 {
            if let Some((_, c_name, _)) = find_primary_debt_of(state, &r.id) {
                let loc = resolve_spatial_node_meta(r.gx, r.gy, r.gz, "").name;
                return format!("聽說{loc}那頭的{}最近手頭緊，欠著{c_name}的帳還沒軋平", r.true_name);
            }
        }
    }
    "這條街上大家多半壓著幾筆貨款來往，真要打聽得找愛傳話的熟面孔".to_string()
}

fn extract_real_secret_lead(
    state: &SocialWorldState,
    speaker_id: &str,
    gx: i32,
    gy: i32,
) -> String {
    if let Some(r) = state.residents.iter().find(|x| x.id == speaker_id)
        && let Some(b) = r.beliefs.last()
    {
        return render_belief_fact(state, b);
    }
    extract_real_debt_lead(state, speaker_id, gx, gy)
}

fn render_belief_fact(state: &SocialWorldState, b: &SubjectiveBelief) -> String {
    match &b.kind {
        BeliefKind::StreetRumor { summary } => summary.clone(),
        BeliefKind::WitnessedDebt {
            debtor_id,
            creditor_id,
        } => {
            let d = resident_name_or(state, debtor_id, "某位街坊");
            let c = resident_name_or(state, creditor_id, "騎樓債主");
            format!("親眼見過{d}與{c}在街口為了舊帳算盤對不上")
        }
        BeliefKind::WitnessedRescue {
            rescuer_id,
            saved_id,
        } => {
            let a = resident_name_or(state, rescuer_id, "山客");
            let b_n = resident_name_or(state, saved_id, "同伴");
            format!("上回林坡濕滑，{a}拉了{b_n}一把")
        }
        BeliefKind::WitnessedTrade { item_label, .. } => {
            format!("這幾天騎樓裡頭{item_label}走貨特別勤")
        }
        BeliefKind::SuspectedPoach { .. } => "東邊林子裡近來有人暗地摸別家做過記號的鮮貨".to_string(),
        BeliefKind::SecretGroveTip { .. } => "雨後東側高坡竹根下常冒嫩筍，熟路人才找得到".to_string(),
    }
}

fn resident_name_or(state: &SocialWorldState, id: &str, fallback: &str) -> String {
    state
        .residents
        .iter()
        .find(|r| r.id == id)
        .map(|r| r.true_name.clone())
        .unwrap_or_else(|| fallback.to_string())
}

fn compose_deterministic_speech(
    state: &SocialWorldState,
    r: &SocialResident,
    action: &str,
    player_input: &str,
    lever_fact: &str,
) -> String {
    match action {
        "SocialBargain" => format!(
            "{}上下打量了你一回，點點頭：「行，既然你肯講實在話，這批鮮貨我先認你這個熟客手路，往後出貨先報價給你留著。」",
            r.true_name
        ),
        "SocialTreat" => {
            let lead = extract_real_secret_lead(state, &r.id, r.gx, r.gy);
            format!(
                "{}接過你遞上的熱湯啜了一口，壓低聲說：「承你這份情，我也不藏著——{lead}。你自個兒心裡有數便成。」",
                r.true_name
            )
        }
        "SocialDebtInquiry" => {
            let lead = extract_real_debt_lead(state, &r.id, r.gx, r.gy);
            format!(
                "{}朝街口瞄了一眼，小聲道：「你問這個可問對人了——{lead}。」",
                r.true_name
            )
        }
        "SocialSettleDebt" => {
            if lever_fact.contains("擔保展期") {
                format!(
                    "{}怔了一瞬，隨即長舒一口氣，鄭重朝你拱了拱手：「這筆舊帳壓得我幾宿沒睡穩，承你肯出面替我周旋展期，這份人情我記在帳底！」",
                    r.true_name
                )
            } else {
                format!(
                    "{}擺擺手笑道：「我自個兒手頭還沒到欠帳的地步，倒是街頭那幾家這陣子週轉得緊。」",
                    r.true_name
                )
            }
        }
        "SocialPrivateTalk" => format!(
            "{}左右瞟了一眼，低聲應道：「你說得是，公開街面耳目太雜，往屋裡內室走一步再說。」",
            r.true_name
        ),
        _ => {
            if !player_input.trim().is_empty()
                && let Some(b) = r.beliefs.last()
            {
                let fact = render_belief_fact(state, b);
                return format!(
                    "{}聽罷你這話，順口接腔：「說起這個，我才聽見{fact}，這西麓街上風向變得快著呢。」",
                    r.true_name
                );
            }
            let tone = match r.temperament.obsession {
                CoreObsession::CraftPerfection => {
                    "「手頭傢伙若是差了一分，做出來的活計便站不住腳，馬虎不得。」"
                }
                CoreObsession::HoardAndOwn => {
                    "「西麓正街一天比一天熱鬧，趁早攢下本錢盤口店面才是實在。」"
                }
                CoreObsession::ProtectKin => {
                    "「街坊鄰里互相照應，誰碰上難處總不好袖手旁觀。」"
                }
                CoreObsession::OutshinePeers => {
                    "「同一條街做買賣，不拿出真本事哪壓得住對面那幾家？」"
                }
                CoreObsession::PlaySafe => {
                    "「夏漲一來低窪水路說漫就漫，還是石板正街最穩妥。」"
                }
                CoreObsession::SeekWonders => {
                    "「東邊竹林深處聽說還藏著少人走透的冷泉窪地，遲早得去瞧瞧。」"
                }
                CoreObsession::FineAppetite => {
                    "「忙活半天，趕上灶攤剛起鍋的鮮筍熱湯才算沒白忙。」"
                }
                CoreObsession::HonorLedger => {
                    "「欠債還錢、立約交貨，在這條街上站得住腳靠的就是信用。」"
                }
                CoreObsession::SelfMadeShop => {
                    "「替人幫工總有盡頭，攢夠本錢我也想在騎樓下立塊自己的招牌。」"
                }
                CoreObsession::BoldVengeance => {
                    "「有恩報恩、有怨算清，吞聲忍氣可不是我的性子。」"
                }
            };
            format!("{}{tone}", r.true_name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::SHALU_ZHULIN_WORLD_SEED;
    use crate::grid::zhulin::social::sim::DEFAULT_RESIDENT_COUNT;

    #[test]
    fn test_social_levers_preserve_zero_sum_and_trigger_public_witness() {
        let mut world =
            SocialWorldState::new_seeded(SHALU_ZHULIN_WORLD_SEED, DEFAULT_RESIDENT_COUNT);
        // 找兩名同格公開層居民
        let target_idx = world
            .residents
            .iter()
            .position(|r| r.gz == 0)
            .expect("ground resident");
        let (gx, gy) = (world.residents[target_idx].gx, world.residents[target_idx].gy);
        let other_idx = world
            .residents
            .iter()
            .position(|r| r.id != world.residents[target_idx].id)
            .expect("second resident");
        world.residents[other_idx].gx = gx;
        world.residents[other_idx].gy = gy;
        world.residents[other_idx].gz = 0;

        // 建立真實欠債以測試周旋清帳守恆
        let debtor_id = world.residents[target_idx].id.clone();
        let creditor_id = world.residents[other_idx].id.clone();
        world
            .ledger
            .record_loan(&creditor_id, &debtor_id, 12, 1, 5, gx, gy);

        // 1. 易物議價
        let p1 = prepare_resident_dialogue_turn(
            &mut world,
            target_idx,
            "test_player",
            (gx, gy, 0),
            "SocialBargain",
            "",
            true,
        );
        assert!(p1.witness_line.is_some(), "z=0 bargain must trigger bystander witness");
        assert_eq!(p1.tactical_choices.len(), 4);
        assert_eq!(world.conservation_diff(), 0);
        let out1 = finalize_resident_dialogue_unlocked(p1);
        assert!(out1.narrative.contains("〔街坊目擊〕"));

        // 2. 請食套話
        world.residents[target_idx].deficit.hunger = 0.7;
        let p2 = prepare_resident_dialogue_turn(
            &mut world,
            target_idx,
            "test_player",
            (gx, gy, 0),
            "SocialTreat",
            "",
            false,
        );
        assert!(world.residents[target_idx].deficit.hunger < 0.4);
        assert_eq!(world.conservation_diff(), 0);
        let _ = finalize_resident_dialogue_unlocked(p2);

        // 3. 周旋清帳（零和守恆不破）
        let p3 = prepare_resident_dialogue_turn(
            &mut world,
            target_idx,
            "test_player",
            (gx, gy, 0),
            "SocialSettleDebt",
            "",
            false,
        );
        assert_eq!(world.conservation_diff(), 0);
        let out3 = finalize_resident_dialogue_unlocked(p3);
        assert!(out3.narrative.contains("人情") || out3.narrative.contains("展期"));
    }
}
