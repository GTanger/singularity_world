//! 2D 開放世界近場角色對談（真實數據錨定）與「鎂」守恆當面交易 HTTP 端點。

use axum::Json;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::ai::openai_chat::{
    DEFAULT_OPENROUTER_BASE_URL, DEFAULT_OPENROUTER_TALK_MODEL,
    call_openai_compatible_chat_with_history, get_player_openrouter_key,
};
use crate::grid::substrate::rim_economy::{
    quote_item, settlement_market_barter, settlement_of_pos,
};
use crate::grid::substrate::rim_life::{hunger_tag, magnesium_tag, stamina_tag, thirst_tag};
use crate::grid::substrate::rim_sim::{ActorFaction, RimActor, RimWorldState, global_rim};
use crate::grid::substrate::rim_world::RimItemKind;

use super::rim_api::{item_code_str, parse_item_code, zone_for_micro};
use super::rim_auth::{build_sky_time_payload, resolve_viewer_pid};

const LOCAL_TALK_BASE_URL: &str = "http://127.0.0.1:11436/v1";
const LOCAL_TALK_MODEL: &str = "/home/tanger/Models/Qwen3-4B-Instruct-2507-Q8_0.gguf";

#[derive(Deserialize)]
pub struct RimTalkBody {
    pub target_actor: usize,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Deserialize)]
pub struct RimTradeBody {
    pub target_actor: usize,
    pub item_code: String,
    pub mode: String,
}

fn collect_trade_lists(world: &RimWorldState, player: &RimActor, npc: &RimActor) -> (Vec<Value>, Vec<Value>) {
    let s = settlement_of_pos(npc.micro_pos().0, npc.micro_pos().1);
    let npc_offers: Vec<Value> = npc
        .inventory
        .iter()
        .filter(|s| s.count > 0)
        .map(|st| {
            let q = quote_item(world, s, st.item);
            let price = q.ask_mg.max(1);
            json!({
                "code": item_code_str(st.item),
                "label": st.item.label(),
                "count": st.count,
                "price_mg": price,
                "market_tag": q.status_tag,
                "can_afford": player.magnesium >= price,
            })
        })
        .collect();

    let player_sellable: Vec<Value> = player
        .inventory
        .iter()
        .filter(|s| s.count > 0)
        .map(|st| {
            let q = quote_item(world, s, st.item);
            let sell_p = q.bid_mg;
            json!({
                "code": item_code_str(st.item),
                "label": st.item.label(),
                "count": st.count,
                "price_mg": sell_p,
                "market_tag": q.status_tag,
                "npc_can_pay": sell_p > 0 && npc.magnesium >= sell_p,
            })
        })
        .collect();

    (npc_offers, player_sellable)
}

