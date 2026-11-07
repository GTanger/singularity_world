# 《奇點世界》後端鋼骨都更計畫與結構審計紀錄（Backend Urban Renewal Plan）

- **建立日期**：2026-11-05
- **架構總則**：45 年 MUD / RPI / IF 黃金正典 × Rust 型別與標籤驅動鋼骨（「木造蓋不出台北 101，要用鋼骨」）
- **基線規模（都更前 `code_guard_rs source-size` 實測）**：
  - 總檔案數：`249` 個 `.rs` 檔
  - 總行數：`192,517` 行
  - `hard` 級超大單體檔（`> 1,200` 行）：`29` 檔（最大檔 `peninsula_catalog.rs` 達 `18,771` 行、`causal_core.rs` `10,924` 行、`object.rs` `10,343` 行、`player_scene.rs` `8,368` 行、`outer_households.rs` `8,339` 行）
  - `warn` 級偏大檔（`600–1,200` 行）：`39` 檔

- **都更後驗收實測（2026-11-05 `code_guard_rs source-size` ＋ `tra-sw-gate --deploy`）**：
  - 總檔案數：`454` 個 `.rs` 檔（模組化拆分新增 `205` 個高內聚子模組檔）
  - `hard` 級超大單體檔（`> 1,200` 行）：**`0` 檔**（由 `29` 檔全面清零！全倉最大檔由 `18,771` 行降至 `1,150` 行）
  - 狀態評級：由 `status=debt` 提升為 `status=warn ok=true`
  - 全鏈路守衛：`tra-sw-gate --deploy` 六道關卡、`white_tower_living_world_check --live` 四大驗算器（`violations=0`、`conservation_gap=0`）與 `causal_core::tests` 43 項守衛測試全數 `PASS`
---

## 一、 六大「木造違建（Wooden-Shack）」結構斷層診斷

經平行四路結構審計（`ScoutNpcAndHouseholds`、`ScoutSceneAndObjects`、`ScoutEcologyAndWorld`、`ScoutCombatAiAndActions`）逐行盤點，定位出後端六大木造違建根源：

### 斷層 1：場景感知表面（Perceptual Surface）僅半套特例化，缺乏全島伺服端固定陳設編織器
- **現況病灶**：
  - `src/server/handler/movement.rs:3126-3133` 僅為早期里程碑 `M3_027`（半嶺水岸渡口）手寫 `north_bank_object_surface`，將蘆叢、貨擔、熱湯攤寫死在靜態字串；其餘 300 餘間房間之固定陳設（`ViewObject`：水輪石磨、鍛爐鐵砧、當值櫃檯、井台、牌坊、石碑）未被伺服端織入 `final_room_desc`。
  - `src/game/room.rs:1136-1160`（`get_grid_room_view`）與 `src/server/handler/movement.rs:2219-2263` 對 `RoomVenueObject`（當值櫃檯與外借工具）的組裝邏輯分叉，導致 REST/內部視圖與 WebSocket 進場視圖不一致。
- **鋼骨都更方案（Stage 1）**：
  - 於 `src/server/handler/movement.rs` 實作全島統一之伺服端場景編織器 `weave_scene_fixtures_into_desc(desc, room_name, objects)`：依據正交三維標籤（`category_tag`、`attribute_tags` 之 `"immovable"` / `"public_fixture"` / `"venue_counter"`）將房間內不可搬移之核心陳設自然織入 `final_room_desc` 結尾，使後端輸出的 `room_enter` / `grid_view` 原生即為完整文學空間描寫，不依賴前端 JS 事後補丁。

### 斷層 2：物件觀察（`Examine` / `Look`）充斥千篇一律的百科全書前綴與產地括號傾倒
- **現況病灶**：
  - `src/game/peninsula_catalog.rs` 中共 `1,109` 筆 `PeninsulaCatalogEntry.look_text` 全部以機械式模板 `"這是白塔半島常見的「{name}」，取材自…"` 開頭，違反 45 年 MUD 正典「觀察即第一人稱感官體驗、非博物館說明牌」原則。
  - `src/server/action/object.rs:2348-2351` 於玩家執行 `Look` 時，機械式硬塞 `\n【產地溯源：{}】` 資料庫欄位標籤。
- **鋼骨都更方案（Stage 1）**：
  - 全面洗除 `peninsula_catalog.rs` 1,109 筆 `"這是白塔半島常見的「X」，"` 百科前綴，直接以器物形制、材質肌理與工藝用途起筆（`"「X」取材自…"`）。
  - 拔除 `src/server/action/object.rs:2348-2351` 之 `\n【產地溯源：{}】` 括號傾倒（`peninsula_catalog.rs` 描寫本身已自然蘊含原料與工坊脈絡）。

