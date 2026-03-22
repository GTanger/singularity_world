# 房間資料 (data/rooms)

**封存說明**：重構地圖前，原房間檔案已移至 **`data/rooms_archive/`**（界壁、浮生大街、向陽大街、夜鴞巷、打鐵巷、梧桐大街、飛霜大街、飛霜湖等）。霜林已自 archive 遷回本目錄並刪除舊版封存。本目錄供新地圖使用。

一房一檔：每個 `.json` 對應一個房間。檔名不限，以內容的 `id` 為準。**檔名以 `_` 開頭的 JSON 不會被載入**，僅供參考。

---

## 檔案格式約定

| 用途 | 格式 | 說明 |
|------|------|------|
| **一房一檔**（本目錄下各 `.json`） | **段落** | 使用 2 空格縮排、換行（pretty-print），方便閱讀與編輯。 |
| **合併檔**（`data/rooms.json`） | **一房一行** | 由腳本從一房一檔合併產生；陣列內每個房間為一行，方便快速捲動瀏覽、比對描述與用詞。 |

- 一房一檔為來源，維持段落格式。
- 合併檔列版控，需更新時執行：`node tools/js/merge-rooms-one-per-line.js`
- 若一房一檔格式跑掉，可執行：`node tools/js/format-rooms-pretty.js` 統一重排為段落。

---

## id 規則與出口融合

- **id 規則**：前綴第一層 = zone 的英文代碼（例：浮生大街→lifestreet）；前綴第二層 = 該建築/場所的英文名（例：綠意別墅→green）。同一建築內所有格子的 id 皆為 `zone_place_` 開頭，例：`lifestreet_green_大廳`、`lifestreet_green_r1`。
- **出口融合**：`exits` 保留完整供 NPC 尋路；可選 `ui_hidden: true` 讓前端不顯示按鈕。玩家改由描述內 `〔物件名〕` 點擊觸發移動，該物件需在 `objects` 中且設 `move_to_room_id`。見 `docs/房間非人物件互動.md` §2.3。

---

## 欄位說明

### 根層

| 欄位 | 型別 | 必填 | 說明 |
|------|------|------|------|
| **id** | string | ✅ | 房間唯一識別碼，全域不可重複。用於出口 `to`、尋路、實體所在房間等。 |
| **name** | string | ✅ | 房間顯示名稱，玩家與介面會看到。 |
| **description** | string | ✅ | 房間敘述。可用 **〔方括號〕** 標出可互動物件名稱，須與 `objects[].name` 一致，供前端辨識可點擊物件。 |
| **tags** | string[] | 選 | 標籤陣列，用於篩選（如 `street`、`indoor`、`inn`）。可空陣列 `[]`。 |
| **zone** | string | 選 | 所屬區域名稱（如「浮生大街」「向陽大街」），用於分區與第二層目錄分類。 |
| **exits** | array | ✅ | 出口列表，見下方。 |
| **objects** | array | 選 | 房內可互動物件（Look/Read/Smell 等），見下方。可省略。 |

### exits[] 每個出口

| 欄位 | 型別 | 說明 |
|------|------|------|
| **direction** | string | 出口方向／選項文字，供 NPC 尋路與對應。 |
| **to** | string | 目標房間的 **id**，須與某個房間的 `id` 一致。 |
| **ui_hidden** | boolean | 選填。若為 `true`，前端不將此出口渲染為按鈕（出口欄已移除後仍可保留，供未來或 Debug 用）。玩家改由描述內 `〔物件名〕` 點擊觸發移動。 |

**說明**：`exits` 一律保留完整，供 NPC 尋路；玩家移動改由描述中的可點擊物件（`objects` 內具 `Move` 或 `Look`+`Move`）觸發，見 `docs/房間非人物件互動.md` §2.3。

### objects[] 每個可互動物件

| 欄位 | 型別 | 說明 |
|------|------|------|
| **id** | string | 物件唯一識別碼，全域不可重複。建議格式：`房間id_物件簡稱`。 |
| **name** | string | 物件顯示名稱，**須與 description 中 〔〕 內文字一致**，前端才能對應。 |
| **owner** | string | 所屬場所或擁有者標記，可留空 `""`。 |
| **sockets** | string[] | 該物件支援的動詞，如 `["Look", "Read", "Smell"]`。導航用：巷道／路段可僅 `["Move"]`；建築門戶可 `["Look", "Move"]` 或 `["Look", "Enter"]`。 |
| **responses** | object | 動詞 → 回應文字。key 為動詞名（須在 `sockets` 內），value 為玩家執行該動詞時顯示的敘事。 |
| **move_to_room_id** | string | 選填。當此物件可觸發移動時，填目標房間的 **id**；後端對該物件執行 `Move` 時依此切房。須與本房 `exits[].to` 之一對應。 |

---

## 目錄結構約定

- 第一層：依 **zone** 分資料夾（如 `浮生大街/`、`向陽大街/`）。
- 第二層：依專案腳本或手動依「建築／區域」分子資料夾（如 `客棧/`、`民一/`），同一 zone 的 hub 房（name 等於 zone 的房間）通常放在 zone 根目錄。
- 實際載入時會遞迴掃描所有 `.json`，**檔名以 `_` 開頭的 JSON 會被略過**，不會當成房間載入。

---

## 完整範本（複製後改 id / name / description / exits / objects）

```json
{
  "id": "zone_place_房間代碼",
  "name": "房間顯示名稱",
  "description": "房間敘述。可互動物件用〔名稱〕標記，與 objects[].name 一致。出口可寫進描述（例：延伸向〔大街三段〕的坡道、左側〔焦黑木門〕），對應 objects 中具 Move 或 Look+Move 的物件。",
  "tags": ["tag1", "tag2"],
  "zone": "所屬區域名稱（與 id 第一層前綴對應）",
  "exits": [
    {
      "direction": "出口方向（供 NPC 尋路與對應）",
      "to": "目標房間的 id",
      "ui_hidden": true
    }
  ],
  "objects": [
    {
      "id": "zone_place_房間代碼_object_1",
      "name": "物件顯示名稱（須與 description 中 〔〕 內一致）",
      "owner": "所屬場所或留空",
      "sockets": ["Look", "Read", "Smell"],
      "responses": {
        "Look": "玩家對該物件執行 Look 時的回應文字。",
        "Read": "玩家對該物件執行 Read 時的回應文字。"
      }
    },
    {
      "id": "zone_place_房間代碼_exit_1",
      "name": "大街三段",
      "sockets": ["Move"],
      "move_to_room_id": "zone_place_目標房間id",
      "responses": {
        "Move": "你朝坡道走去。"
      }
    }
  ]
}
```

### 簡短範例（浮生大廳）

```json
{
  "id": "life_hall",
  "name": "浮生大廳",
  "description": "挑高的大廳中央懸著一盞〔六角宮燈〕，櫃台後方掛著〔匾額〕。",
  "tags": ["inn", "lobby"],
  "zone": "浮生大街",
  "exits": [
    { "direction": "庭院", "to": "life_garden" },
    { "direction": "食堂", "to": "life_dining" }
  ],
  "objects": [
    {
      "id": "life_hall_lantern",
      "name": "六角宮燈",
      "owner": "inn",
      "sockets": ["Look"],
      "responses": {
        "Look": "你抬頭端詳那盞六角宮燈。..."
      }
    }
  ]
}
```
