# 房間管理：新增、修改、刪除

## 方式一：Web 管理頁（建議）

啟動伺服器後，在瀏覽器開啟 **`/admin.html`**（例：`http://localhost:8080/admin.html`）：

- **新增房間**：填 ID、名稱、描述後送出
- **編輯房間**：點該房間的「編輯」，改名稱或描述後儲存
- **刪除房間**：點「刪除」（lobby 不可刪），房內的人會自動移到大廳
- **新增出口**：填「從房間 ID、出口代號（東／天／101…）、目標房間 ID」後送出
- **刪除出口**：點該出口旁的 ✕

---

## 方式二：直接編輯 JSON（進階）

房間與出口以 **JSON 為唯一數據源**，位於 `data/rooms/`（一房一檔 `&lt;id&gt;.json`）。每檔含該房 id、name、description、tags、zone 與其 **exits**（direction、to）。實體所在房間在 `data/runtime/entity_rooms.json`。建議日常用 **Web 管理頁**；需批次或版控時再直接改 JSON。

### 出口代號（direction）可自訂

- **傳統 MUD**：用 東、西、南、北 表示四方連接。
- **同層多房**：同一走廊接多間房時，代號用「房間代稱」即可。例如客棧二樓走廊接八間包廂，出口代號可設 **天、地、玄、黃、日、月、星、辰**，分別連到天字房、地字房…；或使用 **101、102** 等編號。
- 遊戲內路徑按鈕會顯示**目標房間名稱**（如「天字房」），代號僅供系統辨識，不限定東西南北。

### 視野與空間約定（決策 005）

**一格 ＝ 一間房 ＝ 玩家視線所及的空間。** 玩家所在房間即為「當前格」；視線範圍即該房間，**視野內實體**＝與玩家同房之實體。詳見 [decisions/005_room_cell_view_scope.md](decisions/005_room_cell_view_scope.md)。

---

## 1. 新增房間（JSON 範例）

在 `data/rooms/` 新增一檔 `新房間id.json`：

```json
{
  "id": "新房間id",
  "name": "顯示名稱",
  "description": "房間描述文字。",
  "tags": [],
  "zone": "區域名",
  "exits": [
    { "direction": "西", "to": "lobby" }
  ]
}
```

大廳 `lobby.json` 的 `exits` 需手動加入 `{ "direction": "東", "to": "新房間id" }`。重啟伺服器後生效。

---

## 2. 修改房間

直接編輯 `data/rooms/&lt;id&gt;.json` 的 name、description、tags、zone 或 exits，存檔後重啟。

---

## 3. 刪除房間

刪除 `data/rooms/&lt;id&gt;.json`；並在其它房間的 `exits` 中移除指向該 id 的出口。`data/runtime/entity_rooms.json` 內若有人在此房，需改到 lobby 或其它房。重啟後生效。

---

## 4. 房間來源

房間來自 `data/rooms/*.json`，啟動時由 store 載入。新增、修改、刪除請用 **Web 管理頁**（/admin.html）或直接編輯 JSON 後重啟。
