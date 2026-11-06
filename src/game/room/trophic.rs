/// 記錄已被玩家「查探獸跡／尋跡」而破霧顯形的鎮場霸主座標 (x, y) 與顯形時間戳。
static TRACKED_APEX_COORDS: LazyLock<RwLock<HashSet<(i32, i32)>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));

/// 標記某處獵場座標之鎮場霸主已被尋跡顯形。
pub fn mark_apex_beast_tracked_at(x: i32, y: i32) {
    TRACKED_APEX_COORDS.write().insert((x, y));
}

/// 檢查某處獵場座標之鎮場霸主是否已因尋跡而破霧顯形。
pub fn is_apex_beast_tracked_at(x: i32, y: i32) -> bool {
    TRACKED_APEX_COORDS.read().contains(&(x, y))
}

#[derive(Debug, Clone)]
pub struct RecentHuntKillSite {
    pub prey_name: String,
    pub loot_id: String,
    pub loot_name: String,
    pub hunter_label: String,
    pub killed_at: i64,
}

pub static SHANHAI_SLOT_RESPAWN_GEN: LazyLock<RwLock<HashMap<(usize, usize), u64>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub static RECENT_HUNT_KILL_SITES: LazyLock<RwLock<HashMap<(i32, i32), RecentHuntKillSite>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, Clone)]
pub struct ZoneTrophicState {
    pub z_idx: usize,
    pub zone_name: &'static str,
    pub detritus_s: f32,
    pub flora_p: f32,
    pub n_tier4: f32,
    pub k_tier4: f32,
    pub n_tier3: f32,
    pub k_tier3: f32,
    pub n_tier2: f32,
    pub k_tier2: f32,
    pub apex_presence: f32,
    pub timber_w: f32,
    pub mineral_m: f32,
    pub fishery_f: f32,
    pub resource_fallow_mode: bool,
    pub last_step_tick: i64,
    pub recent_trophic_event: String,
}

pub static LIVE_ZONE_TROPHIC_WEB: LazyLock<RwLock<Vec<ZoneTrophicState>>> =
    LazyLock::new(|| {
        let mut states = Vec::with_capacity(CANONICAL_HUNTING_ZONES.len());
        let now_tick = crate::game::now_unix() / 180;
        for (z_idx, &(zone_name, _, _)) in CANONICAL_HUNTING_ZONES.iter().enumerate() {
            let entries = &ZONE_HABITAT_LAYOUTS[z_idx];
            let k_tier2 = 6.0_f32;
            let k_tier3 = entries.iter().filter(|e| e.3 == 3).count() as f32;
            let k_tier4 = entries.iter().filter(|e| e.3 == 4).count() as f32;
            states.push(ZoneTrophicState {
                z_idx,
                zone_name,
                detritus_s: 100.0,
                flora_p: 1000.0,
                n_tier4: k_tier4,
                k_tier4,
                n_tier3: k_tier3,
                k_tier3,
                n_tier2: 6.0,
                k_tier2,
                apex_presence: 1.0,
                timber_w: 1000.0,
                mineral_m: 1000.0,
                fishery_f: 1000.0,
                resource_fallow_mode: false,
                last_step_tick: now_tick,
                recent_trophic_event: "水草豐茂，林間鹿群與山禽自在覓食，偶見猛獸伏行其間，萬物生息如常。".to_string(),
            });
        }
        RwLock::new(states)
    });

