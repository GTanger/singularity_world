# NPC 間對話：記憶與情境——完整設計概念與預期結果

> 目的：回答「為什麼一開始像胡言亂語、跑久了是否自然變準」——在**完整實作**前提下，定義**分層記憶、情境約束、對話管線與驗收標準**。
> 定位：設計規格（可交給實作拆 phase），**不是**最小改動 patch 清單。
> 對齊：`docs/NPC活化系統.md`（含 **§零點五** 活化＝持續演進、無最終版、Ollama 模型可換）、`docs/reference/autoresearch_backend.md`、`docs/implementation/NPC對話記憶與背版—設計.md`、`docs/reference/NPC之間交互行為.md`。
>
> **v1.1 補充**（Go+SQLite 落地決策）：L1/L2 直接 SQLite 持久化；L0 in-memory 滑窗；Thread 最大 3 輪冷卻 300s；NPC-NPC 對話寫入雙方 archival；配對改為分數化選擇。
>
> **目前實作狀態（2026-03）**：已改為 **Go + JSON/store** 路線（非 SQLite）完成 P1~P5，並追加 P6~P11：`npc_thread.json` / `npc_dyad.json` / `npc_rumors.json` / `npc_rumor_digest.json`、分數化配對、L0 房間事件滑窗注入、L3 傳聞 top-K 注入與衰減、P5 品質門檻細分統計、P6 傳聞離線摘要批次、P6.5 來源分層（room_event/economy/spawn/job）與來源權重、P7 同來源配額與同文本冷卻去重、P8 被引用升權與長期未引用降權、P9 衝突傳聞反事實懲罰（降權 + 15 分鐘封鎖）、P10 debug 面板可觀測欄位（blocked/penalty/reason）、P11 debug reset 可選清空 rumor 動態訊號。  
> **玩家體感補強（無需調參）**：NPC↔NPC 台詞強制短句＋現場錨定（房間描述節錄注入）；玩家在場時略降隨機閒聊頻率、多帶一條傳聞並偏短閒聊主題。  
> **續**：房間 `tags` 注入 prompt；情境列去重；依選定主題從 `npc_to_npc_topics.json` 抽一句「口吻種子」；可選環境變數 `NPC_NPC_QUALITY_MAX_RUNES`、`NPC_NPC_SOCIAL_TICK_*`（未設則與內建預設相同）。  
> **玩家餘音**：同房玩家 Talk 後，節錄對白暫存於 session；待 NPC 閒聊可觸發時（Talk 後已逾 60s、4 分鐘內）將「餘音」列注入 NPC↔NPC prompt，讓路人閒聊可側面呼應剛才氣氛。

---

## 一、願景與範疇

### 1.0 設計哲學：誰在塑造世界

**單一玩家不足以「定義」整個世界**——再怎麼選擇，也只是鎮上的一條視線與一組行為；街道的節奏、職場的生態、流言從哪裡長出來、誰與誰變熟，都不是一人能獨力寫完的劇本。

**大量 NPC 在共享規則下反覆互動，才可能自然生成這個世界的樣貌**：移動與排班讓人同框，需求與經濟讓人競合，對話與記憶讓關係與話題**可累積**——鎮上因此會長出可辨識的**社會紋理**（誰常出現在哪、哪類場所話題多、謠言如何形變傳播），而不是每次 LLM 呼叫各自幻覺一個平行宇宙。

因此本設計的定位是：**世界不是一次性寫死的背景設定，而是「許多主體過日子」堆疊出來的；玩家走進其中一條時間線，既是觀測者也是擾動者。**  
技術上對應：**持久化狀態（現場、話題線、兩兩關係、社會傳聞等）+ 節流與管線**負責「世界的慣性」，LLM 負責在該慣性之下**說得像活人**——前者定義**可長成什麼樣**，後者決定**聽起來像不像**。

> **產品敘事（一句）**：世界不是劇本寫死的，是鎮上許多人過日子堆出來的；玩家走進其中一條時間線。

### 1.1 願景

同房 NPC 的對話應表現為：

1. **情境一致**：內容與當下房間、時段、剛發生事件、在場人物相符。  
2. **關係一致**：熟人有熟人語氣，陌生人有距離感；有過節者帶火藥味（可隱晦）。  
3. **時間連續**：同一對、同一房、短時間內可形成**話題線**（thread），而不是每句重置世界觀。  
4. **世界可累積**：玩得越久，**可檢索的社會事實與關係事實**越多，對話越「像活在同一個鎮上」，而非單次 prompt 幻覺。

