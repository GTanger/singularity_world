# Hex 地形與移動係數表

本文件整理目前 Hex 編輯器/Hex 地圖系統中，地形的可通行與移動成本設定。

## 依據來源

- `src/hex/cell.rs` 的 `Terrain::walkable()` 與 `Terrain::move_cost()`
- `src/hex/grid.rs` 的 `HexGrid::can_walk()`、`HexGrid::find_path()`

## 地形係數總表

| Terrain 枚舉 | 建議中文名稱 | 可步行 | 移動成本倍率 `move_cost` |
|---|---|---:|---:|
| `Road` | 道路 | 是 | `0.5` |
| `Bridge` | 橋樑 | 是 | `0.5` |
| `Grassland` | 草原 | 是 | `1.0` |
| `Farmhouse` | 農舍 | 是 | `1.0` |
| `Inn` | 旅店 | 是 | `1.0` |
| `Tavern` | 酒館 | 是 | `1.0` |
| `Blacksmith` | 鐵匠鋪 | 是 | `1.0` |
| `GeneralStore` | 雜貨店 | 是 | `1.0` |
| `Clinic` | 醫館 | 是 | `1.0` |
| `Workshop` | 工坊 | 是 | `1.0` |
| `Market` | 市集 | 是 | `1.0` |
| `GuildHall` | 公會大廳 | 是 | `1.0` |
| `Temple` | 神殿 | 是 | `1.0` |
| `Academy` | 學院 | 是 | `1.0` |
| `Library` | 圖書館 | 是 | `1.0` |
| `Barracks` | 兵營 | 是 | `1.0` |
| `GuardPost` | 衛所 | 是 | `1.0` |
| `Warehouse` | 倉庫 | 是 | `1.0` |
| `Granary` | 糧倉 | 是 | `1.0` |
| `Dock` | 碼頭 | 是 | `1.0` |
| `Bathhouse` | 浴場 | 是 | `1.0` |
| `Courthouse` | 法院 | 是 | `1.0` |
| `Jail` | 監所 | 是 | `1.0` |
| `TownHall` | 市政廳 | 是 | `1.0` |
| `Bank` | 銀行 | 是 | `1.0` |
| `Mint` | 鑄幣所 | 是 | `1.0` |
| `Stables` | 馬廄 | 是 | `1.0` |
| `Caravanserai` | 商旅驛站 | 是 | `1.0` |
| `Theater` | 劇院 | 是 | `1.0` |
| `Arena` | 競技場 | 是 | `1.0` |
| `Observatory` | 觀測台 | 是 | `1.0` |
| `Alchemist` | 鍊金工房 | 是 | `1.0` |
| `MageTower` | 法師塔 | 是 | `1.0` |
| `Embassy` | 使館 | 是 | `1.0` |
| `PrisonYard` | 囚院 | 是 | `1.0` |
| `Forest` | 森林 | 是 | `1.5` |
| `Hills` | 丘陵 | 是 | `1.5` |
| `Desert` | 沙漠 | 是 | `1.5` |
| `Tundra` | 凍原 | 是 | `1.5` |
| `FarmField` | 農田 | 是 | `1.5` |
| `Jungle` | 叢林 | 是 | `2.0` |
| `Swamp` | 沼澤 | 是 | `2.0` |
| `Water` | 水域 | 否 | `Infinity` |
| `Mountain` | 山地 | 否 | `Infinity` |
| `Wall` | 牆體 | 否 | `Infinity` |

## 規則重點

- `walkable = false` 的地形（`Water`、`Mountain`、`Wall`）不可進入。
- `move_cost` 越高代表越慢，`1.0` 為基準，`0.5` 代表更快。
- `Infinity` 表示不可通行（在設計語義上等同阻擋）。

## 目前路徑計算的注意事項

目前 `HexGrid::find_path()` 是 BFS（按步數最短），**尚未使用 `move_cost` 作加權最短路**。  
也就是說現況會把「每走一步」視為同成本，只要可通行就會納入路徑。

若未來要讓道路優先、沼澤繞行，需改成加權路徑演算法（例如 Dijkstra/A*）並把 `move_cost` 納入邊權重。

## 相關文件

- [Hex探索揭露與漸進生成—規格草案](Hex探索揭露與漸進生成—規格草案.md)：黑格／彩格、單調精煉、局部缺席與全域多樣性、觸發與 FallbackMin。
