# 方格 MUD 小村替換無邏輯地圖 — 工單

狀態：已修正為「清舊地圖，只留中性起點」
依據：2026-06-04 專案回盤；`溪橋小村` 29 格模板已列為舊地圖內容。

## 結論

可以刪掉目前由「座標揭露 + 獨立隨機地形」形成的無邏輯地圖內容，但不要刪掉 `SquareGrid` / `grid_cells` / 移動 / L4 出口卡片這套底層。要刪的是舊地圖內容與舊生成邏輯的主導權，不是方格世界容器。

2026-06-04 修正：`溪橋小村` 29 格模板也視為舊地圖內容，不再作為登入後世界。世界未接入正式地圖前，只保留 `grid:0:0` 單一中性起點格；不得自動灌村莊、野外環、或座標隨機生成格。

2026-06-05 載入狀態：已把目前規劃中可執行的 `M3-027 中河渡口聚落帶` 7 張 M4 初始可玩圈載入 PostgreSQL `grid_cells`。這是奇點世界目前登入後的玩家層地圖，不是舊村莊。

| grid | M4 | 名稱 |
|---|---|---|
| `grid:0:0` | `M4-027-01` | 北岸渡口 |
| `grid:0:-1` | `M4-027-02` | 渡船踏板 |
| `grid:0:-2` | `M4-027-03` | 南岸泥坡 |
| `grid:1:0` | `M4-027-04` | 岸邊短市 |
| `grid:1:-1` | `M4-027-05` | 船戶作業點 |
| `grid:1:-2` | `M4-027-06` | 倉棚前場 |
| `grid:-1:0` | `M4-027-07` | 臨河歇腳處 |

備份：載入前的單格中性起點已備份到 `grid_cells_backup_20260605_before_m3_027_load`。

2026-06-05 建立背景、2026-07-17 身分校正：`codex` 不是測試資料或固定驗收角色，而是 Codex 本人在奇點世界中的永久角色。前端實際畫面檢查可以登入此角色，但其位置、持有物、金錢、狀態與歷史都必須視為真實世界狀態；不得因測試清理而刪除、重建、傳送、歸零或抹除經歷。需要乾淨起點與重跑能力的情境改用隔離重演或其他臨時帳號，不再把 `codex` 當可回復 fixture。

2026-06-08 第一屏進度：`codex` 登入後的 `北岸渡口` 已從功能占位句推進為玩家層場景段：L3 顯示北岸候渡、渡船、短市、歇腳與等船人群的氛圍；L4 仍由出口卡與觀察態人物卡構成，折疊群眾顯示為【候渡人群】，不顯示精確剩餘人數。畫面驗收截圖：`docs/artifacts/singularity-north-bank-first-screen-20260608.png`。

2026-06-08 接續進度：`room_enter` 新增北岸專用 `entry_log`，讓登入/進格後 log 顯示候渡人群的表面行為（包袱、張望踏板、船繩響動），而不是只補泛用「站定」句。L4 仍保持【行人甲】【行人乙】【行人丙】【候渡人群】觀察態，不揭露 NPC 真名、職業或精確人數。畫面驗收截圖：`docs/artifacts/singularity-north-bank-npc-presence-20260608.png`。

2026-06-08 觀察互動進度：方格 `do_action` 已支援只有 `grid_x/grid_y`、沒有舊 `entity_rooms` 的玩家/NPC 同格互動。點北岸 L4 行人後，`Look` log 使用【候渡者】與可見行為描述，不把測試 NPC 的私名/生成名當成觀察結果；若多名已觀察候渡者同場，L4 按既有甲乙規則顯示【候渡者甲】【候渡者乙】。畫面驗收截圖：`docs/artifacts/singularity-north-bank-look-observed-20260608.png`。

2026-06-08 觀察後續動作收斂：未知道姓名的 NPC 被 `Look` 後，後續動作列先只顯示【對話】，不再一次倒出【借物】【留人】【送行】【交易】。這讓第一屏的節奏從「觀察候渡者」自然進到「搭話」，避免把底層預設 socket 直接變成玩家負擔。畫面驗收截圖：`docs/artifacts/singularity-north-bank-look-talk-only-20260608.png`。

2026-06-08 鄰接格在場感：第一屏三個出口格已補場景化 `entry_log`、群眾折疊名與觀察公共標籤。`岸邊短市` 顯示【短市人群】/【看貨人】，`臨河歇腳處` 顯示【歇腳人群】/【歇腳旅人】，`渡船踏板` 顯示【候船人群】/【候船者】；Look log 依場景描述價牌/短貨、長凳/茶碗/包袱、濕木扶手/踏板積水。實際移動截圖：`docs/artifacts/singularity-m3-027-market-presence-20260608.png`、`docs/artifacts/singularity-m3-027-rest-presence-20260608.png`、`docs/artifacts/singularity-m3-027-plank-presence-20260608.png`。

2026-06-08 七格閉環在場感：`M3-027 中河渡口聚落帶` 7 張 M4 初始可玩圈已補齊玩家層 L3 覆寫、`entry_log`、群眾折疊名與觀察公共標籤。`南岸泥坡` 顯示【上岸人群】/【上岸客】，`船戶作業點` 顯示【船邊人群】/【渡工】，`倉棚前場` 顯示【倉棚人群】/【搬貨人】；Look log 依場景描述濕泥上岸、纜繩木楔、麻袋木箱。另修正 L4 出口混入目前格自己的 self-exit 卡片問題，玩家不再看見【北岸渡口】這類「原地出口」。實際畫面驗收截圖：`docs/artifacts/singularity-m3-027-south-bank-presence-20260608.png`、`docs/artifacts/singularity-m3-027-boat-work-presence-20260608.png`、`docs/artifacts/singularity-m3-027-warehouse-presence-20260608.png`、`docs/artifacts/singularity-m3-027-north-self-exit-filter-20260608.png`。

2026-06-08 L3 動態現象層實作：`北岸渡口` 的 L3 不再只取固定句，改為「場景骨架 + 時辰/氣候現象 + 人群在場投影」組合；目前已接入時辰、範圍氣候與 L4 人群密度，不顯示精確人數、不露後設/debug/資料庫字樣。`codex` 實際登入畫面抽到晨霧版本：「晨霧從水面慢慢抬起，先露出船繩，再露出踏板前的人影」，L4 仍保留【渡船踏板】【岸邊短市】【臨河歇腳處】與觀察態人物/人群卡。畫面驗收截圖：`docs/artifacts/singularity-l3-dynamic-phenomenon-20260608.png`。

2026-06-08 L3 動態現象層落地：北岸 L3 五槽位已接到實機：場景骨架、時辰/氣候現象、渡船相位、人群在場投影、近期/表面痕跡。渡船相位先由遊戲時間推導成靠北岸、往南、靠南岸、回北岸四態；近期痕跡優先讀房間事件滑窗並轉成玩家可見表面痕跡，不直接暴露 NPC 私名或事件資料。若無房間事件，痕跡由氣候、渡船相位與人群密度投射。實際登入 `codex` 畫面看到夜間月亮 L2、渡船停在南岸、人群壓岸、腳印與貨擔拖痕；L4 仍維持出口與觀察態人物卡。畫面驗收截圖：`docs/artifacts/singularity-l3-dynamic-ferry-traces-20260608.png`。

2026-06-08 L3 釘住語意落地：`room_enter` payload 補 `room_id`，前端以同格判斷保留 L3；同一格因觀察 NPC、刷新 L4 或重推 `room_enter` 時，只更新 L1/L2/L4，不重抽 L3。這讓「踏入抽一次、離格前不跳動」變成實際 UI 行為，而不是只在規格裡。實際登入 `codex` 後觸發同格 `observe`，`room_enter` 重推 1 次，L3 前後完全相同，L4 出口與觀察態人物仍正常顯示。畫面驗收截圖：`docs/artifacts/singularity-l3-pinned-after-observe-20260608.png`。

2026-06-08 L3 再踏入刷新驗收：補齊「離格再回來才重抽」的實機驗收，並修正 L4 出口同時混入 v5/legacy 資料時按方向去重仍會重複顯示同一地點的問題。`codex` 從 `北岸渡口` 點【岸邊短市】，等待遊戲時間從 09:42 推進到 09:57，再點【北岸渡口】返回；L3 從對岸船邊人影/北岸踏板空著刷新為晨霧抬起/渡船正從南岸回頭，L4 只顯示一次【臨河歇腳處】，未出現精確人數或後設/debug 字樣。畫面驗收截圖：`docs/artifacts/singularity-l3-reenter-refresh-20260608.png`。