### 1.2 範疇（本設計涵蓋）

- **NPC ↔ NPC** 口語對話（含一來一往與可擴充的多輪）。  
- **驅動對話的記憶子系統**（與玩家↔NPC 記憶**可分可合**，但語意上要對齊）。  
- **觸發與節流**（誰和誰說、多久說一次、優先級）。  

### 1.3 非目標（可列為後續 phase）

- NPC 為了聊天而**專程尋路**找某人（屬「社交意圖引擎」，本設計只預留 hook）。  
- 全服即時大語言模型「世界觀一致審查」（成本過高）；改以**結構化狀態 + 局部 LLM**達成。  

---

## 二、問題陳述（為何純 LLM + 薄記憶不會自動變準）

### 2.1 LLM 不具「時間積分」

模型**不會**因伺服器多跑幾小時就變聰明；每次呼叫獨立，品質完全取決於**當次上下文**與**抽樣隨機性**。  
「跑久了變準」在工程上必須等同於：**持久化狀態變豐富 + 每次呼叫餵入的約束變強**。

### 2.2 現況缺口（概念層，不限於某一版程式）

| 缺口 | 後果 |
|------|------|
| 配對近似隨機 | 缺乏共同前情，容易空談 |
| 每對僅極短、覆寫式摘要 | 無法形成多輪語意與因果 |
| 與「房間事件」弱連動 | 進出、換班、天氣、任務不進對話 |
| 與「關係／身分」弱約束 | 人設與親疏漂移 |
| 無「話題型態」分層 | 閒聊、打探、交接、衝突混成同一溫度 |

本設計即為補齊上表。

---

## 三、設計原則

1. **結構先於文采**：先決定「誰、為何、在何種關係下、談哪類事」，再交給 LLM 潤色台詞。  
2. **記憶可審計**：每條注入 LLM 的記憶應能追溯到**來源事件或摘要版本**，便於除錯與回放。  
3. **單房一貫、跨房可選**：優先保證「玩家眼前這間房」邏輯自洽；跨房傳聞可漸進引入。  
4. **成本可控**：不是每次對話都長 context；用**檢索 + 分層摘要 + 話題狀態機**控制 token。  
5. **失敗可退化**：LLM 失敗或品質檢查不過 → 模板句／微互動／沉默，避免洗版垃圾句。

---

## 四、分層模型：記憶與情境

建議採用**四層**（可由不同表或同一 store 不同 namespace 實作）：

### 4.1 L0：現場層（Ephemeral Situation）——每次對話必帶

**內容**（示例欄位）：

- `room_id`、`room_name`、`room_tags` / `zone`  
- `game_time`（時段標籤 + 可選具體鐘點）  
- `present_entities`：在場 id 列表 + 顯示名 + kind（npc/player）  
- `recent_room_events`（滑動窗口，例如最近 **60～120 秒**或最近 **N 條**）：  
  - 誰從哪個方向進房／離房（若敘事已生成）  
  - 排班「出發／抵達」類敘事（可抽象成事件型別）  
  - 玩家可見的顯著動作（可選）

**預期效果**：對話會提到「剛走的那個」「從浮生那邊過來的」等，**降低與現場脫節**。

### 4.2 L1：話題層（Conversation Thread）——同一對 / 同一房小群

**不是**「一句摘要覆寫」，而是**有限狀態**：

- `thread_id`（例如 `pair:min(A,B)|room:R` 或 `clique:room:R:topic`）  
- `topic_type`（enum）：`gossip` / `work_handover` / `weather_smalltalk` / `player_reference` / `conflict` / `trade_intent` …（可對齊 `npc_to_npc_topics`）  
- `phase`：`opening` → `elaborate` → `closing`（或 2～3 輪固定）  
- `anchors`：本 thread 已確立的**不可否認事實**（短句列表，例如「約好去東邊茶館」「提到葉卅在找人」）  
- `last_turn_at`、`turn_count`、`cooldown_until`

**預期效果**：**同一對 NPC 連續 2～3 輪**會接續同一話題，而不是每輪重置；話題結束後才允許抽新 topic 或進入閒聊池。

### 4.3 L2：關係層（Dyadic & Clique Relation）

以**有序或無序對**為 key（無序對建議 canonical id）：

