//! M3-029 北岸市集/倉棚局部經濟閉環：正式 NPC 個體共用需求板與物質來源。

use crate::db;
use crate::event::{self, types};
use crate::world::materials;

pub(crate) const PACK_RUNNER_ID: &str = "npc_m3_029_pack_runner";

const NORTH_BANK_ROOM_ID: &str = "grid:0:0";
const SHORT_MARKET_ROOM_ID: &str = "grid:1:0";
const WAREHOUSE_ROOM_ID: &str = "grid:1:-2";
const REED_ITEM_ID: &str = "mat_reed_bundle";
const REED_PAY_MG: i32 = 4;

const M3_027_REED_SOLD_STATE: &str = "m3_027_short_market|sold_reed_bundle";
const M3_027_REED_ACTION: &str = "m3_027_market_reed_buyer|SellReeds";
const RUNNER_GATHER_ACTION: &str = "m3_029_pack_runner|GatherReeds";
const RUNNER_DELIVER_ACTION: &str = "m3_029_pack_runner|DeliverReeds";
const RUNNER_WAREHOUSE_ACTION: &str = "m3_029_pack_runner|MoveToWarehouse";
const RUNNER_HOLDING_REEDS_STATE: &str = "m3_029_pack_runner|holding_reed_bundle";
const RUNNER_DELIVERED_REEDS_STATE: &str = "m3_029_pack_runner|reed_delivered_to_short_market";
const RUNNER_WAREHOUSE_TRACE_STATE: &str = "m3_029_pack_runner|warehouse_trace_logged";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MarketRunnerTick {
    pub room_id: &'static str,
    pub actor: &'static str,
    pub detail: String,
}

/// 將正式 NPC 的局部經濟事件合併進玩家視角的物件歷史。
pub(crate) fn merge_shared_market_history(
    actions: &mut Vec<String>,
    states: &mut Vec<String>,
    now_unix: i64,
) {
    let from = now_unix.saturating_sub(6 * 60 * 60);
    let Ok(rows) = event::events_in_range(PACK_RUNNER_ID, from, now_unix) else {
        return;
    };
    for row in rows {
        match row.event_type.as_str() {
            types::SCENE_OBJECT_ACTION => actions.push(row.payload),
            types::SCENE_OBJECT_STATE => states.push(row.payload),
            _ => {}
        }
    }
}

/// 正式 NPC 是否已用同一需求板交過蘆草。
pub(crate) fn pack_runner_delivered_reeds(states: &[String]) -> bool {
    states
        .iter()
        .any(|payload| payload == RUNNER_DELIVERED_REEDS_STATE)
}

/// 正式 NPC 的跨物件、跨房間可見回聲。
pub(crate) fn shared_economy_echo(object_id: &str, states: &[String]) -> Option<String> {
    if states
        .iter()
        .any(|payload| payload == RUNNER_WAREHOUSE_TRACE_STATE)
        && object_id == "m3_027_warehouse_tally_board"
    {
        return Some(
            "搬貨人已把市集那隻墊了蘆草的貨籃推到倉棚點貨牌旁，記牌人先把這行壓住；剩下的包貨需求仍要照市集包貨單補齊。"
                .to_string(),
        );
    }
    if pack_runner_delivered_reeds(states) {
        match object_id {
            "m3_027_market_reed_buyer" => Some(
                "搬貨人已把一束蘆草交到包貨單旁，秤邊貨籃已先墊住一角；蘆草墊箱只差最後一束。"
                    .to_string(),
            ),
            "m3_027_warehouse_tally_board" => Some(
                "市集那邊先收進搬貨人的一束蘆草，倉棚門前已有一只貨籃不再直接磨地；記牌人等剩下的包貨料補齊。"
                    .to_string(),
            ),
            _ => None,
        }
    } else {
        None
    }
}