2026-06-08 重返 log 連續化：`room_enter` 追加玩家層 `reentry_log`，前端記錄本次 session 已踏入過的房間；第一次進場仍顯示原 `entry_log`，同格 `observe` / L4 刷新不追加 log，離格後再回同一房間才顯示重返句。實測 `codex` 走 `北岸渡口` →【岸邊短市】→【北岸渡口】，log 變成「你在北岸渡口站定」→「你走進岸邊短市」→「你回到北岸渡口」，沒有重播第二次「站定」；再點【行人甲】觀察候渡者，也沒有追加第二次重返句。畫面驗收截圖：`docs/artifacts/singularity-reentry-log-continuity-20260608.png`。

2026-06-08 路徑感重返 log：`room_enter` 追加 `reentry_logs` 路徑句表，前端用上一個實際踏入房間選句；目前先補北岸與短市相鄰路徑。實測 `codex` 走 `北岸渡口` →【岸邊短市】→【北岸渡口】，返回時 log 顯示「你從短市那頭回到北岸渡口。身後的價牌聲被河風拉遠……」，不再只用泛用「你回到北岸渡口」。L4 仍維持出口與觀察態人物/人群，未出現後設/debug 或精確人數。畫面驗收截圖：`docs/artifacts/singularity-route-reentry-log-20260608.png`。

2026-06-08 盤古開天第一步：新增資料化世界骨架 `data/world/singularity_world_skeleton.json` 與非破壞式 loader。啟動時 `grid_manager` 會從骨架補缺到 PostgreSQL `grid_cells`，保留既有打磨過的名稱/描述，只補缺格、缺 tags 與缺出口；現有 `M3-027` 從 7 格擴成 11 格，新增 `中河谷地路口`、`短市外谷地路`、`南側谷地路`、`渡口貨路` 四個可走世界邊。`codex` 實測登入北岸後 L4 顯示【中河谷地路口】與原有渡船/短市/歇腳出口；點入後 L3 顯示谷地路口把北岸接回谷地泥路，L4 只顯示【北岸渡口】【短市外谷地路】兩個有效出口，已修正 room_enter 與 grid_view 交錯時殘留上一格 legacy exits 造成的幽靈出口。畫面驗收截圖：`docs/artifacts/singularity-pangu-north-bank-world-edge-20260608.png`、`docs/artifacts/singularity-pangu-valley-road-20260608.png`。

2026-06-08 盤古開天第二步：外層路骨從 11 格擴成 17 格。北岸北向路鏈已能從 `北岸渡口` 走到 `中河谷地路口` → `北堤泥路` → `田埂水口` → `上田小路`，並在 `北堤泥路` 分出 `挑擔小徑`；南側谷地與貨路各補 `下游田埂路`、`貨路外彎`，讓世界邊不只是一圈出口牌。`codex` 手機寬度實測登入後第一屏仍顯示北岸候渡、渡船、短市、歇腳與人群；點入新增路骨後 L4 只顯示當前鄰格，例如 `北堤泥路` 顯示【田埂水口】【中河谷地路口】【挑擔小徑】，`田埂水口` 顯示【上田小路】【北堤泥路】，未出現上一格幽靈出口、後設/debug 字樣或精確人數。畫面驗收截圖：`docs/artifacts/singularity-pangu-north-bank-first-screen-20260608.png`、`docs/artifacts/singularity-pangu-north-bank-second-road-20260608.png`、`docs/artifacts/singularity-pangu-north-bank-road-chain-20260608.png`、`docs/artifacts/singularity-pangu-field-watermouth-20260608.png`。

2026-06-08 盤古開天第三小步：外層路骨補上玩家層進格與重返 log。`中河谷地路口`、`北堤泥路`、`田埂水口`、`上田小路`、`挑擔小徑`、`短市外谷地路`、`南側谷地路`、`下游田埂路`、`渡口貨路`、`貨路外彎` 不再只改 L3/L4；玩家走入時 log 會描述泥路、田埂水口、挑擔腳印、車轍與貨路外彎，重返時使用不同句，部分路段依前一格產生路徑感句子。另把 `短市外谷地路` 放在泛用「短市」判斷之前，避免外側道路誤顯示成岸邊短市本體。實測 `codex` 走 `北岸渡口` → `中河谷地路口` → `北堤泥路` → `田埂水口` → `北堤泥路` → `中河谷地路口` → `北岸渡口`，log 連續顯示各路段進格/重返句，未出現後設/debug 字樣或精確人數。畫面驗收截圖：`docs/artifacts/singularity-pangu-road-entry-log-20260608.png`、`docs/artifacts/singularity-pangu-road-reentry-log-20260608.png`。

2026-06-08 盤古開天第四步：`M3-034 東部稻地` 首批 6 張正式 M4 已接入同一份世界骨架，live `grid_cells` 從 17 格擴成 23 格。玩家可從 `北岸渡口` 沿 `中河谷地路口` → `北堤泥路` → `田埂水口` → `上田小路` 走到 `田區路口`，再進 `灌渠木閘`、`秧苗田邊`，並可繞到 `農舍前坪`、`穀場小棚`、`田埂深處`。稻地 6 格都有 L3 短描述、L4 出口與 entry/reentry/route reentry log；實測畫面顯示水田、木牌、泥腳印、灌渠木閘、秧苗、蛙聲與農舍/穀場方向，未出現後設/debug 字樣或精確人數。畫面驗收截圖：`docs/artifacts/singularity-pangu-m3-034-first-screen-20260608.png`、`docs/artifacts/singularity-pangu-m3-034-field-entry-20260608.png`、`docs/artifacts/singularity-pangu-m3-034-seedling-edge-20260608.png`。

2026-06-11 盤古開天地圖骨架完成：依 `docs/design/奇點世界場景骨架清冊.md`，`data/world/singularity_world_skeleton.json` 已從 23 格擴到 95 格，覆蓋 85 張正式 M4 + 10 個外層路骨。新增 72 張 M4 先作 `scene_stub`：有 L3 短描述與 L4 出口，未在本步補完整 NPC 排班/物件行為。`scene_stub` 佔位描述可由骨架刷新，已打磨玩家文案不覆蓋；已修掉「承擔／路徑接向」等文件語氣。DB 實測 `grid_cells=95`、formal M4 標籤 85、硬句殘留 0。Playwright/Chrome 以 `codex` 實走北港、西北邊城與東南外貿港端點，未出現後設/debug 字樣或精確人數；驗收後 `codex` 回到 `grid:0:0`。畫面驗收截圖：`docs/artifacts/singularity-pangu-map-complete-first-screen-20260611.png`、`docs/artifacts/singularity-pangu-map-complete-north-harbor-20260611.png`、`docs/artifacts/singularity-pangu-map-complete-northwest-border-20260611.png`、`docs/artifacts/singularity-pangu-map-complete-southeast-trade-20260611.png`。

2026-06-11 東湖諸港第一輪補天：`M3-032 東湖諸港` 六格已從 `scene_stub` 升級成 `scene_polished`，並新增骨架升級規則：資料庫中仍標 `scene_stub` 的格子，若骨架 JSON 已改成正式場景，就更新描述並移除舊 stub 標籤，不覆蓋沒有 stub 的已打磨場景。玩家可從北岸沿稻地走到 `水網入口棧`，再進 `湖港木棧`、`魚貨分揀場`、`水倉前場`；L3 顯示濕木棧、霧牌、短船、魚籃、蘆草、水倉與倉管/分揀工/引船人等公共在場感，L4 出口仍是卡片，人物仍維持觀察態，不顯示精確人數。Playwright 以手機寬度登入 `codex` 實走並回到北岸，未出現後設/debug/資料庫字樣。畫面驗收截圖：`docs/artifacts/singularity-m3-032-first-screen-20260611.png`、`docs/artifacts/singularity-m3-032-water-entry-20260611.png`、`docs/artifacts/singularity-m3-032-fish-sorting-20260611.png`、`docs/artifacts/singularity-m3-032-water-warehouse-20260611.png`。

2026-06-12 南部糧倉第一輪補天：`M3-045 南部糧倉` 六格已從 `scene_stub` 升級成 `scene_polished`。玩家可從北岸經渡船、南岸泥坡、南側谷地與 `下游田埂路` 走到 `糧倉外秤`，再進 `倉門前場`、`車行口`、`糧車泊位`、`曬場邊`；L3 顯示秤桿、糧袋、倉門封條、木栓、曬席、木耙、排隊糧車、守倉冊簿與搬糧人，L4 仍保留出口卡與觀察態人物卡，群眾只顯示折疊名，不顯示精確人數。Playwright 以手機寬度登入 `codex` 實走並回到北岸，未出現後設/debug/資料庫字樣。畫面驗收截圖：`docs/artifacts/singularity-m3-045-first-screen-20260612.png`、`docs/artifacts/singularity-m3-045-granary-scale-20260612.png`、`docs/artifacts/singularity-m3-045-granary-gate-20260612.png`、`docs/artifacts/singularity-m3-045-grain-cart-berth-20260612.png`、`docs/artifacts/singularity-m3-045-drying-yard-edge-20260612.png`。

