# Singularity World — 當前交班狀態 (SESSION.md)

- **更新日期**：2026-11-12
- **當前狀態**：`in_progress`（已拍板並記錄 `v2.2` 能元台灣全島 `20m` 母圖打底、整數膨脹倍率 `k = 2`、全島統一 TWD97 座標定位與「北大甲溪～南烏溪」首塊不等大陸塊切割；當前正專注收斂「晶片底板物理針腳與立體槽位分割細節」，尚未動程式碼）

## 一、 近期完成里程碑摘要（2026-10-27 ～ 2026-11-12）

1. **白塔半島底盤與四大防鬼打牆驗算閉環（2026-10-27 ～ 2026-11-08）**：
   - 完成「人版」五大數理成相模組（`src/combat/human_phase.rs`）、《白塔山海志》四層棲地引擎、1,713 戶槽位與千物總典 `PENINSULA_WORLD_OBJECTS` 外鍵硬閘門、第 1～47 刀去紙紮化清創與「零跳動 × 零沖刷 × 零走脫」四層耦合介面（`CLS = 0px`）。
2. **新生世界竹林地區第 1～10.3 章實裝與真機驗收（`Commit 204d4901` ～ `a558636e`）**：
   - 實裝竹林地形模版、6 純竹插件、5 天氣物件與 `竹葉營火`（`zl_bamboo_campfire`）地塊插件兼主畫面插座；支援 `[劈竹開路]`、`[紮槌並開路]` 就地綁紮施作與【行旅足跡輿圖】加權尋路**逐格正交自動邁步推進（絕不瞬移）**，通過 `Pixel 9 Pro XL` 真機驗收。
3. **能元台灣全島 `20m` 母圖打底、`k = 2` 膨脹倍率與不等大陸塊切割定案（`v2.2`，2026-11-12）**：
   - **全島母圖先行、零拼接破口（`tra-council` Room `9154c446`）**：以內政部免費公開《全臺灣 20 公尺網格數值地形模型（DTM）》與全島連續水系一次鋪平**唯一全島底板母圖**，鎖死單一整數膨脹倍率 **`k = 2`**（面積 4 倍 ≈ `14.4 萬 km²`，1 點平滑展為 `2×2` 格零半格鋸齒，窄溪展寬為 `3～5` 格並執行四連通加厚）；全島唯一整數座標鎖定為 `gx = round((E-E₀)/10), gy = round((N-N₀)/10)`（原點 `(0,0)` 釘於西麓斷層湧泉處 `E₀=205,600m, N₀=2,681,000m`）。
   - **不等大自然陸塊遮罩與首塊開放範圍**：每格分配唯一 `LandmassId`，順應真實大溪、斷層與山脊切分約 `16～20` 片不等大自然陸塊；**首塊開放陸塊（`Landmass #01`）為「北界大甲溪 ～ 南界烏溪、西界海峽海岸 ～ 東界筏子溪」**（`42 km × 24 km` ＝ `2,100 × 1,200` 格，佔全島 `0.7%`，精準對應 `300～400` 名現代人後裔騎樓定居點；東界以斷橋湍流與對岸濃霧獸威取代空氣牆）。

## 二、 關鍵檔案與正典錨點

- **新生世界設計正典與 Wiki ADR**：
  - `/home/tanger/Projects/singularity_world/docs/design/energetic_taiwan_landmass_and_substrate_spec.md`（全島母圖、`k = 2` 與陸塊切割正典）
  - `/home/tanger/Projects/singularity_world/docs/design/people_and_relations_plugin_socket_spec.md`（v2.2 定案版）
  - `/home/tanger/Projects/singularity_world/docs/design/shalu_zhulin_terrain_and_objects.md`（§1～§11 正式定案）
  - `/home/tanger/Projects/obsidian-vault/wiki/05-專案/Singularity-World/10-正典與ADR/星球暴脹橫斷面與人際關係插件插座正典.md`
- **Rust 實機代碼與體積邊界（單檔硬上限 `< 1,200` 行）**：
  - `src/grid/shalu_zhulin.rs`（`1,046` 行；線上仍為舊 `121×81` 平面格網）、`src/grid/zhulin/terrain_bamboo.rs`（`1,123` 行）、`src/grid/zhulin/objects/affordance.rs`（`1,073` 行）

## 三、 下一步接續提示 (Next Cue)

- **當前唯一聚焦議題**：逐項敲定**「晶片底板（Substrate）物理欄位」與「單格立體槽位分割（Slot Partitioning）契約」細節**（拒絕空談長程史詩計畫，一步一議把底板與槽位插座定透後再動手實作）。
