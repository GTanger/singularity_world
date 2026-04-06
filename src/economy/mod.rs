// economy 模組 — 經濟 tick 與鎂轉帳門面，對齊既有 economy/。

use std::time::Duration;

/// 將 `from_id` 的 `amount` 鎂轉給 `to_id`；實作於 `db`（store）。對齊既有 `economy.TransferMagnesium`。
pub fn transfer_magnesium(from_id: &str, to_id: &str, amount: i32) -> anyhow::Result<()> {
    crate::db::transfer_magnesium(from_id, to_id, amount)
}

/// 在背景 task 中以固定間隔執行 `on_tick`（例：產出事件、更新價格）；第一版 `on_tick` 可為空閉包。
/// 對齊既有 `economy.Run`（背景執行緒 + ticker）；此處使用 `tokio::spawn` + `interval`。
pub fn run(interval: Duration, on_tick: impl Fn() + Send + 'static) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            on_tick();
        }
    });
}
