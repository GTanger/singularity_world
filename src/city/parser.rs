// 解析 Watabou / MFCG 匯出的 GeoJSON FeatureCollection，抽出道路、建築、區域等。

use std::fs;
use std::path::Path;

use anyhow::Context;
use serde_json::Value;

use super::{Building, CityGeo, District, Road};

/// Watabou district 英文名 → 繁中：完整名優先，再依空白／連字號分詞，每詞內長詞根優先、不分大小寫替換。
fn translate_district_name(raw: &str) -> String {
    use std::collections::HashMap;

    let trimmed = raw.trim();
    let exact: HashMap<&str, &str> = HashMap::from([
        ("Castle", "城塞"),
        ("Redgate", "赤門"),
        ("Skychurch", "天壇"),
        ("Greenfire", "青火"),
    ]);
    if let Some((_, &v)) = exact
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(trimmed))
    {
        return v.to_string();
    }

    // 長詞根須在前，避免 Southern→南ern、Merchants 被 Merchant 類短詞截斷等。
    let roots: &[(&str, &str)] = &[
        ("Merchants", "商"),
        ("Warehouse", "倉棧"),
        ("Southern", "南"),
        ("Northern", "北"),
        ("Eastern", "東"),
        ("Western", "西"),
        ("Chapel", "禮拜堂"),
        ("Gardens", "園"),
        ("Village", "村"),
        ("Meadow", "草甸"),
        ("Harbor", "港灣"),
        ("Market", "市集"),
        ("Bridge", "橋"),
        ("Square", "廣場"),
        ("Garden", "園"),
        ("Shadow", "影"),
        ("Bright", "明"),
        ("Fields", "田"),
        ("Church", "教堂"),
        ("Temple", "廟"),
        ("Castle", "城塞"),
        ("Frostsoul", "霜魂"),
        ("Undertide", "深潮"),
        ("Downbloom", "深花"),
        ("Blackrise", "黑崗"),
        ("Greenfire", "青火"),
        ("North", "北"),
        ("South", "南"),
        ("East", "東"),
        ("West", "西"),
        ("Upper", "上"),
        ("Lower", "下"),
        ("Great", "大"),
        ("Inner", "內"),
        ("Outer", "外"),
        ("Old", "舊"),
        ("New", "新"),
        ("Fort", "堡"),
        ("Keep", "塔樓"),
        ("Trade", "商"),
        ("Docks", "碼頭"),
        ("Stone", "石"),
        ("Bloom", "花"),
        ("Thorn", "荊"),
        ("Tower", "塔"),
        ("Mount", "山"),
        ("Ridge", "嶺"),
        ("River", "河"),
        ("Creek", "溪"),
        ("Storm", "風"),
        ("Field", "田"),
        ("Cross", "十字"),
        ("Mile", "驛道"),
        ("Rose", "薔薇"),
        ("Frost", "霜"),
        ("Silver", "銀"),
        ("Pale", "蒼"),
        ("Dawn", "曉"),
        ("Dusk", "暮"),
        ("Gate", "門"),
        ("Wall", "城牆"),
        ("Port", "港"),
        ("Hill", "丘"),
        ("Lake", "湖"),
        ("Ford", "淺渡"),
        ("Green", "草地"),
        ("Black", "黑"),
        ("White", "白"),
        ("Blue", "青"),
        ("Grey", "灰"),
        ("Gray", "灰"),
        ("Gold", "金"),
        ("Iron", "鐵"),
        ("Salt", "鹽"),
        ("Bone", "骨"),
        ("Dark", "暗"),
        ("Fire", "火"),
        ("Red", "赤"),
        ("Yard", "院"),
        ("Park", "苑"),
        ("Road", "路"),
        ("Town", "坊"),
        ("City", "城"),
        ("Down", "深"),
        ("Soul", "魂"),
        ("Tide", "潮"),
        ("Under", "深"),
        ("Rise", "崗"),
        ("Sky", "天"),
    ];

    let mut parts: Vec<String> = Vec::new();
    for token in trimmed.split(|c: char| c.is_whitespace() || c == '-') {
        if token.is_empty() {
            continue;
        }
        if let Some((_, &v)) = exact.iter().find(|(k, _)| k.eq_ignore_ascii_case(token)) {
            parts.push(v.to_string());
            continue;
        }
        parts.push(translate_district_token(token, roots));
    }
    parts.concat()
}

