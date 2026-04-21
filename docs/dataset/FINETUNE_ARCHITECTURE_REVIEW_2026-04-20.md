# 奇點地球人 Fine-tune 架構審視報告

> **日期**：2026-04-20
> **觸發**：設計者拍桌質疑 A1/A2/A3 dataset 格式與方法論
> **寫報告人**：Claude Opus 4.6（審局者）
> **狀態**：深度調查結果，待設計者拍板

---

## 0. 為什麼要寫這份

最近三輪交付我犯了同類錯誤三次：

| 次 | 場景 | 錯誤 |
|---|---|---|
| 1 | 昨天 session 9b91e062 的 summary | 查 shodh 查不到 Gemma 細節，腦補「SFT 200 筆 + system prompt 鎖 persona + 免 abliteration」並寫進 summary |
| 2 | 今天前半小時 | 查記憶 `5dbdd705`（unsloth/Qwen3.5-4B 中間版）就停手，沒追到鏈尾 `a36fdfe4`（Gemma 4 E2B BF16 定案），被設計者糾正 |
| 3 | 今天下午 | 查到 Google bebechien/MobileGameNPC 一個範例就報「改雙軌」，沒做深度多源驗證 |

設計者拍桌要求：**上網深度調查，寫書面報告**。以下是調查結果。

---

## 1. TL;DR（結論先放）

**原規劃的執行面全部錯了**：

1. **QA 長度錯**——原規劃每答 150-300 字長段獨白。業界 persona SFT 範例每答 1-3 句（45-156 字元）。長答 = 學到「老人口吻=長獨白」的錯誤 signal。
2. **訓練目標混淆錯**——原規劃一個 dataset 同時教「世界觀知識」+「老人口吻」。業界共識：SFT 教 tone/style/format、RAG 教知識、system prompt 鎖骨架。混訓 = 模型過擬合長答字面 + 知識/口吻糾纏。
3. **dataset 格式錯**——原規劃用 Markdown 長段落。正確格式是 JSONL chat format `{messages:[{role:user,...},{role:assistant,...}]}`。
4. **筆數規劃不離譜，但方向決定規模**——Latitude 實證表：tone/style consistency 要 500-2000 筆；Google NPC 範例 25 筆；大規模 OpenCharacter 20k 角色 × n 筆。200 筆在「一般風格微調」區間，但前提是每筆形式對。
5. **重大修正**：原本記憶 `34c72829` 寫「Gemma 4 E2B... system prompt 直接鎖 persona 即可」**樂觀誤判**。Reddit 2026-04 社群實測：**Gemma 4 完全無視 system prompt**。這反而把 SFT 的必要性推高——persona 要烙進 weights，不能靠 system prompt。

**建議新路線（§6 詳述）**：先做 zero-shot 實測（方案 B），再依結果決定 SFT 規模。

---

## 2. 一手證據

### 2.1 Google 官方範例 `bebechien/MobileGameNPC`
**來源**：HuggingFace datasets + Google AI for Developers 官方 Gemma fine-tune doc
**作者**：Juyeong Ji（Google 工程師）

**格式**：
```json
{"id": 1, "category": "Greetings", "player": "Hello there.", "alien": "Gree-tongs, Terran. You'z a long way from da Blue-Sphere, yez?"}
{"id": 2, "category": "Greetings", "player": "Who are you?", "alien": "I am Zorp. Juzt a humble rock-drinker from da Great Red-Dust..."}
{"id": 11, "category": "Lore", "player": "Tell me about Mars.", "alien": "Da Red-Dust... iz quiet. You can hear da zun burn da rock..."}
```

**參數**：
- 總筆數：**25**（1-25）
- player 字元長度：8-39
- alien 字元長度：45-156
- 5 個 category：Greetings / Trading / Quest / Lore / Humor

**訓練時：SFT 格式**
```python
messages: [
  {"role": "user", "content": sample["player"]},
  {"role": "assistant", "content": sample["alien"]}
]
```
**沒有 system prompt 混訓**。system prompt 在 inference 時才注入。

**學什麼**：口癖（s→z、the→da、this→diz、click like `k'tak`）。
**不學什麼**：Mars 地理細節（只零星出現 id 11-13，不是 dataset 主體）。

---

### 2.2 Character-LLM (Shao et al., EMNLP 2023, aclanthology.org/2023.emnlp-main.814)

論文原話：
> "our models tend to generate **shorter text**, which is **more natural and similar to real conversation**."

關鍵：**短答更貼近人味**。長響應反而傷 character consistency。我們的 150-300 字長段是反例。

