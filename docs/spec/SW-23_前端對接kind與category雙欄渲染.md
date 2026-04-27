# SW-23 前端對接 kind / category 雙欄渲染

> 狀態：待派碼農
> 優先級：P1（後端 SW-22 已落地，前端還在用中文 terrain 當色表 key）
> 對齊：`docs/spec/SW-22_room_payload地形與地標補強.md`、`docs/design/地圖渲染—單一管線與雙前端共用.md`
> 後端唯一真理：`kind`、`category`、`terrain` 三欄出處 = `src/game/room.rs:170-208` `get_grid_cells_around`

## 痛點

`web/grid-map.js` 的 `TERRAIN_COLOR` 表 key 是中文（`'草原'` / `'密林'` / `'地塊'` …），有兩個壞處：

1. **地標格全打成一個顏色**：以前後端 `terrain_name_zh` 對 Inn/Tavern/Blacksmith 等走 fallback `_ => "地塊"`，前端就只看到 `'地塊'` 一律灰褐色。SW-22 後端補完了 `terrain_name_zh` 窮舉（客棧/酒館/鐵匠鋪等），但前端色表沒新增對應 key，這些地標格現在仍是「無色 → UNKNOWN 灰褐」。
2. **中文是 UI 語言不該當 protocol key**：日後若改翻譯（簡中/英文/別字）色表整個失效。`kind`（snake_case）才是穩定的後端契約。

## 規格

### 1. `web/grid-map.js`：色表 key 切到 `kind`

**`TERRAIN_COLOR` 整表重寫，key 從中文改 `kind` snake_case**。`category === 'terrain'` 的全列舉：

```javascript
var TERRAIN_COLOR = {
    // category=terrain（自然地塊）
    'plain':         '#b8ba82',
    'grassland':     '#a8b06a',
    'forest':        '#6b8e5a',
    'forest_light':  '#7fa868',
    'forest_heavy':  '#4d7040',
    'hills':         '#9a7a55',
    'mountain':      '#7a6a5a',
    'water':         '#6a8aa8',
    'water_deep':    '#5a7a9a',
    'swamp':         '#7a9080',
    'desert':        '#d8c89a',
    'tundra':        '#d8dce8',
    'jungle':        '#4a7848',

    // category=landmark（人造地標，統一暖金底色 + 視覺差異交給 §2 邊框/icon）
    'urban':         '#a89060',
    'inn':           '#a89060',
    'tavern':        '#a89060',
    'blacksmith':    '#a89060',
    'general_store': '#a89060',
    'clinic':        '#a89060',
    'workshop':      '#a89060',
    'market':        '#a89060',
    'guild_hall':    '#a89060',
    'temple':        '#a89060',
    'farmhouse':     '#a89060',
    'farm_field':    '#c0b878',

    // category=infra（通行建設）
    'road':          '#8a7a5a',
    'bridge':        '#9a8060',
    'wall':          '#5a4a3a'
};

var UNKNOWN = '#2a2418';

function colorFor(cell) {
    if (!cell) return UNKNOWN;
    return TERRAIN_COLOR[cell.kind] || UNKNOWN;
}
```

`renderMinimap()` 內兩處 `TERRAIN_COLOR[cell.terrain]` 改呼 `colorFor(cell)`。

### 2. `web/sw-grid-render.js`：landmark 加 class 標記

`category === 'landmark'` 的格子追加 class `gmap-cell-landmark`：

```javascript
var el = document.createElement('div');
el.className = 'gmap-cell' + (isPlayer ? ' gmap-cell-player' : '');
if (cell.category === 'landmark') {
    el.classList.add('gmap-cell-landmark');
} else if (cell.category === 'infra') {
    el.classList.add('gmap-cell-infra');
}
el.style.backgroundColor = window.SwGrid.colorFor(cell); // 改用共享 helper
```

把 `colorFor` 透過 `window.SwGrid` 暴露出去（在 grid-map.js 末尾的 SwGrid object 加一欄 `colorFor: colorFor`）。

