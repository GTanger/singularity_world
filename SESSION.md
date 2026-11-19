# Singularity World — 當前交班狀態 (SESSION.md)

- **更新日期**：2026-11-19
- **當前狀態**：`active`（已依操作者拍板完成**「單格即物件」LPC MUD 方案 B 改版：大地圖置於行囊列正上方＋全域 48 階天色連動整片大地圖＋修復 `setPointerCapture` 吞點格、舊 `order:2` 反轉與 7 人擁擠／墨韻腰斬**（`v=20261119a10`）：移除大地圖頂列與「已踏足／目及」設計端統計，將「定位 ◎／緊湊／圖例」移至大地圖下方緊貼行囊列，並通過 `tra-sw-gate` 6/6 全綠、實體 `Pixel 9 Pro XL` 與桌機 1280×800 雙端全程動態互動驗收（四向直行、跨格行旅、拖曳防誤觸、緊湊/圖例/全圖、行內插座零跳動、四時段天色變色）`PASS`；2D 畫布實驗留存於 `/rim` 不抹除歷史，下一步自 `微步 3a′` 接軌續推）

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
    - 聚落淨空帶外被採空之竹叢／春筍／`山林青草藥`隨開譜曆雨露累積 `eco_ticks` 後自然萌發長回並自動清除覆寫列（`ov.is_pristine() == true`，防資料庫膨脹）；前端於 `web/rim-world.js` 補齊手繪宣紙白描隆起土壟、鋸齒藥草蓮座與紫白花苞、心形甘薯葉與露土塊根及 5 張 128×128 離屏白描物資圖素，並擴充西麓 `2×4` 儲物席、將狩獵／採收接入存量節流＋獵獲優先入囊＋露天生鮮自然消解以防滿地肉堆（`47/47` `grid::substrate` 單元測試通過）。
14. **2026-11-18 `v22` 1m 真實尺度農莊倉廩與無配額供需湧現經濟引擎（`v=20261118a20`）**：
    - **真實 1m 農業與倉儲建築尺度**：廢除廣場小地墊與 `1×3` 花盆小圃並無損遷入新倉；西麓擴建 `56 m²` `西麓倉廩棚屋` `(-13..=-6, -1..=5)`（室內 15 格乾燥貨架）＋ 8 格露天`竹木石料場` ＋ 40 格四大 `5m×2m` 連畦農莊田區；水尾寮擴建 `30 m²` `山貨倉房` `(21..=26, -12..=-8)` ＋ 32 格四大 `4m×2m` 溪坪田區；點擊田壟自動蛇行串接整畦同態細格並直接收成入囊（不再灑滿地面），前哨白描渲染同步區分連貫長畦土壟、倉廩木架與露天枕木料場。
    - **全代數比例不變市場湧現 (`rim_economy.rs` + `rim_market.rs` + PostgreSQL `rim_settlement_vault`)**：徹底拔除 `meat < 8`／`herb < 12` 等硬寫配額，改依聚落人口 `N_s`、在途＋在倉有效儲滿度 `r_eff`、金庫流動性 `L_s` 與倉位物理擁擠懲罰即時算出一口馬歇爾等彈性買賣牌價（`quote_item`）；驅動打獵／整畦／烹烤／百工製具／全 14 物資雙向空間套利、生物乘載力平衡、工具磨損，以及嚴格「鎂」總量守恆之公倉買賣與對談易物。
    - **手操介入零回拉頓卡修復（`v22.1`, `v=20261118a21`）**：`rim_cmd.rs`／`rim_sim.rs`／`rim_api.rs` 於下達手操或關閉自主時清空 `stall_tracker` 與 `parcel_queues`、手操免疫 `側身禮讓` 劫持並改用與前端一致之 `find_micro_path`；`web/rim-world.js` 補齊 `pollSeq`/`meCmdInFlight`/`meArriveHoldUntil` 防過時輪詢覆寫、行進與抵達純單調預測零逆向回彈。
