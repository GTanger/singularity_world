//! Periodic full-sync — cache→PG 一致性兜底。
//!
//! 理由：writer queue 滿時 submit 採 try_send 丟棄策略（避免阻塞 store lock holder）。
//! 丟棄會造成 cache 和 PG 短暫漂移——本 service 每 interval 把 in-memory entities
//! 全量寫回 PG，把累積的漂移消化掉。
//!
//! 實作走 background thread + 直接打 DbPool（不經 writer queue），
//! 不跟玩家 hot path 爭資源。

use crate::store::sql::DbPool;
use std::sync::OnceLock;
use std::thread::JoinHandle;
use std::time::Duration;

static STARTED: OnceLock<JoinHandle<()>> = OnceLock::new();

/// 啟動 periodic full-sync（server 啟動時呼叫一次）。
/// 預設 30 秒週期，可由環境變數 `SW_PG_SYNC_INTERVAL_SEC` 覆寫。
pub fn start(pool: DbPool) {
    let _ = STARTED.get_or_init(|| {
        let interval_sec = std::env::var("SW_PG_SYNC_INTERVAL_SEC")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30);
        let interval = Duration::from_secs(interval_sec.max(5));
        tracing::info!("[pg::sync] periodic full-sync 啟動，週期 {}s", interval.as_secs());
        std::thread::Builder::new()
            .name("pg-full-sync".into())
            .spawn(move || loop {
                std::thread::sleep(interval);
                run_once(&pool);
            })
            .expect("spawn pg-full-sync thread")
    });
}

fn run_once(pool: &DbPool) {
    use crate::store;
    let Some(arc) = store::get_store() else {
        return;
    };
    // 短暫 read lock 拿 snapshot；clone 完立刻 drop lock（不會擋 writer 久）
    let snapshot: Vec<crate::store::Entity> = {
        let s = arc.read();
        s.entities.values().cloned().collect()
    };
    let n = snapshot.len();
    let t0 = std::time::Instant::now();
    let mut ok = 0usize;
    let mut fail = 0usize;
    for e in &snapshot {
        match super::entity::upsert(pool, e) {
            Ok(_) => ok += 1,
            Err(err) => {
                fail += 1;
                tracing::debug!("[pg::sync] upsert {} failed: {}", e.id, err);
            }
        }
    }
    let dt = t0.elapsed();
    tracing::info!(
        "[pg::sync] full-sync {n} entities: ok={ok} fail={fail} took={}ms",
        dt.as_millis()
    );
}
