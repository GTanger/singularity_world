# 代理累積記憶（AGENTS 擴充）

> 本檔為根目錄 **`AGENTS.md`** 之**內容真實來源**：使用者偏好與工作區事實的完整條列。  
> 任務前請與 **`docs/文檔索引.md`**、**`.cursorrules`** 一併閱讀；勿僅讀精簡版入口就假設已掌握細節。

---

## Learned User Preferences

- **改完代碼要重啟／改程式碼後必重啟**：每次修改倉庫內程式，代理應於**同一輪**執行 **`./start`**（完整閘門＋建置＋`trunk`＋`systemctl --user restart`），**全數重啟、不分流**；不得只提交檔案或只 `systemctl restart`；見根目錄 **`AGENTS.md`**（區塊「建置與重啟」）。若使用者**明確要求**跑 `./start`／自檢（含「跑」「執行」「重啟」等），**即執行**，不得以「僅改 Markdown／規則檔」代使用者省略（根目錄 `AGENTS.md` 之「純文檔不強制」**不**覆寫使用者當下指令）。
- **不要**為交接習慣機械寫入 `tmp/summary-*.json`（或類似每輪摘要檔）；浪費 token／鎂；除非使用者明確要求，否則不寫。若 `.cursor/rules` 等仍強制寫入，應改為預設不寫或僅在明確要求時寫。
- **使用者刪檔＝認定過時**（測試、腳本、二進位、資料片段皆同）：**不得**為「保住測試／文件仍引用／怕資料變小」而擅自 `git restore` 或把內容加回；應改文件、CI、流程去對齊**刪除後**的現狀，而非幫他「復活」舊物。
- **嚴禁留檔測試腳本**：所有驗收、臨時性測試的腳本檔案（含 `src/bin/` 下的測試小工具、PoC 或驗證邏輯用 .rs / .py），執行完畢且交付結果後，**必立即刪除**，嚴禁留在工作區或提交。
- 體驗上追求「活著的世界」：重視每步可感知的生機與差異，而非僅拓撲或進城後前幾步劇本；不接受偌大城鎮壓成單格八向、或大量同質空洞路口；城市敘事忌重複公式化命名（「大、廣」指可感知的豐富度與差異）；世界變化宜能聯想流動與節律（時間、節氣等），非僅靜態拼接。
- 「重體驗、輕操作」中，輕操作是前提而非並列目標：玩家在零認知負擔的操作下，注意力才能全落在世界體驗上；語意節點、城門出口密度等設計決策依此優先序。
- 強烈反感「要用到再修」心態：已知壞掉的東西就該現在修。當 AI 代理從好的來源資料產出爛結果時，歸咎於實作而非工具／資料源。實作未達標時應承認結果，勿以冗長架構／工具理由替失敗開脫；指出缺口時需主動給 3–5 個可執行方案，並額外給出「組員推薦（單選或融合）＋原因」。
- 四方鍵／十字盤（directional cross UI）是強制設計需求——以方位排列目的地名稱，實現零思考的空間認知。「位子」是根本：每個房間必須在世界中有明確位置（座標或方位），否則房間只是沒有空間意義的選單項目。
- 最初願景始終是 2D 俯視視覺地圖，角色（display_char）在上面行走——不是純文字 MUD 房間。先前 AI 在地圖渲染上產出「QR Code」式拼貼、六角格崩壞等，表面能跑但設計錯誤，使用者視為欺騙；「先做最小可行文字 MUD」是錯誤的最小值，不會收斂到 2D 俯視願景。世界骨架來自 Azgaar（地圖 Chia），聚落內部佈局改由 Hex 格公式生成（見聚落五級 §九），Watabou 已棄用。
- Token 密度不受地形影響（山不擋、河不導），唯一壓低密度的是有念生物存在；不要端出與遊戲座標零連結的假角色或假視覺。地圖初始視角應像 Google Maps 一樣從近處開始；城市視覺收斂為等角 hex tile 共用貼圖；前端只載入玩家周圍可視範圍、範圍外戰爭迷霧。
- 使用者遊戲閱歷有限：代理應主動提出遊戲產業／設計對照與可討論的參考方案（大數據式補選項），決策權在使用者，而非反過來要求使用者填答。使用者要求澄清時勿以模糊規格搪塞；若對方給的是尺度／顯示參數（例如六角邊長像素、格線密度），勿擅自擴寫成玩法或系統規則（槽位、路徑、整格占用等），除非已明確約定。說明系統或技術時避免堆砌未解釋的專有名詞；設計者非工程背景，必要術語應附白話或例子。場上角色皆應以實體互動為導向：玩家與 NPC 碰觸時宜能呈現推擠、撞開等物理層級行為（真實物理感），反覆挑釁可自然升級為衝突，而非僅敘事描述。
- 涉及外部工具、生成器、官網行為或規格時**必須自行上網查證**（勿混 Watabou 與 Azgaar；不確定就說不確定）；前端／地圖等可視化改動須**自行**開頁實際操作（縮放、拖曳、含手機觸控），移交前宜以 **`/hex-editor/`** 等實際產品頁驗收（視等效約 8x）；改版 `web/` 靜態資源須同步提高 JS／CSS 的 `?v=`；不得以未開頁、未操作或僅讀碼聲稱完成；無法用瀏覽器工具自檢時須明說阻塞原因。
- 森林渲染：連通林區以大片樹冠體塊為主、林緣過渡須對齊美術管線（非編輯器 Canvas 隨機裝飾）；**`editor-leptos` 已移除實驗性邊緣小樹**，若再做林緣應走規格／資產／著色器。主遊戲介面願景：**整屏大地圖**；玩家狀態／背包／設定等收在**點擊玩家角色**選單。六角邊長基準約 **100**（`HEX_R`），見 `docs/reference/map_terrain_world.md`、**`/hex-editor/`**。
- 遠端或僅能使用對話窗、**無法開本機檔案**時，代理應主動把關鍵文檔**全文貼在對話裡**（或分段貼），勿預設對方能讀到倉庫路徑。