15. **2026-11-18 `v23` 真實六基底地貌分區與現代奇幻台灣街鎮生活圈（`v=20261118a22`）**：
    - **六類 `L0` 基底分區與局部湧泉溪（`rim_world.rs`）**：修正 `derive_micro_pristine`，按 `CoastalSand`／`TidalMudflat`／`BareRock`／`PebbleBank`／`RedGravel`／`LoamGrass` 與海拔 `< 18m` 低平帶精確分區，消除西海岸沙灘泥灘荊竹與春筍，並將湧泉溪收斂為局部山麓溪段 `(-46..=38)`。
    - **西麓湧泉街九棟建物、家戶與城鎮設施（`rim_town.rs` + `rim_seed.rs` + `rim_shelter.rs`）**：以自然台灣店名／家宅命名取代營地荒野，建置`春水小吃`（母子 2 人）、`三水商行`（掌櫃夥計 2 人）、`阿木鐵店`（師徒 2 人）、`濟安診所`（父女 2 人＋診間病榻）、`迎風宿屋`（1 人駐守＋雙客榻）、`許家農宅`（一家 3 口）、`陳青宅`（單身 1 人）、`阿茶茶寮`（夫妻 2 人）與`水尾寮倉房`（公倉）共 9 棟零空屋；串接`石板街面`、雨露遮蔽`騎樓亭仔腳`、防雨夜照`導能恆光街燈`、15 名常駐家戶 1 對 1 專屬睡榻歸戶就寢、`濟安診所`救治送榻與`導能獵刀`／`鋼製開山鎬`鍛製升級（`54/54` `grid::substrate` 單元測試通過）。
16. **2026-11-18 `v24` 小鎮活態生活迴圈 Slice 1：專職營業、器物落款、診金記帳與尋常街市（`v=20261118a23`）**：
    - **各店專職營業與嚴格守恆（`rim_shops.rs` + `rim_sim.rs` + `rim_ai.rs` + `rim_shelter.rs`）**：實作`春水小吃`（堂食熱菜溫飽／外帶熱食餐包）、`阿木鐵店`（校磨利器＋定制鍛造打上`阿木鐵店・林阿木`匠師落款）、`濟安診所`（`張濟安`／`張素心`診療消傷，缺鎂可慈悲記帳並在`[自身]`一鍵結清）、`迎風宿屋`（`謝承風`日租投宿分派客榻且夜間享額外調養）與`阿茶茶寮`（`周阿茶`飲茶消乏）；`鎂`與物資嚴格於顧客與店主（及各倉公帑）間原子轉帳、`58/58` 單元測試通過。
    - **單決策輕操作與零數值感官呈現（`rim_api.rs` + `web/rim-world.js`）**：點按街鋪或店主即直接顯示專職選項並於點擊後由後端生成路徑滑順走往櫃台辦妥；工具狀態與檢視全面轉為去數值手感詞條（如`阿木鐵店・林阿木鍛造｜鋒刃雪亮`），前端絕不外露原料編號或耐久數字。
17. **2026-11-18 `v24.1` 視覺與城鎮空間／經濟合理性全面清創（`v=20261118a25`）**：
    - **空間格局、屋頂合併與人物分層（`rim_town.rs` + `rim_ecology.rs` + `web/rim-world.js`）**：將`阿木鐵店`收正為 `7..=11`（拓出 `x=12..=13` 2m 東巷）、`迎風宿屋`增設中牆隔開櫃檯廳與後客榻區（`(-16,-10)`）；把西麓露天料場遷至`三水商行`北院 `(-13..=-10,6..=7)`（加開後門 `(-10,5)`）、4 畦旱田遷至`許家農宅`南坡（加開南門 `(-9,-12)`）還回暢通本街；前端合併整棟完好屋頂、`0.30` 緊湊屋簷、移除室內隔門雨遮與騎樓門罩、戶外／掀頂角色後繪免削頭、導航紅線改正交、室內繪為磚灶大鍋、開彈窗自動收右下檢視卡且禁止拆除全鎮既有門店與公共街路。
    - **店主駐店與物價平滑（`rim_phase.rs` + `rim_life.rs` + `rim_shops.rs` + `rim_economy.rs`）**：限縮跨聚落挑貨僅歸`貨郎`且取消返程無限鎖死與單字洗版、復原飢渴角色並排派六店主白晝駐店；平滑 `quote_item` 缺貨彈性倍數使藥膏（~12–20 鎂）與鍛造武器（~24–38 鎂）回歸合理區間，且鐵店磨刃（4 鎂）同步落款`阿木鐵店・林阿木校刃`（`58/58` 單元測試通過）。
