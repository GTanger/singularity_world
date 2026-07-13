# 奇點世界 GDD 細節文檔索引

狀態：除 21 的上層畫面方向已拍板外，其餘細節設計文檔仍是草案
日期：2026-07-13
上層入口：`docs/design/奇點世界遊戲設計檔.md`
第一權威：`docs/DESIGN_KEYWORDS.md`

本目錄把「奇點世界」拆成可逐層審查的遊戲設計文件。這不是 runtime 工單，也不是世界觀散文；每份文件都必須回答玩家看到什麼、能做什麼、狀態怎麼變、失敗或錯過怎麼呈現、下次回來為什麼不同。

現行玩家主畫面先讀
[21-2d-open-world-primary-surface.md](21-2d-open-world-primary-surface.md)：
使用者已決定重啟 2D 平面大地圖開放世界，原則上採全螢幕、可縮放大地圖。
舊文字 MUD 分冊仍可提供零指令互動、狀態回饋與後端契約素材，但不得再
決定主要畫面外框。

本目錄的審查口徑是「玩家角度 × `DESIGN_KEYWORDS.md`」。使用者以玩家身份判斷設計是否有感、是否想玩；Codex 必須用關鍵字清單約束內容來源與方向。玩家無感或關鍵字追溯不成立，任一項失敗都不得進 runtime。

設計層必須同時具備骨、筋、肉、皮：硬結構、系統牽動、具體可玩內容、玩家表層回饋。只寫理念或概念，不算完成設計層。總結構見 [00-design-layer-bone-muscle-flesh-skin.md](00-design-layer-bone-muscle-flesh-skin.md)。

地圖生成入口：`../world/world-scale-and-systems-v0.md`。任何 M1/M2/M3/M4、地名、路線、場景、NPC、物件，必須先追溯世界觀與 `DESIGN_KEYWORDS.md`，再談場景卡或 runtime。

具名內容包入口：`../world/world-content-bible-v1.md`。該檔只負責把本目錄的規格落成第一批局部場景、NPC、物件、事件、工具、價格、失敗、錯過與重訪痕跡；它不是地圖生成根。若內容只停在本目錄的規格語言，不算補肉。

任何具名內容包必須附 `DESIGN_KEYWORDS.md` 關鍵字追溯表。只在頁首寫「第一權威」不夠；每個場景、NPC、核心物件、事件、路線與玩家腳本都要能反查關鍵字簇，否則視為助手自創內容，不能進下一步設計。

輔助研究：`../research/2026-06-17-遊戲設計本體研究.md`、`../research/2026-06-17-外部遊戲設計參照.md`、`../research/2026-06-19-roleplay-element-research.md`。外部資料只用來校準有意義行動、MDA、核心循環、選擇取捨、MUD 動機、UI、文字、物件、NPC、經濟、角色扮演與元素關係的設計判準，不作為奇點第一權威。

## 完整設計層自審入口