### 斷層 3：座標房間（`grid:x:y`）向舊 P0 地標（`wt-physical-0086`）的木造硬編碼退化
- **現況病灶**：
  - `src/pg/living_world.rs:928-932`（`record_player_world_action`）在遇到 `place_id.starts_with("grid:")` 且實體無舊式 `place_id` 時，硬編碼 fallback 至 `"wt-physical-0086"`（文曜東側水澤林），導致全島網格行動收據座標失真。
- **鋼骨都更方案（Stage 1）**：
  - 改由 `crate::grid::parse_grid_room_id(place_id)` 解析 `SquareCoord`，並透過 `crate::world::place::legacy_place_id(coord)` 映射為正典拓撲地點 ID（`legacy:grid:{x}:{y}`）。

### 斷層 4：里程碑代號（`M3_020_*` / `M3_027_*`）與長鏈 `if/else` 散落於核心動作派發器
- **現況病灶**：
  - `src/server/action/object.rs`（10,343 行）將通用器物解析（`resolve_room_object`、`resolve_grid_scene_object`）、千物總典效果（`apply_peninsula_world_object_effects`）、`M3_020` 石壘門排隊特例（6892–7196 行）、`M3_027` 渡口蘆草特例（7197–8440 行）與 1,557 行測試全部塞進單一萬行巨檔。
  - `src/game/player_scene.rs`（8,368 行）內含長達 4,547 行的 `white_tower_peninsula_objects_for_room` 巨型地名分支與 840 行測試。
- **鋼骨都更方案（Stage 2 & 3）**：
  - 將 `object.rs` 按職責拆分為 `resolve`、`effects`、`m3_milestones`、`followup`、`tests` 子模組；將 `player_scene.rs` 拆分為場景常數、房間器物派發、公眾觀察行、動態攤位與測試子模組。

### 斷層 5：巨型靜態資料表（千物總典 18,771 行、外圍家戶 8,339 行、文曜家戶 3,840 行、NPC 排程 1,245 行）與引擎邏輯混裝
- **現況病灶**：
  - `src/game/peninsula_catalog.rs`（18,771 行）、`src/store/outer_households.rs`（8,339 行）、`src/store/households.rs`（3,840 行）與 `src/server/grid_manager.rs`（2,940 行）將數千至上萬行靜態結構體常數與業務方法寫在同一檔案，造成編譯單元臃腫與閱讀困難。
- **鋼骨都更方案（Stage 2）**：
  - 保持原 `.rs` 檔作為模組門面與型別／方法定義，將巨型常數表按類別／地理聚落切分為緊湊子資料檔（每檔 `< 1,000` 行）。

### 斷層 6：四大核心引擎單體（`causal_core.rs` 10,924 行、`pg/living_world.rs` 6,168 行、`grid/gen.rs` 6,089 行、`object_ecology.rs` 6,041 行、`movement.rs` 6,042 行、`entity.rs` 4,788 行、`sanitize.rs` 4,217 行、`wordplate.rs` 3,934 行、`talk.rs` 3,886 行、`room.rs` 3,642 行、`grid.rs` 3,619 行、`inventory.rs` 3,182 行、`human_phase.rs` 2,939 行）職責過度集中
- **鋼骨都更方案（Stage 2 & 3）**：
  - 依正交領域邊界拆分為高內聚子模組（每個子模組嚴格控制在 `< 1,000` 行），父檔保留公開型別匯出（`pub use`）與 `living_world_evaluator.rs` / `causal_core.rs` 所需之 `include_str!` 契約錨點。

---

## 二、 契約守衛（`include_str!` 零退化鐵律）

1. **禁止刪除被 `include_str!` 引用的父 `.rs` 路徑**：
   - `src/living_world_evaluator.rs:540-569` 與 `src/npc/causal_core.rs:8614-9620` 使用編譯期 `include_str!(...)` 讀取固定相對路徑（如 `src/pg/living_world.rs`、`src/npc/causal_core.rs`、`src/grid/grid.rs`、`src/game/peninsula_catalog.rs`、`src/store/households.rs`、`src/store/outer_households.rs` 等）。
   - Rust 2021 支援在保持 `src/foo/bar.rs` 檔案存在的同時，於 `src/foo/bar/` 目錄放置子模組 `sub.rs` 並於 `src/foo/bar.rs` 內直接宣告 `mod sub; pub use sub::*;`（無須將 `bar.rs` 刪改為 `bar/mod.rs`）。
2. **保留 `living_world_evaluator.rs` 6 大模組必檢錨點字串**：
   - `src/pg/living_world.rs` 保留 `"is_recognizable_room_staff_or_counter_holder"`、`"resolve_nearest_room_by_tag"`、`"ALL_RESIDENTS_SEEDED.call_once"`。
   - `src/npc/causal_core.rs` 保留 `"is_recognizable_room_staff_or_counter_holder_entity"`、`"resolve_nearest_room_by_tag"`。
   - `src/grid/grid.rs` 保留 `"find_nearest_room_by_tags"`、`"is_private_residence_room_name"`、`"is_street_or_gate_room_name"`。