18. **2026-11-19 `v25` 全世界視覺遷移 CDDA `Chibi_Ultica` 圖集與即時紙娃娃系統 ＋ 追獵排隊 AI 修復（`v=20261119a02`）**：
    - **純素材庫邊界與單圖集打包（`scripts/build_cdda_atlas.py` + `web/assets/cdda/{atlas.png,atlas.json,CREDITS.md}`）**：依操作者定案（僅以 CDDA 免去手製素材環節，嚴守《奇點世界》台灣現代奇幻生活圈調性，不引進廢土與繁瑣求生），自官方 `ChibiUltica-2026-11-15.zip` 精煉打包 `294` 枚 `CC-BY-SA 3.0` 圖素至 `1024×374` 單張圖集（`145 KB`），涵蓋四連通 16 態紅瓦／台式青黑瓦屋頂、木／磚／土牆、河岸木橋、室內塌塌米與木地、16 類道具與 `MSXotto+` 紙娃娃分層，並完整移除舊 `web/assets/pawn/`。
    - **前端整數像素網格、統一南北 Y-Sort 與 0ms 紙娃娃（`web/{cdda-tiles.js,rim-portrait.js,rim-world.js,rim-world.css,index.html,sw.js}`）**：以共用整數邊界消除毛細縫，保留南立面牆門與屋簷陰影（無溢街）、室內外家具／自然物／紙娃娃角色統一南北 Y-Sort，裝備行囊／自身／當面交談立繪與 `24×24` 道具圖示均為 0ms 同步渲染（卸戴竹笠／背簍／持器即時連動）。
    - **根治全鎮排隊打鳥與死鳥折返跑 AI 缺陷（`rim_market.rs` + `rim_ai.rs`）**：區分具威脅野獸與無害`鐵羽竹雞`（僅專職獵戶可主動獵鳥、單隻竹雞限 1 名追獵者、防禦野豬上限 2 人並採避讓擁擠尋路）；目標陣亡時立即 `path.clear()` 終止殘餘路徑，`60/60` 單元測試與 30 秒線上軌跡稽核（`0` 非獵戶追鳥、`0` 店主離崗、`0` 死後殘餘路徑）全數通過。
19. **2026-11-19 正式回歸「單格即物件」LPC MUD 純文字主線（`v=20261119a03`，Mnemopi `wm_9943ad5e142c4b5c`）**：
    - 實機比較 2D 像素畫布（`Chibi_Ultica` 與純 `UltiCa` 寫實對照）後，操作者拍板回歸 `11/05～11/12` 定案之正統「單格即物件 × 插座插件」LPC MUD 文字主線：主入口 `web/index.html` 與 `web/sw.js` 還原接回 `7c464445` 四層零跳動文字介面（`CLS=0px`，`Pixel 9 Pro XL` 真機驗收 `PASS`），原 2D 畫布入口改移至 `/rim`（`web/rim.html` ＋ `src/server/run.rs`）完整留痕不抹除。