2026-06-12 河口港群第一輪補天：`M3-048 河口港群` 六格已從 `scene_stub` 升級成 `scene_polished`。玩家可從 `守倉棚` 走到 `河口潮泥碼頭`，再進 `稅站棚`、`潮線貨場`、`船行門面`、`商船候泊線`、`河海倉門`；L3 顯示退潮黑泥、碼頭纜繩、貨單印章、航線板、潮印木箱、候泊船影、雨布封牌與錨繩，L4 仍保留出口卡與觀察態人物卡。另修正兩個畫面錯配：`守倉棚` 不再被泛用 `倉棚前場` L3 覆寫；v5 `room_enter` 接管 L2 後，舊 `grid_view` 氣候輪詢不得把晚間月亮覆蓋回太陽。Playwright 以手機寬度登入 `codex` 實走並回到北岸，未出現後設/debug/資料庫字樣。畫面驗收截圖：`docs/artifacts/singularity-m3-048-first-screen-20260612.png`、`docs/artifacts/singularity-m3-048-estuary-mud-dock-20260612.png`、`docs/artifacts/singularity-m3-048-tax-station-20260612.png`、`docs/artifacts/singularity-m3-048-tide-cargo-yard-20260612.png`、`docs/artifacts/singularity-m3-048-shipping-agency-20260612.png`、`docs/artifacts/singularity-m3-048-merchant-berth-line-20260612.png`、`docs/artifacts/singularity-m3-048-river-sea-warehouse-gate-20260612.png`。

2026-06-12 東南船港第一輪補天：`M3-050 東南船港` 六格已從 `scene_stub` 升級成 `scene_polished`。玩家可從 `河海倉門` 走進 `東南船塢口`，再進 `補給倉`、`碼頭木階`、`修船架`、`帆布曬場`、`船戶街口`；L3 顯示潮線、木屑、瀝青味、半修船板、淡水桶、乾糧箱、濕木階、繩柱、舊帆、針線架、招工牌、飯攤與船期，L4 仍保留出口卡與觀察態人物卡。另修正兩個畫面覆寫錯配：`船戶街口` 不再被泛用 `船戶作業點` L3 覆寫；`帆布曬場` 的進場與路徑 log 不再落到南部糧倉 `曬場邊` 的翻穀語彙。Playwright 以手機寬度登入 `codex` 實走六格並回到北岸，未出現後設/debug/資料庫字樣。畫面驗收截圖：`docs/artifacts/singularity-m3-050-first-screen-20260612.png`、`docs/artifacts/singularity-m3-050-shipyard-gate-20260612.png`、`docs/artifacts/singularity-m3-050-supply-warehouse-20260612.png`、`docs/artifacts/singularity-m3-050-dock-steps-20260612.png`、`docs/artifacts/singularity-m3-050-repair-frame-20260612.png`、`docs/artifacts/singularity-m3-050-sailcloth-yard-20260612.png`、`docs/artifacts/singularity-m3-050-boathouse-street-20260612.png`。

2026-06-12 東南外貿港第一輪補天：`M3-053 東南外貿港` 六格已從 `scene_stub` 升級成 `scene_polished`。玩家可從 `帆布曬場` 走進 `外貿碼頭`，再進 `海關前棚`、`翻譯桌`、`匯兌小屋`、`遠洋倉門`、`商館街口`；L3 顯示港旗、吊繩、外地貨箱、濕貨單、封條、貨樣匣、外語低語、異地箱、雨布、合約、印章、銅秤、賬冊與錢票，L4 仍保留出口卡與觀察態人物卡。Playwright 以手機寬度登入 `codex` 實走六格並回到北岸，未出現後設/debug/資料庫字樣。畫面驗收截圖：`docs/artifacts/singularity-m3-053-first-screen-20260612.png`、`docs/artifacts/singularity-m3-053-foreign-dock-20260612.png`、`docs/artifacts/singularity-m3-053-customs-shed-20260612.png`、`docs/artifacts/singularity-m3-053-translator-table-20260612.png`、`docs/artifacts/singularity-m3-053-exchange-house-20260612.png`、`docs/artifacts/singularity-m3-053-ocean-warehouse-20260612.png`、`docs/artifacts/singularity-m3-053-merchant-house-street-20260612.png`。

2026-06-12 北方港灣第一輪補天：`M3-006 北方港灣` 六格已從 `scene_stub` 升級成 `scene_polished`。玩家可從 `農舍前坪` 走進 `北灣避風碼頭`，再進 `港灣貨棚`、`冷霧魚市`、`漁網曬場`、`北灣守望石階`、`船戶屋排`；L3 顯示冷海水、避風船、結霜纜繩、鹽箱、濕木箱、魚籃、補網、冷霧魚市、火盆、排班板、霧牌與警鈴，L4 仍保留出口卡與觀察態人物卡，群眾只顯示折疊名，不顯示精確人數。Playwright 以手機寬度登入 `codex` 實走六格並回到北岸，未出現後設/debug/資料庫字樣；晚間第一屏月亮圖示與夜水描述一致。畫面驗收截圖：`docs/artifacts/singularity-m3-006-first-screen-20260612.png`、`docs/artifacts/singularity-m3-006-shelter-dock-20260612.png`、`docs/artifacts/singularity-m3-006-harbor-cargo-shed-20260612.png`、`docs/artifacts/singularity-m3-006-cold-fog-fish-market-20260612.png`、`docs/artifacts/singularity-m3-006-net-drying-yard-20260612.png`、`docs/artifacts/singularity-m3-006-watch-stone-steps-20260612.png`、`docs/artifacts/singularity-m3-006-boat-house-row-20260612.png`。

2026-06-12 西北礦場帶第一輪補天：`M3-009 西北礦場帶` 六格已從 `scene_stub` 升級成 `scene_polished`。玩家可從 `田埂深處` 走進 `礦坑外棚`，再進 `洗礦溝`、`工具修棚`、`粗料倉`、`舊道裝車點`、`礦渣坡`；L3 顯示山壁陰影、濕支柱、吊燈、洗礦木槽、石籃、濕礦袋、炭線木牌、裂柄鎬、車輪鐵圈、灰黑碎石、滑坡痕、礦車、稱重木牌與邊城外門方向，L4 仍保留出口卡與觀察態人物卡，群眾只顯示折疊名，不顯示精確人數。Playwright 以手機寬度登入 `codex` 實走六格並回到北岸，未出現後設/debug/資料庫字樣；過程中修掉已 polished DB 文案保護留下的舊意圖名 `支架道` 與 `西北舊道`。畫面驗收截圖：`docs/artifacts/singularity-m3-009-first-screen-20260612.png`、`docs/artifacts/singularity-m3-009-mine-shed-20260612.png`、`docs/artifacts/singularity-m3-009-ore-washing-ditch-20260612.png`、`docs/artifacts/singularity-m3-009-tool-repair-shed-20260612.png`、`docs/artifacts/singularity-m3-009-rough-ore-warehouse-20260612.png`、`docs/artifacts/singularity-m3-009-old-road-loading-point-20260612.png`、`docs/artifacts/singularity-m3-009-slag-slope-20260612.png`。

2026-06-12 盤古骨架 `scene_stub` 歸零：最後 30 張 `scene_stub` 已升級為 `scene_polished`，覆蓋 `M3-011 西北邊城群`、`M3-021 西部工坊帶`、`M3-022 西部牧地`、`M3-037 西南牧場帶`、`M3-040 西南邊鎮`。玩家可從北岸走到邊城外門、工坊街口、牧地柵門、牧場門牌、邊鎮井口等五組節點，L3 不再只是功能句，L4 仍維持出口卡與觀察態人物卡，群眾不顯示精確人數。實機驗收用 Playwright/Chrome 登入 `codex`，按 L4 出口自動尋路走完 30 格並回到北岸；結果 `forbiddenHits=0`、`weakHits=0`、夜間太陽圖示 mismatches=0，DB 實測 `scene_stub=0`、`scene_polished=72`。畫面驗收：`docs/artifacts/singularity-map-complete-v10-first-screen-20260612.png`、`docs/artifacts/singularity-map-complete-v10-contact-sheet-20260612.png`，逐格截圖為 `docs/artifacts/singularity-map-complete-v10-*-20260612.png`。