3. **禁詞守衛測試全覆蓋**：
   - 凡自母檔抽離之資料或業務子模組，於 `causal_core.rs` 禁詞守衛測試中同步納入檢查或由母檔一併涵蓋，確保防仙俠／防現代詞／防時間「剛／方」守衛 100% 有效。

---

## 三、 三階段執行藍圖（Phased Execution Blueprint）

### Stage 1：伺服端感官敘事鋼骨化（Perceptual & Narrative Steel Frame）
1. **`src/game/peninsula_catalog.rs`**：清除 1,109 筆 `"這是白塔半島常見的「X」，"` 百科模板前綴，改為直接器物感官描寫。
2. **`src/server/action/object.rs`**：移除 `Look` 動作末尾硬塞之 `\n【產地溯源：{}】`。
3. **`src/server/handler/movement.rs`**：實作 `weave_scene_fixtures_into_desc`，於伺服端自動將房間內不可搬移之場景陳設織入 `final_room_desc`。
4. **`src/pg/living_world.rs`**：修正 `record_player_world_action` 中 `grid:x:y` 至 `legacy_place_id(coord)` 之正典座標解析，拔除 `"wt-physical-0086"` 硬編碼。
5. **`src/npc/causal_core.rs`**：同步 `test_frontend_sw_and_index_version_sync_v0_39_57` 快取版本檢查至最新前端版本。

### Stage 2：巨型靜態表與生態／世界單體拆解（Data & World Monolith Decomposition）
1. **`src/game/peninsula_catalog.rs`（18,771 行）** $\to$ 拆分為 `src/game/peninsula_catalog/` 分類子片（`part_01.rs`..`part_20.rs` + `methods.rs` + `tests.rs`），母檔降至 `< 300` 行。
2. **`src/store/outer_households.rs`（8,339 行）& `src/store/households.rs`（3,840 行）** $\to$ 拆分為 `src/store/outer_households/` 與 `src/store/households/` 區域家戶子片，母檔降至 `< 150` 行。
3. **`src/pg/living_world.rs`（6,168 行）、`src/grid/gen.rs`（6,089 行）、`src/game/object_ecology.rs`（6,041 行）、`src/game/room.rs`（3,642 行）、`src/grid/grid.rs`（3,619 行）** $\to$ 依領域邊界拆分為 `< 1,000` 行之子模組。

### Stage 3：核心行為、因果大腦、移動與戰鬥單體拆解（Action, Brain & Combat Decomposition）
1. **`src/npc/causal_core.rs`（10,924 行）** $\to$ 拆分為 `src/npc/causal_core/`（`vessel`, `arrival`, `personality`, `catalog`, `economy`, `planning`, `state`, `somatic`, `ripple`, `stranded_cargo`, `relational`, `kotodama`, `offline_autonomy`, `crop_plots`, `bank_stash`, `gazette`, `tests`）。
2. **`src/server/action/object.rs`（10,343 行）& `src/game/player_scene.rs`（8,368 行）& `src/server/handler/movement.rs`（6,042 行）** $\to$ 拆分為各自領域子模組。
3. **`src/store/entity.rs`（4,788 行）、`src/ai/sanitize.rs`（4,217 行）、`src/combat/wordplate.rs`（3,934 行）、`src/server/action/talk.rs`（3,886 行）、`src/server/handler/inventory.rs`（3,182 行）、`src/server/grid_manager.rs`（2,940 行）、`src/combat/human_phase.rs`（2,939 行）** $\to$ 拆分為 `< 1,000` 行之子模組。

### Stage 4：全鏈路驗算、部署與雙視口實機視覺驗收
1. 執行 `/home/tanger/.local/bin/code_guard_rs source-size -- --root /home/tanger/Projects/singularity_world/src --top 30` 驗證巨型單體消除成果。
2. 執行 `/home/tanger/.local/bin/tra-sw-gate --deploy` 通過六道關卡與線上四大驗算器。
3. 透過 `laptop_browser` 於 `https://sw.ygggt.com/` 進行 `390×844` 與 `1440×900` 實機走訪驗收。

---

## 四、 第二期：NPC 認知、身分、義務、記憶與清洗管線「釘子區」SSOT 鋼骨都更計畫（Phase II Urban Renewal）

### 1. 四大結構性分裂（釘子區病根審計紀錄）

1. **分裂一：身分、住家、職場與外顯稱謂之四頭馬車 ＋ 7 套平行稱謂函式**
   - **病灶實證**：PostgreSQL `entities` 共 `1,808` 名 NPC，但 `assignments` 僅 `571` 筆（且多為千篇一律之 `服務生`），排程表 `RESIDENT_SCHEDULES`（59 人）先前殘留舊階級符號且與家戶槽位 `wenyao_household_slot_for` 脫鉤、其餘仰賴 FNV-1a 雜湊兜底。`src/db/identity.rs:59-76` 同時拼接 `assignments` 與雜湊家戶槽，導致同一角色被賦予雙重矛盾職場。未結識外顯稱謂散落於 `grid_view_2.rs`、`scene_desc.rs`、`observation_1.rs`、`server/action/mod.rs`、`look/details.rs` 與前端 `sw-object-list.js` 各自計算。
