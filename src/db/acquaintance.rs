//! 玩家-NPC 觀察態持久層（`players_npc_acquaintance` 表）。
//! 資料表由 store 連線池啟動邊界建立；人物接觸熱路徑只做資料讀寫。

use crate::store;

/// 玩家-NPC 觀察關係。
#[derive(Debug, Clone)]
pub struct NpcAcquaintance {
    pub player_id: String,
    pub npc_id: String,
    /// 玩家已點卡片觀察
    pub observed: bool,
    /// NPC 已交談自報名
    pub named: bool,
}

/// 查詢玩家對特定 NPC 的觀察態（若不存在回傳 None）。
pub fn get_acquaintance(player_id: &str, npc_id: &str) -> anyhow::Result<Option<NpcAcquaintance>> {
    let mem_named = store::player_knows_npc(player_id, npc_id);
    let mem_observed = store::player_observed_npc(player_id, npc_id);

    if mem_named || mem_observed {
        return Ok(Some(NpcAcquaintance {
            player_id: player_id.to_string(),
            npc_id: npc_id.to_string(),
            observed: mem_observed || mem_named,
            named: mem_named,
        }));
    }
    Ok(None)
}

/// 玩家進入格子或 NPC 進場時：INSERT OR IGNORE（兩欄均 false）。
pub fn ensure_acquaintance(_player_id: &str, _npc_id: &str) -> anyhow::Result<()> {
    Ok(())
}

/// 玩家點卡片觀察：設 `observed = true`。
pub fn set_observed(player_id: &str, npc_id: &str) -> anyhow::Result<()> {
    store::mark_player_observed_npc(player_id, npc_id);
    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("後端儲存連線不可用"))?;
    let mut conn = pool.get()?;

    conn.execute(
        "INSERT INTO players_npc_acquaintance (player_id, npc_id, observed, named)
         VALUES ($1, $2, TRUE, FALSE)
         ON CONFLICT (player_id, npc_id) DO UPDATE SET observed = TRUE",
        &[&player_id, &npc_id],
    )?;
    Ok(())
}

/// NPC 交談自報名：設 `named = true`（同時確保 `observed = true`）。
pub fn set_named(player_id: &str, npc_id: &str) -> anyhow::Result<()> {
    store::mark_player_knows_npc(player_id, npc_id);
    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("後端儲存連線不可用"))?;
    let mut conn = pool.get()?;

    conn.execute(
        "INSERT INTO players_npc_acquaintance (player_id, npc_id, observed, named)
         VALUES ($1, $2, TRUE, TRUE)
         ON CONFLICT (player_id, npc_id) DO UPDATE SET observed = TRUE, named = TRUE",
        &[&player_id, &npc_id],
    )?;
    Ok(())
}

/// 取得玩家在指定格子（或房間）內所有 NPC 的觀察態批量回傳。
pub fn get_acquaintances_for_player_npcs(
    player_id: &str,
    npc_ids: &[&str],
) -> anyhow::Result<Vec<NpcAcquaintance>> {
    if npc_ids.is_empty() {
        return Ok(vec![]);
    }
    let mut list = Vec::with_capacity(npc_ids.len());
    for &nid in npc_ids {
        let mem_named = store::player_knows_npc(player_id, nid);
        let mem_obs = store::player_observed_npc(player_id, nid);
        if mem_named || mem_obs {
            list.push(NpcAcquaintance {
                player_id: player_id.to_string(),
                npc_id: nid.to_string(),
                observed: mem_obs || mem_named,
                named: mem_named,
            });
        }
    }
    Ok(list)
}