20. **2026-11-19 方案 B：拔除九宮格，符號大地圖升至第一層主舞台＋直接點格移動（`v=20261119a05`）**：
    - 依操作者指定調整 [index.html](file:///home/tanger/Projects/singularity_world/web/index.html)、[grid-map.js](file:///home/tanger/Projects/singularity_world/web/grid-map.js)、[game-grid.css](file:///home/tanger/Projects/singularity_world/web/game-grid.css) 與 [sw.js](file:///home/tanger/Projects/singularity_world/web/sw.js)：移除視覺九宮格（保留隱藏鍵盤相容節點供 WASD 與 `SwMoveDirection`），將可拖曳符號大地圖直接置於第一層主舞台（手機上半 ~56%、桌機左欄 63% 滿寬），點擊任一可通行鄰格或已探足跡即直接送步移動並自動置中跟隨、附即時 `泉涼↑` 角標與在場人跡紅點，底欄收為 `32px` 單行（含 `[全圖]` 全螢幕展開切換）；通過 `tra-sw-gate` 6/6 全綠、桌機 1280×800 與 `Pixel 9 Pro XL` 真機驗收（`Zero-CLS = 0px`、`0px` 水平溢位、`0` 錯誤）。

## 二、 關鍵檔案與正典錨點

- **新生世界設計正典、關鍵字總表與 Wiki ADR**：
  - `/home/tanger/Projects/singularity_world/docs/DESIGN_KEYWORDS.md`（世界核心關鍵字清單與行為規範）
  - `/home/tanger/Projects/singularity_world/docs/design/energetic_taiwan_landmass_and_substrate_spec.md`（§1～§5.3 完整定案，含微步 3 五小刀契約）
  - `/home/tanger/Projects/singularity_world/docs/design/people_and_relations_plugin_socket_spec.md`（v2.3 定案版）
  - `/home/tanger/Projects/obsidian-vault/wiki/05-專案/Singularity-World/10-正典與ADR/星球暴脹橫斷面與人際關係插件插座正典.md`
- **活躍 2D 世界實作與單檔 `< 950` 行模組群**：
  - **2D 底板與先鋒聚落生命迴圈**：`src/grid/substrate/{l0_cell,coords,moi_dem,slots,render_synth,runtime,facade}.rs`、`src/grid/substrate/{rim_world,rim_town,rim_shops,rim_sim,rim_ai,rim_life,rim_seed,rim_physics,rim_phase,rim_cmd,rim_shelter,rim_ecology,rim_economy,rim_market,rim_portrait_gen}.rs`
  - **持久化與 HTTP 端點**：`src/pg/{cell_overlay,rim_persist}.rs`、`src/server/http_api/{substrate_atlas,rim_api,rim_auth,rim_portrait_api,rim_talk_api}.rs`、`web/{index.html,cdda-tiles.js,rim-world.js,rim-world.css,rim-portrait.js,rim-sky-time.js,sw.js}`、`scripts/build_cdda_atlas.py`、`web/assets/cdda/{atlas.png,atlas.json,CREDITS.md}`

## 三、 下一步接續提示 (Next Cue)

- **已定案紀錄｜「真活世界」最小五件共用骨架與前四步落地順序（2026-11-19，審辯室 Room `e8f2c2d2-d9da-4279-b91f-174e40d9f6d6`，忠實對齊 `DESIGN_KEYWORDS.md` 與兩份正典）**：
  - **凍結與隔離邊界**：凍結 `rim_*`（留存於 `/rim` 不再擴張、嚴禁照搬其固定店鋪／職稱狀態機回主線）並將舊世 `wt_*` 具名角色隔離於 legacy（守 `DESIGN_KEYWORDS.md:143` 暫存不刪，退出新世界 tick）；「單格即物件」於 307 萬格 20m 底板採「虛擬房間／觀測時經 `CellFacade` 合成」，玩家與 NPC 100% 共用管線、LLM 永不裁決世界狀態。
  1. **第一步｜收斂單一空間讀寫權威（`微步 3a′`＋`3c`）**：替 `src/grid/substrate/facade.rs` 補齊統一稀疏寫入（CAS `cell_overlay`），將主線大地圖、移動、西麓亭仔腳 `A1` 騎樓街屋與 `entities.grid_z (0..=3)` 接上 20m 台灣底板，廢除記憶體 `IN_PRIVATE_ROOM` 與舊 `wt_*` 屋契。
  2. **第二步｜上線第一條環境因果 ＋ 共用守恆仲裁（`微步 3b′`）**：以單一時鐘（`game_time_scale = 24`）驅動 `Field`＋`overlay` 之「高水期與通行地帶遷移」；建立玩家／NPC 共用 Rust 四步仲裁（`Observe -> Declare Intent -> Resolve -> Update`）與 `entities.inventory` 同交易 CAS。
  3. **第三步｜無職稱真活居民（`src/grid/zhulin/social/`）**：以 `SHALU_ZHULIN_WORLD_SEED` 零寫死人名生成 300～400 人，實裝 `1 底板 × 4 內在槽`（`10×12` 骨底脾氣、`ActiveDeficit`、`ProvenanceVessel`、`SubjectiveBelief`）× `6 關係插座`（共同履歷轉權利，不設好感條），以 Dave Mark 乘積補償＋Hysteresis 遲滯防抖純 Rust 驅動。
  4. **第四步｜60 天純 Rust Burn-in ＋ 真活硬驗收（`微步 3d`）**：開局前預跑 60 遊戲日寫入 PG，硬驗①無玩家續行與重啟等價、②全守恆零無中生有、③樞紐／債務／口風／改道自歷史湧現、④「作品被活人接手」與有界加權 A\* 遇事停步閉環。