2. **分裂二：自主行為「五腦無鎖踩踏」與「單槽記憶體 vs 多行資料庫」義務斷鏈**
   - **病灶實證**：`LIVE_NPC_OBLIGATIONS`（`src/store/entity/part_01_state.rs:100`）僅為單槽 `HashMap<String, (String, String)>`，新差事直接覆寫舊債務；`grid_manager/simulation.rs:195, 257` 每 25～40 秒無條件清空離駐點 NPC 之義務；`idle/banter.rs:265` 傳入空字串 `""` 無差別刪除所有義務；`npc/grid_ai.rs:565` 於單人完成 `Work`/`Trade` 時隔空自動結清 `repay_debt`，導致 PostgreSQL 中 `repay_debt` 為 `0` 筆，當面還債與追討劇本（`dyadic_kotodama_and_mutation.rs:86`）淪為死碼。
3. **分裂三：記憶與社交圖譜「九表孤島」、67.5% 應聲蟲與 `180s > 90s` 話題死鎖**
   - **病灶實證**：
     - `resident_knowledge`（`7,873` 筆個人親歷知識）與 `data/templates/dialogues/*.json`（19 檔 `716` 行職業語料）在雙人對話中讀取次數為 `0`；
     - `npc_threads`（`2,920` 筆）100% 卡在 `turn_count = 1`，因 `pair_picker.rs:97` 強制配對冷卻 `180s` 大於 `simulation.rs:289` 之 `thread_stale_sec = 90s`（數學交集為 $\emptyset$）；
     - `dyadic_kotodama_and_mutation.rs:269`（`co_enact_script_pack`）在無欠款時（95% 情況）強制覆寫日常話題並塞入 4 件寫死道具；
     - `dialogue.rs:38, 245` 對 A、B 均使用第二人稱 `"你是..."` 且僅計算 A 的立場（不計算 B 的立場），造成角色自稱錯置（吳姨叫豆花伯吳姨）與 67.5% 無條件順從（Yes-Man）。
4. **分裂四：550+ 條正則補丁組成的「清洗怪獸（Sanitizer Swamp）」反噬語義**
   - **病灶實證**：`src/ai/talk.rs:71` 自源頭將 `[WT-XXXX]` 內部代號塞入 Prompt，下游再於 `src/ai/sanitize/`（`dialogue_1.rs`、`dialogue_2.rs`、`grammar.rs`）堆疊 550+ 條替換規則，將所有「剛／才＋動詞」暴力改為「早先＋動詞」（全庫洗出 `1,748` 次「早先」並篡改條件句），且 `dialogue_2.rs:373` 無差別抹除含中文字句子內的所有 ASCII 英文字母。

### 2. 四大單一真相源（SSOT）都更施工切片

- **都更工程 1（SSOT 身分、戶籍、知識接線與第三人稱劇本卡）**：
  - 落實「沒有所謂正典，所有人都是自己的正典」平權公理，拔除 `grid_manager`、`store`、`pg/living_world`、`look` 殘留之階級符號與寫死人名分支，透過 `resident_schedule_home_and_work` 與 `wenyao_household_slot_for` 統一 `src/db/identity.rs` 之住所與職場解析（消除 `assignments` 通用標籤與家戶槽位之雙重職場衝突），接入 `resident_knowledge`（`7,873` 筆）與 `npc_npc_summaries`；
  - 新增 `build_third_person_actor_card()` 供 `NPC↔NPC` 雙人對話使用（徹底隔離玩家專屬 `npc_summaries`「來客」污染與第二人稱 `"你是..."` 角色糾纏）；
  - 對齊 `scene_desc.rs` 與 `look/details.rs` 之未結識外顯稱謂守衛（街巷與民宅絕不顯示「街坊居民」）。
- **都更工程 2（SSOT 多槽義務帳本與人際契約保護鎖）**：
  - 升級 `LIVE_NPC_OBLIGATIONS` 為一人多槽 `HashMap<String, Vec<(String, String)>>`（優先保留與檢索 `repay_debt` / `stranded_cargo`）；
  - 移除 `grid_manager/simulation.rs`、`idle/banter.rs` 與 `npc/grid_ai.rs` 對人際債務 `repay_debt` 之隔空自動抹除，強制人際債務須由債務人與債主同房當面觸發 `settle_debt` 或 `default_extend_debt` 結算。