/// 單一英文詞內反覆套用詞根（每次替換一處），直到無可替換。
fn translate_district_token(word: &str, roots: &[(&str, &str)]) -> String {
    let mut result = word.to_string();
    let mut guard = 0usize;
    while guard < 256 {
        guard += 1;
        let mut replaced = false;
        for &(eng, chn) in roots {
            if let Some(next) = replace_first_ci(&result, eng, chn) {
                result = next;
                replaced = true;
                break;
            }
        }
        if !replaced {
            break;
        }
    }
    result.retain(|c| !c.is_whitespace());
    result
}

/// 找第一處不分大小寫與 `needle` 相同的子位元組區段，替換為 `replacement`（僅 ASCII 詞根，Watabou 區名皆 ASCII）。
fn replace_first_ci(haystack: &str, needle: &str, replacement: &str) -> Option<String> {
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if n.is_empty() || h.len() < n.len() {
        return None;
    }
    for i in 0..=h.len() - n.len() {
        if range_eq_ci(&h[i..i + n.len()], n) {
            let mut s = String::with_capacity(haystack.len() + replacement.len());
            s.push_str(&haystack[..i]);
            s.push_str(replacement);
            s.push_str(&haystack[i + n.len()..]);
            return Some(s);
        }
    }
    None
}

/// 兩段 ASCII 位元組是否長度相同且不分大小寫逐字相等。
fn range_eq_ci(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.eq_ignore_ascii_case(y))
}