### 3. `web/game.css` / `web/game-grid.css`：對應 class 樣式

```css
/* 地標格：金色細邊 + 微微內陰影營造「有東西」感 */
.gmap-cell-landmark {
    border: 1px solid #d4af6a;
    box-shadow: inset 0 0 4px rgba(212, 175, 106, 0.3);
}

/* 通行建設：較細較暗的邊，與一般地塊區隔 */
.gmap-cell-infra {
    border: 1px dashed #6a5a4a;
    opacity: 0.85;
}
```

樣式寫在 `game-grid.css`（已是格子相關樣式集中地）。

### 4. tooltip：保留中文 terrain 作 hover 顯示

`sw-grid-render.js` 渲染格子處：

```javascript
el.title = cell.name && cell.name !== cell.terrain
    ? cell.name + '（' + cell.terrain + '）'
    : cell.terrain;
```

`name` 是地標專名（「明月客棧」），`terrain` 是中文類別（「客棧」）——名與類別都顯示，方便 debug 與將來盲人模式。

### 5. 不在範圍

- 地標 icon（▲/■/⚙）覆疊到格子上 — SW-24
- `infra` 道路用線條而非格子 — SW-25（要動 `sw-grid-render` 拓撲渲染）
- 移動成本影響色彩明暗（明亮道路、黯沉沼澤）— 後端要先把 `walkable: bool` 升級成 `move_cost: f32`，獨立工單

## 修改檔案清單

| 檔案 | 動作 |
|------|------|
| `web/grid-map.js` | 重寫 `TERRAIN_COLOR`、新增 `colorFor(cell)`、SwGrid 暴露 `colorFor`、`renderMinimap` 兩處改用 helper |
| `web/sw-grid-render.js` | landmark/infra class 追加、`backgroundColor` 改用 `colorFor`、tooltip 補完 |
| `web/game-grid.css` | 新增 `.gmap-cell-landmark` `.gmap-cell-infra` |

預估 LOC 動：grid-map.js ~+30/-25、sw-grid-render.js ~+8/-2、game-grid.css ~+10。

## 驗收

- [ ] 站在 Inn 旁邊，地圖上 Inn 那格底色是金棕色（`#a89060`）+ 金色細邊框，不是 UNKNOWN 灰褐
- [ ] 站在草原格周圍有森林、密林、丘陵——四種顏色明確不同
- [ ] hover Inn 格子，tooltip 顯示「明月客棧（客棧）」之類；hover 草原顯示「草原」
- [ ] minimap 9×9 也吃新色表（地標格金棕、地形格自然色）
- [ ] 後端若日後新增 `Terrain` 變體但前端 `TERRAIN_COLOR` 沒補 → 該格 fallback `UNKNOWN`，不爆炸（colorFor 已防）
- [ ] DevTools console 無新增 error/warn
- [ ] 別忘 `./start` 重啟 + bump 版本字串（`grid-map.js` v0.20.46 → v0.20.47，sw-grid-render.js 同步）

## 不動代碼也能驗的線索

碼農做完，設計者用瀏覽器 DevTools 看：

1. F12 → Network → WS → 找 `grid_view` 訊息 → 展開 `cells[0]` 應有 `kind`, `category`, `terrain`, `name`, `explored`, `walkable` 六欄（後端 SW-22 已落地，這步驗的是訊息正確抵達前端）
2. F12 → Elements → 找 `.gmap-cell-landmark` 應該至少有一格（站到聚落附近）
3. 檢查 `.gmap-cell-landmark` 的 `style.backgroundColor` 是 `rgb(168, 144, 96)` 之類金棕色

## 風險與回滾

- 風險：色表 key 大改，若後端 `kind_str()` 命名與本規格 §1 不一致（例如 `forestlight` vs `forest_light`），整片地圖會變 UNKNOWN 灰褐
  - 緩解：碼農做之前先 `cargo run` 起服務，DevTools 抓一筆真實 `grid_view` 訊息驗 `kind` 拼字
- 回滾：本工單純前端，回滾只要 `git revert` 三檔
