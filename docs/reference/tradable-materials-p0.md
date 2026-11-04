# 可交易世界物質 P0

狀態：商品層重寫落地
日期：2026-06-15

## 目標

把交易貨物從占位物 `wild_herb` 推進到有世界來源的普通材料。P0 只處理「採集→材料入背包→NPC/玩家可交易」這一層，不做舊世界特殊設定、店舖庫存、完整市場或新地圖系統。

2026-06-15 修正：前一版把 **70 個來源節點直接做成 70 個玩家可見材料名**，只完成了地圖覆蓋，沒有形成合格商品層。現版改為 **70 個來源節點 → 33 種玩家可見商品**；地點差異留在 `sources[].origin_label`，商品名維持可理解、可交易、可加工。

## 已落地材料批次

目前 `data/world/tradable_materials_p0.json` 有 **33 種商品 / 70 個來源節點**。每個來源節點都有既有地圖房間、來源短語、庫存上限與每日回補；多個來源可指向同一個玩家商品。`data/items.json` 只保留同一批 33 種商品顯示資料；正式行為以 `world::materials` 載入的 P0 配置為準。

覆蓋區域：

| 區域 | 來源數 |
|---|---:|
| 西北礦場帶 | 6 |
| 東部稻地 | 6 |
| 東湖諸港 | 6 |
| 中河渡口聚落帶 | 5 |
| 西南牧場帶 | 5 |
| 西部工坊帶 | 5 |
| 西部牧地 | 5 |
| 東南船港 | 5 |
| 南部糧倉 | 5 |
| 中部城鎮圈 | 4 |
| 北方港灣 | 4 |
| 河口港群 | 4 |
| 西北邊城群 | 3 |
| 西南邊鎮 | 2 |
| 東南外貿港 | 2 |
| 中河谷地 | 1 |
| 西部城邦圈 | 1 |
| 南方大城圈 | 1 |

代表樣本：

| 玩家可見商品 | 商品分類 | 來源節點例 | room_id | 價格錨點 |
|---|---|---|---|---:|
| 蘆草束 | 區域特產 | 北岸河霧蘆叢 | `grid:0:0` | 6 鎂 |
| 麻繩卷 | 繩索/纖維 | 渡船踏板濕繩圈、倉場門口繩捆、遠洋倉門麻繩卷 | `grid:0:-1` 等 | 8 鎂 |
| 麻袋布 | 包裝料 | 岸邊短市舊麻袋堆、水倉前場濕麻袋架 | `grid:1:0` 等 | 7 鎂 |
| 封縫膠料 | 修船料 | 船戶作業點松脂盆、修船架瀝青盆 | `grid:1:-1` 等 | 10 鎂 |
| 木板料 | 木材/板料 | 港灣貨棚破箱板堆、外貿碼頭貨箱板堆 | `grid:1:7` 等 | 8 鎂 |
| 粗礦 | 礦料 | 三號坑口外棚粗料堆、礦渣坡灰黑碎石堆 | `grid:-2:5` 等 | 11 鎂 |
| 粗鹽包 | 鹽 | 邊市口粗鹽袋、補給小市鹽包攤 | `grid:-5:7` 等 | 8 鎂 |
| 稻穀袋 | 食糧 | 糧倉前場糧袋列、糧倉外秤稻穀袋 | `grid:-6:7` 等 | 13 鎂 |
| 草料袋 | 畜產副料 | 草料棚混料袋 | `grid:-7:-2` | 9 鎂 |
| 乾餅袋 | 食糧 | 補給倉乾餅袋 | `grid:6:-6` | 10 鎂 |

商品分類目前覆蓋：食糧、鹽、燃料、繩索/纖維、木材/板料、礦料、修船料、建材、畜產副料、水產副料、包裝料、區域特產。

## 已接線