- `familiarity` 0～100（或離散：陌生／點頭之交／熟人／親近）  
- `sentiment` -100～100（或離散）  
- `last_interaction_at`、`interaction_count`  
- `tags`：`同職場` / `同街坊` / `曾口角` / `欠人情`（由規則或日後 LLM 摘要寫入，**需版本號**）

**預期效果**：語氣與稱謂穩定；「熟人」少試探、多省略；「有過節」帶刺但仍符合世界觀。

### 4.4 L3：情節與社會事實層（Society / Plot Memory）

較慢變、可跨房：

- **社會傳聞**（可衰減）：「東街茶館在徵人」「昨晚客棧鬧事」  
- **與玩家相關的公開線索**：僅當事件已進入「公開敘事」才允許 NPC 提及，避免無端 meta。  
- **職場／指派事實**：職稱、場所、班次（與既有 assignment / schedule 對齊）

**預期效果**：玩得越久，**鎮上的「公共話題池」**與**個人關係網**變厚，對話有「我們一直住在這」的感覺。

---

## 五、資料實體與持久化（概念 schema）

以下為**落地 schema**；實作後端為 Go + SQLite。

### 5.1 `npc_thread`（話題線）—— SQLite 持久化

| 欄位 | 類型 | 說明 |
|------|------|------|
| thread_key | TEXT PK | `canonical(A,B)` = 兩 ID 字典序 `\|` 連接（與現有 summaries key 格式相同） |
| topic_type | TEXT | enum：`gossip` / `work_handover` / `weather_smalltalk` / `player_reference` / `conflict` / … |
| phase | TEXT | `opening` / `elaborate` / `cooling` |
| anchors_json | TEXT | 字串陣列（P1-P2 留空；P3+ 由 LLM structured output 寫入） |
| turn_count | INTEGER | 累計輪數 |
| cooldown_until | INTEGER | Unix 秒；>0 時表示冷卻中，選配對時跳過 |
| updated_at | INTEGER | Unix 秒；供「idle > 90s 進入冷卻」判斷 |

**生命週期狀態機**：
```
不存在 → [選中此對] → active(phase=opening, turn=0)
active  → [對話成功] → turn+1；phase=elaborate (turn≥1)
active  → [turn=3 OR idle>90s] → phase=cooling, cooldown_until=now+300
cooling → [now > cooldown_until] → 刪除紀錄
```

### 5.2 `npc_dyad`（兩兩關係）—— SQLite 持久化

| 欄位 | 類型 | 說明 |
|------|------|------|
| a_id, b_id | TEXT PK | canonical 序（a_id < b_id 字典序） |
| familiarity | INTEGER | 0–100；初值：同職場 venue=30、同區域=10、陌生=0；每輪成功 +2 |
| sentiment | INTEGER | -100–100；依 topic_type 微調（conflict→-5、gossip→+1） |
| tags | TEXT | JSON 字串集合：`同職場` / `曾口角` / `欠人情` … |
| updated_at | INTEGER | Unix 秒 |

### 5.3 L0 `RoomEventWindow`（房間事件滑窗）—— in-memory

```go
type RoomEvent struct {
    At      int64   // Unix 秒
    Kind    string  // "enter" / "leave" / "shift" / "ambient"
    Subject string  // NPC 名稱
    Detail  string  // 如「從浮生方向」「往西街」
}
// map[roomID][]RoomEvent，每房保留最近 5 條、最長 120 秒
```

重啟後遺失可接受（僅影響 L0 現場感，不影響 L1/L2 持久資料）。

### 5.4 `npc_conversation_turn`（可選：完整審計）

若需重播與訓練資料：

- 每輪存 `speaker` / `listener` / `line` / `thread_key` / `model` / `prompt_hash`（隱私與容量需策略）

### 5.5 與現有 `npc_npc_summaries` 的關係

- **冷啟動種子**：若某對 NPC 無 `npc_thread` 記錄，用現有摘要初始化 thread 的第一筆 `anchors_json` 或作為 prompt 過往背景。
- `npc_npc_summaries` 不廢棄，保持目前覆寫邏輯；`npc_thread` 建立後兩者並行，thread 優先。

### 5.6 NPC-NPC 對話寫入 archival（新增）

每次 NPC-NPC 對話成功後，對話摘要寫入**雙方** `archival`（tag=`"npc_npc"`）：
- 寫入量受現有節流保護（時間窗 600s 內最多 3 條）
- `BuildIdentity` 已自動帶入最近事件，因此對話記憶會自然進入下次對話的身份背景

