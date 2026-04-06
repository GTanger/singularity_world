// 遊戲主迴圈 tick，對齊既有 game/loop。

use std::time::Duration;

/// 以固定間隔在背景 task 中呼叫 `on_tick`；對齊既有 `game.Loop`（blocking ticker 改為 tokio interval）。
pub fn spawn_loop(interval: Duration, on_tick: impl Fn() + Send + 'static) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            on_tick();
        }
    });
}
