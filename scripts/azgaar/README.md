# Azgaar → 白塔半島完整作者資料

Azgaar/Fantasy-Map-Generator 是直接採用的離線作者工具，不是只借演算法後自行重造。
固定版本會產出完整可編輯資料；白塔設定只在既有結果上作局部約束與優化。

## 一次完成

在奇點世界專案根目錄執行：

```bash
cargo run --bin white-tower-map-generate
```

這一個命令會依序完成：

1. 取得 Azgaar 固定 commit，或採用 `AZGAAR_ROOT` 指定的同版 checkout。
2. 使用 Node.js 24 以上執行 upstream `npm ci` 與 build。
3. 啟動短命本機 Vite，載入固定 seed 與白塔約束。
4. 調用 Azgaar 原生存檔／匯出能力，先寫入受管 staging。
5. 驗證全部收據、Full JSON 層數、GeoJSON、SVG／PNG、`.map` 與非正典邊界。
6. 驗證通過才安全替換正式輸出，再重新編譯驗證內嵌物理資料。
7. 停止伺服器並清除 checkout、node_modules、log、備份與 staging。

可選環境變數：

- `AZGAAR_ROOT`：採用既有且 commit 相符的 Azgaar checkout；此目錄由呼叫者管理，不刪除。
- `WHITE_TOWER_NODE_BIN`：指定 Node.js 24 以上執行檔。
- `TANGER_TMP_ROOT`：指定受管暫存根，必須是絕對路徑。

## 固定來源

- upstream：`Azgaar/Fantasy-Map-Generator`
- commit：`73e43d2b44b9b85b42c5394a0476d90bb22d688e`
- seed：`white-tower-peninsula-v1`
- profile：`whiteTowerConstrained`

## 正式輸出

`data/world/white_tower_azgaar_v1/` 保留 15 個原生作者輸出與 `manifest.json`：

- 可回載 `.map` 與 Full JSON。
- cells、routes、rivers、markers、zones 五種 GeoJSON。
- political、biomes、heightmap、canon overlay 的 SVG 與 1600×1000 PNG。

`data/world/white_tower_physical_geography_v1.json` 是掛回 37 個白塔正典拓撲節點的
橋接資料。manifest 同時記錄每個輸出的 bytes 與 SHA-256，以及橋接檔的外部收據。

## 資料身分

- 完整保留為非正典候選：生成名稱、聚落、道路、國家、行省、文化、宗教、政治、
  標記、區域、筆記、商品、市場、交易，以及所有原生地理層。
- 白塔權威：`data/world/white_tower_topology_v1.json` 的 37 個 accepted/canon 節點。
- 只拒絕：把 Azgaar renderer 當玩家 UI，或把候選直接寫成 runtime／正典。

遇到細節衝突時先在現有資料中找合適 cells 或物件並局部修正。只有依存關係確實受影響
才重算相關層；不能因單一缺陷重抽整張地圖，也不能另寫聚落、道路或國家生成器。

## 個別驗證

```bash
cargo run --bin white-tower-package-check
cargo run --bin white-tower-geography-check
cargo test world::physical_geography
```

套件檢查器會驗 15 個輸出、SHA-256、原生層數、`.map` 標頭、GeoJSON 與視圖尺寸。
地理檢查器會拒絕候選升格、斷鏈、一 cell 一場所、雪域／濕地／水系退化，以及文曜
落回河道格或南哨落回濕地格。

## 手動除錯入口

`white-tower-bridge-export.mjs` 是低階匯出器；平常不需手動分別啟動伺服器與設定路徑。
只有調查 upstream UI／下載失敗時，才顯式提供 `AZGAAR_ROOT`、`AZGAAR_ORIGIN`、
`WHITE_TOWER_OUTPUT` 與 `WHITE_TOWER_PACKAGE_DIR` 直接執行。低階入口不另行生成
Azgaar 已提供的聚落、道路、國家、文化或交易資料。