- **都更工程 3（SSOT 社交記憶匯流排、雙向立場 `Stance A × Stance B` 與話題多輪推進）**：
  - 解除 `180s > 90s` 時序死鎖：將 `thread_stale_sec` 提升至 `1800` 秒、`thread_cooldown_sec` 設為 `600` 秒，並提高未完話題配對權重；
  - 收斂 `co_enact_script_pack`（僅限同工坊當值且 $\le 20\%$ 機率觸發，不強餵 4 件無關道具），支援 `(A 欠 B)` 與 `(B 欠 A)` 雙向當面還債／追討；
  - 於 `dialogue.rs` 與 `llm_prompts.json` 實裝雙向立場 `Stance A × Stance B`（聽話者 B 具獨立 `Accept / Bargain / Deflect / Refuse` 態度），並於 `outcome.rs` 同步更新 `resident_relationships` 與 `npc_dyads`。
- **都更工程 4（SSOT 源頭乾淨輸入與 Sanitizer 拆違建＋DB 舊記憶排毒）**：
  - 於 `src/ai/talk.rs` 源頭停止注入 `[WT-XXXX]` 內部編號與全街 4 件寫死道具；
  - 拆除 `dialogue_1.rs`、`dialogue_2.rs`、`grammar.rs` 將「剛／才＋動詞」全數壓成「早先」之規則（改為自然無贅詞動補結構，消滅「早先」癌），移除無差別刪 ASCII 英文字母規則，並補齊 `grammar.rs` 稱謂詞尾（`"姨"`、`"掌櫃"`、`"大夫"` 等）。

---

## 五、 第三期：全棧「去便宜行事、去紙紮化、去文曜霸權與千物總典 `wt_cat_*` 貫通」26 刀深層根治紀錄（Phase III Deep Structural Renewal）

1. **第一～十刀（`v0.39.115`～`v0.39.116`）**：
   - 拔除家戶日程寫死字串表與攤商 10 分支兜底台詞，將 `1,713` 戶家戶座標與 `59` 名排程居民座標 100% 落實至 `SquareGrid` 真實格位；
   - 將 `DELIVERABLE_ITEMS` 與職司隨身器物 `VocationalVesselSpec.catalog_id` 對接至 `PENINSULA_WORLD_OBJECTS`（`wt_cat_*`）；
   - `simulation.rs` 工資發放按各聚落公庫動態扣減並建立 `BLDG_COORD_CACHE`；`InventoryItemView` 與 `RoomEnterMsg` 下發 `is_edible`、`terrain`、`venue_kind`、`is_indoor`。
2. **第十一～十四刀（`v0.39.117`）**：
   - `borrow.rs` 對接 `VocationalVesselSpec.catalog_id` 並攔截未結清 `repay_debt` 重複借物；實裝 `strip_trailing_person_name_from_role` 根除人名漢字（如姓「林」誤判鋸木坊、名帶「海」誤判漁港釣客）污染場域標籤與職司器物；
   - `infer_origin_settlement` 與 `normalize_canonical_settlement` 支援 `"grid:x:y"` 座標直解；`InventoryItemView` 下發 `item_stage / can_craft / craft_label`；`venue.rs` 建立 `ROOM_VENUE_CACHE` 與 `ANY_ROOM_VENUE_CACHE`。
3. **第十五～十八刀（`v0.39.118`）**：
   - 嚴格區分「在場值守人員 vs 入內歇腳過客」，拔除 `residents_seed.rs`「口袋有 8 鎂即視為櫃檯掌事」與未核對 `equipment_slots` 即硬套家戶制服之便宜行事；
   - `db/inventory.rs` 實裝 `derive_catalog_item_equip_profile` 打通 `wt_cat_*` 穿戴槽位與加成，並打通 `use_item.rs`、`can_mutate_item`、`campfires_and_refining.rs` 之 `wt_cat_*` 食用、淬火、磨刀與添柴；
   - 修正 `borrow.rs` `rel.balance_score >= 1`（原 `>= 0` 恆真短路），並於 `effects_3.rs / effects_4.rs` 濾除 `shanhai_* / ambient_*` 野獸入席對弈、圍爐與參議鄉約。
