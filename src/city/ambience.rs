// 城市房間環境描述：從 data/config/city_ambience.json 載入可調片段；雙句組合與較佳種子打散重複感。

use std::fs;
use std::path::Path;

use serde::Deserialize;

/// 從 JSON 載入；檔案不存在或解析失敗時回傳內建預設。
#[derive(Debug, Clone, Deserialize)]
pub struct CityAmbienceConfig {
    #[serde(default)]
    pub gate_flat_max_districts: Option<usize>,
    #[serde(default)]
    pub sector_count_cap: Option<usize>,
    #[serde(default)]
    pub districts_per_sector_target: Option<usize>,
    #[serde(default)]
    pub gate_lines: Vec<String>,
    /// 接在城門主句後的第二層細節（聲／光／氣），可與主句池獨立輪替。
    #[serde(default)]
    pub gate_texture_lines: Vec<String>,
    #[serde(default)]
    pub district_default_lines: Vec<String>,
    #[serde(default)]
    pub district_river_lines: Vec<String>,
    #[serde(default)]
    pub district_wall_lines: Vec<String>,
    #[serde(default)]
    pub district_square_lines: Vec<String>,
    #[serde(default)]
    pub district_port_lines: Vec<String>,
    #[serde(default)]
    pub district_texture_lines: Vec<String>,
    #[serde(default)]
    pub building_interior_lines: Vec<String>,
    #[serde(default)]
    pub building_texture_lines: Vec<String>,
    #[serde(default)]
    pub sector_street_lines: Vec<String>,
    #[serde(default)]
    pub sector_texture_lines: Vec<String>,
    /// 外街節點顯示名（出口「往××」用）；長度建議 2～6 字，條數建議 ≥ 扇區數。
    #[serde(default)]
    pub sector_zone_names: Vec<String>,
    /// 非中文 district 名時的稱呼模板，`{name}` 替換為 GeoJSON 原名。
    #[serde(default)]
    pub foreign_district_templates: Vec<String>,
    /// 接在自動推斷的區域核心詞前，如「水湄」「牆陰」。
    #[serde(default)]
    pub district_epithets: Vec<String>,
    /// 中文 GeoJSON 區名後綴，可含空字串。
    #[serde(default)]
    pub chinese_district_suffixes: Vec<String>,
    #[serde(default)]
    pub names_shop: Vec<String>,
    #[serde(default)]
    pub names_tavern: Vec<String>,
    #[serde(default)]
    pub names_temple: Vec<String>,
    #[serde(default)]
    pub names_warehouse: Vec<String>,
    #[serde(default)]
    pub names_residence: Vec<String>,
}

impl Default for CityAmbienceConfig {
    fn default() -> Self {
        Self {
            gate_flat_max_districts: None,
            sector_count_cap: None,
            districts_per_sector_target: None,
            gate_lines: vec![
                "城門下人聲與腳步錯雜，你站在城內外氣息交會之處。".to_string(),
            ],
            gate_texture_lines: vec![
                "風從門洞灌進來，帶著外頭的塵與內城的炊煙，兩邊互不相讓。".to_string(),
            ],
            district_default_lines: vec![
                "街坊輪廓清晰，日常聲息在四周流動。".to_string(),
            ],
            district_river_lines: vec![
                "河畔水氣扑面，搬運與低語混成一片。".to_string(),
            ],
            district_wall_lines: vec![
                "靠牆一帶路窄影深，風聲與內城嘈雜在此變得稀薄。".to_string(),
            ],
            district_square_lines: vec![
                "廣場把人潮與視線一齊撐開，空氣裡塵土與叫賣交錯。".to_string(),
            ],
            district_port_lines: vec![
                "港邊鹹腥與鐵鏈聲交錯，石板潮暗，像半座城繫在水上。".to_string(),
            ],
            district_texture_lines: vec![
                "你站定片刻，就能辨出好幾種節奏疊在一起。".to_string(),
            ],
            building_interior_lines: vec![
                "門內與街上是兩種節奏；此處仍有人進出，有事在發生。".to_string(),
            ],
            building_texture_lines: vec![
                "角落堆著說不清用途的物什，像有人剛離開又會回來。".to_string(),
            ],
            sector_street_lines: vec![
                "外街在此分岔，腳步仍密，卻比城門內收一層。".to_string(),
            ],
            sector_texture_lines: vec![
                "擔子與推車擦肩而過，誰也不想在此久停。".to_string(),
            ],
            sector_zone_names: vec![
                "臨河環".to_string(),
                "舊磚衢".to_string(),
                "深巷口".to_string(),
                "市聲帶".to_string(),
                "牆陰裡".to_string(),
                "港風廊".to_string(),
            ],
            foreign_district_templates: vec![
                "{name}舊廓".to_string(),
                "近{name}".to_string(),
                "{name}北麓".to_string(),
            ],
            district_epithets: vec![
                "水湄".to_string(),
                "牆陰".to_string(),
                "市聲邊".to_string(),
                "斜照裡".to_string(),
                "港風側".to_string(),
            ],
            chinese_district_suffixes: vec![
                String::new(),
                "一帶".to_string(),
                "舊廓".to_string(),
                "北麓".to_string(),
            ],
            names_shop: vec!["雜貨小棧".to_string(), "南北鋪".to_string()],
            names_tavern: vec!["三更燈火".to_string(), "城腳小肆".to_string()],
            names_temple: vec!["小祠".to_string(), "街心壇".to_string()],
            names_warehouse: vec!["堆棧院".to_string(), "河埠倉".to_string()],
            names_residence: vec!["深院".to_string(), "連簷屋".to_string()],
        }
    }
}

