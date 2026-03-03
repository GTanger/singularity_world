-- 奇點世界第一版 schema，對齊人物角色模板與第一版可做清單 §1.8.3。
-- entities：玩家／NPC 共用；event_log：觀測與坍縮用事件日誌。
-- soul_seed：創角時寫入，唯一決定該角色之三軸光譜與 361 拓撲 760 條邊權（見人物屬性彙整 §2.0、361拓撲系統規格 §6.1）。
-- display_title：命途稱謂，空則前端顯示「無名之輩」（邏輯閉環 §4.4）。
-- activated_nodes：星盤已貫通節點 ID 清單，預設僅 N000；前端依此顯示貫通／迷霧（邏輯閉環 §4.2）。

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
	gender TEXT,
	soul_seed INTEGER,
	display_title TEXT,
	activated_nodes TEXT,
	equipment_slots TEXT,
	inventory TEXT DEFAULT '[]'
);

CREATE TABLE IF NOT EXISTS event_log (
	id INTEGER PRIMARY KEY AUTOINCREMENT,
	at INTEGER NOT NULL,
	entity_id TEXT NOT NULL,
	event_type TEXT NOT NULL,
	payload TEXT
);

-- 傳統 MUD 房間機制：節點連接節點
-- tags: JSON 陣列，如 ["inn","social"]，供 NPC 尋路決策
-- zone: 所屬區域名稱，如「浮生客棧」「東城」
CREATE TABLE IF NOT EXISTS rooms (
	id TEXT PRIMARY KEY,
	name TEXT NOT NULL,
	description TEXT NOT NULL DEFAULT '',
	tags TEXT NOT NULL DEFAULT '[]',
	zone TEXT NOT NULL DEFAULT ''
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

-- 物品定義表（裝備分頁規格 §五、背包規格 §四、§六）
CREATE TABLE IF NOT EXISTS items (
	id TEXT PRIMARY KEY,
	name TEXT NOT NULL,
	slot TEXT NOT NULL,
	item_type TEXT NOT NULL DEFAULT 'equipment',
	weight REAL NOT NULL DEFAULT 0,
	stackable INTEGER NOT NULL DEFAULT 0,
	denomination INTEGER NOT NULL DEFAULT 0,
	description TEXT NOT NULL DEFAULT '',
	attributes TEXT,
	tokens TEXT
);

-- 玩家登入密碼（僅 kind=player 有列；決策 006 選項甲）
CREATE TABLE IF NOT EXISTS entity_auth (
	entity_id TEXT PRIMARY KEY,
	password_hash TEXT NOT NULL,
	FOREIGN KEY (entity_id) REFERENCES entities(id)
);

-- NPC 排班表：上班在 work_room、下班移動到 rest_room；shift 可跨午夜
CREATE TABLE IF NOT EXISTS npc_schedules (
	entity_id TEXT PRIMARY KEY,
	work_room TEXT NOT NULL,
	rest_room TEXT NOT NULL,
	shift_start INTEGER NOT NULL,
	shift_end INTEGER NOT NULL,
	FOREIGN KEY (entity_id) REFERENCES entities(id),
	FOREIGN KEY (work_room) REFERENCES rooms(id),
	FOREIGN KEY (rest_room) REFERENCES rooms(id)
);