---

## 六、對話決策管線（完整流程）

### 6.1 觸發（Scheduler）

- **全域節流**：每房每 X 秒最多 1 次「長對話」嘗試；與既有微互動分池。  
- **公平性**：長期統計各 NPC「被說話次數」，避免永遠邊緣人。  
- **玩家感知優先**：玩家所在房可略提高預算；鄰房僅低頻或無 LLM。

### 6.2 參與者選擇（Pairing Policy）—— 分數化選擇

從同房所有 NPC 對組合中計分，選最高者：

```
score(A, B) =
  + 100  若 npc_thread 活躍（延續話題，絕對優先）
  + familiarity / 20  （L2，0–5 分）
  + 3    若 sameVenue（同職場 assignment）
  - 20   若 last_talked_at[pair] < 120s（近期剛說過，強力抑制）
  + rand(0, 3)  （噪音底層，避免永遠同一對）
```

`last_talked_at` 存 in-memory map（重啟歸零可接受）。分數並列時隨機打平。

### 6.3 話題選擇（Topic Selection）

- 從 `npc_to_npc_topics` 與**當前情境**做 **mask**（需在 topics JSON 新增欄位）：

| 欄位 | 說明 |
|------|------|
| `requires_work` | `true` = 只在工作 venue 出現（如「交班」） |
| `night_only` | `true` = 只在夜間時段出現 |
| `follow_up` | `true` = L0 有進出事件時提高權重 |

```go
type TopicMask struct {
    IsWorkVenue  bool  // assignment venue 與 room 一致
    IsNightTime  bool  // hour < 5 || hour >= 21
    HasRoomEvent bool  // L0 滑窗有 enter/leave 事件
}
```

- 選定後寫入 / 更新 `npc_thread.topic_type`。

### 6.4 上下文組裝（Context Builder）

輸出給 LLM 的**固定區塊**（模板化）；**backstory 截 60 字、過往摘要截 40 字，確保 system prompt < 400 tokens**：

```
[硬規則]（禁 meta、禁嵌套引號、兩句輸出等）

說話者：{nameA}（{backstoryA ≤60 字}）
聽者：  {nameB}（{backstoryB ≤60 字}）

【現場】{roomName}，{timeLabel}。
在場：{present_npcs 前 3 人名}。
最近動靜：{L0 最近 2 條，如「XX 剛到」「YY 離開往 ZZ」}

【話題】{topic_type} · {phase}
{若有 anchors：「已知：{anchors}」}

【關係】{一句話，如「A 視 B 為點頭之交」或「兩人初次同框」}

【過往】{npcNpcMemory ≤40 字}（可接續或換話題）
```

L3 傳聞（P4）：可在「現場」區塊末尾加一條「近日鎮上：{rumor}」。

### 6.5 生成與後處理（Generation & Guardrails）

- **結構化輸出**（長期建議）：JSON `{ "a": "...", "b": "..." }` 優於純兩行（便於驗證）。  
- **自動檢查**：長度、禁詞（後設）、人稱一致性、是否違反 anchors（可用輕量規則或二次小模型）。  
- **失敗**：降級模板或沉默；**不**無限重試洗版。

### 6.6 寫回（Commit）

- 更新 `thread`（phase、turn_count、anchors 追加規則要明確：誰有權寫入「新事實」）。  
- 更新 `dyad`（familiarity 微增、sentiment 依 topic_type 微調）。  
- 可選：將本輪**可公開事實**推入 `room_event_log` 或 `gossip`（需「公開性」規則，避免私聊外洩）。

---

## 七、與既有子系統整合

| 子系統 | 整合方式 |
|--------|----------|
| **BuildIdentity / 背版** | 作為角色穩定錨；長度與更新頻率需與 L2、L3 分工 |
| **玩家↔NPC archival** | NPC 間可不共用表，但**語意格式對齊**（便於日後「NPC 在玩家面前轉述」） |
| **favorability / meet_count** | 可作 L2 初值或修正項 |
| **assignments / schedules** | 驅動 topic mask 與「工作交接」真實性 |
| **房間 tags / zone** | L0/L3 篩選傳聞與環境描述 |
| **經濟狀態（鎂等）** | 可進 L3「鎮上景氣」或個人壓力，需防說教式輸出 |

