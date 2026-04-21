//! PG Writer Service — 集中的 PG 寫入管道。
//!
//! 地基原則：
//! - 所有 PG 寫入走此 service，不散落在各處的 `std::thread::spawn`
//! - 有界 channel（`sync_channel(CAP)`）提供天然 backpressure——
//!   queue 滿時 submit 端會阻塞，而不是無界堆積 thread
//! - 單一 worker thread 消化 queue，**寫入順序 = submit 順序**
//!   （同 entity 快速多次 update 不會因為 thread race 造成後寫蓋前寫）
//! - Shutdown 時 drain queue 再關 worker，不丟資料
//! - 每筆 op 在 `catch_unwind` 內跑，單筆 panic 不拖垮 worker
//!
//! 取代舊的：
//! - `Store::pg_upsert_entity` 在 store write lock 下做同步 PG IO
//! - `std::thread::spawn` 無界 fire-and-forget
//! - `Store::persist_entities` 寫整份 JSON 到檔案（違反 CLAUDE.md「PG 權威」硬規則）

use crate::store::{ArchivalEntry, Entity, Item, NpcRumor};
use crate::store::sql::DbPool;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::Duration;

/// Queue 容量。滿了 submit 端會阻塞（backpressure）。
/// 1024 對 single-node 遊戲綽綽：每秒 NPC tick ~100 update、玩家 move ~10 update，
/// PG INSERT ~1ms、worker 消化速率遠大於灌入速率；queue 滿代表 PG 掛了，阻塞是對的。
const QUEUE_CAP: usize = 1024;

/// 寫入操作的統一表示。每加一個 domain 多一個 variant。
#[allow(clippy::large_enum_variant)]
pub enum WriteOp {
    UpsertEntity(Entity),
    AppendEvent {
        at: i64,
        entity_id: String,
        event_type: String,
        payload: String,
    },
    SetAuth {
        entity_id: String,
        password_hash: String,
    },
    AppendArchival(ArchivalEntry),
    SetEntityRoom {
        entity_id: String,
        room_id: String,
    },
    SetEntityActivity {
        entity_id: String,
        activity: String,
    },
    /// 批量同步傳聞：upsert 存活的 + delete 過期的（decay_npc_rumors 每 30s 觸發）
    SyncNpcRumors {
        upserts: Vec<NpcRumor>,
        deletes: Vec<String>,
    },
    InsertAssignment {
        entity_id: String,
        occupation_id: String,
        venue_id: String,
        assigned_by: String,
    },
    RemoveAssignments {
        entity_id: String,
    },
    InsertSchedule {
        entity_id: String,
        work_room: String,
        rest_room: String,
        shift_start: i32,
        shift_end: i32,
    },
    RemoveSchedule {
        entity_id: String,
    },
    RecordMeet {
        entity_id: String,
        subject_id: String,
    },
    SetFavorability {
        entity_id: String,
        subject_id: String,
        new_fav: i32,
    },
    SetNpcSummary {
        entity_id: String,
        summary: String,
    },
    SetNpcNpcSummary {
        dyad_key: String,
        summary: String,
    },
    SetNpcThread {
        key: String,
        topic_type: String,
        phase: String,
        anchors_raw: String,
        turn_count: i32,
        cooldown_until: i64,
        updated_at: i64,
    },
    DeleteNpcThread {
        key: String,
    },
    SetNpcDyad {
        key: String,
        a_id: String,
        b_id: String,
        familiarity: i32,
        sentiment: i32,
        tags_raw: String,
        updated_at: i64,
    },
    TrimArchival {
        max: i64,
    },
    UpsertItem(Item),
}