2026-06-12 中部城鎮圈子區層第一步：`M3-030 中部城鎮圈` 首批 8 張子區入口已接入 `data/world/singularity_world_skeleton.json` v11，世界從 95 格擴為 103 格，正式 M4 從 85 張增為 93 張。玩家可從 `北岸渡口` 經 `中河谷地路口`、`短市外谷地路` 進 `中鎮鎮口`，再走到 `中鎮市集街`、`中鎮客棧廊`、`中鎮工坊街`、`中鎮井邊巷`、`中鎮廟前小場`、`中鎮車馬泊位`、`中鎮倉場門口`；L3 顯示鎮口牌樓、市集布棚、客棧廊、井邊日常、廟前告示、工坊木屑、車馬草料與倉場封牌，L4 仍是出口卡與觀察態人物卡，群眾不顯示精確人數。同步修正骨架 loader 的既有出口合併規則：當 live PostgreSQL 已有同 target 但方向錯的出口時，現在會更新 `dir`，避免 `短市外谷地路` 因方向去重只顯示部分出口。驗收結果：`node --check web/grid-map.js && node --check web/sw-room-desc.js && node --check web/main.js && node --check web/sw.js`、`cargo check`、`cargo test world_skeleton -- --nocapture`、`cargo test player_scene -- --nocapture`、`cargo test final_map -- --nocapture`、`./start` 皆通過；Playwright/Chrome 手機寬度登入 `codex` 實走全 8 格並回到北岸，結果 JSON `/home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/m3-030-town-subdistrict-result-20260612.json` 顯示 `forbiddenHits=[]`、`weakHits=[]`，`codex` 已還原到 `grid:0:0`。畫面驗收截圖：`docs/artifacts/singularity-m3-030-first-screen-20260612.png`、`docs/artifacts/singularity-m3-030-town-gate-20260612.png`、`docs/artifacts/singularity-m3-030-market-street-20260612.png`、`docs/artifacts/singularity-m3-030-well-lane-20260612.png`、`docs/artifacts/singularity-m3-030-warehouse-gate-20260612.png`。

2026-06-12 南方大城圈子區層第一步：依 AGY/Antigravity 設計審辯，`M3-044 南方大城圈` 首批 8 張不採中心節點連多個功能房的選單式做法，改採「縱深主街切片」。`data/world/singularity_world_skeleton.json` 升到 v12，世界從 103 格擴為 111 格，正式 M4 從 93 張增為 101 張，骨架維持 `scene_stub=0`、`scene_polished=88`。玩家可從 `北岸渡口` 經 `渡船踏板`、`南岸泥坡`、`南側谷地路`、`下游田埂路`、`糧倉外秤` 走進 `南城外驛道`，再依序抵達 `南城甕門`、`門內集散廣場`、`中軸大街南段`、`中軸大街中段`、`大十字通衢`，並可從大街南段分到 `外區坊門` 與 `商會長廊`。L3 顯示城牆、甕門、入城集散、主街縱深、坊門生活與商會貨單；L4 仍是出口卡與觀察態人物卡，群眾不顯示精確人數。驗收結果：`node --check web/grid-map.js && node --check web/sw-room-desc.js && node --check web/main.js && node --check web/sw.js`、`cargo check`、`cargo test world_skeleton -- --nocapture`、`cargo test player_scene -- --nocapture`、`cargo test final_map -- --nocapture`、`./start` 皆通過；Playwright/Chrome 手機寬度登入 `codex` 實走全 8 格並回到北岸，結果 JSON `/home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/m3-044-southern-city-result-20260612.json` 顯示 `forbiddenHits=[]`、`weakHits=[]`，`codex` 已還原到 `grid:0:0`。畫面驗收截圖：`docs/artifacts/singularity-m3-044-first-screen-20260612.png`、`docs/artifacts/singularity-m3-044-granary-scale-with-city-road-20260612.png`、`docs/artifacts/singularity-m3-044-south-city-road-20260612.png`、`docs/artifacts/singularity-m3-044-south-city-gate-20260612.png`、`docs/artifacts/singularity-m3-044-inner-square-20260612.png`、`docs/artifacts/singularity-m3-044-axis-street-south-20260612.png`、`docs/artifacts/singularity-m3-044-ward-gate-20260612.png`、`docs/artifacts/singularity-m3-044-merchant-corridor-20260612.png`、`docs/artifacts/singularity-m3-044-axis-street-mid-20260612.png`、`docs/artifacts/singularity-m3-044-crossroads-20260612.png`、`docs/artifacts/singularity-m3-044-returned-north-bank-20260612.png`。非阻塞缺口：長路徑快速驗收時 log tail 可能被截斷而不能代表最後一格全部進場句；部分 M3-044 房間尚未配置固定在場 NPC，只先由 L3 角色與公共標籤提供場景在場感。

2026-06-12 西部城邦圈子區層第一步：沿用大型聚落的「縱深街道切片」規則，`M3-020 西部城邦圈` 首批 8 張不做中心節點選單，先開一條從牧地/工坊外緣進城邦門內的玩家路線。`data/world/singularity_world_skeleton.json` 升到 v13，世界從 111 格擴為 119 格，正式 M4 從 101 張增為 109 張，骨架維持 `scene_stub=0`、`scene_polished=96`。玩家可從 `北岸渡口` 經 `臨河歇腳處`、`工坊街口`、`鍛造棚`、`石粉堆`、`牧地柵門`、`水槽棚`、`獸醫棚` 走上 `城邦外坡道`，再依序抵達 `石壘門`、`門內券道`、`旗石廣場`、`西軸坡街`、`議棚前階`，並可從旗石廣場分到 `鑄幣巷口` 與 `傭工棚邊`。L3 顯示石粉坡道、候檢車隊、石壘門、券道陰影、旗石等候線、鑄幣半開門、短工牌與議棚告示；L4 仍是出口卡與觀察態人物/人群卡，群眾不顯示精確人數。驗收結果：`node --check web/grid-map.js && node --check web/sw-room-desc.js && node --check web/main.js && node --check web/sw.js`、`cargo check`、`cargo test world_skeleton -- --nocapture`、`cargo test player_scene -- --nocapture`、`cargo test final_map -- --nocapture`、`./start` 皆通過；Playwright/Chrome 手機寬度登入 `codex` 實走全 8 格、看過兩個分支並回到北岸，結果 JSON `/home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/m3-020-western-city-state-result-20260612.json` 顯示 `forbiddenHits=[]`、`weakHits=[]`，`codex` 已還原到 `grid:0:0`。畫面驗收截圖：`docs/artifacts/singularity-m3-020-first-screen-20260612.png`、`docs/artifacts/singularity-m3-020-vet-shed-with-city-slope-20260612.png`、`docs/artifacts/singularity-m3-020-city-state-slope-20260612.png`、`docs/artifacts/singularity-m3-020-stone-gate-20260612.png`、`docs/artifacts/singularity-m3-020-vaulted-entry-20260612.png`、`docs/artifacts/singularity-m3-020-flagstone-square-20260612.png`、`docs/artifacts/singularity-m3-020-mint-lane-20260612.png`、`docs/artifacts/singularity-m3-020-hire-shed-edge-20260612.png`、`docs/artifacts/singularity-m3-020-axis-slope-street-20260612.png`、`docs/artifacts/singularity-m3-020-council-steps-20260612.png`、`docs/artifacts/singularity-m3-020-returned-north-bank-20260612.png`。非阻塞缺口：M3-020 目前只是城邦第一條外緣入城切片，尚未補完整住坊、兵棚、倉稅、議事制度、固定 NPC 排班與物件行為。

2026-06-12 西部城邦圈活化第一步：不新增地圖格，先把 `M3-020` 第一切片從「可走、有 L3」推到「L4 有穩定在場與可點物件」。`room_enter` 現在會在 M3-020 8 格追加公共在場卡，例如 `城邦外坡道` 的【牽獸人】【坡道人群】、`石壘門` 的【門衛】【門前人群】、`旗石廣場` 的【旗手】【廣場人群】、`議棚前階` 的【記牌人】【議棚人群】；這些是公共身份/人群卡，不顯示精確人數。另新增 5 個玩家可點場景物件：【通行牌】、【等候線】、【半開鑄幣門】、【短工牌】、【議棚告示】；點擊走既有 `do_action Look`，log 顯示牌面、隊列、門縫、活計與告示的可見資訊。前端同步修正 L4 渲染：有 NPC/人群卡時不再吞掉 room objects，出口、公共在場卡與物件卡會同屏顯示；並 bump `sw-object-list.js`、`grid-map.js` 與 service worker cache。驗收結果：`node --check web/grid-map.js && node --check web/sw-object-list.js && node --check web/sw-room-desc.js && node --check web/main.js && node --check web/sw.js`、`cargo check`、`cargo test player_scene -- --nocapture`、`cargo test final_map -- --nocapture`、`cargo test m3_020 -- --nocapture`、`./start` 皆通過；Playwright/Chrome 手機寬度登入 `codex` 實走 M3-020、檢查 8 格公共在場卡、點擊 5 個物件並回到北岸，結果 JSON 顯示 `forbiddenHits=[]`、`weakHits=[]`、`missingCardHits=[]`，`objectChecks` 五項全通過。非阻塞缺口：這仍是公共表層活化，尚未配置真正固定 NPC 排班、物件狀態變化或制度性後果。