pub fn nearest_hunting_zone_idx(x: i32, y: i32) -> usize {
    CANONICAL_HUNTING_ZONES
        .iter()
        .enumerate()
        .min_by_key(|&(_, &(_, cx, cy))| {
            let dx = (x - cx) as i64;
            let dy = (y - cy) as i64;
            dx * dx + dy * dy
        })
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

pub fn nearest_geographic_zones(z_idx: usize) -> [usize; 2] {
    let num_zones = CANONICAL_HUNTING_ZONES.len();
    if z_idx >= num_zones {
        return [0, 1];
    }
    let (_, cx, cy) = CANONICAL_HUNTING_ZONES[z_idx];
    let mut candidates: Vec<(i64, usize)> = CANONICAL_HUNTING_ZONES
        .iter()
        .enumerate()
        .filter(|&(idx, _)| idx != z_idx)
        .map(|(idx, &(_, ocx, ocy))| {
            let dx = (cx - ocx) as i64;
            let dy = (cy - ocy) as i64;
            (dx * dx + dy * dy, idx)
        })
        .collect();
    candidates.sort_by_key(|&(d, idx)| (d, idx));
    [candidates[0].1, candidates[1].1]
}

pub fn is_coastal_hunting_zone(z_idx: usize) -> bool {
    matches!(z_idx, 3 | 4 | 5 | 6 | 9 | 16)
}

pub fn is_alpine_hunting_zone(z_idx: usize) -> bool {
    matches!(z_idx, 14 | 17 | 18)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneWeatherPhase {
    RainMist,
    ClearBreeze,
    DryWarm,
    ColdFrost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneTidePhase {
    FloodTide,
    EbbTide,
    SlackTide,
}

pub fn current_zone_weather_and_tide(z_idx: usize, tick: i64) -> (ZoneWeatherPhase, ZoneTidePhase) {
    let base_tick = tick.wrapping_add(z_idx as i64);
    let weather_code = base_tick.rem_euclid(4);
    let deterministic_weather = match weather_code {
        0 => ZoneWeatherPhase::RainMist,
        1 => ZoneWeatherPhase::ClearBreeze,
        2 => ZoneWeatherPhase::DryWarm,
        _ => ZoneWeatherPhase::ColdFrost,
    };
    let weather = if let Ok(Some(climate)) = crate::db::get_region_climate("bai_ta_peninsula") {
        match climate.event_type.as_str() {
            "rain" | "plum_rain" => ZoneWeatherPhase::RainMist,
            "snow" => ZoneWeatherPhase::ColdFrost,
            "wind" => ZoneWeatherPhase::ClearBreeze,
            _ => deterministic_weather,
        }
    } else {
        deterministic_weather
    };

    let tide_code = base_tick.rem_euclid(3);
    let tide = match tide_code {
        0 => ZoneTidePhase::FloodTide,
        1 => ZoneTidePhase::EbbTide,
        _ => ZoneTidePhase::SlackTide,
    };
    (weather, tide)
}

pub fn get_zone_trophic_state(z_idx: usize) -> Option<ZoneTrophicState> {
    LIVE_ZONE_TROPHIC_WEB.read().get(z_idx).cloned()
}

pub fn is_canonical_apex_beast_name(name: &str) -> bool {
    matches!(
        name,
        "長風四目水鹿王"
            | "古河樟林雲豹"
            | "赤石鯪鯉穿山甲"
            | "落日黑鳶王"
            | "驚濤遊隼王"
            | "淺灣百年綠蠵龜"
            | "黑石成雙海鱟"
            | "千年樟雲豹王"
            | "清泉櫻花鉤吻鮭"
            | "胸白月熊王"
            | "西麓丹冠藍腹鷴"
            | "淺澤重角水牛王"
            | "黑沼百步蛇王"
            | "寒潭千年花斑鱸鰻"
            | "青籠大冠鷲王"
            | "裂石鯪鯉穿山甲王"
            | "馭風角鷹王"
            | "霧林黑長尾帝雉"
            | "雪巔霜冠大水鹿"
    )
}

pub fn is_canonical_tier2_predator_name(name: &str) -> bool {
    matches!(
        name,
        "淺山錢紋石虎"
            | "錢紋石虎"
            | "海岬百年綠蠵龜"
            | "裂浪灰海獺"
            | "渡海黑面琵鷺"
            | "黑背玄武岩龜"
            | "溪澗食蟹獴"
            | "雲杉霧林帝雉"
            | "長尾山娘"
            | "沼澤食蟹獴"
            | "黑沼飯匙倩"
            | "高山苦花魚王"
            | "關隘錢紋石虎"
            | "礦谷胸白月熊"
            | "破浪遊隼"
            | "雪線胸白月熊"
            | "極頂黑長尾帝雉"
    )
}

pub fn record_shanhai_slot_hunted(
    z_idx: usize,
    slot_idx: usize,
    coord: (i32, i32),
    prey_name: &str,
    loot_id: &str,
    loot_name: &str,
    hunter_label: &str,
    now_unix: i64,
) {
    {
        let mut gen_lock = SHANHAI_SLOT_RESPAWN_GEN.write();
        let entry = gen_lock.entry((z_idx, slot_idx)).or_insert(0);
        *entry = entry.wrapping_add(1);
    }

    {
        let mut sites_lock = RECENT_HUNT_KILL_SITES.write();
        sites_lock.insert(
            coord,
            RecentHuntKillSite {
                prey_name: prey_name.to_string(),
                loot_id: loot_id.to_string(),
                loot_name: loot_name.to_string(),
                hunter_label: hunter_label.to_string(),
                killed_at: now_unix,
            },
        );
    }

    if slot_idx == 0 {
        TRACKED_APEX_COORDS.write().remove(&coord);
    }

    let tier_opt = ZONE_HABITAT_LAYOUTS
        .get(z_idx)
        .and_then(|v| v.get(slot_idx))
        .map(|e| e.3);

    let mut web = LIVE_ZONE_TROPHIC_WEB.write();
    if let Some(st) = web.get_mut(z_idx) {
        if slot_idx == 0 {
            st.apex_presence = 0.0;
        } else {
            match tier_opt {
                Some(2) => {
                    st.n_tier2 = (st.n_tier2 - 1.0).max(1.0);
                }
                Some(3) => {
                    st.n_tier3 = (st.n_tier3 - 1.0).max(2.0);
                }
                Some(4) => {
                    st.n_tier4 = (st.n_tier4 - 1.0).max(4.0);
                }
                _ => match slot_idx {
                    1..=6 => st.n_tier2 = (st.n_tier2 - 1.0).max(1.0),
                    7..=26 => st.n_tier3 = (st.n_tier3 - 1.0).max(2.0),
                    _ => st.n_tier4 = (st.n_tier4 - 1.0).max(4.0),
                },
            }
        }
        st.detritus_s = (st.detritus_s + 10.0).clamp(0.0, 200.0);
        st.recent_trophic_event = format!(
            "草木間尚留有獵捕「{}」後的餘溫血氣，山林鳥獸聞風四散，泥土漸次吸納殘餘生機。",
            prey_name
        );
    }
}

pub fn record_wilderness_kill_at_coord(
    x: i32,
    y: i32,
    target_id: &str,
    prey_name: &str,
    loot_id: &str,
    loot_name: &str,
    hunter_label: &str,
    now_unix: i64,
) {
    if let Some(rest) = target_id.strip_prefix("shanhai_") {
        let parts: Vec<&str> = rest.split('_').collect();
        if parts.len() >= 2
            && let (Ok(z_idx), Ok(slot_idx)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>())
        {
            record_shanhai_slot_hunted(
                z_idx,
                slot_idx,
                (x, y),
                prey_name,
                loot_id,
                loot_name,
                hunter_label,
                now_unix,
            );
            return;
        }
    }

    let z_idx = nearest_hunting_zone_idx(x, y);
    let is_apex = target_id.starts_with("apex_") || is_canonical_apex_beast_name(prey_name);
    if is_apex {
        TRACKED_APEX_COORDS.write().remove(&(x, y));
    }

    {
        let mut sites_lock = RECENT_HUNT_KILL_SITES.write();
        sites_lock.insert(
            (x, y),
            RecentHuntKillSite {
                prey_name: prey_name.to_string(),
                loot_id: loot_id.to_string(),
                loot_name: loot_name.to_string(),
                hunter_label: hunter_label.to_string(),
                killed_at: now_unix,
            },
        );
    }

    let mut web = LIVE_ZONE_TROPHIC_WEB.write();
    if let Some(st) = web.get_mut(z_idx) {
        if is_apex {
            st.apex_presence = 0.0;
        } else if is_canonical_tier2_predator_name(prey_name) {
            st.n_tier2 = (st.n_tier2 - 1.0).max(1.0);
        } else if target_id.contains("flock") {
            st.n_tier4 = (st.n_tier4 - 1.0).max(4.0);
        } else {
            st.n_tier3 = (st.n_tier3 - 1.0).max(2.0);
        }
        st.detritus_s = (st.detritus_s + 10.0).clamp(0.0, 200.0);
        st.recent_trophic_event = format!(
            "草木間尚留有獵捕「{}」後的餘溫血氣，山林鳥獸聞風四散，泥土漸次吸納殘餘生機。",
            prey_name
        );
    }
}

fn sweep_expired_kill_sites(now_unix: i64) {
    let expired: Vec<((i32, i32), RecentHuntKillSite)> = {
        let mut sites_lock = RECENT_HUNT_KILL_SITES.write();
        let mut expired_keys = Vec::new();
        for (&coord, site) in sites_lock.iter() {
            if now_unix.saturating_sub(site.killed_at) > 180 {
                expired_keys.push(coord);
            }
        }
        let mut list = Vec::with_capacity(expired_keys.len());
        for coord in expired_keys {
            if let Some(site) = sites_lock.remove(&coord) {
                list.push((coord, site));
            }
        }
        list
    };

    if expired.is_empty() {
        return;
    }

    let mut web = LIVE_ZONE_TROPHIC_WEB.write();
    for (coord, site) in expired {
        let z_idx = nearest_hunting_zone_idx(coord.0, coord.1);
        if let Some(st) = web.get_mut(z_idx) {
            st.detritus_s = (st.detritus_s + 18.0).clamp(0.0, 200.0);
            st.n_tier2 = (st.n_tier2 + 0.25).clamp(1.0, st.k_tier2 * 1.50);
            st.recent_trophic_event = format!(
                "林間「{}」的殘骸已為山間鳥獸分食殆盡，餘骨沒入厚實腐葉與潮濕泥土，悄然滋養周遭草木。",
                site.prey_name
            );
        }
    }
}

fn step_zone_equations_internal(
    st: &mut ZoneTrophicState,
    n4_left: f32,
    n3_left: f32,
    n2_left: f32,
    n4_right: f32,
    n3_right: f32,
    n2_right: f32,
    next_tick: i64,
) {
    let (weather, tide) = current_zone_weather_and_tide(st.z_idx, next_tick);

    // 1. Stage 0 (Detritus S_z <-> Flora P_z)
    let litter = 0.04 * st.n_tier4 + 0.12 * st.n_tier3 + 0.20 * st.n_tier2;
    let mut uptake = 4.5 * (st.detritus_s / 100.0) * (1.0 - st.flora_p / 1200.0).max(0.0);
    let mut leaching = 0.0_f32;
    if weather == ZoneWeatherPhase::RainMist {
        uptake *= 1.35;
        leaching = 2.5;
    }
    st.detritus_s = (st.detritus_s + litter + leaching - uptake).clamp(20.0, 200.0);

    let mut flora_growth = 0.16 * (0.5 + 0.5 * (st.detritus_s / 100.0)) * st.flora_p * (1.0 - st.flora_p / 1000.0);
    if weather == ZoneWeatherPhase::ColdFrost && is_alpine_hunting_zone(st.z_idx) {
        flora_growth *= 0.65;
    }
    let grazing = 0.06 * st.n_tier4 + 0.20 * st.n_tier3;
    st.flora_p = (st.flora_p + flora_growth - grazing).clamp(80.0, 1200.0);

    // 2. Stage 1 & 2 (Herbivore Growth + Holling Type III Predation with Refuge)
    let phi_p = (st.flora_p / 1000.0).clamp(0.30, 1.25);
    let mut k4_eff = (st.k_tier4 * phi_p).max(8.0);
    if is_coastal_hunting_zone(st.z_idx) && tide == ZoneTidePhase::EbbTide {
        k4_eff *= 1.20;
    }
    let k3_eff = (st.k_tier3 * phi_p).max(4.0);
    let ref4 = (st.k_tier4 * 0.18).max(6.0);
    let ref3 = (st.k_tier3 * 0.18).max(3.0);
    let ref2 = 1.0_f32;

    let exp4 = (st.n_tier4 - ref4).max(0.0);
    let exp3 = (st.n_tier3 - ref3).max(0.0);

    let h4 = (st.k_tier4 * 0.35).max(4.0);
    let h3 = (st.k_tier3 * 0.35).max(2.5);

    let mut f24 = (1.35 * exp4 * exp4) / (h4 * h4 + exp4 * exp4);
    let mut f23 = (0.75 * exp3 * exp3) / (h3 * h3 + exp3 * exp3);
    if is_coastal_hunting_zone(st.z_idx) && tide == ZoneTidePhase::FloodTide {
        f24 *= 1.12;
        f23 *= 1.12;
    }

    let pred_on_4 = st.n_tier2 * f24;
    let pred_on_3 = st.n_tier2 * f23 + (if st.apex_presence >= 0.95 { 0.35 } else { 0.0 });

    // 3. Stage 3 (Apex Turf Suppression & Trophic Cascade)
    let r4 = 0.22 * st.n_tier4 * (1.0 - st.n_tier4 / k4_eff);
    let r3 = 0.18 * st.n_tier3 * (1.0 - st.n_tier3 / k3_eff);

    let apex_supp = if st.apex_presence >= 0.95 {
        0.12 * st.n_tier2 + 0.25 * (st.n_tier2 - st.k_tier2).max(0.0)
    } else {
        0.04 * (st.n_tier2 - st.k_tier2 * 1.5).max(0.0)
    };

    let d_n2 = (0.14 * pred_on_4 + 0.26 * pred_on_3) - 0.10 * st.n_tier2 - apex_supp;

    // Carrion return from natural predation
    st.detritus_s = (st.detritus_s + (pred_on_4 * 0.8 + pred_on_3 * 1.8)).clamp(20.0, 200.0);

    // Neighbor migration (+4% density diffusion: 2% from each neighbor)
    let mig_4 = 0.02 * (n4_left - st.n_tier4) + 0.02 * (n4_right - st.n_tier4);
    let mig_3 = 0.02 * (n3_left - st.n_tier3) + 0.02 * (n3_right - st.n_tier3);
    let mig_2 = 0.02 * (n2_left - st.n_tier2) + 0.02 * (n2_right - st.n_tier2);

    // Altitudinal herbivore migration under ColdFrost
    let downhill_outflow = if weather == ZoneWeatherPhase::ColdFrost && is_alpine_hunting_zone(st.z_idx) {
        0.04 * (st.n_tier3 - ref3).max(0.0)
    } else {
        0.0
    };
    let downhill_influx = if weather == ZoneWeatherPhase::ColdFrost && matches!(st.z_idx, 0 | 7 | 8 | 10 | 15) {
        0.03 * st.k_tier3 * 0.1
    } else {
        0.0
    };

    st.n_tier4 = (st.n_tier4 + r4 - pred_on_4 + mig_4).clamp(ref4, st.k_tier4 * 1.25);
    st.n_tier3 = (st.n_tier3 + r3 - pred_on_3 + mig_3 - downhill_outflow + downhill_influx).clamp(ref3, st.k_tier3 * 1.25);
    st.n_tier2 = (st.n_tier2 + d_n2 + mig_2).clamp(ref2, st.k_tier2 * 1.50);

    if st.apex_presence < 1.0 {
        let recovery_boost = ((st.n_tier2 / st.k_tier2 + st.n_tier3 / st.k_tier3) * 0.5).clamp(0.4, 1.3);
        let coastal_apex_add = if is_coastal_hunting_zone(st.z_idx) && tide == ZoneTidePhase::FloodTide {
            0.04
        } else {
            0.0
        };
        st.apex_presence = (st.apex_presence + 0.16 * recovery_boost + coastal_apex_add).min(1.0);
    }

    // 4. Gordon-Schaefer / Seepage Renewable Resource Pools
    let rain_mult = if weather == ZoneWeatherPhase::RainMist { 1.30 } else { 1.0 };
    let tide_mult = if tide == ZoneTidePhase::FloodTide { 1.25 } else { 1.0 };
    let d_w = 0.08 * rain_mult * (0.6 + 0.4 * (st.detritus_s / 100.0)) * st.timber_w * (1.0 - st.timber_w / 1000.0) + 4.0;
    let d_m = 0.04 * st.mineral_m * (1.0 - st.mineral_m / 1000.0) + 8.0 * rain_mult;
    let d_f = 0.12 * tide_mult * (0.5 + 0.5 * (st.n_tier4 / st.k_tier4.max(1.0))) * st.fishery_f * (1.0 - st.fishery_f / 1000.0) + 5.0;

    st.timber_w = (st.timber_w + d_w).clamp(100.0, 1000.0);
    st.mineral_m = (st.mineral_m + d_m).clamp(100.0, 1000.0);
    st.fishery_f = (st.fishery_f + d_f).clamp(100.0, 1000.0);

    if st.timber_w < 350.0 || st.mineral_m < 350.0 || st.fishery_f < 350.0 {
        st.resource_fallow_mode = true;
    } else if st.timber_w >= 500.0 && st.mineral_m >= 500.0 && st.fishery_f >= 500.0 {
        st.resource_fallow_mode = false;
    }

    st.last_step_tick = next_tick;

    if st.apex_presence < 0.95 {
        st.recent_trophic_event =
            "山中久未見巨獸蹤影，石虎與猛禽失卻忌憚而四處出沒，林間鹿群受驚頻頻奔逃。".to_string();
    } else if st.resource_fallow_mode {
        if is_coastal_hunting_zone(st.z_idx) || matches!(st.zone_name, "淺澤泥口" | "白茫洲" | "貼山深潭") {
            st.recent_trophic_event = "近海潮間帶與水澤休養生息，漁人與採藻戶自發休漁禁採，魚群與藻床漸復生機。".to_string();
        } else if matches!(st.zone_name, "赤石坡" | "西嶺礦谷" | "東海峭壁") {
            st.recent_trophic_event = "山嶺岩脈休養生息，礦工與石匠自發封坑緩採，地脈與溪澗漸復生機。".to_string();
        } else {
            st.recent_trophic_event = "山林地脈休養生息，獵戶與樵夫自發封山禁伐，林木與溪澗漸復生機。".to_string();
        }
    } else if weather == ZoneWeatherPhase::RainMist {
        st.recent_trophic_event =
            "煙雨空濛，山林間腐木菌斑吸飽甘霖，溪澗水氣瀰漫，禽鳥群聚於樹冠避雨啼鳴。".to_string();
    } else if weather == ZoneWeatherPhase::ColdFrost {
        st.recent_trophic_event =
            "寒霜覆嶺，高山草木凝結冰晶，群鹿踏雪下遷覓食，山間一片冷冽靜謐。".to_string();
    } else if is_coastal_hunting_zone(st.z_idx) && tide == ZoneTidePhase::FloodTide {
        st.recent_trophic_event =
            "海潮怒湧，浪沫拍擊礁石灘岸，巡游魚群隨潮湧入淺灣，岸邊水鳥爭相低掠捕食。".to_string();
    } else if is_coastal_hunting_zone(st.z_idx) && tide == ZoneTidePhase::EbbTide {
        st.recent_trophic_event =
            "潮水退去，泥灘與礁縫間露出豐美海藻貝介，灘禽紛紛自林緣飛落啄食。".to_string();
    } else if st.detritus_s >= 125.0 {
        st.recent_trophic_event =
            "林間腐葉與朽木氣息醇厚，潮濕樹根處悄然萌發出赤紅牛樟靈菇，草木亦格外茂盛。".to_string();
    } else if st.flora_p < 450.0 {
        st.recent_trophic_event =
            "山坡草木近期多被採食，嫩葉漸稀，鹿群與山羊正結伴往鄰近水草豐美處遷徙。".to_string();
    } else {
        st.recent_trophic_event =
            "水草豐茂，林間鹿群與山禽自在棲息，山間猛獸偶爾伏行巡獵，萬物生息如常。".to_string();
    }
}

pub fn step_zone_trophic_web_once(z_idx: usize) {
    sweep_expired_kill_sites(crate::game::now_unix());
    let mut web = LIVE_ZONE_TROPHIC_WEB.write();
    let num_zones = web.len();
    if z_idx >= num_zones {
        return;
    }
    let [left_idx, right_idx] = nearest_geographic_zones(z_idx);

    let (n4_left, n3_left, n2_left) = {
        let l = &web[left_idx];
        (l.n_tier4, l.n_tier3, l.n_tier2)
    };
    let (n4_right, n3_right, n2_right) = {
        let r = &web[right_idx];
        (r.n_tier4, r.n_tier3, r.n_tier2)
    };
    let next_tick = web[z_idx].last_step_tick + 1;
    step_zone_equations_internal(
        &mut web[z_idx],
        n4_left,
        n3_left,
        n2_left,
        n4_right,
        n3_right,
        n2_right,
        next_tick,
    );
}

pub fn step_all_zones_trophic_web_if_needed(time_tick: i64) {
    let tick = if time_tick > 1_000_000 {
        time_tick / 180
    } else {
        time_tick
    };
    {
        let web = LIVE_ZONE_TROPHIC_WEB.read();
        if !web.iter().any(|st| tick > st.last_step_tick) {
            return;
        }
    }

    sweep_expired_kill_sites(crate::game::now_unix());

    let num_zones = CANONICAL_HUNTING_ZONES.len();
    let mut web = LIVE_ZONE_TROPHIC_WEB.write();
    if !web.iter().any(|st| tick > st.last_step_tick) {
        return;
    }
    let n4_snap: Vec<f32> = web.iter().map(|s| s.n_tier4).collect();
    let n3_snap: Vec<f32> = web.iter().map(|s| s.n_tier3).collect();
    let n2_snap: Vec<f32> = web.iter().map(|s| s.n_tier2).collect();

    for z_idx in 0..num_zones {
        if tick <= web[z_idx].last_step_tick {
            continue;
        }
        let [left_idx, right_idx] = nearest_geographic_zones(z_idx);

        step_zone_equations_internal(
            &mut web[z_idx],
            n4_snap[left_idx],
            n3_snap[left_idx],
            n2_snap[left_idx],
            n4_snap[right_idx],
            n3_snap[right_idx],
            n2_snap[right_idx],
            tick,
        );
    }
}

pub fn resolve_zone_idx_from_name_or_settlement(settlement_or_zone: &str) -> usize {
    if let Some(idx) = CANONICAL_HUNTING_ZONES
        .iter()
        .position(|(zn, _, _)| settlement_or_zone.contains(zn))
    {
        idx
    } else if settlement_or_zone.contains("風舶") {
        nearest_hunting_zone_idx(-260, 160)
    } else if settlement_or_zone.contains("文曜") {
        nearest_hunting_zone_idx(0, 0)
    } else if settlement_or_zone.contains("水尾") {
        nearest_hunting_zone_idx(-75, -280)
    } else if settlement_or_zone.contains("半嶺") {
        nearest_hunting_zone_idx(160, 210)
    } else if settlement_or_zone.contains("紅土") {
        nearest_hunting_zone_idx(-85, -140)
    } else if settlement_or_zone.contains("南哨") {
        nearest_hunting_zone_idx(0, -220)
    } else if settlement_or_zone.contains("霜林") {
        nearest_hunting_zone_idx(190, 290)
    } else {
        0
    }
}

pub fn harvest_hunting_zone_flora(settlement_or_zone: &str, units: f32) -> f32 {
    let z_idx = resolve_zone_idx_from_name_or_settlement(settlement_or_zone);

    let mut web = LIVE_ZONE_TROPHIC_WEB.write();
    if let Some(st) = web.get_mut(z_idx) {
        let harvestable = (st.flora_p - 80.0).max(0.0);
        let actual = units.min(harvestable);
        st.flora_p -= actual;
        actual
    } else {
        0.0
    }
}

pub fn harvest_zone_natural_resource(
    settlement_or_zone: &str,
    resource_kind: &str,
    units: f32,
) -> (f32, bool, bool) {
    let z_idx = resolve_zone_idx_from_name_or_settlement(settlement_or_zone);
    let mut web = LIVE_ZONE_TROPHIC_WEB.write();
    let Some(st) = web.get_mut(z_idx) else {
        return (0.0, false, false);
    };

    if st.resource_fallow_mode {
        match resource_kind {
            "timber" => st.timber_w = (st.timber_w + 6.0).min(1000.0),
            "mineral" => st.mineral_m = (st.mineral_m + 6.0).min(1000.0),
            "fishery" => st.fishery_f = (st.fishery_f + 6.0).min(1000.0),
            _ => st.flora_p = (st.flora_p + 6.0).min(1200.0),
        }
        if st.timber_w >= 500.0 && st.mineral_m >= 500.0 && st.fishery_f >= 500.0 {
            st.resource_fallow_mode = false;
        }
        return (0.0, false, true);
    }

    let req = units.clamp(0.5, 4.0);
    let (pool_val, is_flora) = match resource_kind {
        "timber" => (&mut st.timber_w, false),
        "mineral" => (&mut st.mineral_m, false),
        "fishery" => (&mut st.fishery_f, false),
        _ => (&mut st.flora_p, true),
    };

    let pool_before = *pool_val;
    let floor = if is_flora { 80.0 } else { 120.0 };
    let harvestable = (pool_before - floor).max(0.0);
    let actual = req.min(harvestable);
    *pool_val -= actual;

    if st.timber_w < 350.0 || st.mineral_m < 350.0 || st.fishery_f < 350.0 {
        st.resource_fallow_mode = true;
    }
    let is_abundant = pool_before >= 750.0;
    (actual, is_abundant, false)
}

pub fn is_zone_predator_spillover_active(z_idx: usize) -> bool {
    let Some(st) = get_zone_trophic_state(z_idx) else {
        return false;
    };
    let apex_present = st.apex_presence >= 0.95;
    !apex_present && (st.n_tier2 > st.k_tier2 * 1.08 || st.n_tier3 < st.k_tier3 * 0.55)
}

pub fn is_zone_in_fallow_stewardship(settlement_or_zone: &str) -> Option<&'static str> {
    let z_idx = resolve_zone_idx_from_name_or_settlement(settlement_or_zone);
    let st = get_zone_trophic_state(z_idx)?;
    if !st.resource_fallow_mode {
        return None;
    }
    let zone_name = CANONICAL_HUNTING_ZONES.get(z_idx).map(|z| z.0).unwrap_or(settlement_or_zone);
    if matches!(zone_name, "驚濤岩岸" | "巨藻淺灣" | "黑石灣" | "落日岬" | "淺澤泥口" | "白茫洲" | "貼山深潭") {
        Some("漁場")
    } else if matches!(zone_name, "赤石坡" | "西嶺礦谷" | "東海峭壁") {
        Some("礦脈")
    } else if st.timber_w <= st.mineral_m && st.timber_w <= st.fishery_f {
        Some("林木")
    } else if st.mineral_m <= st.fishery_f {
        Some("礦脈")
    } else {
        Some("漁場")
    }
}

pub fn harvest_zone_detritus_fungi(x: i32, y: i32) -> Option<(&'static str, &'static str)> {
    let z_idx = nearest_hunting_zone_idx(x, y);
    let time_bucket = (crate::game::now_unix() / 180) as i64;
    let (weather, _) = current_zone_weather_and_tide(z_idx, time_bucket);
    let fungi_threshold = if weather == ZoneWeatherPhase::RainMist {
        105.0
    } else {
        120.0
    };
    let mut web = LIVE_ZONE_TROPHIC_WEB.write();
    let st = web.get_mut(z_idx)?;
    if st.detritus_s >= fungi_threshold {
        st.detritus_s = (st.detritus_s - 15.0).max(20.0);
        st.recent_trophic_event =
            "林間朽木上的牛樟靈菇已被採藥人摘取，周遭濕潤腐土重歸幽靜。".to_string();
        Some(("wt_cat_0064", "深山牛樟芝"))
    } else {
        None
    }
}

pub fn dynamic_hunting_zone_ecology_omen(zone_or_room: &str) -> Option<String> {
    let z_idx = CANONICAL_HUNTING_ZONES
        .iter()
        .position(|(zn, _, _)| zone_or_room.contains(zn))?;

    let base_omen = hunting_zone_apex_omen(zone_or_room)
        .unwrap_or("風過林野，天地生靈各按其時巡行於萬物之間。");
    let st = get_zone_trophic_state(z_idx)?;

    if st.apex_presence < 0.95 {
        Some(format!(
            "{}深處久未見巨獸蹤影，林間石虎與猛禽失卻忌憚而四處出沒，鹿群與山羊受驚頻頻奔逃。",
            st.zone_name
        ))
    } else {
        Some(format!("{}{}", base_omen, st.recent_trophic_event))
    }
}

pub fn peek_recent_hunt_kill_site(x: i32, y: i32, now_unix: i64) -> Option<RecentHuntKillSite> {
    let sites_lock = RECENT_HUNT_KILL_SITES.read();
    if let Some(site) = sites_lock.get(&(x, y)) {
        if now_unix.saturating_sub(site.killed_at) <= 180 {
            return Some(site.clone());
        }
    }
    None
}

pub fn take_recent_hunt_kill_site(x: i32, y: i32, now_unix: i64) -> Option<RecentHuntKillSite> {
    let mut sites_lock = RECENT_HUNT_KILL_SITES.write();
    if let Some(site) = sites_lock.get(&(x, y)) {
        if now_unix.saturating_sub(site.killed_at) <= 180 {
            return sites_lock.remove(&(x, y));
        } else {
            sites_lock.remove(&(x, y));
        }
    }
    None
}