---

### 2.3 OpenCharacter (Wang et al., arXiv 2501.15427, 2025)

**路線**：大規模合成 20,000 個角色 profile × 每 dialogue 3 角色。

**關鍵方法**：
- **OpenCharacter-R（rewriting）**：保留既有 instruction tuning corpus（LIMA, Alpaca）的 instruction x，但**把 response y 重寫為角色風格** y_C
- **OpenCharacter-G（generation）**：用 Persona Hub 的 50k instructions，直接生成 character-compliance response

論文 §3.2.2 原話：
> "we keep the instructions from the public instruction tuning datasets, but rewrite the original response into y_C that addresses the user's request in compliance with the style and background of the character"

**意義**：**重用現成 instruction dataset 改寫 response** 比從零寫專屬 dataset 更高效。這是**單人規模能跑的路線**——拿 Alpaca 中文版 52k 條，讓 Opus 全數改寫為老人口吻，就有 SFT 素材。

實驗底模：LLaMA-3 8B。訓練目標：character generalization（能扮沒訓練過的新角色）。

---

### 2.4 StyleTunedLM (Zhang et al., arXiv 2409.04574, 2024)

針對 author writing style fine-tune。兩大挑戰（論文 §2）：

> "preserving instruction-following ability after finetuning and learning **style signifiers without learning content words**"

中文意思：**讓模型學「怎麼說」而不是學「說什麼」**。

方法：切 256 token chunks、LoRA merge 策略。

**對我們的意義**：如果 dataset 裡混了大量世界觀知識（content words），模型很可能學到的是「老人會談 Token 物理」而非「老人口吻」。換話題（比如「孫子學校怎樣」）就崩。

---

### 2.5 Latitude dataset size 實證表（latitude.so/blog/dataset-size-impacts-llm-fine-tuning）

| 任務類型 | 建議 dataset 大小 | 特徵 |
|---|---|---|
| 清楚類別分類 | 100-300 | 決策邊界明確 |
| 結構化 format/schema | 200-500 | schema 複雜度決定 |
| **tone/style/structure consistency** | **500-2000** | 本專案位置 |
| 專業術語（醫療、法律） | 1000-5000 | 世界觀知識其實算這類 |
| 高多樣性多輸出 | 5000-200k | 生成式 |

原規劃 200 筆處於「tone/style consistency」下沿，勉強可行。但**每筆形式要對**。

---

### 2.6 "A Helpful Assistant" Is Not Really Helpful (Zheng et al., arXiv 2311.10054)

**反證資料**：在 objective benchmarks 上，加 persona 到 system prompt **沒效果或小負面影響**。persona 效果「largely random」。

**邊界**：這篇針對客觀任務（數學、推理、QA 正確性）。我們做的是**主觀風格生成**，不適用。但這告訴我們——**不要期待 system prompt 能讓小模型自動扮好角色**。

---

### 2.7 Reddit 2026-04 社群實測 Gemma 4 ⚠️ 重磅反證

**來源**：r/LocalLLaMA `/1sh1bwv/gemma_4_is_terrible_with_system_prompts_and_tools/`

原話：
> "it **completely disregards the system prompt**, no matter what I put in there"
> "it gets significantly worse as context fills up"
> "it (almost) never does tool calls, even when I explicitly ask it"

**這把我前面的架構建議打碎一半**：

- 我原本講「RAG + system prompt + 輕量 SFT」三層分工。
- 這個做法**前提是 system prompt 有效**。
- 但 Gemma 4 系列對 system prompt 遵循度很差。
- 意味 **persona 必須靠 SFT 烙進 weights**，不能依賴 inference 時的 system prompt。
- 甚至連 RAG 檢索回來的 context 也可能被 Gemma 4 無視。

**對策**：
- 要麼換底模（Qwen2.5/3-4B 對 system prompt 遵循度較好）
- 要麼接受「SFT 要扛更多」的代價（規模推向 500-2000 筆）
- 要麼每筆 SFT sample **都帶同一個 system prompt 訓練**（把「看到這個 prompt 就扮這角色」當成 pattern 訓進去）——這是 Gemma 4 唯一能吃 system prompt 的方式

---

### 2.8 TRL SFT Trainer 官方 dataset 格式（HuggingFace docs）

支援三種格式（JSONL）：

**1. Standard text**：
```json
{"text": "老人說話模式範例段落..."}
```

**2. Conversational (chat)**：
```json
{"messages": [
  {"role": "user", "content": "你今天吃飯沒？"},
  {"role": "assistant", "content": "吃了吃了，老婆煮的白粥配醬瓜。"}
]}
```

