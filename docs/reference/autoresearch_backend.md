# 在線對話篩選（Autoresearch 概念）

靈感來自 [autoresearch 式「生成→評分→汰留」迴圈](https://github.com/andyluo7/autoresearch)。在本專案中，這條迴圈**只存在於遊戲伺服器運行時、在後台執行**——**不另稿離線批次**、不為同一套邏輯再開第二條管線（含 Python 或獨立 CLI）。

> **一句話**：維持 NPC 在**背景**（含玩家未旁聽的 NPC↔NPC）**持續對話**；後台以 **Rust 伺服器**（模擬迴圈觸發、`ai/scorer` 篩選、`db` 寫記憶與傳聞等）**去蕪存菁**，讓通過的內容進入世界；下一輪再**反哺**到對話 **prompt**——主要是 **`ai/talk` 組裝**＋**即時讀出的**記憶／摘要／主題／傳聞上下文，**不是**另開程式自動改 `data` 裡的靜態台詞檔（那些仍由人改版本時手動維護）。

### 與名為「autoresearch」的上游專案（僅概念辨析）

開源 **autoresearch**（如 [Karpathy／nanochat 系](https://github.com/karpathy/autoresearch) 思路）做的是：**單機 GPU 上訓練小語言模型**——代理改 `train.py`、固定**時間預算**（例如約 5 分鐘）、指標 **val_bpb**（愈低愈好）、人類寫 `program.md` 給代理讀。那是 **「練模型」**，不是 **「跑遊戲 NPC 對話」**。

奇點世界**不**把該倉庫當子模組、**不**在專案內跑其 `train.py`／`prepare.py`。我們借用的只是**迴圈隱喻**：嘗試（生成一句）→ 量測（規則分數）→ 保留或丟棄 → 長期累積；**量測對象**是 **NPC 對話能否進世界**，不是訓練損失。

---

## 核心理念

- **遊戲本身就是實驗場**：NPC 每一輪符合條件的背景對話，就是一次「生成→評分→保留或丟棄」；**不需要**另開腳本或平行行程。
- **機制**：對話經 LLM 產出後，在寫入 archival／summary／傳聞鏈之前，經 **qualityGate**、**錨點一致性**，再經 **規則評分**；**高分才寫入世界**，低分丟棄，避免汙染長期記憶。
- **眾口鑠金（長期）**：能留下來的句子堆疊成傳聞、關係與話題；下一輪 prompt 帶入的記憶較乾淨，**有機會形成正向循環**——但**驗收仍以玩家體感為準**，不是分數儀表板。
- **模型會換，規則資產可沿用**：換 Ollama 小模型時，同一套規則門檻仍可迭代；精煉重點在 **`ai/scorer`** 與 **`ai/talk`** 的 prompt；**後端為 Rust**，不以 Python 作正式依賴。

---

## 技術取向：Rust 後端

- **管線**：`ai/talk`（Ollama `/api/chat`）、模擬迴圈內 `tryTriggerNpcNpcInRoom`、`ai/scorer` 規則評分。
- **不改動的設計邊界**（維持模組責任）：原則上不為「篩選」去改 **配對邏輯、topic 選擇、thread 狀態機**；**`store/` 儲存結構**不因本機制而變；prompt 大結構以 **`ai/talk`** 為準、迭代時再調。

---

## 一人運維：不重開第二條線、靠重啟載入

**沒有熱重載**：改程式或設定後，**重啟遊戲伺服器**才會生效。  
**持久化**：寫入 `data/runtime` 等 JSON 的內容，**下次啟動載入**——向來如此；**不是**「線上一套、線下再跑一套」。  
一人運維時，**不預設**邊玩邊開額外批次程式。

---

## 設計意圖：NPC 即世界的說書人

由 **NPC 告訴玩家這世界長怎樣**——風聞、人情、時段與張力從對話長出來，而非只靠靜態文案。  
概念設計者也可從 **NPC 口中採句**，得到下一階段系統／劇情的靈感（被運轉中的世界**頂回來**）。

因此 **prompt、規則、本地模型** 都**沒有封閉最終版**，只有持續演進；與 [NPC活化系統.md](../NPC活化系統.md) §零點五一致。  
**本地 LLM**：`config/config` 的 `OllamaModel`，環境變數 **`OLLAMA_MODEL`** 覆寫即可試新小模型。

---

## 驗收是什麼

**玩家在玩時聽到、看到的 NPC 對話體感，就是驗收。**  
好聽、像活人、不洗版、不尷尬＝過關；反之＝沒過。不依賴另開後台或額外面板當「正式驗收」。

### 產品原則：不是讀表遊戲

**盯儀表板、盯數字＝工作，不是玩。** 奇點世界是**文字 MUD**，驗收在**敘事與耳中體感**，不在「把遊玩變成讀表」。  
後端的統計、debug API、分數明細**只屬開發／除錯**，**不**當成玩家主循環、**不**把產品設計成「要看儀表才算會玩」。

---

## 實作對齊（程式）

**流程**（`tryTriggerNpcNpcInRoom`）：

```
LLM → sanitize → qualityGateNpcLine → anchorConsistencyCheck → `ai.ScoreNpcDialogue` → 通過才寫入摘要／archival／傳聞錨點等
```

- **檔案**：`ai/scorer`（`DialogueScoreDetail`、`ScoreNpcDialogue`）、`main`（嵌入點、門檻、`npcSocialStats`）、`npc/topics`（`FindNpcNpcTopicIDByHint` 反查主題 id）。
- **門檻**：預設總分 ≥ **35**；**`NPC_DIALOGUE_SCORE_THRESHOLD`**：正數＝自訂門檻；**`-1` 或 `off`**＝關閉規則評分（僅保留 qualityGate 等前段）。
- **與 qualityGate 關係**：先擋格式／明顯髒字串；評分器為**第二道防線**（含 poison 否決等）。
- **去重**：`db.RecentNpcNpcArchivalLinesForEntity` 取近期 npc_npc 對白本體，與本輪台詞比對重複／高相似則扣分。
- **廢土漂移（輕懲）**：`tone_drift`——若合併台詞含典型廢土／末世劇本詞（如「廢土」「荒蕪」「死城」「核戰」「世界末日」等），**−10**（命中一項即扣，不重複累加）；**不**懲本作用語如惡地、霜林、輻射雨、游離輻射。見 `wastelandTonePenalty`。
- **配對**：兩 NPC **真名相同**時配對分數降權（避免摘要難區分），與討論稿一致。
- **純規則**：評分不另占 GPU；延遲可忽略。

**除錯（非驗收、非玩法）**：`/api/debug/npc-social`、統計鍵、`last_choice.dialogue_score` 等**僅**供開發／除錯；與「讀表遊戲」無關，見上節。

---

## 評分維度（概念）

正向維度上限合計 **80**（Length 15＋Anchor 20＋Relation 10＋Diversity 10＋DialogueFeel 15＋Identity 10），懲罰維度（Repeat、Narration、ToneDrift）均為負數或 0；髒輸出等可 **KilledBy** 直接否決。維度含（實作以程式為準）：長度、現場錨定、關係一致性、重複懲罰、多樣性、對話感、人設一致性、旁白懲罰、**tone_drift（廢土漂移，非正維度，為 0 或 −10）**。
**單一真相來源**：**`ai/scorer`**；改規則時改該檔與本文件敘述。

---

## 預期效果（敘事層）

1. **短期**：明顯垃圾句較不易進入長期記憶，惡性循環較易被切斷。  
2. **中期**：記憶與摘要相對乾淨，下一輪帶入的上下文較穩。  
3. **長期**：NPC 在門檻篩選下累積出**可辨識的鎮上聲口與傳聞**——「眾口鑠金」是**意象**，不是要你盯儀表板。

---

## 與世界觀敘事的銜接（沃土風、富態、非廢土）

前述「眾口鑠金」若**口徑跑偏**，玩家聽到的仍是別的世界——因此 **prompt 與規則**必須和 [世界觀：富態與拉鋸](世界觀：富態與拉鋸.md)（§沃土風、§二體型範例等）**同一條敘事線**：

| 層級 | 責任 | 說明 |
|------|------|------|
| **Prompt（世界現象級）** | `data/templates/llm_prompts.json` → `WorldPhenomenaCognitionPrompt()`，由 `CallAITalk`、`CallAITalkNPCToNPC` **每次**拼入 system | **次次植入**；文案**維護在 JSON**，後端僅載入與拼接。見 [世界觀認知與對話prompt](世界觀認知與對話prompt.md)。 |
| **規則評分** | `ai/scorer` | 以**現場錨定、重複、人設、毒字串**為主；並有 **`tone_drift`**（`wastelandTonePenalty`）抑制典型廢土／末世劇本詞，**不**懲惡地／輻射雨等本作用語。 |
| **靜態資料** | `data/templates/dialogues/*.json` | 人維護；**不**由 autoresearch 管線自動改寫。 |

**與「讀表遊戲」無關**：世界觀對齊是**敘事品質**，不是給玩家多看一欄分數。

---

## 下一步（建議優先順序）

1. **Prompt 已注入基調**：`data/templates/llm_prompts.json`（含 `world_phenomena_cognition`）；改動後重啟，`CallAITalk`／`CallAITalkNPCToNPC` 即讀新文案。  
2. **試玩驗收**：同一房、同時段多觸發幾輪 NPC↔NPC，耳中是否仍像**本世界**（必要時縮短 JSON 內基調句，避免擠壓 token）。  
3. **✅ 已做**：`ai/scorer` 的 **`tone_drift`**（`wastelandTonePenalty`，−10）；關鍵詞清單可隨試玩再調，避免誤殺。  
4. **✅ 已做**：`data/npc_to_npc_topics.json` 新增 **城外聞**、**天候**（富態威脅、輻射雨、惡地／霜林，對齊沃土風）。  
5. **✅ 已做**：`topicMaskForRoom` 帶入房間 `tags`；`db.topicMaskBaseWeight` 在 **`outdoor`** 時提高「城外聞」「天候」抽中權重（露天街道更易談城外與天候）。  
6. **✅ 已做**：`sentimentDeltaFromDialogue` 對主題 id **城外聞**、**天候**（及對應 hint 後備）+1 關係傾向；`topicMaskBaseWeight` 對房間 tag **social** 提高「閒聊」權重。  
7. **✅ 已做**：`db.FindNpcNpcTopicIDByHint`——由當前 `topicHint`（含 thread 續談存的 `TopicType`、玩家在場加註）**反查主題 id**，讓 `chosenTopicID` 與 sentiment／debug 一致。  
8. **持續**：依試玩調整 `wastelandTonePenalty` 清單與主題 `hint` 長度；可增其他房間 tag 與主題對照。

---

## 參考與延伸閱讀

- [世界觀認知與對話prompt](世界觀認知與對話prompt.md)（層 0 次次植入、與區域／職業／密傳分工）  
- [世界觀：富態與拉鋸](世界觀：富態與拉鋸.md)（沃土風、豐饒與危險同向）  
- [NPC活化系統.md](../NPC活化系統.md) §零點五  
- [NPC間對話—記憶與情境完整設計.md](../design/NPC間對話—記憶與情境完整設計.md)  
- [決策 007：NPC AI 與本地模型](../decisions/007_NPC_AI_API與預設使用規則.md) §7  

若需**一次性**清除歷史 npc_npc 污染資料，請以當前 `tools/` 內維護腳本或資料修復流程為準，完成後**重啟**伺服器；非日常流程。