2026-06-13 西部城邦圈物件二階動作第一步：延續上一筆公共表層活化，不新增格子、不重開架構，先讓五個場景物件從單純 `Look` 推進到可見後續動作。【通行牌】新增【閱讀】，回應牧地牲口、工坊送件與商隊等候規則；【等候線】新增【使用】，回應玩家讓位後旗手分流、廣場推擠短短鬆開；【半開鑄幣門】新增【開啟】，回應兌牌人把門壓回半開、未被叫到的人退回側線；【短工牌】新增【閱讀】，回應商隊卸貨、牧地補柵與工坊半日搬料；【議棚告示】新增【閱讀】，回應欠糧、傭工糾紛與通行牌申訴不得堵路。前端同步修正 `sw-interactions.js`：玩家可見 log 在 `#sw-log`，舊綁定只抓 hidden `#log`，導致【閱讀】/【使用】/【開啟】按鈕看得到卻不送出；現在綁到可見 log 並加一次性 guard。驗收腳本也改為讀 `data/world/singularity_world_skeleton.json` 做 BFS 回北岸，並以 `window.SwGrid.mapData.currentRoomName` 判斷到站，避免 L3 描述提到鄰房時誤判。驗收結果：`node --check web/sw-interactions.js && node --check web/sw.js && node --check web/grid-map.js && node --check web/sw-object-list.js && node --check web/sw-room-desc.js && node --check web/main.js`、`cargo check`、`cargo test m3_020 -- --nocapture`、`cargo test player_scene -- --nocapture`、`./start` 皆通過；Playwright/Chrome 手機寬度登入 `codex` 實走 M3-020、點擊五個物件及二階動作並回到北岸，結果 JSON `/home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/m3-020-western-city-state-result-20260612.json` 顯示 `forbiddenHits=[]`、`weakHits=[]`、`missingCardHits=[]`，五項 `objectChecks` 皆含 followup 並通過。畫面已目視檢查：`docs/artifacts/singularity-m3-020-first-screen-20260612.png`、`docs/artifacts/singularity-m3-020-stone-gate-20260612.png`、`docs/artifacts/singularity-m3-020-flagstone-square-20260612.png`、`docs/artifacts/singularity-m3-020-council-steps-20260612.png`、`docs/artifacts/singularity-m3-020-returned-north-bank-20260612.png`。非阻塞缺口：二階動作仍是表層程序回應，尚未落到持久物件狀態、排隊位置、門禁權限、短工任務接取或議棚申訴後果。

2026-06-13 西部城邦圈物件事件回聲：在二階動作基礎上再推一層，不新增 UI、不新增地圖。`do_action` 物件分支現在會把五個 M3-020 二階動作寫入 `event_log`，事件型別為 `scene_object_action`，payload 例如 `m3_020_flagstone_waiting_lines|Use`。玩家對同一物件 `Look` 時，後端會讀該玩家近 6 小時內的最新相關 `scene_object_action`，並在原本 Look 文後補場景回聲：通行牌仍壓在石案邊、等候線旁保留剛讓出的空位、鑄幣門縫變窄、短工牌仍翻在外側、議棚告示仍被記牌人按著。事件也已跨物件：旗石廣場【等候線】被【使用】後，玩家再看【半開鑄幣門】會看到側線延伸到巷口，再看【短工牌】會看到棚邊等工隊列也被壓到棚柱旁；若之後又對鑄幣門【開啟】，最新的門事件會接管回聲。這一步仍不是完整任務/權限系統，但已讓世界記得並傳遞玩家剛做過的輕操作。驗收結果：`cargo fmt --check`、`cargo check`、`cargo test m3_020_followup -- --nocapture`、`cargo test m3_020 -- --nocapture`、`node --check /home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/verify-m3-020.mjs` 皆通過；`./start` 通過 192 tests、checkrooms、release build，`singularity.service` active。Playwright/Chrome 手機寬度登入 `codex` 逐一執行五個二階動作後再點同物件，結果 JSON 顯示每個 `objectChecks` 都有 `relookText`，鑄幣門與短工牌另有 `lookExtraText`，且 `forbiddenHits=[]`、`weakHits=[]`、`missingCardHits=[]`；PG 查到 `codex` 新增 `scene_object_action`。互動後畫面截圖：`docs/artifacts/singularity-m3-020-stone-gate-pass-echo-20260613.png`、`docs/artifacts/singularity-m3-020-flagstone-line-echo-20260613.png`、`docs/artifacts/singularity-m3-020-mint-door-echo-20260613.png`、`docs/artifacts/singularity-m3-020-hire-board-echo-20260613.png`、`docs/artifacts/singularity-m3-020-council-notice-echo-20260613.png`。非阻塞缺口：事件回聲尚未變成可累積的隊列位置、通行資格、短工任務或申訴流程狀態。

2026-06-13 西部城邦圈候號與應工狀態第一步：延續物件事件回聲，先把兩條流程落成可累積狀態，不新增地圖、不重開任務系統。`event` 新增 `scene_object_state`；`do_action` 物件分支會在玩家先【使用】旗石廣場【等候線】後，讓【半開鑄幣門】的 `Look` 額外提供【候號】。玩家點【候號】時，log 顯示玩家站到剛分出的側線、兌牌人把下一輪牌號往石案邊推近，並寫入 `scene_object_state|m3_020_mint_queue|queued_after_waiting_line`。之後再看【半開鑄幣門】會看到「側線已被兌牌人接住」，回【旗石廣場】再看【等候線】會看到「廣場中間少了一股橫穿的人潮」；【候號】完成後不再重複出現在同一物件的後續按鈕列。第二條狀態是【短工牌】：玩家先【閱讀】短工牌後，再看短工牌才會出現【應工】；點下後棚頭人把半日繩結拋到玩家手邊，讓玩家站到商隊卸貨小隊末尾，並寫入 `scene_object_state|m3_020_short_work|accepted_loading_shift`。後續再看短工牌會看到「短工牌被棚頭人翻到繩結下」，回旗石廣場再看等候線會看到「通往棚口的人流短短薄了一層」。前端補 `Queue`→【候號】、`AcceptJob`→【應工】，並 bump `main.js` 與 service worker cache；驗收腳本開頭會清掉 `codex` 舊 M3-020 場景事件，確保每次驗收都從乾淨流程重演。驗收結果：`node --check web/main.js && node --check web/sw.js && node --check /home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/verify-m3-020.mjs`、`cargo fmt --check`、`cargo check`、`cargo test m3_020 -- --nocapture`、`cargo test player_scene -- --nocapture` 皆通過；`./start` 通過 197 tests、checkrooms、release build，`singularity.service` active。Playwright/Chrome 手機寬度登入 `codex` 實走 M3-020，實點【候號】與【應工】後回北岸，結果 JSON `/home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/m3-020-western-city-state-result-20260613.json` 顯示 `forbiddenHits=[]`、`weakHits=[]`、`missingCardHits=[]`；目視檢查截圖 `docs/artifacts/singularity-m3-020-mint-queue-action-20260613.png`、`docs/artifacts/singularity-m3-020-mint-queue-state-20260613.png`、`docs/artifacts/singularity-m3-020-flagstone-line-state-20260613.png`、`docs/artifacts/singularity-m3-020-hire-board-job-action-20260613.png`、`docs/artifacts/singularity-m3-020-hire-board-job-state-20260613.png`、`docs/artifacts/singularity-m3-020-flagstone-line-short-work-state-20260613.png` 正常。DB 驗證：`codex` 有 `scene_object_state|m3_020_mint_queue|queued_after_waiting_line` 與 `scene_object_state|m3_020_short_work|accepted_loading_shift`，且位置回到 `grid:0:0`。非阻塞缺口：這只是鑄幣側線被接住與短工小隊收人，不是完整排隊號碼、通行資格、鑄幣門禁、短工任務履約或議棚申訴流程。

2026-06-13 西部城邦圈四條場景狀態收口：在【候號】與【應工】之外，補上兩條同層級的可累積程序入口。【通行牌】先【閱讀】後，再看通行牌會出現【候檢】；點下後門衛把玩家去向壓進短冊，寫入 `scene_object_state|m3_020_pass_check|joined_gate_review`，後續再看通行牌與旗石廣場會出現牌序被收進石案側邊、候檢線不再外擠的狀態回聲。【議棚告示】先【閱讀】後，再看告示會出現【押名】；點下後名字被記牌人押在階側短冊，寫入 `scene_object_state|m3_020_council_complaint|registered_notice_name`，後續再看告示與旗石廣場會出現押名短冊與往坡街那側少一股反推人潮。前端補 `JoinCheck`→【候檢】、`RegisterComplaint`→【押名】，後端按鈕仍遵守前置閱讀/使用才顯示、完成後不重複顯示。驗收結果：`node --check web/main.js`、`node --check web/sw.js`、`node --check /home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/verify-m3-020.mjs`、`cargo fmt --check`、`cargo check`、`cargo test m3_020 -- --nocapture`、`cargo test player_scene -- --nocapture` 皆通過；`./start` 通過 199 tests、checkrooms、release build，`singularity.service` active。Playwright/Chrome 手機寬度登入 `codex` 實走四條流程並回北岸，結果 JSON `/home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/m3-020-western-city-state-result-20260613.json` 顯示 `forbiddenHits=[]`、`weakHits=[]`、`missingCardHits=[]`。目視檢查截圖：`docs/artifacts/singularity-m3-020-stone-gate-pass-check-action-20260613.png`、`docs/artifacts/singularity-m3-020-stone-gate-pass-check-state-20260613.png`、`docs/artifacts/singularity-m3-020-flagstone-line-pass-check-state-20260613.png`、`docs/artifacts/singularity-m3-020-council-notice-register-action-20260613.png`、`docs/artifacts/singularity-m3-020-council-notice-register-state-20260613.png`、`docs/artifacts/singularity-m3-020-flagstone-line-council-state-20260613.png`。DB 驗證：`codex` 有四條 `scene_object_state`：`m3_020_pass_check|joined_gate_review`、`m3_020_mint_queue|queued_after_waiting_line`、`m3_020_short_work|accepted_loading_shift`、`m3_020_council_complaint|registered_notice_name`，且位置回到 `grid:0:0`。非阻塞缺口：這仍是程序入口與場景狀態，不是完整通行資格、排隊號碼、鑄幣准入、短工履約或議棚裁決。