---

## 八、節流、公平、效能與成本

- **每房 QPS 上限** + **每 NPC 冷卻** + **每對 dyad 冷卻**。  
- **Token 預算**：對每次 Call 設 `max_context` / `max_output`；L3 傳聞用 **top-K 相關性**（關鍵字或日後向量）。  
- **背景房間**：可用純規則微互動，不呼叫 LLM。  
- **L3 壓縮（若做）**：若需對傳聞做「日報式摘要」，宜在**伺服器運行時**以定時／低峰 tick 執行，**不**另開離線批次程式；與 [autoresearch_backend.md](../reference/autoresearch_backend.md) 共識一致。

---

## 九、驗收標準與預期結果

### 9.1 質性驗收（玩家可感）

- **連續性**：同一對 NPC 在 1～2 分鐘內第二、第三句**明顯接續上一句**，而非話題跳躍。  
- **現場感**：進房／離房後 1～2 輪內，對話可能提及**方向／剛才動靜**（不強求每句）。  
- **關係感**：熟人省略試探；陌生人多禮節與試探；有過節者語氣緊繃但不 OOC。  
- **世界感（長期）**：同一區域反覆遊玩後，會聽到**重複出現的地點／人物／謠言母題**（允許用詞變體，但**指涉穩定**）。

### 9.2 可量化 proxy（內部）

| 指標 | 方向 |
|------|------|
| 同 thread 連續輪次比例 | 上升 |
| 玩家舉報「胡言亂語」或跳過率（若有 UI） | 下降 |
| LLM 後設句觸發率 | 近零 |
| 每玩家每小時 LLM 呼叫數 | 在預算內 |
| 對話與 `recent_room_events` 的關鍵字共現（離線抽樣） | 上升 |

### 9.3 非保證（誠實邊界）

即使完整實作，仍**不保證**：

- 每句都像人工劇本般完美；  
- 永不產生時代錯亂用語（需持續調模型與禁詞）。  

保證的是：**系統上**記憶與情境約束到位，**長期行為統計**往「穩定、連續、可累積」收斂。

---

## 十、實作路線圖（完整，非最小）

建議分 **5 個 phase**，每階段皆可獨立驗收。

| Phase | 內容 | 驗收焦點 |
|-------|------|----------|
| **P1** | SQLite `npc_thread` + 配對分數化 + L0 in-memory 滑窗注入 prompt + NPC-NPC 對話寫入雙方 archival | 對話接現場、接話題、不重複配對 |
| **P2** | SQLite `npc_dyad` + familiarity 累積 + prompt 語氣描述注入 | 熟人語氣穩定，陌生人有距離感 |
| **P3** | Topic mask（`requires_work` / `night_only` / L0 事件加權） + 可選結構化 JSON output | 話題與場所/時段對齊，解析更穩定 |
| P4 | L3 傳聞池（衰減 + top-K）+ 與區域/任務事件接軌 | 長期世界感 |
| P5 | 自動品質門檻（禁詞、後設句偵測）+ 可選離線摘要批次 | 成本與穩定性 |

---

## 十一、結論（直接回答你的問題）

- **「運行久了 LLM 應該更應景」**：在工程上必須落實為 **記憶與情境狀態隨時間變豐富**，並在**每次呼叫**餵入；不是靠時間魔法。  
- **「有在紀錄」**：目前類產品多為**極薄摘要**；要達到你期待的品質，需升格為本文件中的 **L0～L3 + thread + 決策管線**。  
- **預期結果**：短期見「接話、接現場」；中期見「熟人像熟人」；長期見「這鎮子在說同一些事、同一群人」——那才是你要的**邏輯性與精準度**。

---

## 十二、相關文件與程式錨點（現況）

- 行為說明：`docs/reference/NPC之間交互行為.md`  
- 活化總覽：`docs/NPC活化系統.md`  
- 玩家側記憶：`docs/implementation/NPC對話記憶與背版—設計.md`  
- 程式錨點（可隨實作演進）：`main.go`（`tryTriggerNpcNpcInRoom`）、`ai/talk.go`（`CallAITalkNPCToNPC`）、`store`（`NpcNpcSummaries`）、`db/npc_topics.go`

---

*文件版本：1.1｜v1.0 基礎上補充 Go+SQLite 落地決策、Thread 生命週期精確化、配對分數化公式、Prompt 截斷模板、Topic mask 欄位設計。*
