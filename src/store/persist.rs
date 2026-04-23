//! store 備份工具 — atomic_write + 各 persist_* 方法。
//!
//! JSON 備份路徑：`atomic_write` 走先寫 tmp 再 rename 的安全模式；
//! 各 `persist_*` 從記憶體狀態序列化當前快照寫回 data/runtime/。
//!
//! 注意：PG 為唯一權威持久層。此處 JSON 僅作 fallback 備份，
//! 將來 PG writer queue 覆蓋所有路徑後，本模組整體可砍。

use std::fs;
use std::path::Path;

use super::{
    Assignment, AssignmentsFile, EntitiesFile, Entity, EntityRoomEntry, EntityRoomsFile,
    Item, ItemsFile, NpcDyad, NpcDyadsFile, NpcRumor, NpcRumorDigestFile, NpcRumorsFile,
    NpcThread, NpcThreadsFile, Schedule, SchedulesFile, Store,
};

impl Store {
    pub(super) fn atomic_write(path: &Path, data: &[u8]) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, data)?;
        if fs::rename(&tmp, path).is_err() {
            let _ = fs::remove_file(&tmp);
        }
        Ok(())
    }

    pub(super) fn persist_entity_rooms(&self) -> anyhow::Result<()> {
        let entries: Vec<EntityRoomEntry> = self
            .entity_rooms
            .iter()
            .map(|(eid, rid)| EntityRoomEntry {
                entity_id: eid.clone(),
                room_id: rid.clone(),
            })
            .collect();
        let raw = serde_json::to_string_pretty(&EntityRoomsFile { entries })?;
        Self::atomic_write(&self.entity_rooms_path, raw.as_bytes())
    }

    pub(super) fn persist_entities(&self) -> anyhow::Result<()> {
        if self.entities_path.as_os_str().is_empty() {
            return Ok(());
        }
        let list: Vec<Entity> = self.entities.values().cloned().collect();
        let raw = serde_json::to_string_pretty(&EntitiesFile { entities: list })?;
        Self::atomic_write(&self.entities_path, raw.as_bytes())
    }

    pub(super) fn persist_assignments(&self) -> anyhow::Result<()> {
        let entries: Vec<Assignment> = self.assignments.values().flatten().cloned().collect();
        let raw = serde_json::to_string_pretty(&AssignmentsFile { entries })?;
        Self::atomic_write(&self.assignments_path, raw.as_bytes())
    }

    pub(super) fn persist_schedules(&self) -> anyhow::Result<()> {
        let entries: Vec<Schedule> = self.schedules.values().cloned().collect();
        let raw = serde_json::to_string_pretty(&SchedulesFile { entries })?;
        Self::atomic_write(&self.schedules_path, raw.as_bytes())
    }

    pub(super) fn persist_items(&self) -> anyhow::Result<()> {
        let items: Vec<Item> = self.items.values().cloned().collect();
        let raw = serde_json::to_string_pretty(&ItemsFile { items })?;
        Self::atomic_write(&self.items_path, raw.as_bytes())
    }

    pub(super) fn persist_npc_threads(&self) -> anyhow::Result<()> {
        let entries: Vec<NpcThread> = self.npc_threads.values().cloned().collect();
        let raw = serde_json::to_string_pretty(&NpcThreadsFile { entries })?;
        Self::atomic_write(&self.npc_thread_path, raw.as_bytes())
    }

    pub(super) fn persist_npc_dyads(&self) -> anyhow::Result<()> {
        let entries: Vec<NpcDyad> = self.npc_dyads.values().cloned().collect();
        let raw = serde_json::to_string_pretty(&NpcDyadsFile { entries })?;
        Self::atomic_write(&self.npc_dyad_path, raw.as_bytes())
    }

    pub(super) fn persist_npc_rumors(&self) -> anyhow::Result<()> {
        self.persist_npc_rumors_with_deletes(&[])
    }

    /// PG 寫入走 writer queue（不阻塞 store lock）；JSON fallback 保留（快、不卡）。
    pub(super) fn persist_npc_rumors_with_deletes(
        &self,
        deleted_ids: &[String],
    ) -> anyhow::Result<()> {
        // PG 走 writer queue——clone 資料後立刻放鎖，PG IO 在背景做
        let upserts: Vec<NpcRumor> = self.npc_rumors.values().cloned().collect();
        crate::pg::writer::submit(crate::pg::writer::WriteOp::SyncNpcRumors {
            upserts,
            deletes: deleted_ids.to_vec(),
        });

        // JSON fallback 保留（atomic_write 很快，不是瓶頸）
        let entries: Vec<NpcRumor> = self.npc_rumors.values().cloned().collect();
        let raw = serde_json::to_string_pretty(&NpcRumorsFile { entries })?;
        Self::atomic_write(&self.npc_rumor_path, raw.as_bytes())
    }

    pub(super) fn persist_npc_rumor_digest(&self) -> anyhow::Result<()> {
        if self.npc_rumor_digest_path.as_os_str().is_empty() {
            return Ok(());
        }
        let Some(ref d) = self.npc_rumor_digest else {
            return Ok(());
        };
        let raw = serde_json::to_string_pretty(&NpcRumorDigestFile { digest: d.clone() })?;
        Self::atomic_write(&self.npc_rumor_digest_path, raw.as_bytes())
    }
}
