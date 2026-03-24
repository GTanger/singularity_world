// 地形顯示對照：候選字與顏色，對齊 Go world/terrain_display.go。

use rand::Rng;
use std::collections::HashMap;
use std::sync::LazyLock;

/// 單一地形的顯示用資料（名稱、候選字、對應顏色同索引）。
#[derive(Debug, Clone, Copy)]
pub struct TerrainMeta {
    pub name: &'static str,
    pub chars: &'static [&'static str],
    pub colors: &'static [&'static str],
}

static TERRAIN_METAS: LazyLock<HashMap<&'static str, TerrainMeta>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert(
        "木",
        TerrainMeta {
            name: "樹木",
            chars: &["木", "林", "森"],
            colors: &["#a0d080", "#6aa050", "#2a5020"],
        },
    );
    m.insert(
        "山",
        TerrainMeta {
            name: "山脈",
            chars: &["山", "岳", "巒"],
            colors: &["#98af9d", "#4a7c59", "#1f3a30"],
        },
    );
    m.insert(
        "石",
        TerrainMeta {
            name: "碎石",
            chars: &["石", "磊", "岩"],
            colors: &["#d3d3d3", "#8c8c8c", "#4f4f4f"],
        },
    );
    m.insert(
        "沼",
        TerrainMeta {
            name: "沼澤",
            chars: &["沼", "澤", "泥"],
            colors: &["#507050", "#608050", "#608060"],
        },
    );
    m.insert(
        "川",
        TerrainMeta {
            name: "河流",
            chars: &["ㄑ", "巜", "巛"],
            colors: &["#60a0d0", "#50b0e0", "#70c0f0"],
        },
    );
    m.insert(
        "水",
        TerrainMeta {
            name: "水域",
            chars: &["水", "沝", "淼"],
            colors: &["#b0d8f0", "#a0c8e0", "#c0e8ff"],
        },
    );
    m.insert(
        "草",
        TerrainMeta {
            name: "草原",
            chars: &["艸", "芔", "茻"],
            colors: &["#c2d6a4", "#8fb361", "#4d7338"],
        },
    );
    m.insert(
        "荒",
        TerrainMeta {
            name: "荒地",
            chars: &["荒", "旱", "焦"],
            colors: &["#e3d5ca", "#d4a373", "#a98467"],
        },
    );
    m.insert(
        "道",
        TerrainMeta {
            name: "道路",
            chars: &["道", "路", "徑"],
            colors: &["#c4b8a8", "#c4b8a8", "#c4b8a8"],
        },
    );
    m.insert(
        "巷",
        TerrainMeta {
            name: "巷",
            chars: &["巷"],
            colors: &["#c4b8a8"],
        },
    );
    m.insert(
        "火",
        TerrainMeta {
            name: "岩漿",
            chars: &["炎", "焱", "燚"],
            colors: &["#faa307", "#f48c06", "#dc2f02"],
        },
    );
    m.insert(
        "冰",
        TerrainMeta {
            name: "寒冰",
            chars: &["冰", "凍", "冽"],
            colors: &["#e0f2f1", "#b2ebf2", "#80deea"],
        },
    );
    m.insert(
        "田",
        TerrainMeta {
            name: "農地",
            chars: &["田", "畓", "畕"],
            colors: &["#9ef01a", "#70e000", "#38b000"],
        },
    );
    m.insert(
        "谷",
        TerrainMeta {
            name: "深谷",
            chars: &["谷", "豀", "豁"],
            colors: &["#3d405b", "#22223b", "#0d1b2a"],
        },
    );
    m.insert(
        "霧",
        TerrainMeta {
            name: "迷霧",
            chars: &["霧", "靄", "霙"],
            colors: &["#c8c4b8", "#b0aca0", "#989488"],
        },
    );
    m.insert(
        "牆",
        TerrainMeta {
            name: "牆",
            chars: &["牆"],
            colors: &["#8c8c8c"],
        },
    );
    m.insert(
        "門",
        TerrainMeta {
            name: "門",
            chars: &["門"],
            colors: &["#8b7355"],
        },
    );
    m.insert(
        "關",
        TerrainMeta {
            name: "關",
            chars: &["關"],
            colors: &["#6b5344"],
        },
    );
    m.insert(
        "地",
        TerrainMeta {
            name: "地",
            chars: &["地"],
            colors: &["#c4b8a8"],
        },
    );
    m
});

fn pick_char_color(meta: TerrainMeta, t_fallback: &str, idx: usize) -> (String, String) {
    if meta.chars.is_empty() {
        return (t_fallback.to_string(), "#888888".to_string());
    }
    let mut idx = idx;
    if idx >= meta.colors.len() {
        idx = meta.colors.len().saturating_sub(1);
    }
    (
        meta.chars[idx].to_string(),
        meta.colors[idx].to_string(),
    )
}

/// 固定取第一個候選字／色（對齊 Go `Display(t, nil)`）。
pub fn display_terrain(t: &str) -> (String, String) {
    let Some(meta) = TERRAIN_METAS.get(t).copied() else {
        return (t.to_string(), "#888888".to_string());
    };
    pick_char_color(meta, t, 0)
}

/// 多候選時亂數取一（對齊 Go `Display(t, r)`、`r != nil`）。
pub fn display_terrain_rng(t: &str, rng: &mut impl Rng) -> (String, String) {
    let Some(meta) = TERRAIN_METAS.get(t).copied() else {
        return (t.to_string(), "#888888".to_string());
    };
    let idx = if meta.chars.len() > 1 {
        rng.random_range(0..meta.chars.len())
    } else {
        0
    };
    pick_char_color(meta, t, idx)
}

/// 指定地形之顯示表項目；無則 `None`（對齊 Go `TerrainMetaByType`）。
pub fn terrain_meta_by_type(t: &str) -> Option<TerrainMeta> {
    TERRAIN_METAS.get(t).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn unknown_terrain_default_gray() {
        let (c, col) = display_terrain("未知");
        assert_eq!(c, "未知");
        assert_eq!(col, "#888888");
    }

    #[test]
    fn display_no_rng_first_variant() {
        let (c, col) = display_terrain("木");
        assert_eq!(c, "木");
        assert_eq!(col, "#a0d080");
    }

    #[test]
    fn display_seeded_deterministic() {
        let mut r = StdRng::seed_from_u64(42);
        let (c1, _) = display_terrain_rng("草", &mut r);
        let mut r = StdRng::seed_from_u64(42);
        let (c2, _) = display_terrain_rng("草", &mut r);
        assert_eq!(c1, c2);
    }

    #[test]
    fn terrain_meta_by_type_road() {
        let m = terrain_meta_by_type("道").expect("道");
        assert_eq!(m.name, "道路");
        assert_eq!(m.chars.len(), 3);
    }
}