4. **第十九～二十二刀（`v0.39.119`）**：
   - **第十九刀（戰鬥載體、銘文與磨損去紙紮化）**：擴充 `is_weapon_or_tool_item` 納入千物總典工具與釣竿／剪刀／傘／算盤／木尺；將手持兵刃 `CarrierAffinity` 真實傳入 `clash.rs` 三相交鋒物理；修復徒手未共鳴白送 `+15%` 傷害倒掛；同步刻印 `vessel.catalog_id`（`wt_cat_*`）並為攻守雙方全類兵刃工器具記錄磨損與熟稔度。
   - **第二十刀（器物生態守恆、地標聚落化與偽醫館漏洞根治）**：拔除 `campfires_and_refining.rs` 燒鐵片（`mat_repair_strip`）當柴與修繕憑空印鈔 8 鎂；拔除 `supply_and_catalog.rs` 零庫存憑空生貨與泡茶扣鐵礦；擴充 `CANONICAL_SETTLEMENTS` 至全島 15 聚落；修正 `effects_1.rs` 水尾寮客運座標 `(-75, -280)` 與海事水險在地公庫清算；修正 `effects_2.rs / effects_4.rs` 日晷、幫廚、地籍圖之文曜硬編碼；修正 `effects_5.rs` `obj.name.contains("草")` 將馬料箱當醫榻秒消瘴氣滿血之漏洞；補齊 `grid_view_2.rs` 旅社「進」出入口。
   - **第二十一刀（活世界迴圈、AI 對話語料、Sanitizer 防誤傷與 Eco 端點修復）**：修正 `human_life_loop.rs / life_choices_and_factors.rs` 全島外送與食堂寫死文曜；修正 `dialogue_1.rs / dialogue_2.rs` 負數座標誤刪與「打算$\to$打好算、點燈$\to$清點完燈、氧化鎂$\to$受潮走色鎂」語義反噬；接通 `talk/execute.rs` 之 `resident_knowledge` 與 `db/archival.rs` 12 份職業語料 JSON；修復 `co_enact_script_pack` 誤拿玩家指令當 NPC 台詞；修復 `eco.rs` 思緒聚落誤傳職稱與 `O(N^2)` 微瀾去重。
   - **第二十二刀（前端 `web/*.js` 殘留正則清零）**：移除 `web/sw-panes.js` 裝備欄殘留之百字原材／食材正則與 `web/sw-object-list.js` 房間名稱猜謎，全面改用後端結構化欄位（SW 快取推進至 `20261106ap`）。
5. **第二十三～二十六刀（`v0.39.120`）**：
   - **第二十三刀（拔除事後 `.replace("A","B")` 擦屁股鏈與修正 `normalize_archaic_clothing` 語義污染）**：移除 `part_01_state.rs` 與 `eco.rs` 共 5 處每輪重複配置字串之 6 階 `.replace("買米菜回屋自炊熱粥", ...)` 廢棄補丁鏈；修正 `part_03_navigation.rs:241-276` 之 `normalize_archaic_clothing`，停止將千物總典正典物產（`十三檔烏木竹骨算盤` `wt_cat_0834`、`竹骨摺扇`、`硃筆` `wt_cat_0836`、`毛筆`）篡改為禁用現代詞（`隨身懷錶`、`鋼尖蘸水硬筆`、`計數板`）；於 `disposition.rs` 增設 `normalize_legacy_region_text` 零配置快速通道。
   - **第二十四刀（以結構化協定欄位拔除前端中文敘事正則爬蟲）**：於 `server/action/mod.rs` 目標離開同格時回傳 `suggested_action: "target_departed"`，並於 `trade/quote.rs` 與 `trade/settlement.rs` 議價待出價分支回傳 `open_trade: true`；更新 `web/main.js` 優先讀取 `msg.suggested_action === 'target_departed'` 與 `Boolean(msg.open_trade)`，更新 `web/sw-panes.js` 優先檢驗 `blank_manual / wt_blank_manual` 與 `category_tag === 'desk'`（SW 快取推進至 `20261106aq`）。
   - **第二十五刀（拔除 `resolve_venue_for_requirement` 文曜跳過網格檢索與 `db/identity.rs` 排程住民文曜兜底）**：移除 `part_04_store_crud.rs:509-778` 所有場域需求分支之 `if !is_wenyao` 網格檢索繞過，使全島 15 聚落（含文曜多井、多橋、多茶屋、多食堂）一律先於 `SquareGrid` 半徑 24～28 格內就近檢索；修正 `part_02_household.rs` 新增 `HOUSEHOLD_NAME_TO_SLOT_IDX` 解決 1,389 名外圍家戶住民按姓名反查落入文曜之臭蟲，並於 `db/identity.rs` 優先查驗 `resident_schedule_home_and_work` 與 `resolve_vocational_vessel`。
   - **第二十六刀（打通 `mut_wt_cat_*` 千物總典異變定義遞迴解析與基底產地溯源）**：修正 `item_definitions.rs` 之 `lookup_ecology_item_def`，解析 `mut_<base>_<arch>` 時遞迴查詢 `lookup_ecology_item_def(base_part)`，根除千物總典器物異變後被 `.unwrap_or(&STANDARD_FINISHED_GOODS[0])` 靜默洗成獵刀之臭蟲；修正 `crafting_and_mutation.rs` 依 `item_base_price_mg` 計算異變基底價，並於 `market_pricing.rs` 剝除 `mut_` 前後綴溯源真實產地聚落。
