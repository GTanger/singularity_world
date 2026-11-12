# Singularity World — 當前交班狀態 (SESSION.md)

- **更新日期**：2026-11-12
- **當前狀態**：`in_progress`（已完成 `v2.3` 全系統七大範疇統一定案、微步 1～2、**微步 3-0 底座契約修補與白塔 `legacy_grid` 退役／`CellFacade` 四錨點雙讀對帳實裝驗收**；線上玩家行走區仍維持雙讀過渡，下一步實作**微步 3a′**）

## 一、 近期完成里程碑摘要（2026-10-27 ～ 2026-11-12）

1. **白塔半島底盤與四大防鬼打牆驗算閉環（2026-10-27 ～ 2026-11-08）**：
   - 完成「人版」五大數理成相模組（`src/combat/human_phase.rs`）、《白塔山海志》四層棲地引擎、1,713 戶槽位與千物總典 `PENINSULA_WORLD_OBJECTS` 外鍵硬閘門、第 1～47 刀去紙紮化清創與「零跳動 × 零沖刷 × 零走脫」四層耦合介面（`CLS = 0px`）。
2. **新生世界竹林地區第 1～10.3 章實裝與真機驗收（`Commit 204d4901` ～ `a558636e`）**：
   - 實裝竹林地形模版、6 純竹插件、5 天氣物件與 `竹葉營火`（`zl_bamboo_campfire`）地塊插件兼主畫面插座；支援 `[劈竹開路]`、`[紮槌並開路]` 就地綁紮施作與【行旅足跡輿圖】加權尋路**逐格正交自動邁步推進（絕不瞬移）**，通過 `Pixel 9 Pro XL` 真機驗收。
3. **能元台灣 `v2.3` 前後端全套規格定案（2026-11-12，Room `9154c446` / `a06fc981` / `a7e9b40d`）**：
   - **後端 `6B` `L0` 底板與稀疏立體四層槽**：鎖死 `L0Cell` 小端序 `6 Bytes`（`+500m` 偏移高程 `0xFFFF` 無資料、`2×2` 雙線性內插零台階、`landmass_id`、6 類母岩土層、`water_regime` 高 4 位常態水態＋低 4 位 16 階非線性水深桶、`base_energy`）；確立 `U2/U1/S0/V1 + Field` 與稀疏覆寫表 `cell_overlay`（廢除 `grid_cells` 全量刪寫）、台灣亭仔腳 `A1` 單節點連廊（`arcade_mask` 同排雙向互指、背牆封死）與全員共用 `entities.grid_z (0..=3)`。
   - **前端「輕操作後重體驗」深水區收斂**：確立「眼×手×腳」零數值稀缺文法（高處預警 vs. 下樓步差、雙手扛重料佔手隱去攔人／攀爬膠囊、零人工 CD 靠世界反制使情報過期）、在場熟路快走＋岔路當場帶代價押注（地標名不動，感官退居格緣角標如 `泉涼↑`／`水聲↗`）、`124px` 三席畫布互斥地景用途押注（禁野地攔路收過路費）、`z=0` 街坊見證 vs. `z=1..3`／冷徑私密無證，以及開機首屏「作品被具名鎮民接手」15 分鐘閉環。