/// 正式 NPC 市場搬貨 tick：取得蘆草、交付市集、留下倉棚回聲。
pub(crate) fn run_pack_runner_tick(now_unix: i64) -> Option<MarketRunnerTick> {
    #[cfg(not(test))]
    {
        let hour = crate::game::current_game_hour();
        if !(5..=19).contains(&hour) {
            return None;
        }
    }
    ensure_pack_runner_entity(now_unix).ok()?;
    let states = pack_runner_states(now_unix);
    if !has_state(&states, RUNNER_HOLDING_REEDS_STATE)
        && !has_state(&states, RUNNER_DELIVERED_REEDS_STATE)
    {
        return gather_reeds(now_unix);
    }
    if has_state(&states, RUNNER_HOLDING_REEDS_STATE)
        && !has_state(&states, RUNNER_DELIVERED_REEDS_STATE)
    {
        return deliver_reeds(now_unix);
    }
    if has_state(&states, RUNNER_DELIVERED_REEDS_STATE)
        && !has_state(&states, RUNNER_WAREHOUSE_TRACE_STATE)
    {
        return move_trace_to_warehouse(now_unix);
    }
    None
}

fn ensure_pack_runner_entity(now_unix: i64) -> anyhow::Result<()> {
    if db::get_entity(PACK_RUNNER_ID)?.is_none() {
        db::insert_npc(PACK_RUNNER_ID, "搬", "M", "搬貨人")?;
        let _ = event::append(now_unix, PACK_RUNNER_ID, types::OBSERVED, "m3_029_seeded");
    }
    if db::get_entity_room(PACK_RUNNER_ID)
        .unwrap_or_default()
        .is_empty()
    {
        db::set_entity_grid(PACK_RUNNER_ID, 0, 0)?;
    }
    Ok(())
}

fn pack_runner_states(now_unix: i64) -> Vec<String> {
    event::events_in_range(
        PACK_RUNNER_ID,
        now_unix.saturating_sub(6 * 60 * 60),
        now_unix,
    )
    .unwrap_or_default()
    .into_iter()
    .filter(|row| row.event_type == types::SCENE_OBJECT_STATE)
    .map(|row| row.payload)
    .collect()
}

fn has_state(states: &[String], state: &str) -> bool {
    states.iter().any(|payload| payload == state)
}

fn gather_reeds(now_unix: i64) -> Option<MarketRunnerTick> {
    let _ = db::set_entity_grid(PACK_RUNNER_ID, 0, 0);
    let _ = db::set_entity_activity(
        PACK_RUNNER_ID,
        "在三水口北岸蘆叢邊挑一束能墊箱的蘆草，準備交往三水口東街包貨單",
    );
    if let Some(gathered) =
        materials::try_gather_material_for_room_preserving(NORTH_BANK_ROOM_ID, now_unix, 1)
    {
        let _ = db::add_to_inventory(PACK_RUNNER_ID, &gathered.item_id, 1);
        let _ = event::append(
            now_unix,
            PACK_RUNNER_ID,
            types::SCENE_OBJECT_ACTION,
            RUNNER_GATHER_ACTION,
        );
        let _ = event::append(
            now_unix,
            PACK_RUNNER_ID,
            types::SCENE_OBJECT_STATE,
            RUNNER_HOLDING_REEDS_STATE,
        );
        return Some(MarketRunnerTick {
            room_id: NORTH_BANK_ROOM_ID,
            actor: "搬貨人",
            detail:
                "沿水線取走一束能墊箱的蘆草，抱在臂彎裡往市集方向讓路；蘆叢靠木樁那片矮了下去。"
                    .to_string(),
        });
    }
    Some(MarketRunnerTick {
        room_id: NORTH_BANK_ROOM_ID,
        actor: "搬貨人",
        detail: "摸到的只剩短葉，便把空貨籃往市集棚腳退回，等下一輪水氣把長莖養回。".to_string(),
    })
}

