# 2026-06-18 世界根架構外部研究

狀態：外部研究筆記，不是奇點權威
第一權威：`docs/DESIGN_KEYWORDS.md`
對接文件：`docs/design/world/world-foundation-v0.md`

## 研究對象

本輪查的是「世界根架構如何變成可玩的細節」，不是泛泛世界觀。

要回答：

- 活世界為什麼不能只靠場景文案。
- 文字 MUD 的沉浸感靠什麼成立。
- 系統型遊戲如何避免變成作者寫好的劇本。
- 資源、經濟、地貌、NPC 如何互相接上。
- 奇點下一份世界設計檔該補哪些硬細節。

## 來源摘要

| 來源 | 查到的重點 | 對奇點的轉譯 |
| --- | --- | --- |
| Game Developer, `Living Worlds: The Ecology of Game Design` | 可信世界不是圖像問題，而是生物、NPC、物件與環境互相屬於彼此；角色應被環境塑造，也塑造環境。 | 奇點的 NPC 不能只是場景上的人名或可點擊服務點；他要和地貌、天候、貨物、路、工序或人際責任有一條明確關係。 |
| Game Developer, `The Ancient Art of the MUD` | MUD 依靠文字、想像、社群、角色扮演與大量小細節成立；小細節缺席時玩家會感覺世界空。 | 場景卡不能只寫大概氛圍；每張場景卡要至少有可追問的小細節：誰在做什麼、什麼物件可碰、哪條路有條件、什麼痕跡能回訪。 |
| Playtank, `Designing a Systemic Game` | 系統型遊戲不能預知所有結果；重點是物件、狀態、規則交會後產生玩家可探索的可能空間。 | 奇點不應只寫固定事件；要定義物件屬性、狀態變化與規則交會，例如霧影響視線與船序，濕貨影響保存與報價，熟人關係影響通行。 |
| Weines & Borit, `GAS Framework` | 若遊戲世界是社會-生態系統，設計要檢查自然系統、人類系統、管理/規矩、資源流與一致性；不能只列元素。 | 奇點的「生機過剩」要拆成自然、人群、規矩、資源、場所維持五層，並檢查它們是否互相連動。 |
| Iron Realms, `What is a MUD Game?` | MUD 是持續存在的文字多人世界；世界描述、玩家行動、經濟、政治與長期角色關係是深度來源。 | 奇點手機化後雖不用打指令，但不能丟掉 MUD 核心：持續世界、文字描述、可互動物、玩家與 NPC 的長期痕跡。 |
| Interactive Fiction Forum, `Interactive fiction as immersive sim?` | 好的文字互動世界有物件、狀態、關係與多種解法；障礙與解法都應自然來自環境與工具。 | 奇點每個問題都不該只有「點唯一按鈕」；地貌、物件、NPC、工具、時間應提供不同處理方式與代價。 |
| Raph Koster, `Laws of Online World Design` | 靜態故事很快會被看完；持續世界需要多種進展路徑、玩家所有感、經濟消耗、對預期的管理與廣度。 | 奇點不能只靠主線或場景表演；玩家要能在工具、信用、住所/場所、交易、路線、人際、組織上累積位置。 |
| Raph Koster, `UO's resource system` | 早期 UO 資源系統重點在物件屬性，而不是每個配方硬綁 item id；資源設計牽動更大的虛擬世界與 AI/經濟可能。 | 奇點材料表不能只是商品名；材料要有屬性、來源、品質、處理狀態、保存條件，讓不同系統能查詢並產生行為。 |
| Game Developer, `Dwarf Fortress Q&A` | Dwarf Fortress 的吸引力來自世界生成與大量系統共同產生事件，但其規模也證明深模擬會帶來巨大維護成本。 | 奇點不應追求全量深模擬；要先做玩家能看懂的局部因果，把看不見的模擬降成可坍縮、可追問的狀態。 |
| David Mullich, `Actionable GDD Template` | GDD 需要寫清楚系統、目的、行動、元素、消耗、產出、UI、NPC 目標等，而不是只寫設定。 | 奇點下一份設計檔要補每個世界屬性的系統表：玩家行動、NPC 行動、物件狀態、資源消耗/產出、UI 表現、驗收方式。 |

## 對 `world-foundation-v0.md` 的修正判斷

目前 `world-foundation-v0.md` 的根命題方向可以保留，但它還不是完整世界設計。

它已經有：

- 生機過剩的常理化奇幻世界。
- 文明作為邊界維持。
- 世界屬性表。
- 地貌、物候、材料、NPC、時間的根層方向。

它還缺：

- 每個世界屬性的系統表。
- 每種地貌如何產生材料、道路、NPC 工作、危險與交易。
- NPC 為什麼在場、為什麼離開、為什麼改變反應。
- 物件屬性如何被多系統查詢，而不是只當文案名詞。
- 玩家在一分鐘、十分鐘、一天、長期回訪時各能累積什麼。
- 反例清單：哪些細節雖然真實，但不會增加文字玩家的遊玩體感，應刪掉。

## 下一份世界設計檔應補的硬表

### 1. 世界屬性 -> 系統表

每個屬性都要補：

