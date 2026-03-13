# 玩家視角 — NPC 與房間互動

**本文件為已收斂之文檔，可依憑實作及成果驗收；後續若有新增內容再行修改。**

> 本文件從**玩家在 UI 上的體驗**描述：進入房間格後看見什麼、點擊 NPC 會發生什麼、各動作的預期結果。後台雖採「玩家＝NPC＝實體」同一套模型，並不違背；此處僅規範**玩家端**所見與所為。

**對齊**：[決策 005](decisions/005_room_cell_view_scope.md)（一格＝一房＝視線所及）、[決策 002](decisions/002_sockets_plugs_semantic.md)（插頭插座）、[NPC 活化系統](../NPC活化系統.md)、[房間非人物件互動](../房間非人物件互動.md)、[NPC有嘴—設計與實作規劃](../implementation/NPC有嘴—設計與實作規劃.md)、[交易—從對話到交易與面板設計](../implementation/交易—從對話到交易與面板設計.md)。

---

## 一、進入房間格：玩家看見什麼

**一格 ＝ 一間房 ＝ 玩家視線所及的空間**（決策 005）。玩家進入某房間後，伺服器推送 **room_view**（協定類型 `view`），前端依此更新畫面。

| 區塊 | 內容 | 資料來源 |
|------|------|----------|
| **房間名稱** | 當前房間名稱（如「浮生客棧」「四路街口」） | `RoomViewMsg.room_name` |
| **房間描述** | 該房間的敘述文字；內含 **〔可互動物件名〕** 可點擊 | `RoomViewMsg.description`；`objects` 與描述內 〔〕 對應 |
| **出口** | 可選方向（東、西、南、北等），點擊即移動 | `RoomViewMsg.exits[]`（direction, to_room_id, to_room_name） |
| **人物欄（實體列表）** | **與玩家同房**的所有實體（NPC、其他玩家），不含自己 | `RoomViewMsg.entities[]` |
| **同房物件** | 房間內可互動物件（與描述中 〔〕 對應） | `RoomViewMsg.objects[]`（id, name, actions） |
| **遊戲時間** | 日曆與時鐘顯示 | `RoomViewMsg.game_time_*` |

**視野內實體**＝與玩家**同一格／同一房**的實體；僅同房者會出現在人物欄並可被點選互動。

---

## 二、人物欄：誰出現、顯示什麼、點擊行為

### 2.1 誰會出現在人物欄

- **同房 NPC**：與玩家在同一房間的 NPC，由後端依 `entity_room` 查詢後填入 `GetRoomView` → `RoomViewMsg.entities`。
- **同房其他玩家**：若有多人在同一房，亦列於人物欄（顯示為其 ID 或 display_name）；自己**不**出現在列表中。

### 2.2 每個實體顯示什麼

| 欄位 | 說明 |
|------|------|
| **顯示名稱** | NPC：有指派時為**職稱**（如「經理」「服務生」），由 `GetSocketsForNPC` 與指派推導；無指派時為 `display_title` 或 id。其他玩家：id 或 display_name。 |
| **外觀字元** | `DisplayChar`（若前端有渲染）。 |
| **不直接顯示** | 每個實體具備的**可執行動作**（插座）由後端傳給前端，用於觀看後在 Log 顯示「其他動作」。 |

後端對**非玩家**的實體會填 `ViewEntity.actions`：NPC 為 `GetSocketsForNPC(database, entityID, roomID)`（預設 Talk、Attack、Look；在場時加職業插座如 Trade）；玩家為該實體的 `Sockets()`。

### 2.3 點擊人物欄中的 NPC（或玩家）

- **預設行為**：點擊即送出 **do_action { entity_id: 該實體 id, action: "Look" }**。
- **不**在人物欄展開手風琴選單；先送觀看，其餘動作在 **Log 觀看敘事的下一行** 顯示（見 §三）。

前端實作要點（對齊房間非人物件互動）：

- 人物欄以列表呈現（例如 ▸ 名稱），`role="button"`、`tabindex="0"`，點擊／Enter／空格即送 Look。
- `title` 可設為「點擊觀看」。

---

## 三、點擊 NPC 後：觀看敘事與「其他動作」

### 3.1 流程

1. 玩家點擊人物欄某實體 → 前端送出 `do_action { entity_id, action: "Look" }`。
2. 後端檢查目標是否具備 Look 插座、是否同房等，執行觀看邏輯，回傳 **action_result**（含 `narrative`、`target_id`、`action: "Look"`）。
3. 前端將觀看敘事 **append 到 Log**。
4. 前端依 `target_id` 從 `state.entities`（或 `state.objects`）取得該目標的 **actions**，**排除 Look** 後，在敘事**下一行**顯示可點擊的【對話】【攻擊】【交易】等（依後端回傳之插座而定）。