fn deliver_reeds(now_unix: i64) -> Option<MarketRunnerTick> {
    let _ = db::set_entity_grid(PACK_RUNNER_ID, 1, 0);
    let _ = db::set_entity_activity(
        PACK_RUNNER_ID,
        "已把一束蘆草交到三水口東街包貨單，手裡多了幾枚鎂，下一步等倉棚點貨",
    );
    let has_reeds = db::get_entity(PACK_RUNNER_ID)
        .ok()
        .flatten()
        .is_some_and(|ch| db::inventory_has_item(&ch.inventory, REED_ITEM_ID));
    if !has_reeds {
        return Some(MarketRunnerTick {
            room_id: SHORT_MARKET_ROOM_ID,
            actor: "搬貨人",
            detail: "站到市集包貨單前時手上已空，只能退到棚腳等下一輪蘆草。".to_string(),
        });
    }
    let _ = db::remove_from_inventory(PACK_RUNNER_ID, REED_ITEM_ID, 1);
    let _ = db::add_magnesium(PACK_RUNNER_ID, REED_PAY_MG);
    let _ = event::append(
        now_unix,
        PACK_RUNNER_ID,
        types::SCENE_OBJECT_ACTION,
        RUNNER_DELIVER_ACTION,
    );
    let _ = event::append(
        now_unix,
        PACK_RUNNER_ID,
        types::SCENE_OBJECT_ACTION,
        M3_027_REED_ACTION,
    );
    let _ = event::append(
        now_unix,
        PACK_RUNNER_ID,
        types::SCENE_OBJECT_STATE,
        M3_027_REED_SOLD_STATE,
    );
    let _ = event::append(
        now_unix,
        PACK_RUNNER_ID,
        types::SCENE_OBJECT_STATE,
        RUNNER_DELIVERED_REEDS_STATE,
    );
    Some(MarketRunnerTick {
        room_id: SHORT_MARKET_ROOM_ID,
        actor: "搬貨人",
        detail:
            "把蘆草交到市集包貨單旁，收牌人數出幾枚鎂；秤邊貨籃先墊住一角，蘆草墊箱只差最後一束。"
                .to_string(),
    })
}

fn move_trace_to_warehouse(now_unix: i64) -> Option<MarketRunnerTick> {
    let _ = db::set_entity_grid(PACK_RUNNER_ID, 1, -2);
    let _ = db::set_entity_activity(
        PACK_RUNNER_ID,
        "站在倉棚點貨牌旁等三水口東街那批貨入冊，下一步看下一輪包貨單",
    );
    let _ = event::append(
        now_unix,
        PACK_RUNNER_ID,
        types::SCENE_OBJECT_ACTION,
        RUNNER_WAREHOUSE_ACTION,
    );
    let _ = event::append(
        now_unix,
        PACK_RUNNER_ID,
        types::SCENE_OBJECT_STATE,
        RUNNER_WAREHOUSE_TRACE_STATE,
    );
    Some(MarketRunnerTick {
        room_id: WAREHOUSE_ROOM_ID,
        actor: "搬貨人",
        detail:
            "把市集那隻墊了蘆草的貨籃推到倉棚點貨牌旁，記牌人先把這行壓住，等剩下的包貨料補齊。"
                .to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m3_029_shared_history_marks_npc_delivery_as_pack_order_progress() {
        let states = vec![
            RUNNER_DELIVERED_REEDS_STATE.to_string(),
            M3_027_REED_SOLD_STATE.to_string(),
        ];
        assert!(pack_runner_delivered_reeds(&states));
        assert!(
            shared_economy_echo("m3_027_market_reed_buyer", &states)
                .unwrap()
                .contains("搬貨人")
        );
        assert!(
            shared_economy_echo("m3_027_warehouse_tally_board", &states)
                .unwrap()
                .contains("倉棚")
        );
    }

    #[test]
    fn m3_029_warehouse_trace_has_cross_room_echo() {
        let states = vec![RUNNER_WAREHOUSE_TRACE_STATE.to_string()];
        let echo = shared_economy_echo("m3_027_warehouse_tally_board", &states).unwrap();
        assert!(echo.contains("搬貨人"));
        assert!(echo.contains("市集"));
        assert!(echo.contains("倉棚"));
        for forbidden in ["debug", "資料庫", "grid", "NPC", "還有"] {
            assert!(!echo.contains(forbidden), "{echo}");
        }
    }
}
