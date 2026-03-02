# NPC 模板系統 — 檢索文檔

> 最後更新：2026-03-03

---

## 目錄結構

```
data/
├── npc_behaviors.json          ← 現有定點 NPC 行為文本（經理、服務生），伺服器啟動時載入
├── rooms.json                  ← 房間定義檔
└── templates/                  ← NPC 原型 & 模板系統（本資料夾）
    ├── README.md               ← 本文件
    ├── archetypes.json         ← 職業原型總表：10 種職業的基礎屬性
    ├── dialogues/              ← 對話模板：每個職業一個 JSON
    │   ├── merchant.json       ← 商人
    │   ├── blacksmith.json     ← 鐵匠
    │   ├── scholar.json        ← 學者
    │   ├── traveler.json       ← 旅人
    │   ├── drunkard.json       ← 酒客
    │   ├── beggar.json         ← 乞丐
    │   ├── guard.json          ← 守衛
    │   ├── herbalist.json      ← 藥師
    │   ├── performer.json      ← 賣藝人
    │   └── farmer.json         ← 農夫
    └── behaviors/              ← 行為模板：每個職業一個 JSON
        ├── merchant.json
        ├── blacksmith.json
        ├── scholar.json
        ├── traveler.json
        ├── drunkard.json
        ├── beggar.json
        ├── guard.json
        ├── herbalist.json
        ├── performer.json
        └── farmer.json
```

---

## 1. archetypes.json — 職業原型總表

定義每種 NPC 職業的**基礎屬性**，是生成 NPC 個體的「種子模板」。

| 欄位 | 說明 |
|------|------|
| `name` | 職業中文名 |
| `dialogue_file` | 指向 `dialogues/` 下的對話模板 |
| `behavior_file` | 指向 `behaviors/` 下的行為模板 |
| `spawn_weight` | 生成權重（越高越常見），決定城鎮中該職業 NPC 的比例 |
| `wander_radius` | 最大遊走半徑（房間跳數） |
| `trade_tendency` | 交易傾向，引擎讀取用 |
| `wealth_range` | `[min, max]` 初始財富範圍 |
| `goods_pool` | 該職業可能攜帶的物品池 |

### 現有 10 種原型

| ID | 名稱 | 權重 | 遊走 | 財富 | 特徵 |
|----|------|------|------|------|------|
| `merchant` | 商人 | 15 | 3 格 | 80~600 | 精明、愛談價格、走南闖北 |
| `blacksmith` | 鐵匠 | 8 | 1 格 | 120~900 | 粗獷、以手藝為傲、很少離開鍛造坊 |
| `scholar` | 學者 | 5 | 2 格 | 30~300 | 書卷氣、好奇心重、常分心 |
| `traveler` | 旅人 | 12 | 8 格 | 20~400 | 風塵僕僕、見多識廣、行蹤不定 |
| `drunkard` | 酒客 | 10 | 2 格 | 5~150 | 醉醺醺、口齒不清、時而傷感時而豪放 |
| `beggar` | 乞丐 | 6 | 4 格 | 0~20 | 謙卑、世故、偶有洞見 |
| `guard` | 守衛 | 8 | 2 格 | 50~200 | 嚴肅、盡責、不做交易 |
| `herbalist` | 藥師 | 4 | 3 格 | 60~500 | 溫和、熟悉藥草、談論藥方 |
| `performer` | 賣藝人 | 3 | 5 格 | 10~100 | 張揚、戲劇化、渴望關注 |
| `farmer` | 農夫 | 10 | 2 格 | 15~200 | 樸實、務實、關心天氣與收成 |

---

## 2. dialogues/*.json — 對話模板

每個職業的**所有語句**，用於 NPC 與玩家互動時隨機抽取。

### 結構