### 3.2 Log 下一行的「其他動作」

- **來源**：`RoomViewMsg.entities[].actions` 已含該 NPC 在該房間的有效插座（如 `["Talk","Attack","Look"]` 或含 `Trade`）；前端從中篩掉 Look，依序顯示。
- **呈現**：例如【對話】【攻擊】或【對話】【攻擊】【交易】，每項為可點擊 span（`log-object-action`），`data-entity-id`、`data-action` 對應目標與動詞。
- **點擊**：玩家點【對話】→ 送 `do_action { entity_id, action: "Talk", player_input: "…" }`；點【攻擊】→ 送 Attack；點【交易】→ 送 Trade（若該 NPC 有 Trade 插座）。前端需在 actionLabels 中支援 **Trade → 「交易」**（與裝備／背包等用語一致）。
- **多輪延續**：第一輪 NPC 回覆後，前端應**再次顯示**可對話主題或輸入區（同一對話脈絡），玩家選／輸入即對**同一 NPC** 送第二輪、第三輪……，**不需**重新點人物欄或 Log 的【對話】。詳見 [NPC有嘴—設計與實作規劃](../implementation/NPC有嘴—設計與實作規劃.md) §5.6。

---

## 四、各動作的預期結果（玩家視角）

| 動作 | 玩家操作 | 預期結果 |
|------|----------|----------|
| **Look（觀看）** | 點人物欄該實體，或點 Log 中【觀看】 | Log 顯示該實體外觀敘事；同一行或下一行出現該目標的其餘可執行動作（對話、攻擊、交易等）。 |
| **Talk（對話）** | 點 Log 中【對話】 | 出現**可對話主題**＋「其他」；選主題或輸入後送 `player_input`，Log 顯示 NPC 回覆。**第二輪起**：NPC 回覆後**不需重複點 NPC 或【對話】**，同一段對話會再次顯示主題／輸入，玩家直接選或輸入即延續多輪；直到玩家選擇結束對話或點別人、離開。若有 Trade 插座且選了「想看看你有什麼」等，可能回傳 `open_trade: true` → 前端開交易面板。 |
| **Attack（攻擊）** | 點 Log 中【攻擊】 | 戰鬥結算，Log 顯示結果（決策 001 統一戰鬥規則）。 |
| **Trade（交易）** | 點 Log 中【交易】，或由對話意圖觸發 | 後端回傳商品清單或 `open_trade: true`；前端開**交易面板**（彈窗、手風琴陳列、詳述＋價格、可堆疊選數量），與裝備／背包同一套 UI 語言。 |

以上「可執行動作」是否出現，一律由**後端**依插頭插座與在場條件決定；前端僅依 `actions` 顯示對應按鈕／連結。

---

## 五、環境敘事（不經點擊）

與「點擊 NPC」無關、但同房時玩家會看到的內容：

- **閒置動作**：在班 NPC 每 5～12 秒可能推送一句閒置敘事（narrate），顯示在 Log，樣式為 ambient（灰色小字）。
- **進房反應**：玩家**進入**該房間時，同房 NPC 可能觸發進房反應（enter_reaction），延遲 0.5～1.5 秒後以 narrate 推送。
- **換班／巡邏**：排班型 NPC 出發或抵達時會推送敘事；區域巡邏時可能推送瞬移敘事。  
以上皆為 **narrate** 訊息，不改變人物欄列表；人物欄的更新來自 **room_view**（例如 NPC 移動後 `RefreshRoomViews` 重送 view）。

---

## 六、資料與協定摘要

| 項目 | 說明 |
|------|------|
| **視野範圍** | 決策 005：一格＝一房＝視線所及；視野內實體＝同房實體。 |
| **實體列表** | `game.GetRoomView` → `db.GetEntitiesInRoom(roomID)`；NPC 的 display_title 由指派推導（討論 001）。 |
| **插座** | NPC：`db.GetSocketsForNPC(database, entityID, roomID)`（預設 Talk/Attack/Look；在場時加職業 action_sockets 如 Trade）。 |
| **room_view 推送** | 登入後、移動後、NPC 移動後（RefreshRoomViews）推送 `view`；前端更新房間名、描述、出口、**entities**、objects、遊戲時間。 |
| **do_action** | 前端送 `entity_id` + `action`；Talk 必帶 `player_input`。 |
| **action_result** | 含 `narrative`、`target_id`、`action`；若為 Talk 且後端要開交易，可帶 `open_trade: true`。 |

---

## 七、引用（檢索）文檔

