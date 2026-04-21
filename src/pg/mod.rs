//! PG 直寫模組——繞過 store RwLock、直接走 `Arc<DbPool>` 做 IO。
//!
//! 地基原則：**Store 是 in-memory cache、不摸 IO；PG 操作走獨立 pool 不走 store lock**。
//! 舊架構把純 PG CRUD 塞進 `impl Store` 害 writer 拿著 store write lock 做 blocking
//! PG query，login 路徑的 reader 排在 writer 隊伍後面卡住。
//!
//! 所有 function 接受 `&DbPool` 參數、不接觸 store。高頻寫入（append_event、set_auth、
//! NPC rumor digest 等）全部從這裡走、store lock 天生不涉入。

use crate::store::sql::DbPool;
use std::sync::Arc;
use parking_lot::RwLock;

pub mod auth;
pub mod entity;
pub mod event;
pub mod rumor;
pub mod sync;
pub mod writer;

/// 全域 DbPool — 獨立於 store，任何地方可直接取用不經 store lock
static DB_POOL: RwLock<Option<Arc<DbPool>>> = RwLock::new(None);

/// 設定全域 pool（server 啟動時呼叫一次）
/// 同步啟動 writer service（集中 PG 寫入 queue）和 full-sync 兜底服務
pub fn set_pool(pool: DbPool) {
    {
        let mut g = DB_POOL.write();
        *g = Some(Arc::new(pool.clone()));
    }
    writer::init(pool.clone());
    sync::start(pool);
}

/// 取得全域 pool 的 Arc clone（便宜，Arc 內部 refcount）
pub fn pool() -> Option<Arc<DbPool>> {
    DB_POOL.read().clone()
}