## 現況問題

目前 `src/grid/reveal.rs` 的核心行為：

- `reveal_grid_disk(center, radius)` 會一次揭露 `(2r+1)^2` 方形區域。
- 新格由 `generate_wild_cell(world_seed, coord)` 依座標 hash 獨立生成。
- 地形是加權隨機：草原、平地、森林、丘陵、水域、沼澤、荒漠等。
- 每格彼此沒有村落結構、家庭、職業、路網意圖、場所職責。

這正是要淘汰的「無邏輯地圖」。

## 保留範圍

以下不是垃圾，先保留：

- `src/grid/coord.rs`：方格座標、方向、`grid:x:y` room id。
- `src/grid/grid.rs`：`SquareGrid` / `GridCell` 容器、移動判定、尋路基本能力。
- `src/db/grid_world.rs`：`grid_cells` 持久層。
- `src/server/handler/movement.rs`：玩家移動與 L4 出口卡片資料來源。
- `src/server/grid_manager.rs`：伺服器持有當前方格世界。

理由：正式地圖仍需要方格、格子、持久化、玩家/NPC 位置、路口形狀 UI。

## 刪除／降級範圍

### A. 立即降級

- `src/grid/reveal.rs::generate_wild_cell`
  - 從「世界生成器」降級為離線／未接入工具。
  - 不得再用來生成村落、小鎮、道路、民宅。

- `src/grid/reveal.rs::reveal_grid_disk`
  - 不得在登入時半徑 5 自動鋪出 121 格。
  - 不得在 scout 邊界時自動生成新格。

- `src/server/handler/auth.rs`
  - 目前登入時呼叫：`reveal_grid_disk(grid, SquareCoord::new(gx, gy), 5)`。
  - 目前應保持：登入只顯示玩家所在格與已存在出口；世界未初始化時只建立中性起點。

### B. 可清空的資料

- PostgreSQL `grid_cells` 內既有隨機揭露地圖與舊村莊模板可以備份後清空。
- `entity_rooms` 與 `entities.grid_x/grid_y` 裡指向舊 `grid:x:y` 的 entity 位置可以重設到 `grid:0:0`。

清空前需先做備份，因為目前玩家/NPC 位置可能仍參照這些座標。

### C. 可刪或歸檔的舊 UI 檔

以下若已非主畫面，可移到 archive 或刪除：

- `web/grid-map.js`
- `web/map_viewer.html`
- `web/city_map.html`
- `web/world_map.svg`
- repo 根目錄 `map_viewer.html`

但刪前要搜尋是否仍被 `start`、路由、測試或文件引用。

## 新生成主線

正式地圖未接入前的啟動規則：

1. 若 `grid_cells` 空：只寫入 `grid:0:0` 中性起點。
2. 正式地圖需由明確設計／匯入流程接入，不得由登入、探路或啟動流程自動生成。
3. 另建或補充連接資料時，不要用座標四向自動全通決定世界形狀。
4. 玩家 spawn：`grid:0:0`。
5. NPC 暫留為測試載體，之後洗白重灌「靈魂」。

## 驗收

- 登入後不再自動出現 121 格隨機地形。
- 玩家出生點暫為 `grid:0:0` 中性起點，不是小市口、溪橋、茶棚或任何已廢村莊。
- 四方向出口只來自已存在的連接資料；沒有正式地圖時不顯示舊村莊出口。
- `grid_cells` 中不存在舊村莊模板或大量只有地形名、沒有職責的格子。
- scout 不會因邊界不存在而自動生成新格。

## 建議實作順序

1. `src/grid/village_template.rs` 目前只建立中性起點；名稱保留是相容匯出，語意不再是村莊模板。
2. `ensure_seed_village(grid)`：空世界時只插入 `grid:0:0`。
3. 登入流程不得半徑 5 `reveal_grid_disk`。
4. scout 邊界不得動態生成新格。
5. 寫測試：世界初始化後格數為 1；探路不會生成新格。
6. 備份舊 `grid_cells`。
7. 清空舊 `grid_cells`。
8. 重啟服務驗證只剩中性起點。

順序不能顛倒：若只清空資料庫但保留登入半徑揭露或 scout 動態生成，下一次登入／探路會立刻再次生成無邏輯地圖；若只改模板但不清舊資料，`save_square_grid_to_pg` 目前是增量 upsert，不會刪除表中既有舊格。

## 2026-06-08 補充：玩家層背景微敘事

登入後第一屏已從單純 L1-L4 骨架推進到 `M3-027 中河渡口聚落帶` 的玩家層畫面：北岸渡口可直接看出候渡、渡船、短市、歇腳與人群。

本輪補上背景微互動的玩家層保護：

- NPC 自主微互動不再用私名模板，例如 `【某人】對【某人】笑了笑`。
- 渡口場景使用公共身分與場景句，例如候渡人群讓出縫隙、短市看貨、渡工收繩等。
- 玩家在線房間判定以 `entities.grid_x/grid_y` 為權威，舊 `entity_rooms` 只作 fallback；否則模擬迴圈會看不到站在方格世界中的玩家。
- `send_narrate_to_room` 同步改用同一個玩家位置鍵，避免背景敘事觸發後仍送不到當前格玩家。

驗收截圖：

- `docs/artifacts/singularity-north-bank-ambient-public-labels-20260608.png`

## 2026-06-14 補充：置頂世界資訊與場景文字流

第一屏分成獨立置頂的世界資訊列，以及下方可滾動的場景正文。置頂列承載日晷、時段、時間與天氣；場景正文順序為 L3 場景描述、場景內文字流、L4 可操作卡片。場景內文字流不顯示「log」或「現場動態」標題，也不把玩家本人寫成場景中的可見對象；它只接續描述當前場景的表面變化。

接續補上北岸第一屏三行場景流：船繩收繞、短市低聲討價、等船人群往踏板口挪動，呈現在 L3 後、L4 前。後續再把固定三行改成依當前遊戲分鐘、天氣鍵、渡船 phase 與未具名在場人數抽取的玩家可讀表面變化；同一個北岸渡口在清晨霧氣、夜間燈影、渡船離岸、對岸靠船、候渡人群多寡下會讀出不同的三行現場狀態。

Chrome/Playwright 手機寬度登入 `codex` 驗收，世界資訊列仍在 `#sw-worldbar`，場景流仍在 `#sw-desc-panel`，未出現「現場動態」、`【你】`、`你站在`、後設/debug/資料庫字樣或精確人數。驗收截圖：`docs/artifacts/singularity-worldbar-pinned-scene-flow-20260614.png`、`docs/artifacts/singularity-scene-flow-multiline-north-bank-20260614.png`、`docs/artifacts/singularity-scene-flow-dynamic-north-bank-20260614.png`。

## 2026-06-16 補充：北岸渡口可玩第一屏入口

登入 `codex` 後第一屏仍是 `北岸渡口 / grid:0:0 / M4-027-01`。本輪沒有擴地圖、沒有恢復底部方向欄、沒有加入飽腹/精力等生存條；只把北岸第一屏切成玩家能直接操作並承受結果的三條入口。

玩家可見效果：

- 點底部姓名開角色面板，可看見鎂、可用物摘要與手持工具狀態；背包資訊不塞在姓名旁。實測面板顯示 `鎂：2`、`蘆草束 ×3`、`未持工具`。
- 【貨擔】觀察後出現【搬貨收鎂】，執行後 log 顯示短市的人付鎂，玩家鎂從 1 變 4。
- 【熱湯攤】觀察後出現【買滿碗熱湯（2鎂）】與【等分湯】，買湯後玩家鎂從 4 變 2。
- 【候渡隊列】觀察後出現【排隊候船】，執行後船戶認住玩家位置；再看隊列時，候渡者按同一條木牌線往踏板口慢慢挪，表示人群和玩家共享同一個隊列規則，不只是隨機台詞。