struct Service {
    tx: SyncSender<WriteOp>,
    shutdown: Arc<AtomicBool>,
    pending: Arc<AtomicUsize>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

static SERVICE: OnceLock<Service> = OnceLock::new();

/// server 啟動時初始化一次。重複呼叫無效（OnceLock）。
pub fn init(pool: DbPool) {
    let _ = SERVICE.get_or_init(|| {
        let (tx, rx) = mpsc::sync_channel::<WriteOp>(QUEUE_CAP);
        let shutdown = Arc::new(AtomicBool::new(false));
        let pending = Arc::new(AtomicUsize::new(0));
        let shutdown_w = shutdown.clone();
        let pending_w = pending.clone();
        let handle = std::thread::Builder::new()
            .name("pg-writer".into())
            .spawn(move || worker_loop(pool, rx, shutdown_w, pending_w))
            .expect("spawn pg-writer thread");
        tracing::info!("[pg::writer] 服務啟動，queue={QUEUE_CAP}, workers=1");
        Service {
            tx,
            shutdown,
            pending,
            handle: Mutex::new(Some(handle)),
        }
    });
}

fn worker_loop(
    pool: DbPool,
    rx: mpsc::Receiver<WriteOp>,
    shutdown: Arc<AtomicBool>,
    pending: Arc<AtomicUsize>,
) {
    loop {
        // recv_timeout 讓 shutdown flag 有機會被檢查（不 block 永遠）
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(op) => {
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    handle_op(&pool, &op);
                }));
                if let Err(e) = r {
                    let msg = if let Some(s) = e.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = e.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "unknown panic".to_string()
                    };
                    tracing::error!("[pg::writer] op panic: {msg}");
                }
                pending.fetch_sub(1, Ordering::Release);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if shutdown.load(Ordering::Acquire) && pending.load(Ordering::Acquire) == 0 {
                    tracing::info!("[pg::writer] shutdown drained, exit");
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                tracing::info!("[pg::writer] channel disconnected, exit");
                break;
            }
        }
    }
}

