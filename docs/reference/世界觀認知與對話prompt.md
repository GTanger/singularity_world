# 世界觀認知分層與 NPC 對話 Prompt

> **目的**：把「NPC 知道什麼」拆成可維護的層級，並對齊 **LLM system 注入策略**。  
> **實作錨點**：`data/templates/llm_prompts.json`（欄位 `world_phenomena_cognition` 等）；`ai/prompts` 載入，`ai/talk` 的 `CallAITalk`／`CallAITalkNPCToNPC` 組裝 system。

---

## 層級與注入策略

| 層級 | 名稱 | 內容示例 | Prompt 策略 |
|------|------|----------|-------------|
| **0** | **世界現象級** | 沃土風、富態生長、非典型廢土、豐饒與危險可並存 | **每次** LLM 請求 **必植入**（玩家↔NPC、NPC↔NPC **同一則**，避免口徑漂移） |
| **1** | 區域現象級 | 飛霜霜晶、梧桐綠廊、向陽光感、某區謠傳 | 房間描述節錄、`roomTags`、主題 `topicHint`、動靜列表 |
| **2** | 職業／身分層 | 店員懂排班備料、鐵匠懂爐溫 | `npcBackstory`、職業對話池、指派場所 |
| **3** | 親歷／密傳層 | 當事人才知、組織內部說法 | `SearchArchivalForPlayerTalk`（玩家↔NPC）／`SearchArchival`、兩人摘要、任務解鎖後才注入 |

**層 0 的單一來源**：`llm_prompts.json` 的 `world_phenomena_cognition`；修改後**重啟伺服器**（並同步對齊 [世界觀：富態與拉鋸](世界觀：富態與拉鋸.md) §沃土風）。

---

## 與權威世界觀文檔的關係

- 敘事細節與禁止概括：**[世界觀：富態與拉鋸](世界觀：富態與拉鋸.md)**、**[世界觀：Token降維與生命演化](世界觀：Token降維與生命演化.md)**（層 0 **不**強塞 Token 術語給市井台詞；術語屬層 2～3 或民俗版一句話另案）。  
- NPC↔NPC 篩選與廢土漂移懲罰：**[autoresearch_backend.md](autoresearch_backend.md)**。  
- 玩家↔NPC 的寒暄語用（交際性 vs 捏造）：**[玩家NPC對話與交際語用.md](玩家NPC對話與交際語用.md)**。

---

## 維護檢查清單

- [ ] 新增第三條 LLM 對話路徑時，system **開頭或前段**須呼叫／拼接 `WorldPhenomenaCognitionPrompt()`（與現有兩路一致）。  
- [ ] 拉長層 0 文案前評估 token：寧可短而穩，細節放在層 1～2。  
- [ ] 企劃變更「全球常識」時：改 `data/templates/llm_prompts.json` + 本檔 + 富態與拉鋸（必要時）。

---

*奇點世界 — 認知分層與 prompt 對照（reference）*