2026-06-20 後，判斷「設計層是否可交給實作」時，先讀上層總檔的 [0.2 完整設計層落地狀態](../奇點世界遊戲設計檔.md#02-完整設計層落地狀態2026-06-20)。該節是九大塊自審表與交付邊界，不是新正典；它把本目錄分散的 00-20 分冊收束成可審核狀態。

世界本體正典入口：[../world/world-bible-v1.md](../world/world-bible-v1.md)；世界深度正典入口：[../world/world-bible-v2.md](../world/world-bible-v2.md)。判斷世界是否可讀、可追問、能自然長出地圖/材料/勢力/人物/衝突/玩法時，必須先讀 v1；判斷是否有歷史厚度、文化重量、人物牽連、舊怨、口語稱呼與玩家介入時，必須再讀 v2。四大完整設計包是它們的下游設計包，不得反過來用表格取代世界本體。

四大完整設計包入口：

| 設計包 | 負責缺口 | 後續拆工方向 |
|---|---|---|
| [../world/world-map-complete-design-pack-v1.md](../world/world-map-complete-design-pack-v1.md) | 完整世界地圖 | M0/M1/M2/M3/M4 種子、命名驗收、可進入區、探索/旅行切片 |
| [../world/materials-encyclopedia-v1.md](../world/materials-encyclopedia-v1.md) | 完整材料百科 | 材料資料表、狀態軸、價格承認、保存/變質、工具/器物卡 |
| [../world/factions-history-v1.md](../world/factions-history-v1.md) | 完整勢力史 | 勢力控制權、通行/信用、NPC 在場理由、局部衝突與任務來源 |
| [20-combat-formula-design-pack.md](20-combat-formula-design-pack.md) | 衝突與戰鬥故事包 v2.1 | 物件、名字、路、家戶、追索、淡痕與前主牽連如何升級成自動交鋒與世界後果 |

實作前最低順序：

1. 先確認目標切片對應 `docs/DESIGN_KEYWORDS.md`。
2. 再確認世界背景與地圖生成是否通過 `../world/world-foundation-v0.md` 與 `../world/world-scale-and-systems-v0.md`。
3. 再用本目錄對應分冊補足玩家看到、能做、NPC 同規則、物件/金錢/時間/關係變化與回訪差異。
4. 最後用 [13-content-authoring-validation.md](13-content-authoring-validation.md) 與 [19-player-facing-surface.md](19-player-facing-surface.md) 驗收，不得只因章節寫完就宣稱完整。

## 文檔分層

| 檔案 | 負責層面 |
|---|---|
| [00-design-layer-bone-muscle-flesh-skin.md](00-design-layer-bone-muscle-flesh-skin.md) | 遊戲設計層總結構：骨、筋、肉、皮四層與完成標準 |
| [01-player-and-loops.md](01-player-and-loops.md) | 玩家定位、短中長回圈、登入/離線回圈 |
| [02-ui-and-input.md](02-ui-and-input.md) | 舊文字 MUD UI 支援層；只保留零指令輸入、資訊分工與互動原則，不再決定主畫面 |
| [03-scene-card-and-text.md](03-scene-card-and-text.md) | 場景卡、文字呈現、動態現象、痕跡與禁寫規則 |
| [04-object-action-state.md](04-object-action-state.md) | 物件、動作、狀態、條件、失敗、回訪差異 |
| [05-character-tools-growth.md](05-character-tools-growth.md) | 玩家角色、工具、熟練、裝備作為動詞來源 |
| [06-npc-life-simulation.md](06-npc-life-simulation.md) | NPC 同規則生活、目的、記憶、非服務節點設計 |
| [07-economy-material-logistics.md](07-economy-material-logistics.md) | 真實經濟、物資、供需、物流、信用與價格感 |
| [08-world-time-observation.md](08-world-time-observation.md) | 世界時間、歷史即現在態、沿革、變遷、永遠在線、觀測坍縮、離線安排 |
| [09-exploration-adventure-map.md](09-exploration-adventure-map.md) | 探索、冒險、路線、地圖服務玩法；必須服從世界觀生成地圖規格 |
| [10-combat-conflict-risk.md](10-combat-conflict-risk.md) | 戰鬥、衝突、風險、避戰與自動文字結算 |
| [11-management-building-place.md](11-management-building-place.md) | 經營、建造、維護、場所佔用與長期物件 |
| [12-social-reputation-organization.md](12-social-reputation-organization.md) | 關係、名聲、組織、城鎮/門派/氏族/國家玩家可見面 |
| [13-content-authoring-validation.md](13-content-authoring-validation.md) | 內容撰寫規格、驗收表、切片設計檢查 |
| [14-first-playable-slice.md](14-first-playable-slice.md) | 第一階段可玩切片規格，不指定北岸為唯一答案 |
| [15-elements-fantasy-gameview.md](15-elements-fantasy-gameview.md) | 元素、派生狀態與奇幻規則的玩家動詞、世界狀態、NPC 同規則與回訪痕跡 |
| [16-player-fantasy-affordances.md](16-player-fantasy-affordances.md) | 玩家幻想與非日常可為性：現實做不到的身份、規則、風險、物件、社會與痕跡如何落成可操作入口 |
| [17-roleplay-elements.md](17-roleplay-elements.md) | 角色扮演與元素關係：元素如何接到玩家自身、持物、工具、熟練、NPC 同律、世界承認與回訪痕跡 |
| [18-playable-content-modules.md](18-playable-content-modules.md) | 可玩內容模組：自身、點燈、包貨、清障、等待、問路、交易、幫忙、維修、避險、戰鬥前置、離線安排 |
| [19-player-facing-surface.md](19-player-facing-surface.md) | 玩家表層與回饋：第一屏、自身入口、場景狀態、入口、動作面板、事件流、成功/失敗/錯過/重訪文字 |
| [20-combat-formula-design-pack.md](20-combat-formula-design-pack.md) | 衝突與戰鬥故事包 v2.1：前兆、衝突來源、升級、自動交鋒、傷勢、物件痕跡、社會後果、戰報模板與死亡/失敗邊界 |
| [21-2d-open-world-primary-surface.md](21-2d-open-world-primary-surface.md) | 已拍板上層方向：全螢幕可縮放 2D 平面大地圖、開放世界、玩家與 NPC 直接存在地圖上；拓撲與美術仍待定 |

## 共通驗收句

任何層面的設計若不能回答以下問題，不得進入 runtime：

1. 玩家第一眼看到什麼。
2. 玩家可以點什麼。
3. 玩家做了之後哪個狀態改變。
4. 失敗、拒絕、錯過、被搶先怎麼呈現。
5. NPC 是否也能依同規則介入。
6. 世界時間、歷史、沿革或上一秒狀態是否會改變它。
7. 玩家下次登入或重訪時看得出什麼痕跡。
8. 若內容涉及元素，玩家自身、持物、工具、熟練、關係或位置是否因此產生不同選擇。
9. 它是否同時有骨、筋、肉、皮，而不是只停在概念。
10. 玩家為什麼會願意再點一次、再回來一次、再往深處走一步。
11. 它對應 `DESIGN_KEYWORDS.md` 哪些原始關鍵字，是否只是助手自創詞。

## 無感否決

設計可以自洽但仍然失敗。若玩家角度只看到以下結果，視為未通過：

- 只是合理的世界說明，但沒有想點的入口。
- 只是漂亮的場景散文，但沒有可改變的世界事實。
- 只是材料、收購、跑腿或小額交易，不能產生目標、取捨、風險、關係或回訪。
- NPC 只是服務點、商人、任務端點或台詞輪播。
- 地圖只是地名與出口，沒有世界觀生成的必然性。
- 第一屏看不清自己是誰、在哪、身上有什麼、能做什麼。
- 用「沉浸」「活世界」「有生命感」等抽象詞替代玩家實際看到與能做的事。

## 禁止把本目錄當成定案

本目錄是細節設計草案。使用者拍板前，不得把任一文件轉成 runtime 工單、資料表、地圖擴張或前端實作。