6. **第二十七～三十刀（`v0.39.121`：實機遊戲對話與活世界生活迴圈六大怪異設定清創＋PostgreSQL 1,987 筆污染記憶排毒）**：
   - **第二十七刀（拔除全島非磨坊住民兜底磨米漿、晴雨傘／活血膏當百工工具運使千次與秦水生水門台詞外溢）**：於 `life_choices_and_factors.rs` 攔截傘、扇、膏、藥、飲食、衣物、提籃作為百工工具，關閉排程與具名住民之 `allow_inventory_fallback`，補齊石匠、食堂攤商與文書當值之專屬產出分支，並於 `human_life_loop.rs` 限制水門插板與水輪門栓台詞僅對水輪石磨工坊人員生效。
   - **第二十八刀（拔除房間廣播「催動氣脈異變」、第三人稱劇本大綱與橋街劇本誤配）**：移除 `idle.rs` 之 `workshop_presence_reflex` 與單人唸誦 `trigger_and_causal_chain`，修正 `methods.rs` 之 `scripts_for_room`（橋樑與街道無精確劇本時直接回傳空陣列，不繼承聚落工坊劇本）與 `resolve_on_duty_actors_for_room`（無在場店員時不抓路人充當 `{當值主事}`）。
   - **第二十九刀（根除 `dialogue_2.rs` 動補前綴重疊語病、深夜喊「早啊！」與 `cognitive.rs` 英文 `【employment】`／`2/3` 分數報數）**：重排 `dialogue_2.rs` 動補複合詞優先順序（消滅「吃過完／吃過飽／檢視過查完／巡過完／歇過了」），新增 `align_dialogue_sky_time` 校正深夜與黎明前問候語；於 `cognitive.rs` 映射 `"employment"` 為 `"尋覓合適差事與活計"`、排除傘／膏作為修繕工具並將 `2/3` 分數改為自然器況口語。
   - **第三十刀（修復荒野森林誤掛鋸木坊櫃檯、水產行睡通鋪、陸地獵團獵深海土魠魚與資料庫清創）**：於 `venue.rs / venue_p2.rs` 排除荒野森林誤判為水動鋸木坊櫃檯；修正 `part_04_store_crud.rs` `InnOrHome` 回退為 `home_x/y`；修正 `part_01_state.rs` 與 `part_05_store_simulation.rs` 限制陸地獵團僅獵捕陸生山獸並限距 `<= 45` 格招募；執行 PostgreSQL 交易清理 `resident_knowledge`（刪除 1,517 筆）、`entities.thoughts`（清理 470 人）、`archival`（修復 154 筆、刪除 3 筆）與 `npc_npc_summaries`（修復 144 筆、刪除 1 筆）。
7. **第三十一～三十五刀（`v0.39.122`～`v0.39.123`：`ItemPhysicalClass` 強型別物理分類、`Store::load_entities_from_pg` 啟動雙端清創、`held_vessel` 職司矯正與 `Look` 單一外貌真相源重構）**：
   - **第三十一刀（`ItemPhysicalClass` 強型別物理分類與單位制／器況／穿戴動詞 SSOT）**：於 `src/game/object_ecology/item_physical_class.rs` 實裝 `ItemPhysicalClass::classify(item_id, item_name)`，全面取代 `look.rs`、`cognitive.rs`、`schema_and_continuity.rs`、`somatic_and_habits.rs` 與 `equipment.rs` 之中文字串子串猜謎，並於 `VesselStateRecord::status_badge` 改接 `label_for_item`（根除算籌、竹籃、書冊、晴雨傘誤標為「鋒利」）。
   - **第三十二刀（`Look` 單一外貌真相源、未結識 `陌生人` 視角邊界與橋街場域守衛）**：於 `src/server/action/look.rs` 當 `sensory_line` 存在時不再前綴重複體態句（根除雙體態拼接）；於 `look.rs` 與 `residents_seed.rs` 排除橋街路人與挑選顧客誤判為內場禁區掌事，並剝除 `ev.key_objects` 類別前綴（根除「織造服飾大紅錦繡霞帔」）。
   - **第三十三刀（`Talk` 快捷按鈕自然動作敘事與首次結識真名同行呈現）**：修正 `web/sw-object-list.js` 與 `web/sw-interactions.js` 不再將按鈕標籤當作「你說：『寒暄問候』」台詞，並修正 `src/server/action/talk/execute.rs` 將首次結識真名揭露併入同行（SW 快取推進至 `20261107as`）。
   - **第三十四～三十五刀（`Store::load_entities_from_pg` 啟動雙端同步清創、`held_vessel` 職司矯正與全員實機驗算硬閘）**：於 `src/store/entity.rs` 實裝 `sanitize_npc_equipment_and_inventory` 並接線至 `pg_sync.rs` 啟動載入同步回寫 PostgreSQL（阻斷記憶體 Tick 將舊傘、斗篷、432 把錯置獵刀回寫 DB）；修正 `households.rs` 與 `part_02_household.rs` 之 `held_vessel` 改傳 `work_name / work_purpose` 而非 `brief_tag` 外貌短語；於 `white_tower_living_world_check.rs` 增設全員零傘／斗篷污染與零鈍器「鋒利」實機驗算硬閘門。