4. **微步 1～2 實作與全地圖 `/world_atlas` 實裝驗收（`src/grid/substrate/` ＋ `cell_overlay` ＋ `substrate_atlas`，2026-11-12）**：
   - 接入內政部官方《不分幅全台 20m DEM》（`data/earth/moi_20m_dem/dem_20m.tif` + `.tfw`）與台灣 `OSM PBF` 水系（含 `Relation` 成員段與 MST 斷口橋接），完成四連通無斜縫界河加厚與多源 Flood-Fill 自然陸塊切割，產出 `data/runtime/landmass_01_substrate.swl0`（`Landmass #01` 全區 `3,077,437` 格 ≈ `1,231.0 km²`，其中非水域陸地 `2,959,767` 格 ≈ `1,183.9 km²`、西麓湧泉眼 `259` 處、原點 `(0,0)` 海拔 `+29.0m` 感官角標 `泉涼↑`），通過 5 項單元測試、東南西北跨河水密斷言與四宮格全彩母圖親眼視覺驗收。
   - 落地立體四層槽契約（`slots.rs`）、動態合成地名／符號／處境短句／三席畫布卡（`render_synth.rs`）、PostgreSQL 稀疏覆寫表 `cell_overlay` ＋ `entities.grid_z`（`pg/cell_overlay.rs`）、常駐 `SWL0` 執行期與 5 圖層純 Rust PNG 快取（`runtime.rs`），以及 `/api/substrate/{meta,basemap.png,window,cell}` 與設計者專用 `/world_atlas` 全開啟山海大地圖（嚴守玩家端 `index.html` 不暴露上帝視角連結定案）及主畫面九宮格 `edge_cue` 青墨角標。
5. **全系統七大範疇統一「底板插座 × 正交小插件槽 × 觀測時動態合成」定律定案（2026-11-12，規格書 §5 ＋ Wiki ADR v2.3）**：
   - 設計者檢視 `/world_atlas` 後拍板：拒絕「每格獨立大物件」與「地形類別死插件」，確立全世界七大範疇（①土地空間、②建築與店鋪、③人、④人際關係、⑤隨身器物與工具、⑥異獸與生態、⑦刻意不佔實體槽位之天氣與聚念場 `Field`）全面貫穿同一套「固定底板插座 × 正交小插件槽 × 觀測時動態合成（0 寫死標籤）」契約。
6. **微步 3 工程架構五小刀計畫定案（2026-11-12，Room `fba65005` 兩輪紅隊挑刺 ＋ 主模型原始碼收斂，Mnemopi `wm_0b52182326384f2a`）**：
   - **工程拒絕項**：拒絕「先改讀、後改寫」兩階段狀態撕裂、拒絕細狀態獨立子表（維持單列原子樂觀鎖 CAS）、拒絕額外 `6MB` 連通分量表（直用 `L0Cell.landmass_id`）、拒絕個人探索迷霧寫入 `cell_overlay`、拒絕 `307` 萬格同步寫入 `world_places`。
   - **拆分為五小刀依序實作**：`3-0`（底座修補、白塔 `legacy_grid` 解耦與 `CellFacade` 四錨點雙讀對帳）$\rightarrow$ `3a′`（`32×32` 分片正負雙快取、西麓騎樓街屋確定性模組、九宮格與單步切換）$\rightarrow$ `3b′`（竹林施作與 `entities.inventory` 同交易 CAS ＋ `inventory_seq` 序號守衛 ＋ 惰性 `settle()`）$\rightarrow$ `3c`（`grid_z (0..=3)` 三段房號、三維廣播與人數隔離、`floor:{z,exits}` 守 `CLS=0px`）$\rightarrow$ `3d`（新底板出生點、有界加權 A* 行旅與實體手機 `--phone-qa`）。
7. **微步 3-0 實裝與線上六道閘門驗收完成（2026-11-12）**：
   - `cell_overlay` 補齊 `building_id / roll_day / weather_epoch` SQL 讀寫並擴充 `charge SMALLINT DEFAULT -1`、`stamp INT DEFAULT 0`、`ver INT DEFAULT 1` 單列 CAS 樂觀鎖（`src/pg/cell_overlay.rs`）；`Entity` 與 `entities` 補齊 `grid_z` 與單調 `inventory_seq` 序號守衛（`pg/entity.rs`、`pg/writer.rs`、`pg/world_resource.rs`、`pg/world_trade.rs`、`db/inventory.rs`、`store/entity/part_04_store_crud.rs`）。
   - 停用 `sync_places_from_grid` 建置路徑並將 PostgreSQL 既有 `9,801` 處 `legacy_grid` 場所與 `32,561` 條路線全數置為 `status='retired', traversable=false`，線上格網角色 `place_id` 置 `NULL` 且實測單步移動往返零回填（白塔契約 `305` 場所與四大驗算器 `violations=0` 全綠）。
   - 落地 `src/grid/substrate/facade.rs`（`CellFacade`、`CellSnapshot`、`step_delay_ms`、`effective_max_z`）並接線 `/api/substrate/cell`，完成 `(0,0)` 伏流卵石灘、`(0,1)` 伏流冷泉、`(1,0)` 刺竹林、`(1,-1)` 古樟林坡四錨點雙讀對帳與 `tra-sw-gate --deploy` 六道關卡全綠。