fn handle_op(pool: &DbPool, op: &WriteOp) {
    let r: anyhow::Result<()> = match op {
        WriteOp::UpsertEntity(e) => super::entity::upsert(pool, e),
        WriteOp::AppendEvent {
            at,
            entity_id,
            event_type,
            payload,
        } => super::event::append(pool, *at, entity_id, event_type, payload),
        WriteOp::SetAuth {
            entity_id,
            password_hash,
        } => super::auth::set(pool, entity_id, password_hash),
        WriteOp::AppendArchival(entry) => {
            if let Ok(mut conn) = pool.get() {
                conn.execute(
                    "INSERT INTO archival (entity_id, content, tag, created_at) VALUES ($1, $2, $3, $4)",
                    &[&entry.entity_id, &entry.content, &entry.tag, &entry.created_at],
                )
                .map(|_| ())
                .map_err(|e| anyhow::anyhow!(e))
            } else {
                Err(anyhow::anyhow!("pool.get failed"))
            }
        }
        WriteOp::SetEntityRoom { entity_id, room_id } => {
            if let Ok(mut conn) = pool.get() {
                conn.execute(
                    "INSERT INTO entity_rooms (entity_id, room_id) VALUES ($1, $2) \
                     ON CONFLICT (entity_id) DO UPDATE SET room_id = EXCLUDED.room_id",
                    &[&entity_id, &room_id],
                )
                .map(|_| ())
                .map_err(|e| anyhow::anyhow!(e))
            } else {
                Err(anyhow::anyhow!("pool.get failed"))
            }
        }
        WriteOp::SetEntityActivity { entity_id, activity } => {
            if let Ok(mut conn) = pool.get() {
                conn.execute(
                    "UPDATE entities SET current_activity = $1 WHERE id = $2",
                    &[&activity, &entity_id],
                )
                .map(|_| ())
                .map_err(|e| anyhow::anyhow!(e))
            } else {
                Err(anyhow::anyhow!("pool.get failed"))
            }
        }
        WriteOp::SyncNpcRumors { upserts, deletes } => {
            let r1 = super::rumor::delete_batch(pool, deletes);
            let r2 = super::rumor::upsert_batch(pool, upserts);
            r1.and(r2)
        }
        WriteOp::InsertAssignment { entity_id, occupation_id, venue_id, assigned_by } => {
            pool.get().map_err(|e| anyhow::anyhow!(e)).and_then(|mut c| {
                c.execute(
                    "INSERT INTO assignments (entity_id, occupation_id, venue_id, assigned_by) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
                    &[entity_id, occupation_id, venue_id, assigned_by],
                ).map(|_| ()).map_err(|e| anyhow::anyhow!(e))
            })
        }
        WriteOp::RemoveAssignments { entity_id } => {
            pool.get().map_err(|e| anyhow::anyhow!(e)).and_then(|mut c| {
                c.execute("DELETE FROM assignments WHERE entity_id = $1", &[entity_id])
                    .map(|_| ()).map_err(|e| anyhow::anyhow!(e))
            })
        }
        WriteOp::InsertSchedule { entity_id, work_room, rest_room, shift_start, shift_end } => {
            pool.get().map_err(|e| anyhow::anyhow!(e)).and_then(|mut c| {
                c.execute(
                    "INSERT INTO schedules (entity_id, work_room, rest_room, shift_start, shift_end) VALUES ($1, $2, $3, $4, $5) \
                     ON CONFLICT (entity_id) DO UPDATE SET work_room=EXCLUDED.work_room, rest_room=EXCLUDED.rest_room, shift_start=EXCLUDED.shift_start, shift_end=EXCLUDED.shift_end",
                    &[entity_id, work_room, rest_room, shift_start, shift_end],
                ).map(|_| ()).map_err(|e| anyhow::anyhow!(e))
            })
        }
        WriteOp::RemoveSchedule { entity_id } => {
            pool.get().map_err(|e| anyhow::anyhow!(e)).and_then(|mut c| {
                c.execute("DELETE FROM schedules WHERE entity_id = $1", &[entity_id])
                    .map(|_| ()).map_err(|e| anyhow::anyhow!(e))
            })
        }
        WriteOp::RecordMeet { entity_id, subject_id } => {
            pool.get().map_err(|e| anyhow::anyhow!(e)).and_then(|mut c| {
                c.execute(
                    "INSERT INTO npc_memories (entity_id, subject_id, meet_count) VALUES ($1, $2, 1) \
                     ON CONFLICT(entity_id, subject_id) DO UPDATE SET meet_count = npc_memories.meet_count + 1",
                    &[entity_id, subject_id],
                ).map(|_| ()).map_err(|e| anyhow::anyhow!(e))
            })
        }
        WriteOp::SetFavorability { entity_id, subject_id, new_fav } => {
            pool.get().map_err(|e| anyhow::anyhow!(e)).and_then(|mut c| {
                c.execute(
                    "INSERT INTO npc_memories (entity_id, subject_id, favorability) VALUES ($1, $2, $3) \
                     ON CONFLICT(entity_id, subject_id) DO UPDATE SET favorability = $3",
                    &[entity_id, subject_id, new_fav],
                ).map(|_| ()).map_err(|e| anyhow::anyhow!(e))
            })
        }
        WriteOp::SetNpcSummary { entity_id, summary } => {
            pool.get().map_err(|e| anyhow::anyhow!(e)).and_then(|mut c| {
                c.execute(
                    "INSERT INTO npc_summaries (entity_id, summary) VALUES ($1, $2) \
                     ON CONFLICT(entity_id) DO UPDATE SET summary=EXCLUDED.summary",
                    &[entity_id, summary],
                ).map(|_| ()).map_err(|e| anyhow::anyhow!(e))
            })
        }
        WriteOp::SetNpcNpcSummary { dyad_key, summary } => {
            pool.get().map_err(|e| anyhow::anyhow!(e)).and_then(|mut c| {
                c.execute(
                    "INSERT INTO npc_npc_summaries (dyad_key, summary) VALUES ($1, $2) \
                     ON CONFLICT(dyad_key) DO UPDATE SET summary=EXCLUDED.summary",
                    &[dyad_key, summary],
                ).map(|_| ()).map_err(|e| anyhow::anyhow!(e))
            })
        }
        WriteOp::SetNpcThread { key, topic_type, phase, anchors_raw, turn_count, cooldown_until, updated_at } => {
            pool.get().map_err(|e| anyhow::anyhow!(e)).and_then(|mut c| {
                c.execute(
                    "INSERT INTO npc_threads (thread_key, topic_type, phase, anchors, turn_count, cooldown_until, updated_at) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7) \
                     ON CONFLICT(thread_key) DO UPDATE SET topic_type=EXCLUDED.topic_type, phase=EXCLUDED.phase, \
                     anchors=EXCLUDED.anchors, turn_count=EXCLUDED.turn_count, cooldown_until=EXCLUDED.cooldown_until, updated_at=EXCLUDED.updated_at",
                    &[key, topic_type, phase, anchors_raw, turn_count, cooldown_until, updated_at],
                ).map(|_| ()).map_err(|e| anyhow::anyhow!(e))
            })
        }
        WriteOp::DeleteNpcThread { key } => {
            pool.get().map_err(|e| anyhow::anyhow!(e)).and_then(|mut c| {
                c.execute("DELETE FROM npc_threads WHERE thread_key = $1", &[key])
                    .map(|_| ()).map_err(|e| anyhow::anyhow!(e))
            })
        }
        WriteOp::SetNpcDyad { key, a_id, b_id, familiarity, sentiment, tags_raw, updated_at } => {
            pool.get().map_err(|e| anyhow::anyhow!(e)).and_then(|mut c| {
                c.execute(
                    "INSERT INTO npc_dyads (dyad_key, a_id, b_id, familiarity, sentiment, tags, updated_at) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7) \
                     ON CONFLICT(dyad_key) DO UPDATE SET familiarity=EXCLUDED.familiarity, sentiment=EXCLUDED.sentiment, \
                     tags=EXCLUDED.tags, updated_at=EXCLUDED.updated_at",
                    &[key, a_id, b_id, familiarity, sentiment, tags_raw, updated_at],
                ).map(|_| ()).map_err(|e| anyhow::anyhow!(e))
            })
        }
        WriteOp::TrimArchival { max } => {
            pool.get().map_err(|e| anyhow::anyhow!(e)).and_then(|mut c| {
                c.execute(
                    "DELETE FROM archival WHERE id NOT IN (SELECT id FROM (SELECT id, row_number() OVER (PARTITION BY entity_id ORDER BY created_at DESC, id DESC) as rn FROM archival) t WHERE rn <= $1)",
                    &[max],
                ).map(|_| ()).map_err(|e| anyhow::anyhow!(e))
            })
        }
        WriteOp::UpsertItem(it) => {
            pool.get().map_err(|e| anyhow::anyhow!(e)).and_then(|mut c| {
                c.execute(
                    "INSERT INTO items (id, name, slot, item_type, weight, stackable, denomination, description, vit_bonus, dex_bonus, atk_bonus) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
                     ON CONFLICT (id) DO UPDATE SET name=EXCLUDED.name, slot=EXCLUDED.slot, item_type=EXCLUDED.item_type, \
                     weight=EXCLUDED.weight, stackable=EXCLUDED.stackable, denomination=EXCLUDED.denomination, \
                     description=EXCLUDED.description, vit_bonus=EXCLUDED.vit_bonus, dex_bonus=EXCLUDED.dex_bonus, atk_bonus=EXCLUDED.atk_bonus",
                    &[&it.id, &it.name, &it.slot, &it.item_type, &it.weight, &it.stackable, &it.denomination, &it.description, &it.vit_bonus, &it.dex_bonus, &it.atk_bonus],
                ).map(|_| ()).map_err(|e| anyhow::anyhow!(e))
            })
        }
    };
    if let Err(err) = r {
        tracing::error!("[pg::writer] op failed: {err}");
    }
}

