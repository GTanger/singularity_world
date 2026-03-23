# 奇點世界（Singularity World）

遊戲專案：**奇點世界**  
英文專案名：`singularity_world`

**設計核心：從有限框架延伸出無限可能。**  
體驗端以手機使用情境為主，現階段無上架規劃。

---

## 專案現況

- 後端：Go 服務（`main.go`），以 JSON/store 作為執行期資料來源。
- 主要資料：`data/rooms/editor/`（一房一檔）、`data/entities.json`、`data/runtime/*`。
- 前端：文字遊戲主頁 + 管理工具頁（地圖檢視器、房間編輯器、星圖、管理頁）。
- 浮生城資料已擴展至 `1F~11F`：民居樓層、電梯、公共機能層與多格動線均已落檔。

---

## 快速啟動

### 一鍵啟動（建議）

在專案根目錄執行：

```bash
./start
```

用途：建置並啟動奇點服務（預設使用 1721）。

### 奇點 + Chatmery 一起啟動

```bash
./start-with-chatmery
```

安裝 user-level 開機自啟：

```bash
./start-with-chatmery --install
```

systemd user 操作（詳見 `docs/開機啟動.md`）：

```bash
systemctl --user start sw-with-chatmery
systemctl --user stop sw-with-chatmery
systemctl --user restart sw-with-chatmery
systemctl --user status sw-with-chatmery
```

### 手動建置與執行

```bash
go build -o bin/server .
./bin/server
```

指定埠：

```bash
PORT=1721 ./bin/server
```

---

## 常用路由（本機）

- 遊戲主頁：`/`
- WebSocket：`/ws`
- 地圖檢視器：`/map_viewer`
- 房間編輯器：`/room_editor`
- 星圖：`/star_chart`
- 管理頁：`/admin`
- 房間資料 API：`/data/rooms.json`
- 房編 API：`/api/room-editor/*`

---

## 主要目錄

```text
singularity_world/
├── main.go
├── config/             # 伺服器與遊戲參數（含預設埠）
├── server/             # HTTP/WS 路由、session、room editor API
├── game/               # 遊戲主循環與行為流程
├── entity/             # 實體與屬性模型
├── economy/            # 經濟與資源流轉
├── combat/             # 戰鬥判定與事件
├── event/              # 事件資料處理
├── store/              # JSON/store 載入與儲存
├── data/
│   ├── rooms/          # 房間資料（遞迴載入，主體在 rooms/editor）
│   ├── runtime/        # 執行期快照（編輯器座標、NPC 狀態等）
│   └── entities.json
└── web/                # index/map_viewer/room_editor/star_chart/admin
```

---

## 房間資料約定

- 一房一檔，實際來源以 `data/rooms/` 遞迴掃描為準。
- 主編輯目錄：`data/rooms/editor/`。
- `id` 必須全域唯一，`exits[].to` 必須指向存在的房間 `id`。
- `objects` 可提供 `Move` 與 `move_to_room_id` 以支援物件式移動。
- 地圖檢視器顏色依 `zone` 分組；浮生城已使用分層 zone（如 `citylife_4f`、`citylife_11f`）。

---

## 文件導覽

- 文檔索引：`docs/文檔索引.md`
- 協作約定：`docs/COLLABORATION.md`
- 技術約束：`docs/技術約束規則.md`
- 第一版可做清單：`docs/第一版可做清單.md`
- 世界觀主參考：`docs/reference/世界觀：Token降維與生命演化.md`

決策檔位於 `docs/decisions/`，現有主題包含：
- 戰鬥統一規則
- 插頭插座語意
- 技術棧與架構
- 空間與視野
- 登錄與玩家模板
- NPC/AI API 與預設規則
- 架構整頓規劃
- 觀測分級與行程約束

---

*奇點世界專案*
