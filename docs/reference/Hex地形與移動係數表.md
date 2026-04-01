# Hex 地形與移動係數表

本文件整理目前 Hex 編輯器/Hex 地圖系統中，地形的可通行與移動成本設定。

## 依據來源

- `src/hex/cell.rs` 的 `Terrain::walkable()` 與 `Terrain::move_cost()`
- `src/hex/grid.rs` 的 `HexGrid::can_walk()`、`HexGrid::find_path()`

## 地形係數總表

| Terrain 枚舉 | 建議中文名稱 | 可步行 | 移動成本倍率 `move_cost` |
|---|---|---:|---:|
| `Road` | 道路 | 是 | `0.5` |
| `Grassland` | 草原 | 是 | `1.0` |
| `Urban` | 城區 | 是 | `1.0` |
| `Forest` | 森林 | 是 | `1.5` |
| `Hills` | 丘陵 | 是 | `1.5` |
| `Desert` | 沙漠 | 是 | `1.5` |
| `Tundra` | 凍原 | 是 | `1.5` |
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