```json
{
  "greet":           { "lines": [...] },        // 玩家進房時 NPC 招呼（8+ 句）
  "idle": {
    "morning":       [...],                      // 早晨閒置動作敘述（8+ 句）
    "noon":          [...],                      // 中午
    "evening":       [...],                      // 傍晚
    "night":         [...]                       // 深夜
  },
  "talk":            { "lines": [...] },        // 玩家主動交談時回應（8+ 句）
  "trade_announce": {
    "buy":           [...],                      // NPC 主動收購喊價（8+ 句）
    "sell":          [...]                       // NPC 主動販賣喊價（8+ 句）
  }
}
```

### 佔位符

| 佔位符 | 替換為 |
|--------|--------|
| `{name}` | NPC 個體真名（如「王富貴」） |
| `{goods}` | 該 NPC 販賣的品類（從 `goods_pool` 隨機抽取） |

### 格式約定

- NPC 名稱用 `【】` 包裹：`【{name}】`
- 台詞用 `「」` 包裹
- 動作描述不加引號，直接以第三人稱敘述

---

## 3. behaviors/*.json — 行為模板

定義每種職業的**日程作息、巡邏路線、交易邏輯、性格參數**。

### 結構

```json
{
  "schedule": {
    "entries": [
      {
        "hour_start": 6,
        "hour_end": 8,
        "activity": "wake_up",
        "location_pref": ["home"],
        "description": "起床準備"
      }
    ]
  },
  "wander": {
    "preferred_locations": ["market", "tavern"],
    "wander_chance": 0.25,
    "max_distance": 5,
    "leave_texts": ["【{name}】往{dest}方向走去。"],
    "arrive_texts": ["【{name}】從{from}方向走來。"]
  },
  "trade": {
    "buy_markup": 0.7,
    "sell_markup": 1.4,
    "preferred_goods": ["布匹"],
    "refused_goods": [],
    "haggle_tolerance": 3,
    "haggle_texts": {
      "accept": [...],
      "reject": [...],
      "counter": [...]
    }
  },
  "personality": {
    "aggression": 0.1,
    "friendliness": 0.7,
    "curiosity": 0.5,
    "greed": 0.6,
    "bravery": 0.3,
    "reaction_to_combat": "flee",
    "reaction_to_theft": "alert_guard"
  }
}
```

### 日程 (schedule)

每個 `entry` 覆蓋一段時間：

| 欄位 | 說明 |
|------|------|
| `hour_start` / `hour_end` | 遊戲時鐘的起止小時 (0-23) |
| `activity` | 行為類型：`wake_up` / `trade` / `rest` / `sleep` / `patrol` / `socialize` / `beg` 等 |
| `location_pref` | 該時段偏好的地點 ID 列表 |
| `description` | 該時段的敘述，可含 `{name}` |

### 巡邏 (wander)

| 欄位 | 說明 |
|------|------|
| `preferred_locations` | 偏好的房間 ID 列表 |
| `wander_chance` | 每次 tick 的遊走機率 (0.0~1.0) |
| `max_distance` | 最大跳數 |
| `leave_texts` | 離開時的敘述（`{name}`, `{dest}`） |
| `arrive_texts` | 到達時的敘述（`{name}`, `{from}`） |

### 交易 (trade)

| 欄位 | 說明 |
|------|------|
| `buy_markup` | 收購倍率（< 1.0 = 壓價收購） |
| `sell_markup` | 販賣倍率（> 1.0 = 加價販賣） |
| `preferred_goods` | 偏好商品 |
| `refused_goods` | 拒絕商品 |
| `haggle_tolerance` | 最大議價次數 |
| `haggle_texts` | 議價時的台詞：`accept` / `reject` / `counter` |

### 性格 (personality)

五維參數 (0.0~1.0)：

| 欄位 | 說明 |
|------|------|
| `aggression` | 攻擊性：影響被挑釁時的反應 |
| `friendliness` | 友善度：影響招呼與對話態度 |
| `curiosity` | 好奇心：影響對玩家行為的關注度 |
| `greed` | 貪婪度：影響交易價格與議價彈性 |
| `bravery` | 勇氣：影響面對戰鬥時的反應 |
| `reaction_to_combat` | 遇戰反應：`flee` / `stand_ground` / `confront` / 敘述文字 |
| `reaction_to_theft` | 遇竊反應：`alert_guard` / `confront` / 敘述文字 |

