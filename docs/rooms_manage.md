# 房間管理：新增、修改、刪除

## 方式一：Web 管理頁（建議）

啟動伺服器後，在瀏覽器開啟 **`/admin.html`**（例：`http://localhost:8080/admin.html`）：

- **新增房間**：填 ID、名稱、描述後送出
- **編輯房間**：點該房間的「編輯」，改名稱或描述後儲存
- **刪除房間**：點「刪除」（lobby 不可刪），房內的人會自動移到大廳
- **新增出口**：填「從房間 ID、出口代號（東／天／101…）、目標房間 ID」後送出
- **刪除出口**：點該出口旁的 ✕

---

## 方式二：SQL 命令列

房間與出口存在 SQLite 的 `data/world.db`，也可用命令列或任一 SQL 工具操作。

## 資料表

| 表 | 說明 |
|----|------|
| `rooms` | id（主鍵）, name, description |
| `exits` | from_room_id, **direction**（出口代號）, to_room_id（一筆一連接） |
| `entity_room` | 實體所在房間（entity_id → room_id） |

### 出口代號（direction）可自訂

- **傳統 MUD**：用 東、西、南、北 表示四方連接。
- **同層多房**：同一走廊接多間房時，代號用「房間代稱」即可。例如客棧二樓走廊接八間包廂，出口代號可設 **天、地、玄、黃、日、月、星、辰**，分別連到天字房、地字房…；或使用 **101、102** 等編號。
- 遊戲內路徑按鈕會顯示**目標房間名稱**（如「天字房」），代號僅供系統辨識，不限定東西南北。

---

## 1. 新增房間

```sql
-- 新增一間房
INSERT INTO rooms (id, name, description) VALUES
  ('新房間id', '顯示名稱', '房間描述文字。');
```

**記得加出口**（雙向要各寫一筆）。`direction` 可為 東/西/南/北 或任意代號（如 天、地、101）：

```sql
-- 傳統四方：從大廳往東到新房間
INSERT INTO exits (from_room_id, direction, to_room_id) VALUES ('lobby', '東', '新房間id');
INSERT INTO exits (from_room_id, direction, to_room_id) VALUES ('新房間id', '西', 'lobby');

-- 同層多房例：二樓走廊接天字房（代號「天」）
-- INSERT INTO exits (from_room_id, direction, to_room_id) VALUES ('inn_2f_hall', '天', 'inn_2f_room_tian');
-- INSERT INTO exits (from_room_id, direction, to_room_id) VALUES ('inn_2f_room_tian', '走廊', 'inn_2f_hall');
```

---

## 2. 修改房間

```sql
-- 改名稱與描述（id 不變）
UPDATE rooms SET name = '新名稱', description = '新描述。' WHERE id = '房間id';
```

---

## 3. 刪除房間

先處理**出口**與**實體所在房間**，再刪房間：

```sql
-- 1) 刪掉所有與此房有關的出口
DELETE FROM exits WHERE from_room_id = '房間id' OR to_room_id = '房間id';

-- 2) 把還在這間房的人移到大廳（或其它房）
UPDATE entity_room SET room_id = 'lobby' WHERE room_id = '房間id';

-- 3) 刪除房間
DELETE FROM rooms WHERE id = '房間id';
```

---

## 4. 用命令列執行

專案目錄下：

```bash
# 進入 sqlite3（資料庫路徑依你實際的 data/world.db）
sqlite3 data/world.db
```

在 sqlite3 裡貼上上面的 SQL，或把 SQL 存成檔再用：

```bash
sqlite3 data/world.db < 你的.sql
```

---

## 5. 預設房間一覽（SeedRooms）

程式第一次建立房間時會寫入（見 `db/room.go` SeedRooms）：

| id | name |
|----|------|
| lobby | 大廳 |
| east_street | 東街 |
| west_alley | 西巷 |
| south_plaza | 南廣場 |

**注意**：SeedRooms 只在 `rooms` 表為空時執行；之後新增、修改、刪除都要自己用 SQL（或寫管理工具）處理。
