-- 測試用：建立 player1，位於區塊 0_0 內可通行格 (75,8)，避免卡牆（原 75,75 易在牆/建築內）。
INSERT OR IGNORE INTO entities (id, kind, display_char, x, y, move_state, vit, qi, dex, magnesium, created_at)
VALUES ('player1', 'player', '我', 75, 8, 'idle', 10, 10, 10, 0, unixepoch());
-- 若 player1 已存在且卡牆，可手動執行： UPDATE entities SET x=75, y=8 WHERE id='player1';