驗收：`node --check web/sw-panes.js web/main.js web/sw-room-desc.js web/sw.js`、`cargo fmt --check`、`cargo test m3_027 -- --nocapture`、`cargo test player_scene -- --nocapture`、`cargo test north_bank -- --nocapture`、`cargo check`、`./start` 全部通過。Chrome/Playwright 手機寬度實際登入 `codex` 並點完三條入口，結果 JSON：`/home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/north-bank-playable-entry-20260616.json`；截圖：`docs/artifacts/singularity-north-bank-playable-entry-20260616.png`、`docs/artifacts/singularity-north-bank-player-panel-20260616.png`。

## 2026-06-16 補充：M3-027 第一個可玩閉環

延續北岸渡口三條入口，本輪不擴地圖、不新增地圖系統，只把既有 `M3-027 中河渡口聚落帶` 連成第一條玩家可走的物件閉環。新增玩家可見物件：【蘆草收牌】在 `岸邊短市`、【踏板木牌】在 `渡船踏板`、【上岸繩】在 `南岸泥坡`、【倉棚點貨牌】在 `倉棚前場`。

玩家可見因果：

- 【貨擔】搬到踏板邊後，玩家拿到 3 鎂；到 `倉棚前場` 看【倉棚點貨牌】會讀到踏板那頭清出的貨擔已被記牌人看見，並可【對貨領尾款】再拿 1 鎂。
- 【候渡隊列】排隊後，`渡船踏板` 的【踏板木牌】會認出隊列位置，顯示【憑隊上船】；上船後 `南岸泥坡` 的【上岸繩】會出現上岸腳點，顯示【踩泥上岸】。候渡人群同樣沿木牌線挪動，和玩家共用同一套隊列/登船規則。
- `岸邊短市` 的【蘆草收牌】會收玩家背包中的 `蘆草束`，執行【賣蘆草】後玩家拿到 4 鎂並少一束蘆草。

驗收：`node --check web/main.js`、`node --check /home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/verify-m3-027-loop-20260616.mjs`、`cargo fmt --check`、`cargo test m3_027 -- --nocapture`、`cargo check`、`./start` 全部通過。Chrome/Playwright 手機寬度登入 `codex` 實走北岸 → 短市 → 踏板 → 南岸 → 倉棚，結果 JSON `/home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/m3-027-loop-20260616.json` 顯示 `ok=true`、`forbiddenHits=[]`、`missing=[]`。截圖：`docs/artifacts/singularity-m3-027-loop-first-screen-20260616.png`、`docs/artifacts/singularity-m3-027-loop-market-20260616.png`、`docs/artifacts/singularity-m3-027-loop-plank-20260616.png`、`docs/artifacts/singularity-m3-027-loop-south-landing-20260616.png`、`docs/artifacts/singularity-m3-027-loop-warehouse-20260616.png`、`docs/artifacts/singularity-m3-027-loop-player-panel-20260616.png`。

## 2026-06-16 補充：M3-027 蘆草物質交易小循環

本輪把上一筆的短市交易從「背包裡預先有蘆草」改成玩家實際從世界來源取得。`北岸渡口` 的【河霧蘆叢】接入 `P0 可交易世界物質` 來源 `src_m3_027_north_bank_reeds`，玩家採集會從來源庫存扣一份並把 `蘆草束` 放入背包；同一來源也被 NPC 採集流程共用，NPC 採集會保留來源底線，不是另一套隨機台詞。

玩家可見物質流：

- 【河霧蘆叢】採集後，log 顯示 `蘆草束` 收進背包；再看蘆叢會讀到被割矮、前面採過的人也沿同一條水線取草。來源採到暫空時，畫面顯示長莖要等下一輪水氣和人手補上。
- 【蘆草收牌】不再是一次性按鈕。第一束賣出後，短市顯示價牌還沒收滿；第二束賣出後，價牌翻到背面，該輪不再顯示【賣蘆草】。
- 短市收滿後，到 `倉棚前場` 看【倉棚點貨牌】，會讀到濕蘆草已墊進貨籃和木箱角，倉棚前場少了一點箱底磨泥的聲音。這是交易結果跨到另一個場景的可見狀態。

驗收：`node --check web/main.js`、`node --check /home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/verify-m3-027-material-cycle-20260616.mjs`、`cargo fmt --check`、`cargo test m3_027 -- --nocapture`、`cargo check`、`./start` 全部通過。Chrome/Playwright 手機寬度登入 `codex`，背包先空、鎂 20，實走採集兩束 → 短市賣兩束 → 倉棚看回聲 → 玩家面板；結果 JSON `/home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/m3-027-material-cycle-20260616.json` 顯示 `ok=true`、`forbiddenHits=[]`、`missing=[]`，結束面板顯示 `鎂：28`、`背包空`、`未持工具`。截圖：`docs/artifacts/singularity-m3-027-material-cycle-first-screen-20260616.png`、`docs/artifacts/singularity-m3-027-material-cycle-reeds-one-20260616.png`、`docs/artifacts/singularity-m3-027-material-cycle-reeds-empty-20260616.png`、`docs/artifacts/singularity-m3-027-material-cycle-market-one-20260616.png`、`docs/artifacts/singularity-m3-027-material-cycle-market-filled-20260616.png`、`docs/artifacts/singularity-m3-027-material-cycle-warehouse-20260616.png`、`docs/artifacts/singularity-m3-027-material-cycle-player-panel-20260616.png`。

## 2026-06-16 補充：M3-027 多物質短市/倉棚小市場

本輪不擴地圖、不新增地圖系統，把上一筆蘆草單線交易改成同一個短市需求物件承接多種世界物質。`岸邊短市` 新增【舊麻袋堆】，採集後取得 `麻袋布`；`渡船踏板` 新增【濕繩圈】，採集後取得 `麻繩卷`；原【蘆草收牌】玩家可見名改為【短市包貨單】，同時呈現三條需求：蘆草墊箱、麻袋布包貨、麻繩捆箱。

玩家可見物質流：

- 【河霧蘆叢】採集兩次取得兩束 `蘆草束`；來源暫空時，畫面顯示只剩貼水線短葉，並寫出前面採過的人也沿同一條水線取草。
- 【舊麻袋堆】採集取得 `麻袋布`；來源暫空時，畫面顯示能剪成麻袋布的布片被搬貨人取走，表示搬貨人與玩家使用同一堆來源。
- 【濕繩圈】採集取得 `麻繩卷`；來源暫空時，畫面顯示能捲成麻繩卷的繩材被船戶和搬貨人取走，表示船戶/搬貨人與玩家使用同一堆來源。
- 【短市包貨單】初看時同時出現【賣蘆草】【交麻袋布】【交麻繩卷】。兩束蘆草填滿墊箱需求後，麻袋布包貨與麻繩捆箱仍可繼續；三條需求填滿後，再看包貨單不再顯示三個收貨按鈕。
- 到 `倉棚前場` 看【倉棚點貨牌】，會讀到短市送來的箱包已分好：濕蘆草墊住箱角，麻袋布裹住磨邊，麻繩把同一批短貨重新捆成一排。

驗收：`node --check web/main.js`、`node --check /home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/verify-m3-027-multi-material-market-20260616.mjs`、`cargo fmt --check`、`cargo test m3_027 -- --nocapture`、`cargo check`、`./start` 全部通過。Chrome/Playwright 手機寬度登入 `codex`，背包空、鎂 20 起，實走採兩束蘆草 → 取麻袋布 → 取麻繩卷 → 短市交三種材料 → 倉棚看回聲 → 玩家面板；結果 JSON `/home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/m3-027-multi-material-market-20260616.json` 顯示 `ok=true`、`forbiddenHits=[]`、`missing=[]`，結束面板顯示 `鎂：39`、`背包空`、`未持工具`。截圖：`docs/artifacts/singularity-m3-027-multi-material-first-screen-20260616.png`、`docs/artifacts/singularity-m3-027-multi-material-reeds-20260616.png`、`docs/artifacts/singularity-m3-027-multi-material-sackcloth-source-20260616.png`、`docs/artifacts/singularity-m3-027-multi-material-rope-source-20260616.png`、`docs/artifacts/singularity-m3-027-multi-material-market-open-20260616.png`、`docs/artifacts/singularity-m3-027-multi-material-market-filled-20260616.png`、`docs/artifacts/singularity-m3-027-multi-material-warehouse-20260616.png`、`docs/artifacts/singularity-m3-027-multi-material-player-panel-20260616.png`。

## 2026-06-16 補充：M3-028 北岸第一屏遊玩感重錨收口