## 二、 關鍵檔案與正典錨點

- **新生世界設計正典與 Wiki ADR**：
  - `/home/tanger/Projects/singularity_world/docs/design/energetic_taiwan_landmass_and_substrate_spec.md`（§1～§5.3 完整定案，含微步 3 五小刀契約）
  - `/home/tanger/Projects/singularity_world/docs/design/people_and_relations_plugin_socket_spec.md`（v2.3 定案版）
  - `/home/tanger/Projects/singularity_world/docs/design/shalu_zhulin_terrain_and_objects.md`（§1～§11 正式定案）
  - `/home/tanger/Projects/obsidian-vault/wiki/05-專案/Singularity-World/10-正典與ADR/星球暴脹橫斷面與人際關係插件插座正典.md`
- **Rust 實機代碼與體積邊界（單檔硬上限 `< 1,200` 行）**：
  - **新 `L0` 底板與立體槽位模組（微步 1～2）**：`src/grid/substrate/{l0_cell,coords,moi_dem,slots,render_synth,runtime}.rs`、`src/pg/cell_overlay.rs`、`src/server/http_api/substrate_atlas.rs`、`web/world_atlas.html`、`web/world-atlas.{js,css}`
  - **微步 3-0 雙讀門面與背包序號守衛**：`src/grid/substrate/facade.rs`、`src/pg/cell_overlay.rs`、`src/pg/world_place/grid_sync.rs`、`src/store/place.rs`、`src/store/entity/part_04_store_crud.rs`
  - `src/grid/shalu_zhulin.rs`（`1,046` 行；線上仍為舊 `121×81` 平面格網）、`src/grid/zhulin/terrain_bamboo.rs`（`1,123` 行）、`src/grid/zhulin/objects/affordance.rs`（`1,073` 行）

## 三、 下一步接續提示 (Next Cue)

- **微步 3 實作接續入口（已完成 `3-0`，當前下一步：`3a′`）**：
  1. **3-0（已完工）**：底座契約修補、白塔 `legacy_grid` 退役解耦、`inventory_seq` 守衛與 `CellFacade` 四錨點雙讀對帳。
  2. **3a′ 分片正負雙快取、西麓騎樓街屋與九宮格單步切換**：`32×32` 區塊正負快取、`pave_heat` 合併寫入、個人迷霧壓縮點陣圖、西麓騎樓街屋確定性推導模組，九宮格與單步移動切 `CellFacade + render_synth`。
  3. **3b′ 竹林就地施作與背包發物同交易切換**：劈竹、掘鞭、拾葉籜、營火切 `SlotStack + cell_overlay`，與 `entities.inventory`（`FOR UPDATE` ＋ `inventory_seq`）同交易提交，竹林路徑禁呼叫 `with_square_grid_mut`。
  4. **3c `Entity.grid_z (0..=3)` 立體樓層與人數隔離**：`grid:x:y[:z]` 解析、`(gx,gy,z)` 廣播與 `.sw-bustle-strip`、`.sw-cell-pop` 僅計 `z=0`、廢除 `IN_PRIVATE_ROOM`、前端 `floor:{z,exits}` 固定佔位守 `CLS=0px`。
  5. **3d 新底板出生點、有界加權 A* 與真機驗收**：`auth.rs` 出生點切換、有界加權 A*（`20,000` 節點／半徑 `128`／`10ms` 切片／併發 `2`）與木架 `Pixel 9 Pro XL` `--phone-qa` 親眼截圖驗收。