8. **第三十六～三十九刀（`v0.39.124`：靜默均質化兜底根除、`.replace` 擦屁股鏈拔除、非店員稱謂校正與三道自動化防便宜程式探針）**：
   - **第三十六刀（根除跨海 `(0, 0)` 靜默兜底、工坊單一麻繩兜底與異變獵刀退化）**：於 `part_05_store_simulation.rs` 將獵團市集回退改為 60/180 格雙階檢索＋`pick_zone_artisan_waypoint` 在地錨點；於 `harvest_and_production.rs` 補齊紡織、皮革、窯陶、竹編油傘等工坊產線映射；於 `item_definitions.rs` 串接 `find_catalog_entry` 與 `ItemPhysicalClass` 分流，杜絕非刃具異變物退化為 `STANDARD_FINISHED_GOODS[0]`（精鍛防身獵刀）。
   - **第三十七刀（拔除 `crafting_and_mutation.rs` 12 連 `.replace` 鏈與 `resolve_viewer_npc_label` 非店員職稱誤配）**：於 `item_physical_class.rs` 抽離 `ItemPhysicalClass::strip_mutation_affixes` 並取代 `crafting_and_mutation.rs` 之 12 連 `.replace`；修正 `npcnpc/trigger/helpers.rs` 之 `!is_staff` 分支，禁止對非當值顧客套用 `venue.default_host_title` 或依房間名硬冠「匠人／醫工／學者」。
   - **第三十八～三十九刀（離場結構化訊號合約與 `living_world_evaluator.rs` 三道防便宜程式自動化探針）**：於 `server/action/mod.rs` 補齊 `target_departed` 結構化合約測試並推進 SW 快取至 `20261107at`；於 `living_world_evaluator.rs`（Evaluator 4）增設 **① 1,713 戶器物分布熵上限（`<= 30%`）與零傘／零鈍器「鋒利」探針、② 1,713 戶 `npc_sensory_appearance_line` 零括號外洩探針、③ 異變詞綴 SSOT 契約探針**。
9. **第四十刀（`v0.39.125`：`蘇曉萱` 五輪實機對話根因解剖——在途動作前綴優先序、私宅／天井公共場域誤判根除、鍛錘紮緊語病修正與執行期背包污染阻斷）**：
   - **在途移動與休憩優先於職業關鍵字（`src/server/action/talk/stance.rs`）**：修正 `format_npc_talk_line` 先檢查 `is_in_transit` 與 `is_resting_or_dining`，再檢查 `act.contains("鍛造") / act.contains("書") / act.contains("帳")`，根除前往 `【岑氏第一鍛造工坊】` 途中的匠師在 `三水匯流大石橋` 上被冠以 `"停下手中敲打構件的錘子"` 之矛盾敘事。
   - **公共場域檢索排除私宅與天井（`src/store/entity/part_04_store_crud.rs`、`src/npc/causal_core/planning_and_state.rs`）**：於 `resolve_venue_for_requirement` 與 `coherent_place_for_venue_req` 增設 `is_public_cell` / `!is_private_residence_leaf`（排除 `Terrain::House`、`residence`、`居 / 宅 / 舍` 與 `天井`），根除 `.contains("廚")` 誤配私宅 `陳氏庖廚居`、`.contains('井')` 誤配 `石板弄堂天井`、`.contains("湯")` 誤配 `白氏湯屋居`、`.contains("鋪")` 誤配 `海風麵點鋪居 / 修耙補犁鋪居`。
   - **器物保養動詞去地名綁定與工坊本職門檻校正（`src/npc/causal_core/vessel_and_arrival.rs`、`action_catalog.rs`、`src/game/object_ecology/harvest_and_production.rs`、`src/store/entity.rs`）**：修正 `VesselMaintained` 僅對編織容器（`籃 / 簍 / 袋 / 桶 / 網`）輸出 `"洗淨擦拭、重新紮緊"`（根除在天井把鍛錘「重新紮緊」）；將 `work_perform_steady_daily_vocation`（本職日課，`cost_mg: 0`）之 `min_mg` 由 `5` 調為 `0`，並阻斷 `WildernessForaged` 與 `harvest_and_production.rs` 向 `npc_*` 塞入 `mat_timber_log` 或觸發 `npc_autonomous_word_and_equip_step`；於 `sanitize_npc_equipment_and_inventory` 同步清理並回寫 PostgreSQL 歷史私宅／天井污染思緒（`陳氏庖廚居` 非住戶思緒 `117 -> 0`、`石板弄堂天井` 飲水與紮錘思緒 `236 -> 0`、`mat_timber_log` 污染 `-> 0`）。
