-- 奇點世界第一版 schema，對齊人物角色模板與第一版可做清單 §1.8.3。
-- entities：玩家／NPC 共用；event_log：觀測與坍縮用事件日誌。

CREATE TABLE IF NOT EXISTS entities (
	id TEXT PRIMARY KEY,
	kind TEXT NOT NULL,
	display_char TEXT NOT NULL,
	x INTEGER NOT NULL,
	y INTEGER NOT NULL,
	move_state TEXT NOT NULL,
	target_x INTEGER,
	target_y INTEGER,
	walk_or_run TEXT,
	move_started_at INTEGER,
	vit INTEGER NOT NULL,
	qi INTEGER NOT NULL,
	dex INTEGER NOT NULL,
	magnesium INTEGER NOT NULL,
	last_observed_at INTEGER,
	created_at INTEGER NOT NULL,
	gender TEXT
);

CREATE TABLE IF NOT EXISTS event_log (
	id INTEGER PRIMARY KEY AUTOINCREMENT,
	at INTEGER NOT NULL,
	entity_id TEXT NOT NULL,
	event_type TEXT NOT NULL,
	payload TEXT
);

-- 傳統 MUD 房間機制：節點連接節點
CREATE TABLE IF NOT EXISTS rooms (
	id TEXT PRIMARY KEY,
	name TEXT NOT NULL,
	description TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS exits (
	from_room_id TEXT NOT NULL,
	direction TEXT NOT NULL,
	to_room_id TEXT NOT NULL,
	PRIMARY KEY (from_room_id, direction),
	FOREIGN KEY (from_room_id) REFERENCES rooms(id),
	FOREIGN KEY (to_room_id) REFERENCES rooms(id)
);

-- 實體當前所在房間（與 entities 並存，房間制時以此為準）
CREATE TABLE IF NOT EXISTS entity_room (
	entity_id TEXT PRIMARY KEY,
	room_id TEXT NOT NULL,
	FOREIGN KEY (entity_id) REFERENCES entities(id),
	FOREIGN KEY (room_id) REFERENCES rooms(id)
);

-- 玩家登入密碼（僅 kind=player 有列；決策 006 選項甲）
CREATE TABLE IF NOT EXISTS entity_auth (
	entity_id TEXT PRIMARY KEY,
	password_hash TEXT NOT NULL,
	FOREIGN KEY (entity_id) REFERENCES entities(id)
);
