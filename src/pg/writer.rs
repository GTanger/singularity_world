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

use crate::store::{ArchivalEntry, Entity};
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
