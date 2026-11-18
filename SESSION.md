# Singularity World — 當前交班狀態 (SESSION.md)

- **更新日期**：2026-11-18
- **當前狀態**：`active`（已完成 `v21`（`v=20261118a19`）：土地動態演變閉環 —— 持鎬翻土開壟、青草藥圃／紅土甘薯旱田栽收、案台搗製`敷傷藥膏`消傷，以及山林砍伐竹筍與野生藥草雨露自然萌發再生；承接 `v9`～`v20` 全數閉環並同步遠端推存）

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
8. **2026-11-18 `v9`～`v14` 基底與雙軌靜態立繪去紙娃娃化（`A` 帳密登入 ＋ 48 候天色 ＋ 白描建造動作 ＋ `A+B` 靜態生成立繪 ＋ 零測試外露清理）**：
   - **後端與資料庫持久化**：新增 `src/pg/rim_persist.rs`（`rim_http_sessions`、`rim_actor_state`、`rim_micro_overlay` 三表隔離持久化，不污染 `entities` NPC 與 `white-tower-living-world-check --live`）、`src/grid/substrate/rim_seed.rs`（西麓竹屋 `(7..=11, 1..=5)` 含門 `(7,3)`、北溪木橋 `(0, 10..=13)` 與 `林阿木`/`陳青`/`許秋娘`）、`src/server/http_api/rim_auth.rs`（`POST /api/rim/auth`、`GET /api/rim/me`、`POST /api/rim/logout` 與同源 `HttpOnly` Cookie `sw_rim_sid`）。
   - **前端整合與清創**：`web/rim-sky-time.js`（48 色天光與 `destination-out` 營火工案夜燈）、`src/grid/substrate/rim_portrait_gen.rs` ＋ `src/server/http_api/rim_portrait_api.rs` ＋ `web/rim-portrait.js`（淘汰紙娃娃分層，依角色先天相貌與裝備結構轉寫簡約淡彩提示詞直出 `2:3` 靜態立繪，自備 OpenRouter 走 `google/gemini-3.1-flash-image` 否則退守本機 `FLUX.2-klein-4B`；已全數移除測試試玩頁、控制台、後端硬體詞與提示詞外露）。
9. **2026-11-18 `v15` 依 `docs/DESIGN_KEYWORDS.md` 推進活態世界四大閉環（`v=20261118a10`）**：
   - **生理代謝與零數值感官轉譯**：新增 `src/grid/substrate/rim_life.rs`，後端嚴格追蹤飢渴疲與隨身鎂晶，前端與 `/api/rim/state` 僅轉譯為台灣在場體感（如`腹中溫飽`、`喉間潤澤`、`碎鎂少許`）；支援清淺山澗就地`掬泉飲水`、`竹葉營火`火烤`煨冬筍`/`炙獸肉`與居民生理急迫自主求生。
   - **隨身穿戴與飲食打造**：新增 `RoastedShoot`/`RoastedMeat`/`BambooHat`/`BambooPack`，`toggle_wear` 嚴格守恆卸穿不複製、工案可編製竹笠背簍、`食用`即時消解飢腸，且頭戴／肩背與持器同步驅動俯視白描小人與 `AI` 靜態立繪。
   - **陌生人識名、近身當面交談與鎂晶易物**：新增 `src/server/http_api/rim_talk_api.rs`（`POST /api/rim/talk`、`POST /api/rim/trade`）；未結識居民顯示`陌生獵戶`/`陌生茶娘`等職稱，當面開口始永久寫入 `known_by` 揭露真名，對話優先走玩家自備 OpenRouter 或本機 `11436` `Qwen3-4B-Instruct-2507-Q8_0.gguf` 並鎖定聚落名冊在場事實防幻覺編名，同時進行雙向物品與鎂晶原子守恆交易。
   - **延伸第二聚落「水尾寮・溪畔茶寮」**：`rim_seed.rs` ＋ `rim_sim.rs` 冪等闢出東南小徑（`mx: 6..=28`）與水尾寮茶寮（`mx: 28..=36, my: -10..=-4`，營火 `(31,-7)`、工案 `(34,-6)`、儲物墊 `(34,-8)` 堆放 `煨冬筍 × 3`），駐有`周阿茶`（茶寮庖廚）與`鄭石頭`（行腳貨郎），已通過 `30` 項單元測試、`tra-sw-gate` 六關、桌機瀏覽器與 `Pixel 9 Pro XL` 真機視覺驗收。
