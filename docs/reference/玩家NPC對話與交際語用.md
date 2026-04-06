# 玩家↔NPC 對話：交際語用與 Prompt 方針

> **定位**：**語用與設計為什麼**（寒暄的社會功能等）；**不**取代「逐條回答規則」。  
> **若你要改的是「各情節怎麼答」**：請以 [`data/templates/PLAYER_TALK_WEB_LLM.md`](../templates/PLAYER_TALK_WEB_LLM.md) 為分場景條列，並**同步** `llm_prompts.json`。  
> **實作錨點**：`data/templates/llm_prompts.json` 的 `player_npc.*`；程式 `ai/talk`、`server/action_entity_talk`、`db/archival`（`SearchArchivalForPlayerTalk`）。

---

## 一、核心直覺：問候很少「只是」問候

日常經驗裡，人說「你好」「吃飽沒」「最近怎樣」，**字面資訊量常很低**，但**社交上很少是空的**：

- **維持接觸**：我在這裡、我願意跟你處在同一個互動裡。  
- **儀式與禮貌**：遵守場面規矩，給彼此臺階。  
- **開話輪**：試探能不能往下談、節奏對不對。  
- **帶著下一步**：問路、討價、套近乎、求助——**開場白後面往往接的是關係或意圖**（你說的「包袱」可對到這一層：**不是陰謀，而是語用上的「還有下文」**）。

語言學裡常把這類功能稱 **交際性言語／交際功能**（phatic communion / phatic function）：重點在**關係與通道**，不在命題真假。同一句「你好嗎？」在閒聊裡像儀式，在特定語氣與上下文裡又可以變成**真的在問狀態**。

**本專案對 NPC 的期待**：玩家丟一句寒暄時，NPC **可以**像真人一樣做一點**交際性回應**（回禮、點在場、輕問一句是否找我有事），**但不要**在**沒有記憶或玩家沒提到**的情況下，**捏造具體情節**（例如突然「昨晚對帳」）。也就是：**允許關係層的飽滿，禁止事實層的亂編**。

---

## 二、與 `llm_prompts.json` 的對應

| 設計意圖 | JSON 欄位（`player_npc`） | 備註 |
|----------|---------------------------|------|
| 世界基調 | （層 0）`world_phenomena_cognition` | 與 [世界觀認知與對話prompt](世界觀認知與對話prompt.md) 對齊 |
| 當前空間 | `room_context_fmt` | 減少「客棧／高樓」等時空錯亂 |
| **交際 vs 捏造** | `behavior_rules` | 寒暄可帶話輪延續；禁止無據的帳目／劇情 |
| 記憶節錄怎麼用 | `user_with_memory_fmt` | 與玩家**本句主題**無關的節錄視為不可見 |
| 例句僅學語氣 | `style_examples_header` | 勿照搬例句裡的人名、帳目情節 |
| 零命中時少汙染 | （程式）見下節 | 非 JSON |

詳細欄位語句以 JSON 為準；**改動原則**應與本文件一致，避免只堆否定句、讓模型不知道「可以怎麼像真人」。

---

## 三、程式與記憶（為什麼會「無端對帳」）

- **Talk 前**會組：`BuildIdentity` + **archival 節錄** + 口吻例句等。  
- 若檢索在玩家只說「你好」時仍 **fallback 到「最新幾條」**，節錄裡的「對帳」會被模型當成**玩家在延續的話題** → **錯在管線，不只在模型**。  
- 對策：**`SearchArchivalForPlayerTalk`**——寒暄類、**零關鍵字命中**時**不**做「取最新」fallback；見 [記憶系統對照NPC活化系統](記憶系統對照NPC活化系統.md)、`db/archival`。

---

## 四、維護檢查清單

- [ ] 調整玩家↔NPC 語氣或規則時：同步更新 **`llm_prompts.json`** 與**本文件**（目的與對照表）。  
- [ ] 新增「禁止／必須」句前：想一句**對應的允許**（例如允許交際性延續），避免模型過度乾癟或過度防禦。  
- [ ] 拉長 `behavior_rules` 前：估 token；能放本文件的長論述不放進 JSON。  
- [ ] 改記憶注入邏輯時：確認 Talk 仍走 **`SearchArchivalForPlayerTalk`**（除非刻意改行為）。

---

## 五、相關文件

- [世界觀認知與對話prompt](世界觀認知與對話prompt.md) — 認知分層與層 0～3  
- [記憶系統對照NPC活化系統](記憶系統對照NPC活化系統.md) — archival 與 Talk 檢索  
- [對話記憶系統—彙整與探討](對話記憶系統—彙整與探討.md) — 長期記憶架構  
- [`docs/config/gametext_and_prompts.md`](../config/gametext_and_prompts.md) — 文案與 prompt 檔分工  
- [`data/templates/README.md`](../../data/templates/README.md) — `llm_prompts.json` 在模板樹中的位置  

---

*奇點世界 — 玩家 NPC 對話交際語用（reference）*