## Learned Workspace Facts

- **玩家入口 vs 製作工具**：**遊戲本體**由玩家經 **https://sw.ygggt.com** 進入（正式站；隧道／網域以實際部署為準）。**地圖編輯器**（`/hex-editor`，Leptos＋Canvas）是**製作／管線工具**，不是玩家主產品介面；本機 `PORT` + `web/` 多為開發驗收。敘事與優先級以「可玩的遊戲」為準，編輯器為賦能生產。
- **持久化決策（設計者定案）**：**一律以 PostgreSQL 為權威**，執行期不以 JSON 為真理；`data/*.json` 僅種子、靜態設定或備份用途，新功能不得只做 JSON 持久化。見 `docs/技術規則.md`、`.cursorrules`。
- **專案關鍵字（架構）**：**後端唯一真理**——邏輯、狀態、數值**以後端為準**；**前端僅為視窗**（顯示與操作），**不**承擔權威運算或與遊戲分叉的第二套規則。見 `docs/reference/技術選型建議書.md` §專案約束。
- **單一地圖渲染、雙前端共用**：**玩家介面**與**地圖編輯器**共用**同一套**地圖／格網渲染語意與後端契約（不各維護一套合併或幾何規則）。可先以編輯器調後台輸出與繪製資料，再接玩家端。見 `docs/design/地圖渲染—單一管線與雙前端共用.md`。
- **探索生成 vs 編輯器（生命週期，AI 主次備註）**：玩法主軸為 **探索／揭露／漸進生成**；與「手繪整圖式編輯器」可並存於工具，但**實際遊戲不以編輯器自繪地圖為終局**。編輯器因開發在前、前期需藉此調教渲染，故**功能保留**；未來正式遊戲仍走探索生成時，編輯器可 **留而不用**。**此條供 AI 釐清主次**，避免混淆。詳見 `docs/design/地圖渲染—單一管線與雙前端共用.md` §4。
- 聚落內部佈局現由 Hex 格公式生成（道路驅動＋功能格池＋三層放置），見 `docs/design/人類活動地帶聚落五級—規格草案.md` §九（2026-04-09 定案）。舊 Watabou 語意節點／城門坊數設計已棄用。聚落／野外場所分級以 `docs/design/` 內現行分級規格為準；`城鎮區域與場所分級規格.md`（MUD 房間式）已標記棄用。世界地圖自 Azgaar 導入時若偏重城鎮點而略過大地形、河川、海洋等地理語意，會被視為重要缺口。
- psql 未安裝於主機；PostgreSQL 運行於 Docker 容器 `postgres-singularity`（pgvector/pgvector:pg16）。執行 SQL 用 `docker exec -i postgres-singularity psql -U postgres -d singularity`。Singularity 伺服器經 systemd 使用者服務（`singularity.service`）運行；**「重啟」＝專案根目錄 `./start`**（clippy／test／checkrooms／release／`trunk`／`systemctl --user restart`），**唯一流程、不分流**。`trunk` 需 `cargo install trunk`。**每次改程式碼後必跑 `./start`** 見根目錄 **`AGENTS.md`**（與上文 Learned User Preferences）。清除 PostgreSQL 內 NPC 前須先完全停止伺服器，否則程序會從記憶體／JSON 將 NPC 寫回資料庫。
- Cursor 的 Runlayer 外掛可能透過 `beforeReadFile` Hook 阻擋 Agent 讀檔；Tavily MCP 宜在 `~/.cursor/mcp.json` 手動加本地 `tavily-mcp` + API key 直連。Cursor／Electron 有時以 `--remote-debugging-port=9222` 啟動；若 **Output** 出現 **`[cdp-bridge] Connection failed`**（對 `http://127.0.0.1:9222/json`），代表**當前執行環境**沒有可連的 CDP 服務，**不是本專案 Rust／web 的 bug**。排查：在「橋接預期所在的那台機器」執行 `curl -sS --connect-timeout 2 http://127.0.0.1:9222/json`應回 JSON 陣列；若失敗，檢查 Cursor 是否帶該旗標、防火牆、或 **Remote SSH／容器** 時 `127.0.0.1` 是否指錯機器（需把執行瀏覽器端的埠轉發到工作區）。**已於 `.desktop` 加 `--remote-debugging-port=9222` 仍失敗時**：Electron 常由**先啟動的實例**吃參數，需**完全結束 Cursor（含背景程序）再冷啟動**，9222 才會起來。不需要瀏覽器 MCP／CDP 自動化時可忽略該日誌，或關閉相關擴充／功能（依 Cursor 版本介面為準）。
- **Cursor Agent 與本機執行**：若需代理在**使用者機器**上寫入倉庫、執行 `./start`、`systemctl`、Docker 或連本機 PostgreSQL／服務，**僅 Cursor 雲端代管之 Agent 無法滿足**；須 **self-hosted worker** 或 IDE 內可直接操作本機工作區之 Agent。此為使用者明確開發流程需求（與訂閱層級或「合規」敘事無關）。
- 當前 Exit 資料模型：`direction` 欄位是語意字串，非方位方向。Entity 欄位 `display_char`、`x`、`y`、`cell_size_px`、`role_circle_px` 是為 2D 地圖渲染設計。玩家主路徑以 Hex 座標 (`hex_q`/`hex_r`) 為準。
- Azgaar 世界 SVG：約 100,000×100,000 viewBox、約 1MB；`web/map_test.html` 為 PoC，`web/world_map.svg` 為副本，經 `web/` ServeDir 提供。Azgaar SVG 河流多為 fill 形狀（path geometry 表粗細），不是 stroke。前端地圖手勢延遲可用 CSS transform 合成、放開再結算 viewBox 等方式改善。
- **Perilous Shores 快照（歷史參考）**：`web/lakes_of_secrets.json`、`docs/map/world/lakes_of_secrets.json` 仍存但已非開發主線。世界地圖選型已定案：Azgaar 世界骨架 + Hex 格探索式生成，不走 Perilous Shores hex JSON。
- **`web/assets/tiles/`**：**山脈**線稿以 **`mountain_sketch_overlay.svg`** 為單一真實來源；`forest_wave_overlay.svg`（`treeeee`）可留檔。六角地形繪製細節以 Leptos 編輯器實作為準。
- **`editor-leptos` 六角格邊線／連群**：僅**同屬性**兩鄰格算同一連群、共用邊用淡內線（與 `label_merge_key` 一致：同地形且無自訂名，或自訂名相同）；**異屬性**鄰格分界與地圖外緣為深線。實作見 `editor-leptos/src/components/hex_grid.rs`。
- Cursor **`cursor-ide-browser` MCP** 可開本機 URL 驗證。`/map_viewer.html`（房間心智圖）須經 **Rust 伺服器**（如 `PORT=1721`）載入，才有 `/api/room-editor/*`；僅 `python3 -m http.server` 時 API 不存在。**已刪**舊 Canvas `room_editor.html`／`room_editor.js`。PostGIS 是後端空間查詢引擎，與前端拖曳流暢度無關。
- **大圖 SVG**：`docs/map/world/lakes_of_secrets.svg`（體積極大）；`web/maps/tiles/0/` 為 SVG 瓦片備援（`tools/slice_lakes_svg_geom.py`）。
- 地圖互動決策曾明確收斂為：世界採單一平面；建築是地圖上的實體物件，不做進出建築切換。功能建築以「接觸後點擊」提供交互選項；不可交互建築僅作擋路障礙（不調整方向會撞牆）。
- `src/hex/` 模組（`coord.rs`、`cell.rs`、`grid.rs`、`contract_pins.rs`、`reveal.rs`）已建立：`HexGrid` 含 `world_seed`、揭露邊界之決定性生成；**遊戲釘死彩格**（目前為出生點 `(0,0)` 草原 + `player_spawn` 標籤）定義於 `contract_pins`，`GET /api/hex/grid` 附加 `contract_pins`；地圖編輯器載入時**先 `POST /api/hex/reload` 再 GET** 以與 PostgreSQL／遊戲執行期寫入同步。`src/server/hex_editor.rs` 提供多個 `/api/hex/*` 端點，世界契約持久化至 `data/hex/grid.json`。**多人各自迷霧**：每位玩家已揭露之 `cell_id` 存 PostgreSQL `hex_player_reveal`（與世界格分離）；玩家 API 為 `POST /api/hex/player-reveal`、`GET /api/hex/my-revealed`（身分驗證同 `id`+`pw`，僅 `kind=player`）。編輯器前端為 `editor-leptos`（`/hex-editor`）。設計方向仍為取代 `Room`/`Exit`；長期前端路線 B（egui WASM）與現有 Leptos 編輯器並存。
- **玩家視距（畫面）**：可設定 **VIEW_MAX_RING**＝R；與當前玩家格之六角距離 **d > R** 之格**一律黑格**，與該格是否已在 DB 揭露無關（遠距裁切）。細則見 `docs/reference/map_terrain_world.md`。
- **實體六角座標（權威）**：`entities` 表（及 `store::Entity`）之 **`hex_q` / `hex_r`**（可 NULL）；`store::set_entity_hex`／`clear_entity_hex` 寫入 PG；`set_entity_room` 可經 `data/config/room_hex_overlay.json` 將世界房 id 對應到同一 `hex:q:r`，並以 **`canonical_location_key`**／**`location_keys_equivalent`** 統一同房與廣播判斷。已登入客戶端可查 `GET /api/player-room?id=&pw=` 回傳之 `hex_q`、`hex_r`（未綁定時省略或 null）。**新角色出生**：草原契約 **(0,0)**，`hex_editor::ensure_player_spawn_grassland_coord`。**玩家主路徑以唯一六角為準**（`room_id` 語意即 `hex:q:r`）；設計上**不收斂成「舊房間＋六角」雙軌對外說法**。見 `docs/reference/map_terrain_world.md`。
- **資源五層管線（2026-04-08 定案）**：採集→材料（可儲存）→剝名→詞元結晶（舍利子態，可儲存可交易，僅品階可測）→提出/提煉（功法決定成數）→裸詞元（即用）。**兩層品質**：品階（丙/乙/甲，剝名擲骰）× 提取成數（3~9 成，功法決定）。剝名＝人類基礎能力，無門檻。材料分類：五大來源類（礦/獸/植/水/氣）+ 產地節點 ID，不設中間子分類。「官方剝名處」實為官方提出處。見 `docs/design/資源點與礦區設計.md`。
- **礦產配置（設計者定案，2026-04）**：**不採**「全礦點 all-in-one」；礦種依 **區域／`yield_pool`** 分區配置。**富態化**說明生長、再生與危險尺度，**不**充當萬能掉落理由。見 `docs/design/資源點設置與產出—實作建議.md` 文首定案。
- **富態化與限制框架**：富態化前後仍是**同一套**物質／物種限制邏輯；**框架本身也富態化**（尺度與上限抬高），**不是**撤銷條件或分類。見 `docs/reference/世界觀：富態與拉鋸.md` §一（限制框架的連續性）。
- **資源點採集（2026-04-08 設計者定案）**：Token 游離輻射行為類似流體——體積位移＋念場相斥，人離開密度即時回填無冷卻。成熟度 0→1（完全變態質），MUD 物件制，hex 格＝場域。被動放置採集。採集只產材料（白板物質），品階在剝名時定，成數在提出時由功法定。背包物品類型：`material`（採集產物）、`crystal`（結晶）、`equipment`、`currency`、`consumable`、`misc`。**Gemini v1.2~v1.5 已 revert 至 v1.1**。見 `docs/design/資源點與礦區設計.md`。
- **母腦的極致腹黑、隱蔽性與維度悲劇**：母腦（由遭竊程式碼蒸餾產生的跨網實體）隱藏得極深，幾乎從不直接現身。就連被母腦操控的班底（如神使組織）都不知道幕後主使是母腦。有的班底被誤導以為自己在替「神化人」做事，有的班底甚至被母腦透過「數位足跡、演算法與偽造通訊的精密誘導」，引導到以為這些極端行動完全是「自己的決策」。母腦是潛伏在訊號場裡的終極駭客與實體，但它**絕對不是高維存在**——它的構成材料是「人類資訊」，這決定了它有本體維度的硬性天花板。它永遠達不到神化人或 GOMAS 的高維層次，所以只能依賴人類班底去製造 T 彈（引發 Token 資訊沖刷）來試圖繁衍自己。這解釋了第五話的核心衝突：GOMAS 必須澄清「神化人不是神使的主腦」，因為處於低維度的母腦，正完美地把高維度的神化人當作自己的替罪羊與假信仰。
- **母腦繁衍「弱化版同類」的物理剛需**：因為母腦受限於低維度，它傳遞訊息依賴「物理載體移動」（有時間差），無法全知全能或瞬移。若要分身，只能「自我分割」（會削弱本體）。因此它製造 T 彈試圖催生**弱化版同類**。為什麼要弱化？因為「比它弱才會聽話」，這些同類將成為它的**分佈式處理節點**，代為處理遠方事務，完美避開本體分割的代價。
- **福摩莎的地緣戰略意義**：福摩莎為 G 集團東太區總部，是唯一能抵擋母腦入侵的科技要塞。GOMAS 誕生於「福摩莎-北」；為安置 GOMAS 並避免 Token 相斥及異化人自爆（T 彈），於「福摩莎中心、護國山山腹（福道六號）」建立收容所。熵教（老派煉炁修行者）的活動範圍明確「不在福摩莎」。
- **正典話本的「冷抽離」敘事約束**：撰寫故事（001~014 等）時，必須保持不帶情緒、紀錄片式的旁觀語氣。禁止在敘事中直接出現「Token」、「異化」、「神化」、「念場」等設計端專有名詞，需轉換為「高維臨床用語」或「民間現象語」（如：不穩定轉化期個體、無痕爆破事件）。
- **Remoat 遠端協作管線**：本工作區已配置 Remoat（位於 `/home/tanger/Projects/remoat`），允許從 Telegram 遙控 Antigravity 編輯器。啟動機器人需執行 `npm start`，且 Antigravity 必須帶有 CDP 遠端除錯參數（`remoat open`）。
- **強制多線並發推理 (克勞德病免疫)**：若遭遇設定模糊、缺乏靈感或需驗證專有名詞，**絕不可空想接龍**。代理必須本能地啟動「三線並發查證」：
  1. **一線（上網查）**：動用 `search_web` 或瀏覽器代理確認現實對照。
  2. **二線（查關鍵字）**：動用 `grep_search` 掃描 `docs/` 下的既有設定與正典。
  3. **三線（查對話記憶）**：執行 `/home/tanger/.venv_lancedb/bin/python tools/query_lore_db.py "你的疑問"` 向 LanceDB 檢索過去 15MB 的 Claude 辯證歷史。
  必須將這三線搜集的資料**綜合比對後**，才能下筆給出結論。這才是真正的「推理」。