/// 讀取 GeoJSON 檔，解析為 CityGeo。吃任意 burg_id（由檔名決定）。
pub fn parse_city_geojson(path: &Path, burg_id: u32) -> anyhow::Result<CityGeo> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("讀取城市檔 {:?}", path))?;
    let root: Value = serde_json::from_str(&raw).context("GeoJSON 非合法 JSON")?;
    let features = root
        .get("features")
        .and_then(|v| v.as_array())
        .context("缺少 features 陣列")?;

    let mut roads: Vec<Road> = Vec::new();
    let mut buildings: Vec<Building> = Vec::new();
    let mut districts: Vec<District> = Vec::new();
    let mut wall_rings: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut river_lines: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut squares: Vec<Vec<(f64, f64)>> = Vec::new();

    for feat in features {
        let id = feat.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let ty = feat.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match (ty, id) {
            ("GeometryCollection", "roads") => {
                if let Some(arr) = feat.get("geometries").and_then(|v| v.as_array()) {
                    for g in arr {
                        if g.get("type").and_then(|v| v.as_str()) == Some("LineString")
                            && let Some(coords) = g.get("coordinates").and_then(linestring_coords)
                        {
                            roads.push(Road { coords });
                        }
                    }
                }
            }
            ("MultiPolygon", "buildings") => {
                push_multipolygon_rings(feat, &mut buildings);
            }
            ("GeometryCollection", "districts") => {
                if let Some(arr) = feat.get("geometries").and_then(|v| v.as_array()) {
                    for g in arr {
                        if g.get("type").and_then(|v| v.as_str()) == Some("Polygon") {
                            let name = g
                                .get("name")
                                .and_then(|v| v.as_str())
                                .map(translate_district_name);
                            if let Some(ring) = g.get("coordinates").and_then(outer_ring_polygon) {
                                districts.push(District { name, ring });
                            }
                        }
                    }
                }
            }
            ("GeometryCollection", "walls") => {
                collect_wall_geometries(feat, &mut wall_rings);
            }
            ("GeometryCollection", "rivers") => {
                if let Some(arr) = feat.get("geometries").and_then(|v| v.as_array()) {
                    for g in arr {
                        if g.get("type").and_then(|v| v.as_str()) == Some("LineString")
                            && let Some(coords) = g.get("coordinates").and_then(linestring_coords)
                        {
                            river_lines.push(coords);
                        }
                    }
                }
            }
            ("MultiPolygon", "squares") => {
                if let Some(coords) = feat.get("coordinates").and_then(|v| v.as_array()) {
                    for poly in coords {
                        if let Some(pa) = poly.as_array()
                            && let Some(fr) = pa.first()
                            && let Some(ring) = ring_coords(fr)
                        {
                            squares.push(ring);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(CityGeo {
        burg_id,
        roads,
        buildings,
        districts,
        wall_rings,
        river_lines,
        squares,
    })
}

fn linestring_coords(v: &Value) -> Option<Vec<(f64, f64)>> {
    let arr = v.as_array()?;
    let mut out = Vec::with_capacity(arr.len());
    for p in arr {
        let pair = p.as_array()?;
        let x = pair.first().and_then(|n| n.as_f64())?;
        let y = pair.get(1).and_then(|n| n.as_f64())?;
        out.push((x, y));
    }
    if out.len() >= 2 {
        Some(out)
    } else {
        None
    }
}

fn ring_coords(v: &Value) -> Option<Vec<(f64, f64)>> {
    let arr = v.as_array()?;
    let mut out = Vec::with_capacity(arr.len());
    for p in arr {
        let pair = p.as_array()?;
        let x = pair.first().and_then(|n| n.as_f64())?;
        let y = pair.get(1).and_then(|n| n.as_f64())?;
        out.push((x, y));
    }
    if out.len() >= 3 {
        Some(out)
    } else {
        None
    }
}

fn outer_ring_polygon(v: &Value) -> Option<Vec<(f64, f64)>> {
    let coords = v.as_array()?;
    coords.first().and_then(ring_coords)
}

fn push_multipolygon_rings(feat: &Value, buildings: &mut Vec<Building>) {
    let Some(coords) = feat.get("coordinates").and_then(|v| v.as_array()) else {
        return;
    };
    for poly in coords {
        let Some(pa) = poly.as_array() else {
            continue;
        };
        let Some(fr) = pa.first() else {
            continue;
        };
        let Some(ring) = ring_coords(fr) else {
            continue;
        };
        buildings.push(Building { ring });
    }
}

fn collect_wall_geometries(feat: &Value, wall_rings: &mut Vec<Vec<(f64, f64)>>) {
    let Some(arr) = feat.get("geometries").and_then(|v| v.as_array()) else {
        return;
    };
    for g in arr {
        match g.get("type").and_then(|v| v.as_str()) {
            Some("Polygon") => {
                if let Some(ring) = g.get("coordinates").and_then(outer_ring_polygon) {
                    wall_rings.push(ring);
                }
            }
            Some("MultiPolygon") => {
                if let Some(coords) = g.get("coordinates").and_then(|v| v.as_array()) {
                    for poly in coords {
                        if let Some(pa) = poly.as_array()
                            && let Some(fr) = pa.first()
                            && let Some(ring) = ring_coords(fr)
                        {
                            wall_rings.push(ring);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod translate_tests {
    use super::translate_district_name;

    /// 021 工單宜林 17 區範例：產出須全中文、無殘留英文字母。
    #[test]
    fn watabou_district_samples_no_ascii_letters() {
        let cases = [
            ("Merchants Town", "商坊"),
            ("Castle", "城塞"),
            ("West Docks", "西碼頭"),
            ("Frostsoul Gardens", "霜魂園"),
            ("Pale Docks", "蒼碼頭"),
            ("Great Docks", "大碼頭"),
            ("Skychurch", "天壇"),
            ("Black Hill", "黑丘"),
            ("Salt Cross", "鹽十字"),
            ("Blackrise Green", "黑崗草地"),
            ("Dawn Mile", "曉驛道"),
            ("Redgate", "赤門"),
            ("White Chapel", "白禮拜堂"),
            ("Downbloom Village", "深花村"),
            ("Southern City", "南城"),
            ("Undertide Town", "深潮坊"),
            ("Greenfire Yard", "青火院"),
        ];
        for (raw, want) in cases {
            let got = translate_district_name(raw);
            assert_eq!(got.as_str(), want, "raw={raw:?}");
            assert!(
                !got.bytes().any(|b| b.is_ascii_alphabetic()),
                "殘留英文: raw={raw:?} got={got:?}"
            );
        }
    }
}
