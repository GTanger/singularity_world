# NPC 之間交互行為

> 說明：同房或跨房的 NPC 與 NPC 之間目前有哪些互動、如何觸發、與設計上的邊界。

---

## 一、現有實作

### 1.1 微互動（同房兩人隨機敘事）

**模組**：`db/npc_social.go`

- **行為**：當同一房間內有 **≥2 個 NPC** 時，可隨機選兩名產生一句「微互動」敘事，例如：
  - 【A】朝【B】點了點頭。
  - 【A】與【B】閒聊了幾句。
  - 【A】看了【B】一眼，沒有說話。
  - 【A】對【B】笑了笑。
  - 【A】低聲問【B】：「知道哪裡有活幹嗎？」
  - 【A】向【B】抱怨道：「唉，日子不好過啊。」
  - 【A】和【B】並肩站著，望向遠方。
  - 【A】拍了拍【B】的肩膀。

- **觸發**：main 迴圈中的**閒置 tick**（約每 60～120 個 tick 一次）。只對「**有玩家在的房間**」依序嘗試；某房呼叫 `PickMicroInteraction(roomID, 15)` 若回傳非空（15% 機率 × 該房 NPC≥2），就廣播該句並 **break**，本輪只觸發一次。

- **效果**：該房所有連線玩家會收到 `narrate` 訊息，log 顯示上述 ambient 敘事。

- **資料**：不寫入 DB／記憶；純敘事推送。NPC 名由 `GetEntitiesInRoom` 取同房 `kind=npc` 的 `DisplayTitle`（或 ID）。

### 1.2 排班／腦驅動造成的「同房」

- **排班**：有排班的 NPC 依 `gameHour` 前往 `work_room` 或 `rest_room`，可能與其他排班或腦驅動 NPC **自然同房**（例如都在浮生客棧大廳）。同房是移動結果，不是專門的「NPC 找 NPC」邏輯。
- **腦驅動**：無排班 NPC 依決策引擎（需求→意圖→尋路）移動，到達後有「對場所」的行為（乞討、採集、求職等），**沒有**「對另一 NPC」的專門動作（不選目標 NPC 說話／交易／攻擊）。

因此：**同房**已能透過微互動產生「兩人之間」的敘事；**誰和誰同房**則由排班＋腦驅動＋巡邏決定，沒有「為了和某 NPC 互動而移動」的設計。

---

## 二、設計上已支援、但未由 NPC 主動觸發的部份

### 2.1 戰鬥（NPC vs NPC）

- **規則**：戰鬥統一規則（[決策 001](decisions/001_combat_unified_rules.md)），**玩家 vs NPC**、**NPC vs NPC**、**玩家 vs 玩家** 皆用同一套 `combat.ResolveV2` 與屬性（含地形、γ 暴擊／偏轉）。
- **現狀**：Attack 流程在 `server/handler.go` 由**玩家**發起（`c.PlayerID` 為攻擊方、`msg.TargetID` 為目標）。**沒有**「NPC 主動對另一 NPC 攻擊」的 tick 或事件（例如仇恨、搶地盤、隨機衝突等）。

### 2.2 對話（NPC 對 NPC 說話）

- **已實作（NPC＝玩家）**：閒置 tick 時，有玩家在的房內若有 ≥2 NPC，**優先**用 Ollama 產生「A 對 B 說一句、B 回一句」的 AI 對話（`ai.CallAITalkNPCToNPC`，帶 A/B 背版、房間、時段）；成功則廣播「【A】對【B】說：「…」【B】說：「…」」。Ollama 未配置或呼叫失敗時，fallback 為原有微互動（`PickMicroInteraction`）。
- **仍無**：多輪 NPC 間對話、或寫入任一 NPC 的 archival／記憶。

### 2.3 交易（NPC 與 NPC）

- **現狀**：Trade 插座存在，但交易流程以**玩家↔NPC**為主。
- **沒有**：NPC 與 NPC 之間自動交易、以物易物、付錢等邏輯。

---

## 三、總結對照

| 類型           | 是否實作 | 觸發方式 | 備註 |
|----------------|----------|----------|------|
| 同房微互動     | ✅       | 閒置 tick、有玩家在的房、15% 機率、NPC≥2 | 隨機兩人、固定句型；Ollama 失敗時 fallback |
| NPC 對 NPC 對話（AI）| ✅ | 閒置 tick、有玩家在的房、NPC≥2、Ollama 已配置 | 優先：CallAITalkNPCToNPC → 一來一往兩句；失敗則微互動 |
| 同房（排班/腦）| ✅       | 排班目標房、腦意圖目標房 | 間接造成「多 NPC 同房」 |
| NPC vs NPC 戰鬥| ⬜       | 設計統一規則，但無 NPC 主動攻擊 | 可擴充：仇恨／事件驅動 |
| NPC 與 NPC 交易| ⬜       | 無 | 可擴充：經濟／需求驅動 |

---

## 四、相關檔案

- **微互動**：`db/npc_social.go`（`PickMicroInteraction`、`getNPCNamesInRoom`）；`main.go` 閒置 tick（NPC-NPC AI 優先，失敗則微互動）。
- **NPC-NPC AI 對話**：`ai/talk.go`（`CallAITalkNPCToNPC`）、`main.go` 閒置 tick（同房兩 NPC、BuildIdentity、房間名、時段 → Ollama 一來一往）。
- **排班／移動**：`db/schedule.go`、`db/npc_movement.go`、`main.go`（TravelerManager、ApplySchedules、腦驅動抵達敘事）。
- **戰鬥**：`combat/combat.go`、`server/handler.go`（Attack 分支、`buildAttackNarrative`）。
- **對照**：`docs/第一版可做清單.md` 10.20（NPC 間互動）、`docs/implementation/NPC活化系統—實作清單與實作計畫.md`（F 微互動 ✅）。
