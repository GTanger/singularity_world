// NPC↔NPC 執行緒／雙元關係 store 門面，對齊既有 db 層對 `npc_thread`／`npc_dyad` 的存取。

use crate::store::{self, NpcDyad, NpcThread};

use super::ErrNoStore;

pub fn get_npc_npc_thread(id_a: &str, id_b: &str) -> anyhow::Result<Option<NpcThread>> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read().unwrap_or_else(|e| e.into_inner());
    Ok(s.get_npc_thread(id_a, id_b))
}

pub fn set_npc_npc_thread(id_a: &str, id_b: &str, t: NpcThread) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap_or_else(|e| e.into_inner());
    s.set_npc_thread(id_a, id_b, t)
}

pub fn delete_npc_npc_thread(id_a: &str, id_b: &str) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap_or_else(|e| e.into_inner());
    s.delete_npc_thread(id_a, id_b)
}

pub fn get_npc_npc_dyad(id_a: &str, id_b: &str) -> anyhow::Result<Option<NpcDyad>> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read().unwrap_or_else(|e| e.into_inner());
    Ok(s.get_npc_dyad(id_a, id_b))
}

pub fn upsert_npc_rumor(r: store::NpcRumor) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap_or_else(|e| e.into_inner());
    s.upsert_npc_rumor(r)
}

/// 傳聞池時間衰減與過期刪除（對齊既有 `DecayNpcRumors`）。
pub fn decay_npc_rumors(now_unix: i64) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap_or_else(|e| e.into_inner());
    s.decay_npc_rumors(now_unix)
}

/// 重建傳聞 digest 摘要檔（對齊既有 `BuildNpcRumorDigest`）。
pub fn build_npc_rumor_digest(now_unix: i64) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap_or_else(|e| e.into_inner());
    s.build_npc_rumor_digest(now_unix)
}

/// 寫入或更新雙元關係（對齊既有 `SetNpcNpcDyad`）。
pub fn set_npc_npc_dyad(id_a: &str, id_b: &str, d: NpcDyad) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap_or_else(|e| e.into_inner());
    s.set_npc_dyad(id_a, id_b, d)
}

/// 取得 NPC↔NPC 對話摘要（對齊 `GetNpcNpcConversationSummary`）。
#[must_use]
pub fn get_npc_npc_conversation_summary(id_a: &str, id_b: &str) -> String {
    let Some(arc) = store::get_store() else {
        return String::new();
    };
    let s = arc.read().unwrap_or_else(|e| e.into_inner());
    s.get_npc_npc_summary(id_a, id_b)
}

/// 寫入 NPC↔NPC 對話摘要（對齊 `SetNpcNpcConversationSummary`）。
pub fn set_npc_npc_conversation_summary(id_a: &str, id_b: &str, summary: &str) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap_or_else(|e| e.into_inner());
    s.set_npc_npc_summary(id_a, id_b, summary)
}