| 欄位 | 要回答 |
| --- | --- |
| 世界規則 | 這件事在後端如何成立 |
| 玩家可見 | 玩家在文字/地圖/UI 哪裡看見 |
| 玩家可做 | 玩家可以等、問、拿、修、繞、交易、跟隨、阻止、保存或放棄什麼 |
| NPC 可做 | NPC 如何同樣受規則影響 |
| 物件狀態 | 哪些物件狀態會變 |
| 消耗/產出 | 錢、貨、時間、工具、信用、路況、傷害、資訊如何變 |
| 回訪差異 | 玩家之後回來會看到什麼不同 |

### 2. 地貌 -> 玩法表

每種大尺度帶都要補：

| 地貌帶 | 主要生機 | 維持成本 | 典型 NPC 工作 | 典型物件 | 可玩衝突 |
| --- | --- | --- | --- | --- | --- |
| 河流帶 | 水、霧、潮、魚、濕貨 | 渡序、泊位、保存、通行 | 渡工、守倉人、魚販、搬運者 | 船、樁、濕貨箱、燈、繩 | 過河時機、貨物受潮、船位、人群等待 |
| 沃野帶 | 草、糧、畜力、蟲害 | 灌溉、放牧、收割、護欄 | 農戶、牧人、分選工、修具工 | 田埂、水渠、畜欄、草垛 | 草勢越界、畜群失控、糧貨保存 |
| 礦坡帶 | 石、金屬、粉塵、滑坡 | 支撐、搬運、分選、工具耗損 | 礦工、分選工、修具工、巡路人 | 礦簍、支架、粗料堆、磨具 | 路滑、工具壞、粗料含雜、傷害 |
| 林澤帶 | 根系、水網、蘆草、獸跡 | 開路、防迷、乾燥、標記 | 採集者、引路人、巡路人、守夜者 | 路標、乾棚、蘆束、火帶 | 迷路、材料返潮、獸徑變路 |
| 城鎮帶 | 人群、貨流、規矩、熟臉 | 門禁、倉儲、排班、價格承認 | 守門人、倉管、工匠、兌票人 | 門、牌、秤、倉、工作面 | 外人信用、插隊、壓價、規矩例外 |

### 3. 物件屬性表

任何 P0 物件至少有：

- 來源地貌。
- 物理狀態：乾/濕、粗/精、鮮/壞、完整/損壞、重/輕。
- 所有權或保管責任。
- 可用動詞：拿、放、搬、修、分選、保存、交易、丟棄、阻擋、作為工具。
- NPC 是否會用。
- 玩家看見它時的文字狀態。
- 它如何變成錢、路、人情、風險或建設的一部分。

### 4. NPC 在場理由表

每個核心 NPC 不是先寫性格，而是先寫：

- 他在維持哪個世界邊界。
- 他今天帶著什麼物件或責任。
- 他想避免什麼損失。
- 他會因什麼狀態離開、停下、求助、拒絕或改口。
- 玩家不管他時，他下一步會做什麼。
- 玩家介入後，哪個世界事實會變。

### 5. 反例刪除表

以下內容即使真實，也不應優先進核心設計：

- 玩家看不出差異的材料百科。
- 只增加名詞、不增加動詞的地名。
- NPC 隨機台詞輪播。
- 只有一次性漂亮描述，回訪沒有差異的場景。
- 全世界級模擬，但玩家只能看到「好像有事件」。
- 太細的先後順序，卻不改變玩家決策。

## 對下一步的結論

下一步不應直接重寫場景，也不應繼續列材料百科。

應先寫 `world-scale-and-systems-v0.md`，把 `world-foundation-v0.md` 的世界屬性轉成可供 GDD、地圖、NPC、物件、經濟共用的硬表。

完成邊界：

- 每個世界屬性都有玩家可見、玩家可做、NPC 可做、物件狀態、消耗/產出、回訪差異。
- 每個大尺度地貌帶都有主要生機、維持成本、NPC 工作、典型物件、可玩衝突。
- 每個物件和 NPC 都能回答「它為什麼在這裡、玩家能怎麼碰、世界事實怎麼變」。
- 明確列出哪些看似細緻但對文字玩家無感的內容要刪。

## 來源

- Kevin Carter, `Living Worlds: The Ecology of Game Design`: https://www.gamedeveloper.com/design/living-worlds-the-ecology-of-game-design
- `The Ancient Art of the MUD`: https://www.gamedeveloper.com/design/the-ancient-art-of-the-mud-a-writer-s-perspective-of-the-crafting-of-a-text-based-game-
- Playtank, `Designing a Systemic Game`: https://playtank.io/2024/06/12/designing-a-systemic-game/
- Melania Borit et al., `Better Game Worlds by Design: The GAS Framework`: https://journals.sagepub.com/doi/pdf/10.1177/15554120211027109
- Iron Realms, `What is a MUD Game?`: https://www.ironrealms.com/mud-games/what-is-a-mud-game/
- Interactive Fiction Community Forum, `Interactive fiction as immersive sim?`: https://intfiction.org/t/interactive-fiction-as-immersive-sim/63049
- Raph Koster, `The Laws of Online World Design`: https://www.raphkoster.com/gaming2/laws-of-online-world-design/the-laws-of-online-world-design/
- Raph Koster, `UO's resource system`: https://www.raphkoster.com/2006/06/03/uos-resource-system/
- Game Developer, `Q&A: Dissecting the development of Dwarf Fortress with creator Tarn Adams`: https://www.gamedeveloper.com/design/q-a-dissecting-the-development-of-i-dwarf-fortress-i-with-creator-tarn-adams
- David Mullich, `An Actionable Game Design Document Template`: https://davidmullich.com/2018/06/25/an-actionable-game-design-document-template/