/// 讀取設定檔；失敗則 Default。
pub fn load_city_ambience(path: Option<&Path>) -> CityAmbienceConfig {
    let Some(p) = path else {
        return CityAmbienceConfig::default();
    };
    let raw = match fs::read_to_string(p) {
        Ok(s) => s,
        Err(_) => return CityAmbienceConfig::default(),
    };
    serde_json::from_str::<CityAmbienceConfig>(&raw).unwrap_or_default()
}

/// 打散索引，避免 di 連號總撞到同一句（builder 命名池也可用）。
pub fn mix_seed_index(seed: u64, salt: u32, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let x = seed
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(u64::from(salt));
    let x = x ^ (x >> 27).wrapping_mul(0x85EB_CA6B);
    (x as usize) % len
}

fn pick_line(lines: &[String], seed: u64, salt: u32) -> String {
    if lines.is_empty() {
        return String::new();
    }
    lines[mix_seed_index(seed, salt, lines.len())].trim().to_string()
}

fn compose_two(primary: &[String], texture: &[String], seed: u64, salt_base: u32) -> String {
    let a = pick_line(primary, seed, salt_base);
    if texture.is_empty() {
        return a;
    }
    let b = pick_line(texture, seed, salt_base.wrapping_add(7919));
    if b.is_empty() || a == b {
        return a;
    }
    format!("{} {}", a, b)
}

impl CityAmbienceConfig {
    pub fn gate_flat_max(&self) -> usize {
        self.gate_flat_max_districts.unwrap_or(6).max(3)
    }

    pub fn sector_cap(&self) -> usize {
        self.sector_count_cap.unwrap_or(6).clamp(2, 12)
    }

    pub fn districts_per_sector(&self) -> usize {
        self.districts_per_sector_target.unwrap_or(4).max(2)
    }

    pub fn pick_gate(&self, seed: u64) -> String {
        let fb = CityAmbienceConfig::default();
        let p = if self.gate_lines.is_empty() {
            &fb.gate_lines[..]
        } else {
            &self.gate_lines[..]
        };
        let t = if self.gate_texture_lines.is_empty() {
            &fb.gate_texture_lines[..]
        } else {
            &self.gate_texture_lines[..]
        };
        compose_two(p, t, seed, 1)
    }

    pub fn pick_district(
        &self,
        seed: u64,
        near_river: bool,
        near_wall: bool,
        near_square: bool,
        is_port_city: bool,
    ) -> String {
        let fb = CityAmbienceConfig::default();
        let primary: &[String] = if is_port_city && !self.district_port_lines.is_empty() {
            &self.district_port_lines
        } else if near_river && !self.district_river_lines.is_empty() {
            &self.district_river_lines
        } else if near_square && !self.district_square_lines.is_empty() {
            &self.district_square_lines
        } else if near_wall && !self.district_wall_lines.is_empty() {
            &self.district_wall_lines
        } else if !self.district_default_lines.is_empty() {
            &self.district_default_lines
        } else {
            &fb.district_default_lines
        };
        let tex = if self.district_texture_lines.is_empty() {
            &fb.district_texture_lines[..]
        } else {
            &self.district_texture_lines[..]
        };
        compose_two(primary, tex, seed, 3)
    }

    pub fn pick_sector_street(&self, seed: u64) -> String {
        let fb = CityAmbienceConfig::default();
        let p = if self.sector_street_lines.is_empty() {
            &fb.sector_street_lines[..]
        } else {
            &self.sector_street_lines[..]
        };
        let t = if self.sector_texture_lines.is_empty() {
            &fb.sector_texture_lines[..]
        } else {
            &self.sector_texture_lines[..]
        };
        compose_two(p, t, seed, 5)
    }

