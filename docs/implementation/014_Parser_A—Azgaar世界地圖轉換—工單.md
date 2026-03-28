# 014 Parser A：Azgaar 世界地圖 → rooms.json 轉換器（組長領單）

## 目標

將 Azgaar Fantasy Map Generator 的 Full JSON 匯出檔（如 `Celia Full 2026-01-25-04-11.json`，9MB）
轉換為本專案的 rooms.json 格式，生成世界地圖等級的房間節點與連線。

**這是一個獨立的 Node.js / Python 腳本**，不嵌入 Rust 後端。
產出的 JSON 直接放進 `data/rooms/editor/` 供 map_viewer 載入。

---

## 輸入格式（Azgaar Full JSON）

```
{
  info: { mapName, seed, width, height },
  pack: {
    cells: [{ i, c(鄰居), p([x,y]), h(高度), biome, state, province, culture, religion, burg, pop }],
    burgs: [{ i, name, x, y, cell, state, culture, population, type, capital, port, citadel, plaza, walls, temple, shanty }],
    states: [{ i, name, neighbors, diplomacy, provinces }],
    provinces: [{ i, name, state, burg, fullName, color }],
    cultures: [{ i, name }],
    religions: [{ i, name }],
    rivers: [{ i, name, source, mouth, cells }],
    routes: [{ i, group("roads"|"trails"|"searoutes"), points([[x,y,cell_id],...]) }],
    markers: [{ i, type, cell, x, y }],
    zones: [{ i, name, type, cells }]
  },
  biomesData: { name: [...], color: [...] },
  notes: [{ id, name, legend }]
}
```

### 關鍵欄位對照

| Azgaar 欄位 | 意義 | 對應我們的 |
|-------------|------|-----------|
| `pack.burgs[].name` | 城鎮名 | `room.name` |
| `pack.burgs[].x, y` | 座標 | `room.x, y`（map_viewer 用） |
| `pack.burgs[].state` | 所屬國家 index | 查 `states[].name` → `room.zone` |
| `pack.burgs[].culture` | 文化 index | 查 `cultures[].name` → `room.tags[]` |
| `pack.burgs[].type` | 城鎮類型 | `room.tags[]`（River/Naval/Lake/Highland/Generic） |
| `pack.burgs[].citadel/plaza/walls/temple/shanty/port` | 設施旗標 | `room.tags[]` + 未來餵 Watabou 參數 |
| `pack.burgs[].population` | 人口（×1000） | `room.tags[]`（分級：hamlet/village/town/city） |
| `pack.routes[].points` | 路線點序列 | 路線經過的 burg → 建立 Exit 連線 |
| `pack.markers[].type` | 地標類型 | `room.tags[]`（mines/ruins/dungeons/inns 等） |

---

## 輸出格式

每個房間一個 JSON 檔，存入 `data/rooms/editor/`：

```json
{
  "id": "world_hunageria_kadur",
  "name": "卡杜爾",
  "description": "",
  "zone": "hunageria",
  "tags": ["city", "naval", "capital", "citadel", "plaza", "walls", "port"],
  "exits": [
    { "direction": "官道往洪克什", "to": "world_sauntalur_hunkesh" }
  ],
  "objects": [],
  "meta": {
    "source": "azgaar",
    "burg_id": 1,
    "cell_id": 4004,
    "population": 6976,
    "culture": "Koryo",
    "religion": "Old Koryo Ancestors",
    "biome": "草原气候",
    "x": 73500.99,
    "y": 50959.85,
    "watabou_params": {
      "size": 15,
      "citadel": 1,
      "plaza": 1,
      "walls": 1,
      "temple": 0,
      "shantytown": 0,
      "coast": 0,
      "river": 1,
      "port": 1
    }
  }
}
```

同時產出一份 `room_editor_layout.json` 格式的座標映射（供 map_viewer 顯示位置）。

---

## 轉換規則

### 第一步：建立查找表

從 Azgaar JSON 讀取，建立以下查找表（index → 名稱）：

```python
state_names = { s['i']: s['name'] for s in pack['states'] if s.get('cells', 0) > 0 }
culture_names = { c['i']: c['name'] for c in pack['cultures'] }
religion_names = { r['i']: r['name'] for r in pack['religions'] }
biome_names = { i: name for i, name in enumerate(biomesData['name']) }
province_names = { p['i']: p['name'] for p in pack['provinces'] if p }
```