- `world::materials` 讀取 P0 配置，提供材料查詢、來源查詢、價格錨點、可交易判斷與舊背包相容升級。
- `Store::ensure_p0_material_items` 啟動時把 P0 材料補進 items 快取，避免 PG-first 舊表空資料讓遊戲畫面拿不到名稱。
- `Store::normalize_legacy_material_inventories` 將舊 `wild_herb` 與前一版 70 個來源專名材料升級到 33 種商品，避免玩家交易畫面再次看見占位物或來源碎屑名。
- `NPC Gather` 與未觀測 tick 改為依目前 `room_id` 查來源節點；沒有來源節點就不憑空加物。
- NPC 背景採集仍共用同一批 P0 材料與來源節點，但不會把可見來源扣到完全枯竭；最後一份保留給玩家可見物件互動。玩家採集可拿走最後一份，並看見枯竭狀態。
- 方格 NPC 採收改為 `grid:x:y` → 來源節點，不再依 terrain 直接吐 `grain` 或 `wood`。
- Trade / NPC 街頭兜售只挑 `tradable=true` 的 P0 材料，且依 `trade_priority` 讓食糧、鹽、繩、礦、修船料等主商品優先於魚骨、魚鱗、水苔、木料邊材等副料。
- `北岸渡口` L4 增加 `河霧蘆叢`；玩家 Look 後可點 `採集`，成功取得 `蘆草束`，來源庫存歸零後再次採集只回枯竭文案、不新增物品。
- 測試新增商品層防線：P0 必須維持 70 個來源節點、25-35 種玩家商品、分類完整、來源房間不可重複，且多來源必須合併到共享商品。

## 實機驗收

- 2026-06-15：Playwright/Chrome 以手機寬度登入 `codex`，在 `北岸渡口` 對 NPC `陸志` 輸入「想看看你有什麼」，Talk 回 `open_trade=true` 後自動取得 Trade 報價。畫面 log 顯示：「北岸蘆草束」一份，來源是「北岸河霧蘆叢」，開價 4 鎂，並出現交易輸入列。截圖：`docs/artifacts/singularity-tradable-material-p0-20260615.png`；結果 JSON：`/home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/tradable-material-p0-20260615.json`。
- 2026-06-15：Playwright/Chrome 以手機寬度登入 `codex`，第一屏看見 `北岸渡口`、時間/天氣、渡船/短市/候渡人群與 L4 `河霧蘆叢`。點 `河霧蘆叢` → `採集` 後，log 顯示取得「北岸蘆草束」，背包顯示 `北岸蘆草束`；來源庫存從 1 扣到 0，再點一次 `採集` 顯示「暫時被人取走」的枯竭文案且不新增物品。截圖：`docs/artifacts/singularity-north-bank-reed-gathering-20260615.png`；結果 JSON：`/home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/north-bank-reed-gathering-20260615.json`。
- 2026-06-15：前一版來源覆蓋後，Playwright/Chrome 以手機寬度登入 `codex` 做重啟後冒煙驗收。第一屏仍顯示半夜月相、夜風、北岸候渡、渡船、短市、歇腳、人群與 L4 `河霧蘆叢`，未出現後設/debug/資料庫字樣。截圖：`docs/artifacts/singularity-material-p0-70-smoke-20260615.png`；結果 JSON：`/home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/material-p0-70-smoke-20260615.json`。
- 2026-06-15：商品層重寫後，Playwright/Chrome 以手機寬度登入 `codex`。第一屏仍在 `北岸渡口`，可見渡船、短市、歇腳、候渡人群與 L4 `河霧蘆叢`；對 `河霧蘆叢` 執行採集後，log 顯示取得「蘆草束」收進背包，背包顯示 `蘆草束 ×3`，未再顯示前一版來源專名。當次同屏 NPC 交易回「暫無可交易之物」，交易商品優先級由 Rust 測試驗證。截圖：`docs/artifacts/singularity-material-commodity-layer-20260615.png`；結果 JSON：`/home/tanger/.local/share/tanger/tmp/playwright-singularity-verify/material-commodity-layer-20260615.json`。

## 庫存

P0 每個來源節點有 `stock_max` 與 `recover_per_day`。採集成功會扣 `data/runtime/material_sources.json` 內的節點庫存；跨日依 `recover_per_day` 回補，上限不超過 `stock_max`。

這是最小庫存，不是完整能量密度、在場人數、工具、成熟度或協作採集系統。

## 未完成

- 工具門檻、採集效率、採集動作耗時與多人協作尚未接上；目前是最小一次採集閉環。
- 店舖庫存、交易面板成交、NPC↔NPC 交易與完整市場價格發現尚未完成。
- 舊世界特殊設定不接入 P0 材料層；材料只作為普通世界物資流通。
- 70 個來源目前是物質資料層與後端查詢層；除了北岸 `河霧蘆叢` 外，其他房間尚未逐一做 L4 可點物件與不同取得動作。
- 背包目前只保存 `item_id/qty`，不保存每一份商品的產地批次；因此 Trade 報價只能顯示商品層泛來源，採集當下才顯示精確來源節點。