fn build_talk_snapshot(
    world: &RimWorldState,
    pid: &str,
    target_actor: usize,
) -> Option<(Value, Value, String)> {
    let p = world
        .actors
        .iter()
        .find(|a| a.entity_id.as_deref() == Some(pid))?;
    let npc = world
        .actors
        .iter()
        .find(|a| a.id == target_actor && a.faction == ActorFaction::Settler)?;

    let (npc_offers, player_sellable) = collect_trade_lists(world, p, npc);
    let npc_pos = npc.micro_pos();
    let zone = zone_for_micro(
        crate::grid::substrate::runtime::global(),
        npc_pos.0,
        npc_pos.1,
    );
    let (world_clock, _) = build_sky_time_payload();

    let inv_desc: Vec<String> = npc
        .inventory
        .iter()
        .filter(|s| s.count > 0)
        .map(|s| s.item.label().to_string())
        .collect();
    let wear_parts = [
        npc.equipped_hat.then_some("細編竹笠"),
        npc.equipped_pack.then_some("竹編背簍"),
        npc.equipped_tool.map(RimItemKind::label),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("、");

    let personal_diary = npc
        .recent_diary
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join("；");
    let recent_ev = world
        .events
        .iter()
        .take(2)
        .map(|e| e.text.as_str())
        .collect::<Vec<_>>()
        .join("；");
    let combined_memories = match (personal_diary.is_empty(), recent_ev.is_empty()) {
        (false, false) => format!("個人親歷：{personal_diary}；聚落消息：{recent_ev}"),
        (false, true) => format!("個人親歷：{personal_diary}"),
        (true, false) => recent_ev,
        (true, true) => "山澗風清，林間安穩".into(),
    };

    let inv_joined = if inv_desc.is_empty() {
        "暫無餘貨".to_string()
    } else {
        inv_desc.join("、")
    };
    let injury_note = if npc.downed_ticks > 0 {
        "（剛力竭倒地，說話吃力）".to_string()
    } else if npc.injury_ticks > 0
        && let Some(ref part) = npc.injury_part
    {
        format!("（身帶「{part}」，需多調息）")
    } else {
        String::new()
    };
    let system_prompt = format!(
        "你是《奇點世界》裡生活在白塔半島西麓的一位台灣普通居民，只用台灣正體中文口語回答，禁止簡體字、禁止網路用語與修仙黑話，禁止出現任何數字、百分比、數值或系統詞彙。\n\
         你的真實資料：姓名「{}」、身分「{}」、所在地「{}」、時辰「{}」、當下感受「{}、{}、{}」{}、正忙著「{}」、身上穿戴「{}」、手邊帶著「{}」、近期親歷見聞「{}」。\n\
         附近聚落與水尾寮茶寮熟悉人員：百工木匠林阿木、巡山獵戶陳青、採筍山民許秋娘、茶寮庖廚周阿茶、行腳貨郎鄭石頭（若未經手或不知情就直說不知，切勿自行編造不存在的人名或物品）。\n\
         請根據剛才對方的搭話，用 1～2 句自然樸實的生活口吻回覆（不超過 45 字，不得捏造自己沒有的物品）。",
        npc.name,
        npc.role_tag,
        zone,
        world_clock,
        npc.health_state_tag(),
        hunger_tag(npc.hunger),
        thirst_tag(npc.thirst),
        injury_note,
        npc.activity_tag,
        if wear_parts.is_empty() {
            "輕便粗麻衣裝"
        } else {
            &wear_parts
        },
        inv_joined,
        combined_memories
    );

    let card = json!({
        "npc": {
            "id": npc.id,
            "name": npc.name,
            "glyph": npc.glyph,
            "gender": npc.gender,
            "soul_seed": npc.soul_seed.to_string(),
            "role": npc.role_tag,
            "tool": npc.equipped_tool.map(RimItemKind::label),
            "wear_hat": npc.equipped_hat,
            "show_pack": npc.equipped_pack || npc.carrying.is_some(),
            "hp_ratio": (npc.hp as f32 / npc.max_hp.max(1) as f32).clamp(0.0, 1.0),
            "tags": [
                npc.role_tag,
                npc.health_state_tag(),
                hunger_tag(npc.hunger),
                thirst_tag(npc.thirst),
                npc.activity_tag,
            ],
        },
        "player_magnesium_tag": magnesium_tag(p.magnesium),
        "player_magnesium": p.magnesium,
        "player_feels": [hunger_tag(p.hunger), thirst_tag(p.thirst), stamina_tag(p.stamina)],
        "npc_offers": npc_offers,
        "player_sellable": player_sellable,
    });

    Some((
        card,
        json!({
            "name": npc.name,
            "role": npc.role_tag,
            "act": npc.activity_tag,
            "diary": npc.recent_diary.front().cloned().unwrap_or_default()
        }),
        system_prompt,
    ))
}

fn fallback_natural_reply(
    npc_name: &str,
    role: &str,
    act: &str,
    latest_diary: &str,
    newly_met: bool,
    topic: &str,
) -> String {
    if newly_met && topic == "open" {
        return format!("初會面，我叫{npc_name}，平日做些{role}的營生，眼下正在{act}。");
    }
    match topic {
        "greet" => format!("這時辰正好忙著{act}，山道風涼，有事慢慢說。"),
        "rumor" => {
            if !latest_diary.is_empty() {
                format!("說起近況嘛，我前不久才剛{latest_diary}。")
            } else {
                "東南紅土路通往水尾寮溪畔茶寮，那邊茶火暖著，貨物也齊。".into()
            }
        }
        "needs" => "若手邊有多餘竹稈、鮮筍或想換竹笠背簍，咱們都能以鎂晶當面算清。".into(),
        _ => {
            if !latest_diary.is_empty() {
                format!("剛才才忙著{latest_diary}，這會兒先順手{act}。")
            } else {
                format!("我在這帶做{role}好些時日了，正忙著{act}呢。")
            }
        }
    }
}

pub async fn rim_talk_post(headers: HeaderMap, Json(body): Json<RimTalkBody>) -> Response {
    let Some(pid) = resolve_viewer_pid(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "ok": false, "msg": "請先登入角色" })),
        )
            .into_response();
    };

    let topic = body.topic.clone().unwrap_or_default();
    let user_msg = body.message.clone().unwrap_or_default().trim().to_string();

    let (newly_met, base_card, meta_info, sys_prompt, fallback) = {
        let Ok(mut world) = global_rim().lock() else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };
        world.touch_player_activity(&pid);

        let mut newly_met = false;
        if let Some(npc) = world
            .actors
            .iter_mut()
            .find(|a| a.id == body.target_actor && a.faction == ActorFaction::Settler)
        {
            if !npc.known_by.iter().any(|k| k == &pid) {
                npc.known_by.push(pid.clone());
                newly_met = true;
            }
        }
        let Some((card, meta_info, sys_prompt)) =
            build_talk_snapshot(&world, &pid, body.target_actor)
        else {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "ok": false, "msg": "對方不在近旁或無法對話" })),
            )
                .into_response();
        };

        let npc_name = meta_info["name"].as_str().unwrap_or("對方");
        let npc_role = meta_info["role"].as_str().unwrap_or("行旅之人");
        let npc_act = meta_info["act"].as_str().unwrap_or("駐足");
        let npc_diary = meta_info["diary"].as_str().unwrap_or("");
        let fb = fallback_natural_reply(npc_name, npc_role, npc_act, npc_diary, newly_met, &topic);

        if newly_met {
            world.persist_if_needed(true);
        }
        (newly_met, card, meta_info, sys_prompt, fb)
    };

    let reply = if topic == "open" && user_msg.is_empty() {
        fallback
    } else {
        let prompt_line = if !user_msg.is_empty() {
            user_msg
        } else {
            match topic.as_str() {
                "greet" => "對方上前跟你招呼近況，請簡單寒暄回應。".into(),
                "rumor" => "對方想打聽附近風聞、水尾寮方向去處與注意事項。".into(),
                "needs" => "對方詢問你手頭有什麼物資可換，或目前還缺什麼。".into(),
                _ => "對方靠過來想聊幾句。".into(),
            }
        };
        let pid_key = pid.clone();
        let fb_clone = fallback.clone();
        tokio::task::spawn_blocking(move || {
            if let Some(api_key) = get_player_openrouter_key(&pid_key)
                && let Ok(ans) = call_openai_compatible_chat_with_history(
                    DEFAULT_OPENROUTER_BASE_URL,
                    &api_key,
                    DEFAULT_OPENROUTER_TALK_MODEL,
                    &sys_prompt,
                    &[],
                    &prompt_line,
                    90,
                )
            {
                return ans;
            }
            call_openai_compatible_chat_with_history(
                LOCAL_TALK_BASE_URL,
                "",
                LOCAL_TALK_MODEL,
                &sys_prompt,
                &[],
                &prompt_line,
                90,
            )
            .unwrap_or(fb_clone)
        })
        .await
        .unwrap_or(fallback)
    };

    // 同步將回應氣泡浮現在角色頭頂，並刷新地圖姓名認知
    if let Ok(mut world) = global_rim().lock() {
        let tick = world.tick;
        if let Some(npc) = world
            .actors
            .iter_mut()
            .find(|a| a.id == body.target_actor && a.faction == ActorFaction::Settler)
        {
            let short_pop: String = reply.chars().take(18).collect();
            npc.pop_text = Some((format!("「{short_pop}」"), tick + 16));
        }
    }

    let npc_name = meta_info["name"].as_str().unwrap_or("");
    let mut out = base_card;
    if let Some(obj) = out.as_object_mut() {
        obj.insert("ok".into(), json!(true));
        obj.insert("newly_met".into(), json!(newly_met));
        obj.insert("reply".into(), json!(reply));
        if newly_met {
            obj.insert("meet_note".into(), json!(format!("已與{npc_name}互通名姓")));
        }
    }
    Json(out).into_response()
}