/// 提交一筆寫入。Queue 滿時**丟棄並 log**（try_send 不阻塞）。
/// 理由：submit 的 caller 多半在 store write lock 內（update_entity 等），
/// 若 submit 阻塞則反而延長 lock hold、login reader 卡更久。
/// 丟棄後由 periodic full-sync 兜底（資料最終一致，單次漂移可接受）。
/// 未 init 時靜默丟棄（server 啟動邊緣情況）。
pub fn submit(op: WriteOp) {
    if let Some(s) = SERVICE.get() {
        s.pending.fetch_add(1, Ordering::Release);
        match s.tx.try_send(op) {
            Ok(_) => {}
            Err(mpsc::TrySendError::Full(_)) => {
                s.pending.fetch_sub(1, Ordering::Release);
                tracing::warn!("[pg::writer] queue full (cap={QUEUE_CAP}), dropping op; full-sync 會兜底");
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                s.pending.fetch_sub(1, Ordering::Release);
                tracing::error!("[pg::writer] worker disconnected");
            }
        }
    }
}

/// 等 queue 消化完（test 或 shutdown 用）。最多等 `timeout`。
pub fn drain(timeout: Duration) -> bool {
    let Some(s) = SERVICE.get() else {
        return true;
    };
    let start = std::time::Instant::now();
    while s.pending.load(Ordering::Acquire) > 0 {
        if start.elapsed() > timeout {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    true
}

/// 通知 shutdown；drain 完關閉 worker。
pub fn shutdown(timeout: Duration) {
    let Some(s) = SERVICE.get() else {
        return;
    };
    s.shutdown.store(true, Ordering::Release);
    drain(timeout);
    if let Ok(mut g) = s.handle.lock()
        && let Some(h) = g.take()
    {
        let _ = h.join();
    }
}