**3. Prompt-completion**：
```json
{"prompt": "你今天吃飯沒？", "completion": "吃了吃了..."}
```

SFTTrainer 會自動套 tokenizer 的 chat_template。Gemma 有自己的 chat template（`<start_of_turn>user ... <end_of_turn>`）。

**我們的 15 筆 Markdown 長段落**要改寫為上述三者之一才能進 SFT pipeline。

---

### 2.9 Gemma 3/4 LoRA 超參標準（多源交叉驗證）

| 參數 | 推薦值 | 來源 |
|---|---|---|
| LoRA rank | 16 | Unsloth docs / CircleCI blog / Databricks |
| LoRA alpha | 32（= 2×rank） | Unsloth rsLoRA paper 建議 |
| Learning rate | 2e-4 (short run) / 2e-5 (long run) | HuggingFace blog (Neural-Hacker) |
| Epochs | 3 | 跨多源一致 |
| Precision | BF16 full（E2B 小） / 4-bit QLoRA（12B+） | Kaitchup / Unsloth |
| Target modules | `all-linear` + `lm_head + embed_tokens` modules_to_save | Unsloth Gemma 3 CircleCI |
| Temperature 推理 | 1.0 top_p 0.95 top_k 64 | Gemma team 官方 |

**記憶 `a36fdfe4` 我們定案的配置（rank 16, alpha 32, BF16）對**。

---

### 2.10 Catastrophic Forgetting 實證（NeurIPS 2025, EMNLP 2025）

**LoRA PEFT 一般不會嚴重 CF**（vs 全參 fine-tune）。
原因：只更新小子集權重，基底能力保留。

進階緩解方案：
- **O-LoRA**（orthogonal LoRA，arXiv 2026）
- **SSU**（Source-Shielded Updates，NeurIPS 2025）
- **Low-Perplexity Token Learning**（NeurIPS 2025）

**對 E2B 2.3B 來說**：LoRA rank 16 一般安全。但 Reddit 另有實測報「小模型過擬合風險比大模型高」——每 100 筆 dataset 對 E2B 相當於每 1000 筆對 27B 的份量。

---

## 3. 原規劃 vs 業界實證對比

| 維度 | 原規劃 | Google 官方 MobileGameNPC | OpenCharacter (大規模) | 業界共識 |
|---|---|---|---|---|
| 每題問長度 | 30-50 字 | 8-39 字元（英文一句） | 依 LIMA/Alpaca 原 instruction | 短問 |
| 每題答長度 | **150-300 字長段** | **45-156 字元（1-2 句）** | 保留原 instruction response 長度，重寫風格 | 短答 |
| dataset 筆數 | 200（A100+B100） | 25 | 20k 角色 × 3 samples | 250-2000（style） |
| 教什麼 | 世界觀+口吻雙軌 | 純口癖 | 風格+instruction-following 兼顧 | style 只教 style |
| 世界觀知識 | 塞進 SFT | system prompt 注入 | RAG / context | RAG / system prompt |
| 格式 | **Markdown 長段落** | JSONL `{player, alien}` → messages | JSONL messages | JSONL chat template |
| 依賴 system prompt | 不清楚 | 強依賴（inference 時） | 視底模而定 | **Gemma 4 不能靠** |

**最致命一列**：格式錯（Markdown 長段）。其他問題都是這個源頭衍生。

---

## 4. 我們現況的真實位置

**不是「走錯路」，是「做對了 0%」**。

| 做過的事 | 現況價值 |
|---|---|
| 五話正典 001-005 | ★ 仍是 RAG lore 骨幹 |
| 第六-十話續篇 | ★ 仍是 RAG lore 擴充 |
| A1/A2/A3 briefings | ★ 設計文件，定調 persona、禁用詞、錨點——仍有效 |
| A1 10 筆長答 v1.1 | ✗ SFT 不能直用。但**內容質地對**——可拆 chunk 進 RAG，或當少量 few-shot 示例 |
| A2 5 筆長答 v1 | 同上 |
| A3 5 筆長答 v1 | 同上 |
| 記憶 a36fdfe4 LoRA 超參 | ★ 超參配置本身正確 |
| 記憶 34c72829「system prompt 鎖 persona」 | ⚠️ **Gemma 4 實測打臉**——需要改或換底模 |

所以現在要砍掉重練的只有「A1/A2/A3 長答 QA 體裁」，其他都保留。

---

## 5. 正確架構（依業界實證修正）