### 第二步：Burg → Room（每座城鎮 = 一個房間）

遍歷 `pack.burgs`（跳過空物件），每個 burg 生成一個房間：

**ID 格式**：`world_{state_name}_{burg_name}`（全小寫，空格轉底線，非 ASCII 保留原樣）

**Zone**：`state_names[burg.state]`（國名，全小寫）

**Tags 生成規則**：

```python
tags = []

# 人口分級（population 欄位 ×1000）
pop = burg['population'] * 1000
if pop >= 20000: tags.append('city')
elif pop >= 5000: tags.append('town')
elif pop >= 1000: tags.append('village')
else: tags.append('hamlet')

# 城鎮類型
if burg['type'] != 'Generic': tags.append(burg['type'].lower())

# 設施旗標
if burg.get('capital'): tags.append('capital')
if burg.get('port'): tags.append('port')
if burg.get('citadel'): tags.append('citadel')
if burg.get('plaza'): tags.append('plaza')
if burg.get('walls'): tags.append('walls')
if burg.get('temple'): tags.append('temple')
if burg.get('shanty'): tags.append('shantytown')
```

**Meta 欄位**：保留原始資料供後續使用（culture、religion、biome、Watabou 參數等）。
`meta.watabou_params` 直接對應 Watabou City Generator 的 URL 參數，
未來可自動生成 Watabou URL 展開城市內部。

**Watabou size 對照人口**：

```python
if pop >= 20000: size = 25    # 大城
elif pop >= 10000: size = 18  # 中城
elif pop >= 5000: size = 12   # 小城
elif pop >= 1000: size = 7    # 村莊
else: size = 4                # 聚落
```

**Watabou river 判定**：burg.type == 'River' 或 'Lake' 時 river=1

### 第三步：Route → Exit（路線 = 房間連線）

路線的 `points` 陣列每個點是 `[x, y, cell_id]`。

**演算法**：

```python
burg_by_cell = { b['cell']: b for b in burgs }  # cell → burg 查找

for route in pack['routes']:
    if route['group'] == 'searoutes':
        route_type = '海路'
    elif route['group'] == 'roads':
        route_type = '官道'
    else:
        route_type = '小徑'

    # 沿路線找出經過的 burg（按順序）
    touched_burgs = []
    for point in route['points']:
        cell_id = int(point[2])
        if cell_id in burg_by_cell:
            burg = burg_by_cell[cell_id]
            if not touched_burgs or touched_burgs[-1]['i'] != burg['i']:
                touched_burgs.append(burg)

    # 相鄰 burg 之間建立雙向連線
    for i in range(len(touched_burgs) - 1):
        a = touched_burgs[i]
        b = touched_burgs[i + 1]
        # a → b
        add_exit(a, direction=f"{route_type}往{b['name']}", to=room_id(b))
        # b → a
        add_exit(b, direction=f"{route_type}往{a['name']}", to=room_id(a))
```

**去重**：同一對 burg 之間可能被多條路線經過，Exit 去重（同 from+to 只保留一條，優先保留 roads > trails > searoutes）。

### 第四步：Marker → 特殊房間

地標（mines、ruins、dungeons、inns 等）生成獨立房間，掛到最近的 burg：

```python
for marker in pack['markers']:
    # 找最近的 burg（歐幾里得距離）
    nearest = min(burgs, key=lambda b: dist(b, marker))

    marker_room = {
        'id': f"world_marker_{marker['type']}_{marker['i']}",
        'name': marker_type_name(marker['type']),  # 見下方映射表
        'zone': state_of(nearest),
        'tags': [marker['type']],
        'exits': [{ 'direction': f"返回{nearest['name']}", 'to': room_id(nearest) }]
    }
    # nearest 也加一條 exit 指向 marker
    add_exit(nearest, direction=marker_type_name(marker['type']), to=marker_room['id'])
```

**地標類型名稱映射**：

