## Learned User Preferences

- 體驗上追求「活著的世界」：重視在這世界裡每一步是否可感知生機與差異，而非僅有空間拓撲或進城後前幾步劇本。
- 不接受偌大城鎮被壓成單一格僅剩八向出口；也不接受大量同質、空洞的路口房；兩者都與「活著的世界」相衝突。
- 城市敘事若重複感強、命名過度公式化，會直接要求改進（文案池、拆層、命名規則等）。「大、廣」主要指可感知的豐富度與差異，不限於地圖格數或節點數量。
- 世界生機與變化應能聯想到流動與節律（含時間推移、節氣等），而非僅靜態空間拼接。
- 「重體驗、輕操作」中，輕操作是前提而非並列目標：玩家在零認知負擔的操作下，注意力才能全落在世界體驗上；語意節點、城門出口密度等設計決策依此優先序。
- 強烈反感「要用到再修」心態：已知壞掉的東西就該現在修。當 AI 代理從好的來源資料產出爛結果時，歸咎於實作而非工具／資料源。實作未達標時應承認結果，勿以冗長架構／工具理由替失敗開脫；指出缺口時需主動給 3–5 個可執行方案，並額外給出「組員推薦（單選或融合）＋原因」。
- 四方鍵／十字盤（directional cross UI）是強制設計需求——以方位排列目的地名稱，實現零思考的空間認知。「位子」是根本：每個房間必須在世界中有明確位置（座標或方位），否則房間只是沒有空間意義的選單項目。
- 最初願景始終是 2D 俯視視覺地圖（Azgaar 世界＋Watabou 城市），角色（display_char）在上面行走——不是純文字 MUD 房間。先前 AI 在地圖渲染上產出「QR Code」式拼貼、六角格崩壞等，表面能跑但設計錯誤，使用者視為欺騙；「先做最小可行文字 MUD」是錯誤的最小值，不會收斂到 2D 俯視願景。
- Token 密度不受地形影響（山不擋、河不導），唯一壓低密度的是有念生物存在；不要端出與遊戲座標零連結的假角色或假視覺。地圖初始視角應像 Google Maps 一樣從近處開始；城市視覺收斂為等角 hex tile 共用貼圖；前端只載入玩家周圍可視範圍、範圍外戰爭迷霧。
- 使用者遊戲閱歷有限：代理應主動提出遊戲產業／設計對照與可討論的參考方案（大數據式補選項），決策權在使用者，而非反過來要求使用者填答。使用者要求澄清時勿以模糊規格搪塞；若對方給的是尺度／顯示參數（例如六角邊長像素、格線密度），勿擅自擴寫成玩法或系統規則（槽位、路徑、整格占用等），除非已明確約定。說明系統或技術時避免堆砌未解釋的專有名詞；設計者非工程背景，必要術語應附白話或例子。場上角色皆應以實體互動為導向：玩家與 NPC 碰觸時宜能呈現推擠、撞開等物理層級行為（真實物理感），反覆挑釁可自然升級為衝突，而非僅敘事描述。
- 涉及外部工具、生成器、官網行為或規格時**必須自行上網查證**（勿混 Watabou 與 Azgaar；不確定就說不確定）；前端／地圖等可視化改動須**自行**開頁實際操作（縮放、拖曳、含手機觸控），移交前縮至約 8x（`web/watabou_test.html` 可用 `?zoom=8` 或 `?z=8`）；改版 `web/` 靜態資源須同步提高 JS／CSS 的 `?v=`；不得以未開頁、未操作或僅讀碼聲稱完成；無法用瀏覽器工具自檢時須明說阻塞原因。
- Watabou 式森林的語意是連通林區以大片樹冠體塊為主、僅在與非林地形交界處有一兩圈帶樹幹的邊緣樹，不是逐格密植單木。僅靠結構化 JSON 硬畫「圖章式」細節易像貼紙、難復原手繪美學，與原作差距會直接摧毀信任；實作須以實際畫面（含約 8x）為交付物，不能只依文字規格改完自以為完成。
- 主遊戲介面願景：**整個畫面就是大地圖**；與玩家自身相關的（狀態、背包、設定等）收在**點擊玩家角色**後彈出的選單，不佔主畫面固定欄。六角格預設尺度目前約定**邊長 100**（世界／美術基準，可再調），見 `docs/reference/map_terrain_world.md` 與 `web/watabou_test.html` 之 `HEX_R`。

## Learned Workspace Facts

