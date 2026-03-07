# DB 轉 JSON 遷移規劃 — 全專案走 JSON、拆碎利於搜尋

> 產出日期：2026-02-12  
> 目標：**全專案以 JSON 為唯一數據源**，拆碎為多檔利於搜尋與維護；執行期載入記憶體，**不再使用 DB 檔**。

---

## Store 的定義

**Store** = 以 JSON 為唯一數據源的記憶體層：

- **載入**：啟動時從指定 JSON 檔與目錄（如 `data/rooms/`、`data/runtime/`、`data/*.json`）載入全部資料到記憶體。
- **執行期**：遊戲邏輯只讀寫記憶體中的 store（`store.Default`）；db 層的 GetRoom、GetEntity、SetEntityRoom 等皆改為由此提供。
- **持久化**：變更時原子寫回對應 JSON（先寫 `.tmp` 再 `Rename`），不寫入任何 DB 檔。
- **結論**：本專案不再開啟或依賴 SQLite；所有該拆、該寫、該生成的資料皆由 JSON 載入／寫回 JSON。

---

## 一、為什麼 AI 常預設生成 DB？

- **訓練資料**：多數教學、範例、生產專案都用關聯式 DB（Postgres、MySQL、SQLite），模型學到「持久化 = 用資料庫」。
- **通用場景**：DB 擅長多使用者、交易、查詢、索引；問「怎麼存資料」時，最常見答案就是 DB。
- **JSON 在範例中的角色**：多被當成「設定檔、i18n、靜態資料」，較少被當成「唯一數據源」；模型較少在「文字遊戲、單機、背板驅動」情境下學到「全 JSON」。
- **結論**：不是 JSON 不好，而是 AI 預設偏向 DB；你的情境（文字遊戲、可搜尋、你掌控內容）用 JSON 拆碎更合適。

---

## 二、原則

- **靜態／背板**：一房一檔、一實體一檔等，利於搜尋與版本控制。
- **動態**：由程式即時或定期覆寫（如 `entity_rooms.json`、`event_log.json`）；可與靜態同目錄樹，以路徑或命名區分。
- **執行期**：啟動時將所需 JSON 全部載入記憶體；運行中只讀寫記憶體，必要時再寫回 JSON（原子寫入）。

---

## 三、JSON 目錄與拆碎規劃

```
data/
├── rooms/                    # 一房一檔（已實作；store.Init("data/rooms", ...) 從此目錄載入）
│   ├── lobby.json
│   ├── life_hall.json
│   └── ...
├── entities.json             # 實體清單（store 讀寫）
├── assignments.json          # 指派清單（store 讀寫）
├── schedules.json            # 排班清單（store 讀寫）
├── venues.json               # 場所清單（store 讀寫）
├── items.json                # 物品定義（store 讀寫）
├── runtime/                  # 動態、由程式覆寫
│   ├── entity_rooms.json     # 誰在哪房
│   ├── event_log.json        # 事件日誌
│   └── auth.json             # 玩家密碼雜湊
├── npc_behaviors.json
├── room_objects.json
└── templates/
    ├── occupations.json
    ├── archetypes.json
    ├── dialogues/
    └── behaviors/
```

---

## 四、遷移階段

| 階段 | 內容 | 產出 |
|------|------|------|
| **Phase 1** | 房間＋出口＋尋路改為從 JSON 載入，pathfind 與 GetRoom 改讀 store | ✅ store 層、BuildGraphFromStore、db/room 讀 store |
| **Phase 2** | entity_room 改為 JSON（runtime/entity_rooms.json），SetEntityRoom 寫入 store 並原子寫回 | ✅ 已實作 |
| **Phase 3** | venues、schedules、assignments 改為 JSON 載入與寫回 | ✅ data/venues.json、assignments.json、schedules.json |
| **Phase 3+** | entities 改為 JSON 載入與寫回 | ✅ data/entities.json；GetEntity/InsertEntity/InsertNPC 等皆走 store |
| **Phase 4** | event_log、items、auth 改 JSON | ✅ data/items.json、data/runtime/event_log.json、auth.json |
| **Phase 5** | 房間拆檔、移除 DB | ✅ data/rooms/*.json 一房一檔；main 不再 OpenDB；無 SyncFromDB，全由 JSON 載入／寫回 |

---

## 五、技術要點

- **原子寫入**：先寫 `*.json.tmp`，成功後 `os.Rename` 覆蓋原檔。
- **Store 單例**：啟動時 `store.Init("data/rooms", "data/runtime", "data")` 從目錄 `data/rooms/`（一房一檔）及 data/runtime、data 載入；`db.GetRoom` / `GetEntityRoom` / `SetEntityRoom` 等皆經由 `store.Default`，main 不開啟 DB。
- **打破 import cycle**：`Room`、`Exit` 型別放在 `model` 包，`store` 只依賴 `model`，不依賴 `db`；`db` 可依賴 `store`。
- **無 DB**：本專案不以 SQLite 為數據源；所有資料由 JSON 載入、寫回 JSON。

---

## 六、殘餘 DB 檢查（已處理）

| 項目 | 狀態 |
|------|------|
| **實體 .db 檔** | 專案內不納入 .db；data/ 下若有 world.db 已被 .gitignore。備份檔 *.db.*.bak 不追蹤。 |
| **main** | 已不呼叫 OpenDB；`database` 為 nil，建目錄改為固定 `data`、`data/runtime`，不依賴 cfg.DBPath。 |
| **cmd/wipe_entities** | 已改為 store.Init + store.ClearAllEntities，不再 OpenDB。 |
| **db 包** | 仍接受 `*sql.DB` 參數以相容呼叫方；執行期 store.Default != nil 時一律走 store，傳 nil 不觸碰 DB。 |
| **測試** | db/schedule_test、npc_activation_simulation_test 使用 temp 目錄的 test.db/sim.db，僅測試用，保留。 |
| **文件** | README、rooms_manage、NPC 相關已改為「JSON/store、不開 DB」。config 仍含 DBPath 欄位（未使用，可保留或日後移除）。 |

---

*DB 轉 JSON 遷移規劃 v1.1 — 全 JSON、房間拆檔、無 DB*