pub async fn rim_trade_post(headers: HeaderMap, Json(body): Json<RimTradeBody>) -> Response {
    let Some(pid) = resolve_viewer_pid(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "ok": false, "msg": "請先登入角色" })),
        )
            .into_response();
    };
    let Some(item) = parse_item_code(&body.item_code) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "msg": "無法辨識此物品" })),
        )
            .into_response();
    };
    let player_buys = match body.mode.as_str() {
        "buy" => true,
        "sell" => false,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "ok": false, "msg": "無效的交換方式" })),
            )
                .into_response();
        }
    };

    let Ok(mut world) = global_rim().lock() else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let Some(p_idx) = world
        .actors
        .iter()
        .position(|a| a.entity_id.as_deref() == Some(pid.as_str()))
    else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(n_idx) = world
        .actors
        .iter()
        .position(|a| a.id == body.target_actor && a.faction == ActorFaction::Settler)
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "ok": false, "msg": "找不到交易對象" })),
        )
            .into_response();
    };

    let dist = (world.actors[p_idx].px - world.actors[n_idx].px)
        .hypot(world.actors[p_idx].py - world.actors[n_idx].py);
    if dist > 8.0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "msg": "距離太遠，請走近再當面交換" })),
        )
            .into_response();
    }

    let s = settlement_of_pos(
        world.actors[n_idx].micro_pos().0,
        world.actors[n_idx].micro_pos().1,
    );
    let trade_res = settlement_market_barter(&mut world, p_idx, n_idx, item, s, player_buys)
        .map(|(_, msg)| msg);

    match trade_res {
        Ok(msg) => {
            world.actors[p_idx].last_active_unix = crate::game::now_unix();
            if !world.actors[n_idx].known_by.iter().any(|k| k == &pid) {
                world.actors[n_idx].known_by.push(pid.clone());
            }
            world.push_event(msg.clone());
            world.persist_if_needed(true);
            let mut card = build_talk_snapshot(&world, &pid, body.target_actor)
                .map(|(c, _, _)| c)
                .unwrap_or_else(|| json!({}));
            if let Some(obj) = card.as_object_mut() {
                obj.insert("ok".into(), json!(true));
                obj.insert("msg".into(), json!(msg));
            }
            Json(card).into_response()
        }
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "msg": err })),
        )
            .into_response(),
    }
}
