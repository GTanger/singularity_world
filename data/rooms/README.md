# 房間資料 (data/rooms)

一房一檔：每個 `.json` 對應一個房間。檔名不限，以內容的 `id` 為準。**檔名以 `_` 開頭的 JSON（如 `_template.json`）不會被載入**，僅供參考或複製使用。

---

## 房間生成模板

複製 `_template.json` 並重新命名（例如 `my_room.json`），再依下方欄位說明填寫。

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
| **direction** | string | 出口方向／選項文字（玩家輸入或點選的內容）。 |
| **to** | string | 目標房間的 **id**，須與某個房間的 `id` 一致。 |

### objects[] 每個可互動物件

| 欄位 | 型別 | 說明 |
|------|------|------|
| **id** | string | 物件唯一識別碼，全域不可重複。建議格式：`房間id_物件簡稱`。 |
| **name** | string | 物件顯示名稱，**須與 description 中 〔〕 內文字一致**，前端才能對應。 |
| **owner** | string | 所屬場所或擁有者標記，可留空 `""`。 |
| **sockets** | string[] | 該物件支援的動詞，例如 `["Look", "Read", "Smell"]`。 |
| **responses** | object | 動詞 → 回應文字。key 為動詞名（須在 `sockets` 內），value 為玩家執行該動詞時顯示的敘事。 |

---

## 目錄結構約定

- 第一層：依 **zone** 分資料夾（如 `浮生大街/`、`向陽大街/`）。
- 第二層：依專案腳本或手動依「建築／區域」分子資料夾（如 `客棧/`、`民一/`），同一 zone 的 hub 房（name 等於 zone 的房間）通常放在 zone 根目錄。
- 實際載入時會遞迴掃描所有 `.json`，**檔名以 `_` 開頭的 JSON 會被略過**，不會當成房間載入。

---

## 範例片段

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