本輪不擴地圖、不新增通用生活條、不恢復底部方向欄，把 M3-028 的玩家體感目標收口到實際畫面：登入 `codex` 後第一屏仍是 `北岸渡口`，場景正文直接呈現渡船、短市、歇腳處、蘆叢、貨擔、熱湯攤、隊列木牌與人群動線；L4 保留出口與觀察態人物/人群卡，並露出【河霧蘆叢】【濕長凳】【熱湯攤】【貨擔】【船繩】【避風棚】【炭火盆】【候渡隊列】等可點入口。

玩家可見效果：

- 點底部 `codex` 姓名開角色面板，可見 `[ 所在 ] 北岸渡口`、`鎂：20`、`背包空`、`未持工具`；背包沒有塞到姓名旁，玩家本人也沒有被寫進場景正文。
- 第一屏 5 條起手入口已驗到：`貨擔`＝搬貨收鎂/接活，`候渡隊列`＝通行狀態，`河霧蘆叢`＝材料與交易準備，`船繩`＝工作手勢/熟練線索，`熱湯攤`＝消耗/歇腳選擇；另有【中河谷地路口】作探索出口。
- 【貨擔】執行【搬貨收鎂】後，玩家鎂從 20 變 23；【船繩】在搬貨後執行【搭手拉繩】，log 顯示「記住船腹貼岸前那一下手勢」；【候渡隊列】執行【排隊候船】，船戶認住玩家位置；點【中河谷地路口】後，前端狀態與畫面同步到 `grid:0:1 / 中河谷地路口`。
- 最小同規則行動者已落地：玩家停在北岸時，simulation loop 內的公共身份「搬貨人」會依局部排程消耗同一個北岸蘆草來源並留下房間事件；玩家畫面看到「搬貨人沿水線取走一束蘆草，塞進踏板邊待墊箱的空貨籃；蘆叢靠木樁那片矮了下去。」不是隨機台詞。
- 修正驗收時抓出的兩個破口：`room_enter` 現在會同步前端 `gameState` 的 `room_id/room_name`，舊 `北岸等候處/北岸候渡處/北岸後渡處` 玩家層描述會被清洗成「北岸渡口」。

驗收：`node --check web/main.js web/sw-room-desc.js web/sw.js web/sw-panes.js web/sw-object-list.js`、`node --check /home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/verify-m3-028-north-bank-playfeel-20260616.mjs`、`cargo fmt --check`、`cargo test m3_027 -- --nocapture`、`cargo check`、`./start` 全部通過；`./start` 內 230 tests、checkrooms、release build 通過並重啟 `singularity.service`。Chrome/Playwright 手機寬度登入 `codex` 實測結果 `/home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/m3-028-north-bank-playfeel-20260616.json` 顯示 `ok=true`、`forbiddenHits=[]`、`missing=[]`。截圖：`docs/artifacts/singularity-m3-028-first-screen-20260616.png`、`docs/artifacts/singularity-m3-028-player-panel-20260616.png`、`docs/artifacts/singularity-m3-028-npc-actor-trace-20260616.png`、`docs/artifacts/singularity-m3-028-rope-skill-trace-20260616.png`、`docs/artifacts/singularity-m3-028-moved-crossroad-20260616.png`。

下一個未完成缺口：M3-028 只證明北岸第一屏微型局部可玩；NPC 同規則行動者仍是北岸單點公共身份 actor，尚未升級成可跨房間持物、排班、交付、資產與需求的正式 NPC 個體模型。

## 2026-06-16 下一步：M3-029 北岸短市/倉棚局部經濟閉環

重新檢視 `docs/DESIGN_KEYWORDS.md`、`docs/KEYWORDS_ASSESSMENT.md`、`docs/design/遊玩感關鍵字清單—研究推擬稿.md` 與 wiki 的遊玩感/經濟/NPC 觀測頁後，下一步不應再擴地圖、補材料百科或美化場景文字。M3-028 已完成第一屏玩家自身錨點、5 條起手入口與最小公共身份 actor；真正缺口是玩家與 NPC 還沒有在同一個局部經濟需求中持物、移動、競爭、交付、結算。

M3-029 目標：把既有 `北岸渡口`、`岸邊短市`、`渡船踏板`、`倉棚前場` 做成一個可反覆操作的需求、取得、交付、結算循環。物質只作為玩家與 NPC 共用規則的輸入；沒有玩家可見需求、用途、交付結果與 NPC 同規則接點的物質，不得進第一版。

必須落地的玩家可見結果：

- `短市包貨單` 或等價物件升級成局部需求板，能讓玩家看懂缺什麼、誰在收、交付後改變什麼、報酬或交換條件是什麼。
- 至少一名正式 NPC 個體有位置、鎂、持物、當前工作/目的、下一個去處與時間成本；玩家不操作時，它仍會推進同一張需求板。
- 玩家與 NPC 共用至少一個物質來源與同一個交付端點；玩家交付與 NPC 交付都會改變需求板、來源或倉棚狀態。
- 玩家能完成一次完整循環：看見需求 -> 取得或搬運物品 -> 交付 -> 鎂或物品改變 -> 短市/倉棚/來源留下可見後果。
- 至少一個結果跨房間可見，例如短市收滿後倉棚點貨牌變化，或 NPC 先交付後玩家抵達時看到剩餘機會。

驗收邊界維持：實際登入 `codex`，用 Browser/Chrome/Playwright 截到第一屏、需求板、玩家交付後、NPC 推進後、跨房間回聲；通過 `node --check` 相關前端、`cargo fmt --check`、`cargo test m3_027 -- --nocapture` 或新增 `m3_029` 測試、`cargo check`、`./start`。不得使用 usurper-reborn、剝名、詞元、Token、念場、110V、通用生活條、材料百科或新地圖系統。

## 2026-06-16 補充：M3-029 北岸短市/倉棚局部經濟閉環收口

本輪不擴地圖、不新增材料百科、不加入通用生活條，把 `北岸渡口`、`岸邊短市`、`倉棚前場` 連成一條玩家與正式 NPC 共用的局部經濟循環。`短市包貨單` 仍是玩家可讀需求板，`河霧蘆叢` 是共用來源，`倉棚點貨牌` 是跨房間結果回聲。

玩家可見效果：

- 第一屏仍是 `北岸渡口`。L3 能讀到渡船、短市、歇腳、蘆叢、貨擔與候渡人群；L4 仍把出口、觀察態人物、人群與可點物件放在同一層，不顯示精確人數。
- 玩家點底部 `codex` 姓名，角色面板顯示 `[ 所在 ] 北岸渡口`、`鎂：20`、`背包空`、`未持工具`；背包沒有塞在姓名旁。
- 正式 NPC 個體 `npc_m3_029_pack_runner / 搬貨人` 會在玩家不操作時使用同一個北岸蘆草來源：先在北岸取一束能墊箱的蘆草，再到短市交到同一張 `短市包貨單`，最後移到 `倉棚前場` 等點貨。
- 玩家到 `岸邊短市` 看【短市包貨單】時，能看到「搬貨人先交的一束蘆草」已被收進秤邊，蘆草還差最後一束，且文字明確寫出搬貨人和玩家照同一張單交料。
- 玩家回北岸採集【河霧蘆叢】取得 `蘆草束`，再回短市賣蘆草後，鎂從 20 變 24；短市文字顯示這一束墊住最後一只貨籃，價牌翻到背面。
- 玩家到 `倉棚前場` 看【倉棚點貨牌】時，能看到短市那只墊了蘆草的貨籃已到倉棚，且搬貨人把貨籃推到點貨牌旁，記牌人先把這行壓住。

驗收：`node --check web/main.js && node --check web/sw-room-desc.js && node --check web/sw.js && node --check web/sw-panes.js && node --check web/sw-object-list.js`、`node --check /home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/verify-m3-029-local-economy-loop-20260616.mjs`、`cargo fmt --check`、`cargo test m3_029 -- --nocapture`、`cargo test m3_027 -- --nocapture`、`cargo check`、`./start` 全部通過；`./start` 內 Clippy、232 tests、checkrooms、release build 通過並重啟 `singularity.service`。Chrome/Playwright 手機寬度登入 `codex` 的結果 JSON：`/home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/m3-029-local-economy-loop-20260616.json`，顯示 `ok=true`、`forbiddenHits=[]`、`missing=[]`。

截圖：`docs/artifacts/singularity-m3-029-first-screen-20260616.png`、`docs/artifacts/singularity-m3-029-npc-gather-trace-20260616.png`、`docs/artifacts/singularity-m3-029-demand-board-after-npc-20260616.png`、`docs/artifacts/singularity-m3-029-player-delivery-20260616.png`、`docs/artifacts/singularity-m3-029-warehouse-echo-20260616.png`。

下一個未完成缺口：M3-029 只完成一名正式 NPC 在蘆草墊箱需求上的取料、交付、結算與跨房間回聲；麻袋布、麻繩卷尚未接入正式 NPC 的同規則搬運循環，也還沒有形成多名 NPC 彼此競爭同一張短市需求板的局部市場節奏。