---

## 4. 與現有系統的關係

| 檔案 | 用途 | 狀態 |
|------|------|------|
| `data/npc_behaviors.json` | **定點 NPC**（經理、服務生）的閒置/反應/換班文本 | 已上線，`db/behavior.go` 讀取 |
| `data/templates/` | **量產 NPC** 的原型 + 對話 + 行為模板 | 已建立，待引擎串接 |

**演進路線：**

1. **已完成** — 定點 NPC 活化（`npc_behaviors.json` → `db/behavior.go` → `main.go` game loop）
2. **已建立** — 模板檔案結構（本資料夾）
3. **下一步** — 模板讀取引擎（`db/archetype.go`），從模板 + SoulSeed 生成 NPC 個體
4. **之後** — 三層模擬架構（全模擬 / 輕量 / 統計），實現萬人 NPC

---

## 5. AI 內容生成指引

這些模板的格式設計為可直接交給 LLM（如 Gemini）擴充內容。

### 給 AI 的 Prompt 範例

```
請依照以下 JSON 格式，為「獵人」職業生成對話模板：
- greet: 8 句進房招呼
- idle: morning/noon/evening/night 各 8 句
- talk: 8 句交談回應
- trade_announce: buy/sell 各 8 句

規則：
1. 繁體中文
2. 【{name}】包 NPC 名
3. 「」包台詞
4. {goods} 替換商品類型
5. 獵人性格：沉默寡言、敏銳、熟悉山林、不善社交

（附上 merchant.json 作為格式範例）
```

### 新增職業步驟

1. 在 `archetypes.json` 新增原型條目
2. 在 `dialogues/` 新增對話 JSON（可用 AI 生成）
3. 在 `behaviors/` 新增行為 JSON（可用 AI 生成）
4. 重啟伺服器生效

---

## 6. 快速查閱表

### 各職業的對話模板句數統計

| 職業 | greet | idle (每時段) | talk | buy | sell | 總句數 |
|------|-------|-------------|------|-----|------|--------|
| 商人 | 8 | 8 | 8 | 8 | 8 | ~72 |
| 鐵匠 | 8 | 8 | 8 | 8 | 8 | ~72 |
| 學者 | 8 | 8 | 8 | 8 | 8 | ~72 |
| 旅人 | 8 | 8 | 8 | 8 | 8 | ~72 |
| 酒客 | 8 | 7 | 8 | 8 | 8 | ~70 |
| 乞丐 | 8 | 7 | 8 | 8 | 8 | ~70 |
| 守衛 | 8 | 8 | 8 | 8 | 8 | ~72 |
| 藥師 | 8 | 8 | 8 | 8 | 8 | ~72 |
| 賣藝人 | 8 | 8 | 8 | 8 | 8 | ~72 |
| 農夫 | 8 | 8 | 8 | 8 | 8 | ~72 |
| **合計** | | | | | | **~716** |

### 各職業的行為模板特徵

| 職業 | 日程條目 | 遊走機率 | 遊走距離 | 議價次數 | 遇戰反應 |
|------|---------|---------|---------|---------|---------|
| 商人 | 9 | 25% | 5 格 | 3 次 | flee |
| 鐵匠 | 9 | 8% | 2 格 | 2 次 | stand_ground |
| 學者 | 9 | 12% | 3 格 | 4 次 | flee |
| 旅人 | 9 | 65% | 8 格 | 3 次 | evaluate |
| 酒客 | 8 | 85% | 2 格 | 1 次 | swing_wildly |
| 乞丐 | 7 | 90% | 6 格 | 5 次 | flee |
| 守衛 | 9 | 15% | 2 格 | 0 次 | confront |
| 藥師 | 9 | 18% | 3 格 | 3 次 | flee |
| 賣藝人 | 8 | 50% | 5 格 | 2 次 | flee |
| 農夫 | 9 | 20% | 2 格 | 2 次 | flee |
