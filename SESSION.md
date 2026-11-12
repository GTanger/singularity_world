# Singularity World — 當前交班狀態 (SESSION.md)

- **更新日期**：2026-11-12
- **當前狀態**：`in_progress`（已完成 `v2.3` 前後端全套規格定案紀錄［`6B` 晶片底板四層槽、亭仔腳 `grid_z` 契約、三席母模型「眼×手×腳」前端輕操作遊戲性］，並完成 **微步 1：內政部 `20m` 真實 DEM ＋ OSM 連續水系建構首塊陸塊 `Landmass #01` `L0` 晶片底板實檔與四宮格視覺驗收**；線上舊 `121×81` 格網保持零破壞隔離）

## 一、 近期完成里程碑摘要（2026-10-27 ～ 2026-11-12）

1. **白塔半島底盤與四大防鬼打牆驗算閉環（2026-10-27 ～ 2026-11-08）**：
   - 完成「人版」五大數理成相模組（`src/combat/human_phase.rs`）、《白塔山海志》四層棲地引擎、1,713 戶槽位與千物總典 `PENINSULA_WORLD_OBJECTS` 外鍵硬閘門、第 1～47 刀去紙紮化清創與「零跳動 × 零沖刷 × 零走脫」四層耦合介面（`CLS = 0px`）。
2. **新生世界竹林地區第 1～10.3 章實裝與真機驗收（`Commit 204d4901` ～ `a558636e`）**：
   - 實裝竹林地形模版、6 純竹插件、5 天氣物件與 `竹葉營火`（`zl_bamboo_campfire`）地塊插件兼主畫面插座；支援 `[劈竹開路]`、`[紮槌並開路]` 就地綁紮施作與【行旅足跡輿圖】加權尋路**逐格正交自動邁步推進（絕不瞬移）**，通過 `Pixel 9 Pro XL` 真機驗收。
3. **能元台灣 `v2.3` 前後端全套規格定案（2026-11-12，Room `9154c446` / `a06fc981` / `a7e9b40d`）**：
   - **後端 `6B` 晶片底板與稀疏立體四層槽**：鎖死 `L0Cell` 小端序 `6 Bytes`（`+500m` 偏移高程 `0xFFFF` 無資料、`2×2` 雙線性內插零台階、`landmass_id`、6 類母岩土層、`water_regime` 高 4 位常態水態＋低 4 位 16 階非線性水深桶、`base_energy`）；確立 `U2/U1/S0/V1 + Field` 與稀疏覆寫表 `cell_overlay`（廢除 `grid_cells` 全量刪寫）、台灣亭仔腳 `A1` 單節點連廊（`arcade_mask` 同排雙向互指、背牆封死）與全員共用 `entities.grid_z (0..=3)`。
   - **前端「輕操作後重體驗」深水區收斂**：確立「眼×手×腳」零數值稀缺文法（高處預警 vs. 下樓步差、雙手扛重料佔手隱去攔人／攀爬膠囊、零人工 CD 靠世界反制使情報過期）、在場熟路快走＋岔路當場帶代價押注（地標名不動，感官退居格緣角標如 `泉涼↑`／`水聲↗`）、`124px` 三席畫布互斥地景用途押注（禁野地攔路收過路費）、`z=0` 街坊見證 vs. `z=1..3`／冷徑私密無證，以及開機首屏「作品被具名鎮民接手」15 分鐘閉環。
4. **微步 1 實作與視覺驗收完成（`src/grid/substrate/` ＋ `energetic-substrate-build`，2026-11-12）**：
   - 接入內政部官方《不分幅全台 20m DEM》（`data/earth/moi_20m_dem/dem_20m.tif` + `.tfw`）與台灣 `OSM PBF` 水系（含 `Relation` 成員段與 MST 斷口橋接），完成四連通無斜縫界河加厚與多源 Flood-Fill 自然陸塊切割，產出 `data/runtime/landmass_01_substrate.swl0`（`Landmass #01` 實測 `2,959,767` 格 ≈ `1,183.9 km²`、西麓湧泉眼 `259` 處、原點 `(0,0)` 海拔 `+29.0m` 感官角標 `泉涼↑`），通過 5 項單元測試、東南西北跨河水密斷言與四宮格全彩母圖親眼視覺驗收。

## 二、 關鍵檔案與正典錨點

- **新生世界設計正典與 Wiki ADR**：
  - `/home/tanger/Projects/singularity_world/docs/design/energetic_taiwan_landmass_and_substrate_spec.md`（§1～§4 完整定案）
  - `/home/tanger/Projects/singularity_world/docs/design/people_and_relations_plugin_socket_spec.md`（v2.3 定案版）
  - `/home/tanger/Projects/singularity_world/docs/design/shalu_zhulin_terrain_and_objects.md`（§1～§11 正式定案）
  - `/home/tanger/Projects/obsidian-vault/wiki/05-專案/Singularity-World/10-正典與ADR/星球暴脹橫斷面與人際關係插件插座正典.md`
- **Rust 實機代碼與體積邊界（單檔硬上限 `< 1,200` 行）**：
  - **新晶片底板模組（微步 1）**：`src/grid/substrate/l0_cell.rs:157`（`6B L0Cell`）、`src/grid/substrate/coords.rs:106`（`SubstrateRegion` / `SWL0` / 感官角標）、`src/grid/substrate/moi_dem.rs:34`（內政部 20m DEM 雙線性內插與四連通加厚）、`src/bin/energetic_substrate_build.rs:172`（母圖建置與四宮格審計）
  - `src/grid/shalu_zhulin.rs`（`1,046` 行；線上仍為舊 `121×81` 平面格網）、`src/grid/zhulin/terrain_bamboo.rs`（`1,123` 行）、`src/grid/zhulin/objects/affordance.rs`（`1,073` 行）

## 三、 下一步接續提示 (Next Cue)

- **微步 2 接續入口**：在 `src/grid/substrate/` 之上建立**立體槽位型別契約（`U2 / U1 / S0 / V1 / Field`）與 PostgreSQL 稀疏覆寫表 `cell_overlay` + `entities.grid_z (0..=3)` 持久層**，實作純底板＋槽位動態合成地名／符號／處境短句／感官角標之無手寫文字推導器，並做局部切片真機預覽驗收。