    pub fn pick_building_interior(&self, seed: u64) -> String {
        let fb = CityAmbienceConfig::default();
        let p = if self.building_interior_lines.is_empty() {
            &fb.building_interior_lines[..]
        } else {
            &self.building_interior_lines[..]
        };
        let t = if self.building_texture_lines.is_empty() {
            &fb.building_texture_lines[..]
        } else {
            &self.building_texture_lines[..]
        };
        compose_two(p, t, seed, 7)
    }

    /// 外街節點短名；`si` 為扇區序、`burg_id` 一併攪動避免多城重疊。
    pub fn sector_short_name(&self, si: usize, burg_id: u32) -> String {
        let fb = CityAmbienceConfig::default();
        let names = if self.sector_zone_names.is_empty() {
            &fb.sector_zone_names[..]
        } else {
            &self.sector_zone_names[..]
        };
        if names.is_empty() {
            return format!("外街{}", si + 1);
        }
        let seed = u64::from(burg_id)
            .wrapping_mul(0x1F)
            .wrapping_add(si as u64 * 0x27);
        pick_line(names, seed, si as u32)
    }

    /// 非中文 district 名時的顯示用短稱（不含城市前綴）。
    pub fn pick_district_epithet(&self, seed: u64) -> String {
        let fb = CityAmbienceConfig::default();
        let p = if self.district_epithets.is_empty() {
            &fb.district_epithets[..]
        } else {
            &self.district_epithets[..]
        };
        pick_line(p, seed, 19)
    }

    pub fn format_chinese_district_raw(&self, raw: &str, seed: u64) -> String {
        let fb = CityAmbienceConfig::default();
        let suf = if self.chinese_district_suffixes.is_empty() {
            &fb.chinese_district_suffixes[..]
        } else {
            &self.chinese_district_suffixes[..]
        };
        let s = pick_line(suf, seed, 23);
        format!("{}{}", raw.trim(), s)
    }

    pub fn format_district_room_display(&self, city: &str, label: &str, di: usize, burg: u32) -> String {
        let city = city.trim();
        let seed = u64::from(burg)
            .wrapping_mul(0x51)
            .wrapping_add(di as u64 * 0x3D);
        match mix_seed_index(seed, 29, 4) {
            0 => format!("{}「{}」", city, label),
            1 => format!("{}（{}）", label, city),
            2 => format!("{}·{}", city, label),
            _ => format!("{} {}", city, label),
        }
    }

    pub fn format_sector_room_display(
        &self,
        city: &str,
        si: usize,
        burg_id: u32,
        short: &str,
    ) -> String {
        let city = city.trim();
        let seed = u64::from(burg_id)
            .wrapping_mul(0x61)
            .wrapping_add(si as u64 * 0x41);
        match mix_seed_index(seed, 31, 3) {
            0 => format!("{}·外街·{}", city, short),
            1 => format!("{} {}", short, city),
            _ => format!("{}（外街·{}）", city, short),
        }
    }

    pub fn foreign_district_label(&self, raw_name: &str, seed: u64) -> String {
        let fb = CityAmbienceConfig::default();
        let tpls = if self.foreign_district_templates.is_empty() {
            &fb.foreign_district_templates[..]
        } else {
            &self.foreign_district_templates[..]
        };
        if tpls.is_empty() {
            return raw_name.trim().to_string();
        }
        let t = pick_line(tpls, seed, 11);
        t.replace("{name}", raw_name.trim())
    }

    /// 可進建築顯示名（取代「街坊舖子·n」公式）；`kind`：shop / tavern / temple / warehouse / residence。
    pub fn building_place_name(&self, kind: &str, seed: u64) -> String {
        let fb = CityAmbienceConfig::default();
        let pool: &[String] = match kind {
            "shop" => {
                if self.names_shop.is_empty() {
                    &fb.names_shop[..]
                } else {
                    &self.names_shop[..]
                }
            }
            "tavern" => {
                if self.names_tavern.is_empty() {
                    &fb.names_tavern[..]
                } else {
                    &self.names_tavern[..]
                }
            }
            "temple" => {
                if self.names_temple.is_empty() {
                    &fb.names_temple[..]
                } else {
                    &self.names_temple[..]
                }
            }
            "warehouse" => {
                if self.names_warehouse.is_empty() {
                    &fb.names_warehouse[..]
                } else {
                    &self.names_warehouse[..]
                }
            }
            _ => {
                if self.names_residence.is_empty() {
                    &fb.names_residence[..]
                } else {
                    &self.names_residence[..]
                }
            }
        };
        if pool.is_empty() {
            return kind.to_string();
        }
        let base = pick_line(pool, seed, 13);
        // 同種多棟時加極短區分，避免 id 撞名
        let suffix = (mix_seed_index(seed, 17, 97) + 1) % 100;
        format!("{}·{}", base, suffix)
    }
}