```python
MARKER_NAMES = {
    'mines': '礦脈', 'ruins': '遺跡', 'dungeons': '地牢',
    'inns': '旅店', 'brigands': '盜賊據點', 'battlefields': '古戰場',
    'statues': '雕像', 'pirates': '海盜巢穴', 'sacred-forests': '聖林',
    'sacred-pineries': '聖松林', 'circuses': '競技場', 'necropolises': '亡者之城',
    'volcanoes': '火山', 'hot-springs': '溫泉', 'water-sources': '水源地',
    'bridges': '橋', 'lighthouses': '燈塔', 'waterfalls': '瀑布',
    'caves': '洞穴', 'libraries': '藏書閣', 'encounters': '遭遇點',
    'migration': '遷徙路線', 'fairs': '集市', 'jousts': '馬上比武場',
    'canoes': '渡口', 'dances': '舞場',
    'lake-monsters': '湖怪棲地', 'sea-monsters': '海怪棲地', 'hill-monsters': '丘怪棲地'
}
```

### 第五步：座標映射

Azgaar 座標空間是 100000×100000。需要縮放到 map_viewer 的顯示範圍：

```python
# 歸一化到 -2000 ~ 2000 的範圍（map_viewer 預設視野）
def normalize_coords(burgs, markers):
    all_x = [b['x'] for b in burgs] + [m['x'] for m in markers]
    all_y = [b['y'] for b in burgs] + [m['y'] for m in markers]
    min_x, max_x = min(all_x), max(all_x)
    min_y, max_y = min(all_y), max(all_y)
    scale = 4000 / max(max_x - min_x, max_y - min_y)

    for item in burgs + markers:
        item['viz_x'] = (item['x'] - (min_x + max_x) / 2) * scale
        item['viz_y'] = (item['y'] - (min_y + max_y) / 2) * scale
```

產出 `room_editor_layout.json`：

```json
{
  "world_hunageria_kadur": { "x": 523.4, "y": -102.7 },
  ...
}
```

---

## 產出統計預估

基於 Celia 地圖資料：

| 類型 | 來源 | 預估數量 |
|------|------|----------|
| 城鎮房間 | 623 burgs | ~623 |
| 地標房間 | 71 markers | ~71 |
| 路線連線 | 53 roads + 368 trails + 83 sea | ~800+ exits |
| **總房間數** | | **~694** |

---

## 執行方式

```bash
# Python 版
python3 tools/azgaar_parser.py "Celia Full 2026-01-25-04-11.json" --output data/rooms/editor/

# 產出：
#   data/rooms/editor/world_*.json          （每房間一個檔案）
#   data/runtime/room_editor_layout.json    （座標映射，合併到既有的）
#   tools/azgaar_output_summary.txt         （轉換報告）
```

**腳本放在** `tools/azgaar_parser.py`

---

## 注意事項

1. **不要引入新依賴**。Python 標準庫 + json 模組就夠了，不需要 geopandas 或其他套件
2. **ID 全小寫**，空格轉 `_`，保留非 ASCII 字元（城鎮名可能是非英文）
3. **description 留空**。之後會由碼農根據 tags 批量生成描述
4. **meta 欄位不進 rooms.json 正式 schema**——它是掛在 editor 檔案裡的附加資料，後端載入時忽略未知欄位即可（`#[serde(flatten)]` 或 `deny_unknown_fields` 要注意）
5. **先只處理陸地 burg**。`pack.burgs[].feature` 對應 `pack.features[]`，需確認不是海洋 feature
6. **去重 exits**：同一對 from↔to 只保留一條（優先 roads > trails > searoutes）
7. **跑完後用 map_viewer 目視確認**——開 zone filter 逐國檢查節點位置和連線是否合理
8. **封存現有浮生城**。轉換前先將 `data/rooms/editor/zonelife_*.json` 整批移至 `data/rooms/archive/zonelife/`，不刪除。新生成的 world_* 房間取代現有地圖，不共存

---

## 驗收標準

- [ ] `tools/azgaar_parser.py` 能吃任何 Azgaar Full JSON（不只 Celia）
- [ ] 產出的 world_*.json 每個都能被後端 `serde_json::from_str` 反序列化（除了 meta 欄位）
- [ ] map_viewer 載入後能看到世界地圖，zone filter 有各國名稱
- [ ] 城鎮之間有連線（官道/小徑/海路），箭頭方向正確
- [ ] 地標房間掛在最近的城鎮上，有雙向連線
- [ ] 座標分佈合理，不擠成一團
- [ ] 轉換報告列出：總房間數、各國房間數、孤島（無連線）數量
