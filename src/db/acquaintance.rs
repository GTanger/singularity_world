//! 玩家-NPC 觀察態持久層（`players_npc_acquaintance` 表）。
//!
//! M2 migration：`CREATE TABLE IF NOT EXISTS players_npc_acquaintance`
//! 在每次呼叫時自動確保存在。

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

const CREATE_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS players_npc_acquaintance (
        player_id TEXT    NOT NULL,
        npc_id    TEXT    NOT NULL,
        observed  BOOLEAN NOT NULL DEFAULT FALSE,
        named     BOOLEAN NOT NULL DEFAULT FALSE,
        PRIMARY KEY (player_id, npc_id)
    )
";

/// 查詢玩家對特定 NPC 的觀察態（若不存在回傳 None）。
pub fn get_acquaintance(player_id: &str, npc_id: &str) -> anyhow::Result<Option<NpcAcquaintance>> {
    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("資料庫連線不可用"))?;
    let mut conn = pool.get()?;
    conn.execute(CREATE_TABLE, &[])?;

    let rows = conn.query(
        "SELECT player_id, npc_id, observed, named
         FROM players_npc_acquaintance
         WHERE player_id = $1 AND npc_id = $2",
        &[&player_id, &npc_id],
    )?;

    if rows.is_empty() {
        return Ok(None);
    }
    let row = &rows[0];
    Ok(Some(NpcAcquaintance {
        player_id: row.get(0),
        npc_id: row.get(1),
        observed: row.get(2),
        named: row.get(3),
    }))
}

/// 玩家進入格子或 NPC 進場時：INSERT OR IGNORE（兩欄均 false）。
pub fn ensure_acquaintance(player_id: &str, npc_id: &str) -> anyhow::Result<()> {
    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("資料庫連線不可用"))?;
    let mut conn = pool.get()?;
    conn.execute(CREATE_TABLE, &[])?;

    conn.execute(
        "INSERT INTO players_npc_acquaintance (player_id, npc_id, observed, named)
         VALUES ($1, $2, FALSE, FALSE)
         ON CONFLICT (player_id, npc_id) DO NOTHING",
        &[&player_id, &npc_id],
    )?;
    Ok(())
}

/// 玩家點卡片觀察：設 `observed = true`。
pub fn set_observed(player_id: &str, npc_id: &str) -> anyhow::Result<()> {
    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("資料庫連線不可用"))?;
    let mut conn = pool.get()?;
    conn.execute(CREATE_TABLE, &[])?;

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
    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("資料庫連線不可用"))?;
    let mut conn = pool.get()?;
    conn.execute(CREATE_TABLE, &[])?;

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
    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("資料庫連線不可用"))?;
    let mut conn = pool.get()?;
    conn.execute(CREATE_TABLE, &[])?;

    let npc_ids_vec: Vec<String> = npc_ids.iter().map(|s| s.to_string()).collect();
    let rows = conn.query(
        "SELECT player_id, npc_id, observed, named
         FROM players_npc_acquaintance
         WHERE player_id = $1 AND npc_id = ANY($2)",
        &[&player_id, &npc_ids_vec],
    )?;

    Ok(rows
        .iter()
        .map(|r| NpcAcquaintance {
            player_id: r.get(0),
            npc_id: r.get(1),
            observed: r.get(2),
            named: r.get(3),
        })
        .collect())
}