以下為撰寫與收斂本文件時檢索之全專案相關設計文檔；實作與驗收時可依此對照。

### 決策

| 文件 | 用途 |
|------|------|
| [決策 001 戰鬥規則統一](../decisions/001_combat_unified_rules.md) | Attack 插頭插座、戰鬥結算、Log 結果 |
| [決策 002 插頭／插座語義](../decisions/002_sockets_plugs_semantic.md) | 名詞插座、動詞插頭、可執行性由後端判斷、Agent 預設插座 |
| [決策 004 技術棧與架構](../decisions/004_tech_stack_architecture.md) | 視野內即時模擬、room_view 推送 |
| [決策 005 空間單位與視野範圍](../decisions/005_room_cell_view_scope.md) | 一格＝一房＝視線所及、視野內實體定義 |
| [決策 006 登入與玩家模板](../decisions/006_login_and_player_template.md) | 玩家實體、登入後 room_view |
| [決策 007 NPC AI API 與預設使用規則](../decisions/007_NPC_AI_API與預設使用規則.md) | CallAITalk、對話池、fallback、use_ai_for_talk |

### 核心設計（房間／視野／互動）

| 文件 | 用途 |
|------|------|
| [NPC 活化系統](../NPC活化系統.md) | NPC 行為能力、互動能力（Look/Talk/Attack）、移動、Talk/Trade 階段、room_view／narrate、程式碼速查 |
| [房間非人物件互動](../房間非人物件互動.md) | 點擊即觀看、Log 下一行其他動作、人物欄與描述物件一致、RoomViewMsg.Objects、〔〕標記 |
| [rooms_manage](../rooms_manage.md) | 房間與出口資料、視野與空間約定（對齊 005） |

### 對話與交易

| 文件 | 用途 |
|------|------|
| [NPC有嘴—設計與實作規劃](../implementation/NPC有嘴—設計與實作規劃.md) | Talk UI、可對話主題、player_input、多輪延續（§5.6）、AI/模板優先 |
| [交易—從對話到交易與面板設計](../implementation/交易—從對話到交易與面板設計.md) | 從對話到交易、open_trade、交易面板（彈窗／手風琴）、雙入口 |
| [經濟彙整](經濟彙整.md) | 玩家↔NPC 交易、店舖庫存 vs 背包、Trade 插座語義 |

### 身份／職業／實體

| 文件 | 用途 |
|------|------|
| [討論 001 身份與職業分離](../discussions/001_身份與職業分離—角色無身份綁定.md) | 職稱自指派推導、在場才開放職業插座、公共動作 |
| [NPC生成流程調整—依討論001](../implementation/NPC生成流程調整—依討論001.md) | 插座與在場對接、GetSocketsForNPC、display_title fallback |
| [人物角色模板](人物角色模板.md) | 玩家/NPC 共用欄位、識別與插座 |

### 對話記憶（第四階段）

| 文件 | 用途 |
|------|------|
| [對話記憶系統—彙整與探討](對話記憶系統—彙整與探討.md) | 背版＋archival、Talk 時檢索與寫入、共識與最適作法 |
| [NPC對話記憶與背版—設計](../implementation/NPC對話記憶與背版—設計.md) | 背版 blocks、archival、與 Talk 接點 |
| [NPC對話記憶與背版—實作步驟與檔案流程](../implementation/NPC對話記憶與背版—實作步驟與檔案流程.md) | 記憶系統實作步驟與檔案清單 |

### 實作清單與驗收

| 文件 | 用途 |
|------|------|
| [第一版可做清單](../第一版可做清單.md) | §六 插頭插座、§十 NPC 行為、§十一 房間物件；前端點擊即觀看、do_action → action_result |
| [NPC活化系統—實作清單與規劃](../implementation/NPC活化系統—實作清單與規劃.md) | 有嘴／有記憶階段、I3／I5、GetSocketsForNPC、驗收子項 |

### 其他參考

| 文件 | 用途 |
|------|------|
| [三軸推導性格—實作規劃](../implementation/三軸推導性格—實作規劃.md) | 對話抽句權重（Boldness 等） |
| [裝備分頁規格](裝備分頁規格.md) | 彈窗／分頁 UI 語言（與交易面板一致性） |
| [背包規格](背包規格.md) | 彈窗 UI 語言（與交易面板一致性） |
| [房間與建築設計規範](房間與建築設計規範.md) | 房間描述、objects 與 〔〕 對應 |
| [向陽大街—環境概念與風格討論](向陽大街—環境概念與風格討論.md) | 描述內 objects、sockets、Move、點擊觸發移動 |

---

*奇點世界專案 — 玩家視角：NPC 與房間互動 v1（已收斂）*
