# SW-24 地標 icon 覆疊 + 格名 fallback 修正

> 狀態：待派碼農
> 優先級：P2（SW-23 落地後地標仍視覺同質，靠 hover 才能辨識）
> 對齊：`docs/spec/SW-23_前端對接kind與category雙欄渲染.md`、CLAUDE.md「文字 MUD」風格
> 後端唯一真理：`src/grid/cell.rs` Terrain enum + `kind_str()`

## 痛點

1. **地標視覺同質**：SW-23 把所有 landmark 統一金棕底色 `#a89060` + 金色邊框，但客棧/鐵匠/神殿在地圖上長一樣，玩家必須 hover 才知道是什麼。
2. **沒名字的格子文字寫死「地塊」**：`web/sw-grid-render.js:96` 的 fallback `cell.name || '地塊'`，後端現在所有 terrain 都已有中文名（密林、深水、道路…），這個寫死字面比 `cell.terrain` 沒意義。

## 設計決策（已拍板，碼農不要重議）

- **用單字漢字而非 emoji**：對齊文字 MUD 風格，emoji 跨字體不一致
- **取首字優先，重複時取辨識度高的字**：例：客棧→客、酒館→酒、神殿→神
- **顏色與邊框同色 `#d4af6a`**：視覺一體感
- **位置：右上角**：不擋格名（格名在中央）

## 規格

### 1. `web/grid-map.js`：新增 `LANDMARK_ICON` 表

放在 `TERRAIN_COLOR` 表後面：

```javascript
// ── 地標 icon（單字漢字，僅 category=landmark 用）──────────────
var LANDMARK_ICON = {
    'urban':         '邑',
    'inn':           '客',
    'tavern':        '酒',
    'blacksmith':    '鐵',
    'general_store': '雜',
    'clinic':        '藥',
    'workshop':      '工',
    'market':        '市',
    'guild_hall':    '會',
    'temple':        '神',
    'farmhouse':     '舍',
    'farm_field':    '田',
    'academy':       '學',
    'library':       '書',
    'barracks':      '兵',
    'guard_post':    '哨',
    'warehouse':     '倉',
    'granary':       '糧',
    'dock':          '港',
    'bathhouse':     '浴',
    'courthouse':    '法',
    'jail':          '牢',
    'town_hall':     '議',
    'bank':          '銀',
    'mint':          '幣',
    'stables':       '馬',
    'caravanserai':  '商',
    'theater':       '戲',
    'arena':         '武',
    'observatory':   '星',
    'alchemist':     '煉',
    'mage_tower':    '塔',
    'embassy':       '使',
    'prison_yard':   '獄'
};

function iconFor(cell) {
    if (!cell || cell.category !== 'landmark') return null;
    return LANDMARK_ICON[cell.kind] || null;
}
```

`SwGrid` 物件末尾暴露 `iconFor: iconFor`（與 `colorFor` 並排）。

### 2. `web/sw-grid-render.js`：渲染 icon + 修 fallback

#### 2a. icon 元素

格名 span 之後追加：

```javascript
// 既有：
nameEl.className = 'gmap-cell-name';
nameEl.textContent = cell.name || cell.terrain || '？';  // ← 這行同時修 fallback（§2b）
el.appendChild(nameEl);

// 新增：
var iconChar = window.SwGrid.iconFor(cell);
if (iconChar) {
    var iconEl = document.createElement('span');
    iconEl.className = 'gmap-cell-icon';
    iconEl.textContent = iconChar;
    el.appendChild(iconEl);
}
```

#### 2b. fallback 修正

`web/sw-grid-render.js:96` 一行改：

```javascript
// 改前
nameEl.textContent = cell.name || '地塊';
// 改後
nameEl.textContent = cell.name || cell.terrain || '？';
```

理由：所有 terrain 都已有中文名（密林、深水、道路…），`cell.terrain` 比寫死「地塊」有資訊。終極 fallback `'？'` 防 cell.terrain 也意外缺欄時不爆。

### 3. `web/game-grid.css`：icon 樣式

```css
/* 地標 icon：右上角單字，金色 */
.gmap-cell-icon {
    position: absolute;
    top: 2px;
    right: 5px;
    font-size: 16px;
    line-height: 1;
    color: #d4af6a;
    font-weight: bold;
    text-shadow: 0 0 2px rgba(0, 0, 0, 0.6);
    pointer-events: none;
}
```

`text-shadow` 讓淺色字在金棕底色上仍可讀。

### 4. 不在範圍

- 為 infra（道路/橋/牆）配特殊渲染（線條、邊接邊）— SW-25
- 動態 icon（隨營業時間變化）— 不做
- icon 點擊互動（如「進入客棧」）— 物件欄已有，地圖只負責顯示
- 後端 `walkable: bool` → `move_cost: f32`— 不在前端範圍

## 修改檔案清單

| 檔案 | 動作 | 預估 LOC |
|------|------|---------|
| `web/grid-map.js` | 新增 `LANDMARK_ICON` 表（34 條）+ `iconFor()` + SwGrid 暴露 | +45/-1 |
| `web/sw-grid-render.js` | 加 icon 元素、修 fallback、bump 版本 | +10/-2 |
| `web/game-grid.css` | 新增 `.gmap-cell-icon` | +11 |

版本字串：`grid-map.js` v0.20.47 → v0.20.48；`sw-grid-render.js` 同步。

## 驗收（碼農做完，自檢方式）

`node /tmp/sw24-check.mjs` 跑 playwright 注入假 cells，驗：

- [ ] kind=`inn` 格 DOM 含 `.gmap-cell-icon`，文字 `客`
- [ ] kind=`temple` icon 為 `神`、`blacksmith` 為 `鐵`
- [ ] terrain 格（如 plain/forest_heavy）**無** `.gmap-cell-icon` 元素
- [ ] infra 格（road）也無 icon
- [ ] 沒名字的密林格，文字顯示「密林」（不是「地塊」）
- [ ] 沒名字也沒 terrain 的格子退到「？」（防爆）
- [ ] icon 顏色 `rgb(212, 175, 106)` = `#d4af6a`
- [ ] 0 console errors
- [ ] `cell.kind = 'unknown_xyz'` 但 `category='landmark'` 時 `iconFor()` 回 null，不爆

## 風險

- 漢字選字主觀：「使（embassy）」「議（town_hall）」可能玩家看不懂
  - 緩解：tooltip 已含中文名，icon 是輔助辨識，看不懂 hover 即可
  - 後續：玩家測試後若特定 icon 太隱晦，單獨工單調整

## 回滾

純前端三檔，`git revert` 即可。