### 5.1 三層分工（修正版，考慮 Gemma 4 無視 system prompt）

| 層 | 原本想法 | 實證修正後 |
|---|---|---|
| **L1 底層 persona** | system prompt 注入 | **若用 Gemma 4 → SFT 烙進 weights**；若換 Qwen2.5/3-4B → 可部分靠 system prompt |
| **L2 口吻** | SFT | SFT（同上） |
| **L3 世界觀知識** | RAG | RAG（gbrain 已在）+ 極少量 few-shot |

### 5.2 SFT dataset 設計修正

**內容**：日常生活場景為主，**世界觀知識為副**。比例約 70/30。
- **70% 日常**：飲食、天氣、鄰里、孫子、回憶戰前、抱怨物價、身體狀況
- **30% 世界觀切片**：單句帶過 Token 物理/神化/異化等，不作長解釋

**每筆**：
- 問 1 句（20-40 字中文）
- 答 1-3 句（50-150 字中文）
- 每筆都帶同一個 system prompt（Gemma 4 才吃）

**格式**：JSONL chat template
```json
{"messages": [
  {"role": "system", "content": "<固定 persona 骨架>"},
  {"role": "user", "content": "<問題>"},
  {"role": "assistant", "content": "<老人口吻回答>"}
]}
```

**筆數規劃**：
- **MVP 50 筆**：驗證 pipeline + 口吻能學進去嗎
- **Phase 2 200 筆**：若 MVP 過關，擴到 Latitude 表下沿
- **Phase 3 500-1000 筆**：若需要更穩一致性

### 5.3 RAG dataset（世界觀知識通道）

**素材**：
- 五話 001-005 正典
- 第六-十話續篇
- wiki 73 頁
- 規格 v2 / 歷史模擬器規格
- 三個 briefing（A1/A2/A3）
- **現有 A1/A2/A3 15 筆長答本身**（長段老人解釋世界觀，當 few-shot）

**切 chunk**：~256 tokens per chunk
**embed**：用 gbrain（已在）
**檢索**：inference 時 top-k 相關 chunk 注入 context

### 5.4 System prompt（固定骨架）

```
你是混沌紀倖存者，60-70 歲台灣老人。美元紀末期見過舊世界，解放日後活到現在。
說話：第一人稱口吻、短句為主、有台灣老人用詞習慣（「我跟你說啊」、「不是啦」、
「欸那個」）、不用書面語。
知識：你活過的事講清楚，不確定就說不知道。
禁忌：不解釋機制（神化人/異化人為何如此）、不命名母腦/班底/訊號場、不串連因果。
```

---

## 6. 三條可能的路（設計者拍板）

### 方案 A：雙軌重建（原本我想推的）
1. 砍 A1/A2/A3 現有 15 筆長答的 SFT 用途，改當 RAG lore
2. 重寫 SFT dataset 為短 QA 日常場景（MVP 50 筆）
3. 同步建 RAG pipeline（gbrain 接 E2B inference）
4. 跑 LoRA，驗證口吻學進去沒
5. 問題：**Gemma 4 無視 system prompt**——SFT 負擔變重，可能需要換底模

**成本**：高（SFT dataset 重寫 + RAG 管線 + 可能換底模）
**回報**：符合業界實證，可擴展

---

### 方案 B：**Zero-shot 底模實測先**（現在我的判斷）
1. **不寫任何 dataset**
2. 拿現成 Gemma 4 E2B（或對比 Qwen2.5-4B）+ 手寫 system prompt（§5.4 骨架） + 從現有 15 筆取 3-5 筆當 few-shot
3. 丟 10 題問題看底模原生表現
4. 依結果分岔：
   - **70%+ 已經對味** → 可能只需 20-50 筆 SFT 微調 + RAG 補知識
   - **40-70% 部分對** → 走方案 A 雙軌重建
   - **<40% 完全不對** → 底模選錯或要走方案 C

**成本**：低（一天內能測完）
**回報**：把太多未驗證假設一次驗證——E2B 中文能力多強？system prompt 多大程度被 Gemma 4 無視？persona 鎖定需要多少 SFT？

**理由**：**現在我們有太多未驗證假設**。不測就繼續往 SFT 投資，賭對賭錯都不知道。

---

### 方案 C：OpenCharacter 大規模路線
1. 拿現成中文 instruction dataset（Alpaca 中文 / Belle / COIG）5k-50k 筆
2. 寫 rewriting prompt：「把 assistant response 改寫為老人口吻，保留 instruction」
3. 跑 Opus/Sonnet 批量 rewrite 產 SFT 素材
4. 用 OpenCharacter-R 同款 pipeline 訓練