10. **2026-11-18 `v16`～`v18` 實體非重疊碰撞、全地圖白描圖素升級與 `gemini-3.1-flash-image` 切換（`v=20261118a16`）**：
    - 新增 `src/grid/substrate/rim_physics.rs`（`670` 行）實作 4-pass 硬 PBD 圓對圓實體防重疊碰撞與狹路讓行，並與前端 `findClientPath`/`trimPassedWaypoints` 達成 `0ms` 無卡頓移動與自動採筍入囊（`34/34` 單元測試通過）。
    - 補齊 15 張正 `90°` 垂直俯視「宣紙白描淡彩」去背 PNG 圖素（營火、百工案台、竹編草席、渡溪木橋、原木角柱、春筍、全物資與工具、紅土野山豬／鐵羽竹雞），拔除 `[竹5]` 白底方框改為實物堆疊與 `×N` 描邊字，通過桌機與 `Pixel 9 Pro XL` 雙機真機驗收；並將 OpenRouter BYOK 預設繪圖模型由 `google/gemini-2.5-flash-image` 升級為 `google/gemini-3.1-flash-image`。
11. **2026-11-18 `v19` 人版歷練成相、力竭救援與雙聚落貨郎經歷閉環（`v=20261118a17`，`c88e8457`）**：
    - 新增 `src/grid/substrate/{rim_phase,rim_cmd}.rs` 並於 PostgreSQL `rim_actor_state` 冪等擴充 `phase_json` 持久化：將先天骨相（`InnateBonePhase::from_seed`）與實地採作／烹煮／解剖／戰鬥歷練接入 `crate::combat::human_phase` 三相武鬥、時序斷招與庖丁解牛（熟練解牛出肉增至 `3` 份），每位居民依其命盤與歷練凝出專屬去數值武百工招式名號。
    - 取消戰敗原地抹除與玩家瞬移重生，實作`力竭倒地`（側臥視覺姿態）、患處傷勢降速、`RescueAlly` 健全同袍就近背回營火，以及營火畔調理復原；並打通`西麓湧泉 ↔ 水尾寮`貨郎守恆擔貨往返與 `recent_diary` 見聞自動寫入 `POST /api/rim/talk` 事實提示詞。
    - 前端 `[自身]` 彈窗補齊可點按身手名號與白話詳解、`林緣巡採／百工營造／護衛獵巡／爐畔靜養`四擇行事取向與`近期駐世經歷`，居民檢視卡同步顯示專屬身手名號（`38/38` `grid::substrate` 單元測試通過）。
12. **2026-11-18 `v20` 開譜曆山雨、單平面圍合室內、`竹編睡榻`與「做法 B 屋外見頂進屋掀頂」閉環（`v=20261118a18`，`a416b465`）**：
    - 新增 `src/grid/substrate/rim_shelter.rs`（`494` 行）：依開譜曆季節與兩時辰窗口決定`晴風和暢／山嵐微雨／瀟瀟山雨`；四連通洪泛自動掃描竹木圍牆與院門閉合區域為單平面`EnclosedRoom`，並為既有西麓草寮 `(8,4),(8,2)` 與水尾寮茶寮（補西門 `(28,-7)`、床榻 `(35,-5),(35,-9)`）及玩家自建屋舍統一提供屋簷遮蔽、屋外戴笠擋雨 vs. 無笠濕冷疲勞，以及上榻加速消傷回氣（`42/42` `grid::substrate` 單元測試通過）。
    - 前端按拍板定案的**做法 B（屋外見頂、進屋掀頂）**於屋外繪製正 `90°` 俯視宣紙白描茅竹斜脊屋頂與出簷門罩，進門、靠近或選取室內時平滑掀開，露地營火受雨轉弱冒煙、室內溫潤竹席＋`竹編睡榻`棉被階層與夜雨門燈餘暈同步生效。
