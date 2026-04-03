# 021 Watabou 區名英翻繁 — 組長工單

## 問題

Watabou GeoJSON 的 district name 全是英文（"Merchants Town"、"West Docks"、"Castle" 等）。
`src/city/builder.rs` 的 `district_label_semantic()` 遇到非 CJK 名字時，會套 `foreign_district_templates`（如 `"{name}舊廓"`），
結果產出 **"Merchants Town舊廓"**、**"近West Docks"** 這種中英混搭房間名。

## 解法

在 `src/city/parser.rs` 的 district 解析處，加一層詞根翻譯。不是硬對照 17 條，而是拆詞根對照，這樣未來 30 座城市的新名字也能覆蓋。

## 做法

### 1. 在 `parser.rs` 加詞根對照表

在檔案頂部（`use` 區下方）加一個函式：

```rust
/// Watabou district 英文名 → 繁中：先查完整名，再拆詞根逐段翻。
fn translate_district_name(raw: &str) -> String {
    use std::collections::HashMap;

    // 完整名直翻（優先）
    let exact: HashMap<&str, &str> = HashMap::from([
        ("Castle", "城塞"),
        ("Redgate", "赤門"),
        ("Skychurch", "天壇"),
    ]);
    if let Some(&v) = exact.get(raw.trim()) {
        return v.to_string();
    }

    // 詞根表（順序無關，逐一替換）
    let roots: &[(&str, &str)] = &[
        // 地形 / 方位
        ("North", "北"), ("South", "南"), ("East", "東"), ("West", "西"),
        ("Upper", "上"), ("Lower", "下"), ("Great", "大"), ("Old", "舊"),
        ("New", "新"), ("Inner", "內"), ("Outer", "外"),
        // 建築 / 功能
        ("Castle", "城塞"), ("Fort", "堡"), ("Keep", "塔樓"),
        ("Chapel", "禮拜堂"), ("Church", "教堂"), ("Temple", "廟"),
        ("Gate", "門"), ("Tower", "塔"), ("Wall", "城牆"),
        ("Market", "市集"), ("Merchants", "商"), ("Trade", "商"),
        ("Docks", "碼頭"), ("Port", "港"), ("Harbor", "港灣"),
        ("Warehouse", "倉棧"),
        // 地貌
        ("Hill", "丘"), ("Mount", "山"), ("Ridge", "嶺"),
        ("River", "河"), ("Creek", "溪"), ("Lake", "湖"),
        ("Bridge", "橋"), ("Ford", "淺渡"),
        ("Green", "草地"), ("Gardens", "園"), ("Garden", "園"),
        ("Park", "苑"), ("Yard", "院"), ("Square", "廣場"),
        ("Fields", "田"), ("Field", "田"), ("Meadow", "草甸"),
        ("Cross", "十字"), ("Mile", "驛道"), ("Road", "路"),
        ("Village", "村"), ("Town", "坊"), ("City", "城"),
        // 色彩 / 形容
        ("Black", "黑"), ("White", "白"), ("Red", "赤"),
        ("Blue", "青"), ("Grey", "灰"), ("Gray", "灰"),
        ("Gold", "金"), ("Silver", "銀"), ("Iron", "鐵"),
        ("Salt", "鹽"), ("Stone", "石"), ("Bone", "骨"),
        ("Dark", "暗"), ("Pale", "蒼"), ("Bright", "明"),
        ("Dawn", "曉"), ("Dusk", "暮"), ("Shadow", "影"),
        ("Fire", "火"), ("Frost", "霜"), ("Storm", "風"),
        ("Rose", "薔薇"), ("Bloom", "花"), ("Thorn", "荊"),
        // 複合詞尾（Watabou 常見）
        ("rise", "崗"), ("soul", "魂"), ("tide", "潮"),
        ("under", "深"),
    ];

    // 嘗試拆詞翻譯
    let mut result = raw.trim().to_string();
    let mut translated = false;
    for &(eng, chn) in roots {
        if result.contains(eng) {
            result = result.replace(eng, chn);
            translated = true;
        }
    }

    if translated {
        // 清掉殘餘空格
        result = result.replace("  ", "").replace(' ', "");
        result
    } else {
        // 完全無法翻譯，保留原名（builder 會套 foreign_district_templates）
        raw.trim().to_string()
    }
}
```

### 2. 在 district 解析處呼叫

修改 `parser.rs` 第 51-54 行，解析 name 後過一層翻譯：

```rust
// 改前：
let name = g
    .get("name")
    .and_then(|v| v.as_str())
    .map(std::string::ToString::to_string);

// 改後：
let name = g
    .get("name")
    .and_then(|v| v.as_str())
    .map(|s| translate_district_name(s));
```

### 3. 預期產出

以宜林（burg_id=32）的 17 個 district 為例：

| 原名 | 翻譯結果 |
|------|---------|
| Merchants Town | 商坊 |
| Greenfire Yard | 火園草地 → 不好，需特殊處理 |
| Castle | 城塞 |
| West Docks | 西碼頭 |
| Frostsoul Gardens | 霜魂園 |
| Pale Docks | 蒼碼頭 |
| Great Docks | 大碼頭 |
| Skychurch | 天壇 |
| Black Hill | 黑丘 |
| Salt Cross | 鹽十字 |
| Blackrise Green | 黑崗草地 |
| Dawn Mile | 曉驛道 |
| Redgate | 赤門 |
| White Chapel | 白禮拜堂 |
| Downbloom Village | 深花村 → "Down" 需加入詞根 |
| Southern City | 南城 |
| Undertide Town | 深潮坊 |

**注意**：有些組合翻出來會不自然（如「火園草地」）。這是預期的——`builder.rs` 的 `district_label_semantic()` 拿到中文後會走 `has_cjk` 分支（第 588 行），套 `chinese_district_suffixes`（如「一帶」「舊廓」），最終顯示為「**霜魂園一帶**」「**商坊**」這類。不需要完美翻譯，只需要全中文、不出現英文字母。

### 4. 額外詞根補充

跑完 17 個名字後，如果有漏翻的（`result` 裡還含 `[a-zA-Z]`），加到 `exact` 或 `roots` 裡。加入後重新 `cargo build --release` 確認。

某些 Watabou 會生成的常見詞根但本城沒出現的，也建議預先加入：
- "Down" → "深" / "下"
- "Southern" → "南"（注意 "South" 已有，但 "Southern" 要額外處理）
- "Northern" → "北"

### 5. 驗證

```bash
cargo build --release && PORT=1721 ./target/release/singularity_world
```

啟動後 log 會印城市注入結果。確認：
1. 所有 district 房名不含英文字母
2. `cargo clippy -- -D warnings` 零警告

## 不要做的事

- **不要動 `builder.rs`**——翻譯在 parser 層完成，builder 只看到中文
- **不要動 `ambience.rs` 或 `city_ambience.json`**
- **不要引入新 crate**
- **不要改 `CityGeo` 或 `District` 結構**——name 欄位型別不變（`Option<String>`）