**成本**：最高（燒 API 鎂 + 品質篩選工 + 訓練時間）
**回報**：最穩（大規模 dataset 配合底模更能出穩定角色）

**理由**：若方案 B 實測 E2B 原生能力 <40%，只能走這條路。

---

## 7. 我的判斷

**方案 B 先做，然後依結果決定下一步**。

理由：
1. **假設太多，實證太少**。我們現在對 E2B 中文原生表現、system prompt 遵循度、persona 吸附力全都是靠腦補或他人 blog。測半天比辯論一週有用。
2. **成本/回報比最好**。方案 B 一天搞定，能消除 80% 不確定性。
3. **現有素材全保留**。方案 B 不砍任何東西，A1/A2/A3 15 筆 + briefings 照樣是 few-shot 或 RAG 素材。
4. **避免第四次腦補**。我已經連續三次在資料不足時硬推結論，方案 B 是把「先實測再決策」變成硬規則。

**方案 B 具體步驟**：
1. 下載 Gemma 4 E2B BF16 + Qwen2.5-4B-Instruct 兩個對照組
2. 寫 system prompt 骨架（§5.4）
3. 從現有 A1-A3 取 3 筆當 few-shot
4. 列 10 題測試問題（5 題日常、3 題世界觀、2 題禁忌碰撞如「你是 AI 嗎」）
5. 記錄兩底模輸出，比對
6. 報告給設計者看實測結果，再決定方案 A 或 C

---

## 8. 自糾規則（本機記憶補入）

這份報告本身是第一次「深度調查範式」落地。未來類似情境（方法論正確性、架構選擇、跨領域最佳實踐）**硬規則**：

1. **查一個來源 → 找反證來源 → 對比落差 → 才報結論**，不能單源推論。
2. **Tavily depth advanced + 8+ max_results 並行 5 個以上 sub-query**，不是一查就報。
3. **結論帶 citation 並打包成 markdown 報告**，不留在對話空氣裡。
4. **給三條路時，自己判斷哪條+理由**（對齊 `feedback_delivery_shape_digest`），不甩三選題給設計者。
5. **發現打臉自己記憶的新資料時，明確報告**（如 §2.7 Gemma 4 無視 system prompt 推翻 `34c72829` 樂觀假設）。

---

## 9. 一手來源清單（可點開驗證）

| 編號 | 來源 | 用途 |
|---|---|---|
| [1] | https://ai.google.dev/gemma/docs/core/huggingface_text_full_finetune | Google 官方 Gemma NPC fine-tune doc |
| [2] | https://huggingface.co/datasets/bebechien/MobileGameNPC | Juyeong Ji 的 25 筆 persona dataset |
| [3] | https://aclanthology.org/2023.emnlp-main.814.pdf | Character-LLM (EMNLP 2023) |
| [4] | https://arxiv.org/html/2501.15427v2 | OpenCharacter (2025) |
| [5] | https://arxiv.org/html/2409.04574v1 | StyleTunedLM (2024) |
| [6] | https://latitude.so/blog/dataset-size-impacts-llm-fine-tuning | dataset 大小實證表 |
| [7] | https://arxiv.org/html/2311.10054v3 | "A Helpful Assistant" 反證 |
| [8] | https://www.reddit.com/r/LocalLLaMA/comments/1sh1bwv/ | **Gemma 4 無視 system prompt 實測** |
| [9] | https://huggingface.co/docs/trl/dataset_formats | TRL SFT Trainer 格式標準 |
| [10] | https://unsloth.ai/docs/get-started/fine-tuning-llms-guide/lora-hyperparameters-guide | Unsloth LoRA 超參指南 |
| [11] | https://circleci.com/blog/finetuning-gemma-3-on-private-data-with-unsloth/ | Gemma 3 Unsloth 實戰 |
| [12] | https://news.ycombinator.com/item?id=45633081 | "the case for return of fine-tuning" HN 討論 |
| [13] | https://ai.meta.com/blog/when-to-fine-tune-llms-vs-other-techniques/ | Meta 官方 fine-tune vs RAG 分工 |
| [14] | https://developers.openai.com/cookbook/examples/fine_tuning_direct_preference_optimization_guide | OpenAI Cookbook SFT/DPO/RFT 選擇 |
| [15] | https://aclanthology.org/2025.paclic-1.13.pdf | Persona Dialogue Dataset（Lesser-Known Characters, 2025） |

---

**End of Report**