13. **2026-11-18 `v21` 土地動態演變：翻土開壟、藥圃甘薯旱田、敷傷藥膏治傷與山林雨露再生閉環（`v=20261118a19`）**：
    - 新增 `src/grid/substrate/rim_ecology.rs` 並於 PostgreSQL `rim_micro_overlay` 冪等擴充 `eco_ticks`：持`石刃木鎬`（含行囊自動換持）可於沃土紅礫翻鬆出`翻鬆空田壟`，單擊播栽`青草藥苗壟 -> 青草藥圃`或`甘薯新苗壟 -> 紅土甘薯壟`，成熟收成 `青草藥 × 2`／`紅土甘薯 × 2`（營火可烤成`炭烤甘薯`）且原地保留田壟免瑣碎重翻，山雨加倍催芽；案台可用`青草藥`搗製`敷傷藥膏`，於 `[裝備／行囊]` 點 `[敷貼治傷]` 或由負傷同伴自主敷藥／搗藥，立即消除 `injury_part` 傷勢。
    - 聚落淨空帶外被採空之竹叢／春筍／`山林青草藥`隨開譜曆雨露累積 `eco_ticks` 後自然萌發長回並自動清除覆寫列（`ov.is_pristine() == true`，防資料庫膨脹）；前端於 `web/rim-world.js` 補齊手繪宣紙白描隆起土壟、鋸齒藥草蓮座與紫白花苞、心形甘薯葉與露土塊根及 5 張 128×128 離屏白描物資圖素（`46/46` `grid::substrate` 單元測試通過）。

## 二、 關鍵檔案與正典錨點

- **新生世界設計正典、關鍵字總表與 Wiki ADR**：
  - `/home/tanger/Projects/singularity_world/docs/DESIGN_KEYWORDS.md`（世界核心關鍵字清單與行為規範）
  - `/home/tanger/Projects/singularity_world/docs/design/energetic_taiwan_landmass_and_substrate_spec.md`（§1～§5.3 完整定案，含微步 3 五小刀契約）
  - `/home/tanger/Projects/singularity_world/docs/design/people_and_relations_plugin_socket_spec.md`（v2.3 定案版）
  - `/home/tanger/Projects/obsidian-vault/wiki/05-專案/Singularity-World/10-正典與ADR/星球暴脹橫斷面與人際關係插件插座正典.md`
- **活躍 2D 世界實作與單檔 `< 1,000` 行模組群**：
  - **2D 底板與先鋒聚落生命迴圈**：`src/grid/substrate/{l0_cell,coords,moi_dem,slots,render_synth,runtime,facade}.rs`、`src/grid/substrate/{rim_world,rim_sim,rim_ai,rim_life,rim_seed,rim_physics,rim_phase,rim_cmd,rim_shelter,rim_ecology,rim_portrait_gen}.rs`
  - **持久化與 HTTP 端點**：`src/pg/{cell_overlay,rim_persist}.rs`、`src/server/http_api/{substrate_atlas,rim_api,rim_auth,rim_portrait_api,rim_talk_api}.rs`、`web/{index.html,rim-world.js,rim-world.css,rim-portrait.js,rim-sky-time.js,sw.js}`

## 三、 下一步接續提示 (Next Cue)

- **依 `docs/DESIGN_KEYWORDS.md` 續推下一階段活態細節**：
  1. **沿真實地形外推鄰近市鎮與五大店堂（`v22`）**：自西麓與水尾寮順平坦幹道鋪設首座鄰近市鎮街屋，串聯百工鍛造護具鞋靴、食堂熱食、總商行鎂晶匯兌與脈絡診所。
  2. **20m 底板微步 `3a′`～`3d` 空間銜接**：持續將長期探索區塊快取、`grid_z` 樓層隔離與跨區有界尋路合入同一 2D 世界。