- **持久化決策（設計者定案）**：**一律以 PostgreSQL 為權威**，執行期不以 JSON 為真理；`data/*.json` 僅種子、靜態設定或備份用途，新功能不得只做 JSON 持久化。見 `docs/技術約束規則.md`、`.cursorrules`。
- Watabou 進城後的玩家圖已改為語意節點（城門＋各區坊等），並以 `data/config/city_ambience.json` 驅動環境與門檻；坊數超過門檻時，城門經外街／扇區中繼再連到各坊。設計共識：Azgaar／Watabou 幾何輸出作系統層基底；介面層需編譯成較少語意節點，不宜僅做 GeoJSON 與 Room 的 1:1 直譯。聚落／野外場所分級以 `docs/design/` 內現行分級規格（如城鎮區、野外大地圖區）為準；早期純 MUD 式討論留下的分級表述已過時，引用或對照前需先對照現行文件。世界地圖自 Azgaar 導入時若偏重城鎮點而略過大地形、河川、海洋等地理語意，會被視為重要缺口。
- psql 未安裝於主機；PostgreSQL 運行於 Docker 容器 `postgres-singularity`（pgvector/pgvector:pg16）。執行 SQL 用 `docker exec -i postgres-singularity psql -U postgres -d singularity`。Singularity 伺服器經 systemd 使用者服務（`singularity.service`）運行；`./start` 觸發 `systemctl --user restart`。清除 PostgreSQL 內 NPC 前須先完全停止伺服器，否則程序會從記憶體／JSON 將 NPC 寫回資料庫。
- Cursor 的 Runlayer 外掛可能透過 `beforeReadFile` Hook 阻擋 Agent 讀檔；Tavily MCP 宜在 `~/.cursor/mcp.json` 手動加本地 `tavily-mcp` + API key 直連。Cursor／Electron 有時以 `--remote-debugging-port=9222` 啟動；若 **Output** 出現 **`[cdp-bridge] Connection failed`**（對 `http://127.0.0.1:9222/json`），代表**當前執行環境**沒有可連的 CDP 服務，**不是本專案 Rust／web 的 bug**。排查：在「橋接預期所在的那台機器」執行 `curl -sS --connect-timeout 2 http://127.0.0.1:9222/json` 應回 JSON 陣列；若失敗，檢查 Cursor 是否帶該旗標、防火牆、或 **Remote SSH／容器** 時 `127.0.0.1` 是否指錯機器（需把執行瀏覽器端的埠轉發到工作區）。**已於 `.desktop` 加 `--remote-debugging-port=9222` 仍失敗時**：Electron 常由**先啟動的實例**吃參數，需**完全結束 Cursor（含背景程序）再冷啟動**，9222 才會起來。不需要瀏覽器 MCP／CDP 自動化時可忽略該日誌，或關閉相關擴充／功能（依 Cursor 版本介面為準）。
- 當前 Exit 資料模型：`direction` 欄位是語意字串，非方位方向。Azgaar 房間有 `meta.x`／`meta.y` 世界座標；Watabou GeoJSON 含完整幾何與 2D 座標——空間資料齊全但尚未從中導出方位方向。Entity 欄位 `display_char`、`x`、`y`、`cell_size_px`、`role_circle_px` 是為 2D 地圖渲染設計。
- Azgaar 世界 SVG：約 100,000×100,000 viewBox、約 1MB；`web/map_test.html` 為 PoC，`web/world_map.svg` 為副本，經 `web/` ServeDir 提供。Azgaar SVG 河流多為 fill 形狀（path geometry 表粗細），不是 stroke。前端地圖手勢延遲可用 CSS transform 合成、放開再結算 viewBox 等方式改善。
- Watabou Perilous Shores 區域野外地圖（even-q hex、terrain／rivers／roads／features 等結構化資料）：試驗頁 `web/watabou_test.html` 以同目錄的 `web/lakes_of_secrets.json` 載入；`docs/map/world/lakes_of_secrets.json` 為同快照副本（供文件／對照）。名稱與枚舉見 `docs/map/watabou_lakes_of_secrets_name_list.md`。Watabou 原站視覺是「圖章系統」；巨大 SVG 因海量獨立 path。世界地圖選型：Azgaar 世界級 SVG vs Watabou Perilous Shores（hex JSON），二選一；世界圖與城市圖比例尺不同，進城切圖、出城切回。
- `web/watabou_test.html`（Perilous Shores JSON 六角底圖）：河／湖視覺銜接須維持既有 `hexToPixel`，勿在未確認前改投影。匯流與入湖應只平移該動的一端，不要在折線上 `splice` 插入頂點；繪製順序固定為支流 → 主流 → 水域 hex；河線與湖同色。驗收時宜開 `?zoom=8`；`zoom≥4` 時 URL 可自動把鏡頭對準臨水／陸格，可加 `&focus=center` 看幾何中心。陸／水／山脈底與裁切共用 **`hexCellCircumradius()`**（`HEX_R`＋**`HEX_AA_OVERLAP_PX`**／`camScale`）。**`WOBBLE` 須維持 0**（正六角正接）。山脈為「地形色塊＋**`mountain_sketch_overlay.svg`** 線稿置中疊加」（`drawHexMountainOverlay`），非整格含六角底之 `mountain_sketch.svg`。
- `web/assets/tiles/`：PNG 同上；**山脈**線稿以 **`mountain_sketch_overlay.svg`** 為單一真實來源，但 `watabou_test.html` 內另嵌 **`MOUNTAIN_OVERLAY_INLINE_SVG` → data URL** 載入（避免相對路徑、損壞註解導致 SVG 無法畫上 Canvas）。`forest_wave_overlay.svg`（`treeeee`）可留檔，**試驗頁已暫撤**森林疊圖，森林僅地形色塊；繪製：先地形色塊再疊 `drawHexMountainOverlay`。`MERGED_LAND_FILL=false` 時 `TERRAIN_ALPHA=1`。
- Cursor **`cursor-ide-browser` MCP** 可開本機 URL 驗證 Canvas／靜態頁。`web/room_editor.html`（房間心智圖編輯器）須經 **Rust 伺服器**（如 `PORT=1721`）載入，才有 `/api/room-editor/*` 與圖譜資料；僅用 `python3 -m http.server` 開頁時 API 不存在、圖譜無法正常載入，不應以此當作驗收依據。PostGIS 是後端空間查詢引擎，與前端拖曳流暢度無關。
- **Perilous Shores（Lakes of Secrets）主入口**：`web/watabou_test.html`（**even-q 六角格**＋`web/lakes_of_secrets.json` Canvas 渲染）。`web/map_tiles.html` **僅轉址**至 `watabou_test.html`（保留 query，如 `?zoom=8`）。若改動六角頁或 JSON 管線，交付前須瀏覽器 MCP 自檢（縮放／拖曳／`?zoom=8`），**console** 無錯。大圖 SVG 位於 `docs/map/world/lakes_of_secrets.svg`（體積極大，非主路線）；`web/maps/tiles/0/` 為 **SVG 瓦片備援**（`tools/slice_lakes_svg_geom.py` 真實切片）。
- 地圖互動決策曾明確收斂為：世界採單一平面；建築是地圖上的實體物件，不做進出建築切換。功能建築以「接觸後點擊」提供交互選項；不可交互建築僅作擋路障礙（不調整方向會撞牆）。
- `src/hex/` 模組（`coord.rs`、`cell.rs`、`grid.rs`、`reveal.rs`）已建立：`HexGrid` 含 `world_seed`、野外決定性揭露生成；`src/server/hex_editor.rs` 提供多個 `/api/hex/*` 端點，世界契約持久化至 `data/hex/grid.json`。**多人各自迷霧**：每位玩家已揭露之 `cell_id` 存 PostgreSQL `hex_player_reveal`（與世界格分離）；玩家 API 為 `POST /api/hex/player-reveal`、`GET /api/hex/my-revealed`（身分驗證同 `id`+`pw`，僅 `kind=player`）。編輯器前端為 `editor-leptos`（`/hex-editor`）。設計方向仍為取代 `Room`/`Exit`；長期前端路線 B（egui WASM）與現有 Leptos 編輯器並存。
- **玩家視距（畫面）**：可設定 **VIEW_MAX_RING**＝R；與當前玩家格之六角距離 **d > R** 之格**一律黑格**，與該格是否已在 DB 揭露無關（遠距裁切）。細則見 `docs/reference/map_terrain_world.md`。
- **實體六角座標（權威）**：`entities` 表（及 `store::Entity`）之 **`hex_q` / `hex_r`**（可 NULL）；`store::set_entity_hex`／`clear_entity_hex` 寫入 PG；`set_entity_room` 可經 `data/config/room_hex_overlay.json` 將世界房 id 對應到同一 `hex:q:r`，並以 **`canonical_location_key`**／**`location_keys_equivalent`** 統一同房與廣播判斷。已登入客戶端可查 `GET /api/player-room?id=&pw=` 回傳之 `hex_q`、`hex_r`（未綁定時省略或 null）。**新角色出生**：草原契約 **(0,0)**，`hex_editor::ensure_player_spawn_grassland_coord`。**玩家主路徑以唯一六角為準**（`room_id` 語意即 `hex:q:r`）；設計上**不收斂成「舊房間＋六角」雙軌對外說法**。見 `docs/reference/map_terrain_world.md`。
