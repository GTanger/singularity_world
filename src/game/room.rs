use crate::db::{self, object_has_socket};
use crate::entity::{Character, EntityKind};
use crate::grid::{Direction, SquareCoord, grid_room_id_from_coord, terrain_name_zh};
use crate::model::{Exit, Room, RoomObject};
use crate::store;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

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
/// 當前房間視野。
#[derive(Debug, Clone)]
pub struct RoomView {
    pub room: Room,
    pub exits: Vec<Exit>,
    pub entities: Vec<Character>,
    pub objects: Vec<RoomObject>,
}

/// 過濾視野中不合適顯示的實體：
/// 1. 氣血歸零（無生命）
/// 2. 測試殘留（含底線命名或測試標記）
/// 3. 超過 180 秒無活動觀測的離線殭屍角色
pub fn filter_room_entities(entities: Vec<Character>) -> Vec<Character> {
    let now = crate::game::now_unix();
    entities
        .into_iter()
        .filter(|e| {
            if e.vit <= 0 {
                return false;
            }
            if e.display_char.contains('_') {
                return false;
            }
            match e.kind {
                crate::entity::EntityKind::Player => {
                    if e.id.contains('_') {
                        return false;
                    }
                    let last_active = e.last_observed_at.unwrap_or(e.created_at);
                    if last_active > 0 && now.saturating_sub(last_active) > 180 {
                        return false;
                    }
                }
                crate::entity::EntityKind::Npc => {
                    let lower_id = e.id.to_ascii_lowercase();
                    if lower_id.starts_with("test")
                        || lower_id.contains("test_")
                        || lower_id.contains("_test")
                        || lower_id.starts_with("stub")
                        || lower_id.starts_with("dummy")
                    {
                        return false;
                    }
                }
            }
            true
        })
        .collect()
}

/// 依房間 id 載入視野；`game_hour` 0–23 供 NPC 職稱，`-1` 表示不套用收工休歇規則。
pub fn get_room_view(room_id: &str, game_hour: i32) -> anyhow::Result<Option<RoomView>> {
    let Some(room) = db::get_room(room_id)? else {
        return Ok(None);
    };
    let exits = db::get_exits_for_room(room_id)?;
    let entities = filter_room_entities(db::get_entities_in_room(room_id, game_hour)?);
    let objects = db::get_objects_in_room(room_id)?;
    Ok(Some(RoomView {
        room,
        exits,
        entities,
        objects,
    }))
}

/// 以正式 `world_places` / `world_routes` 組裝玩家當前所見。
pub fn get_place_room_view(place_id: &str, game_hour: i32) -> anyhow::Result<Option<RoomView>> {
    let Some(store) = store::get_store() else {
        return Ok(None);
    };
    let (place, exits) = {
        let guard = store.read();
        let Some(place) = guard.get_place(place_id).cloned() else {
            return Ok(None);
        };
        if !place.traversable || place.status != "active" {
            return Ok(None);
        }
        let graph = guard.place_graph_snapshot();
        let exits = graph
            .routes_from(place_id)
            .iter()
            .filter(|route| route.status == "active")
            .filter_map(|route| {
                let target = graph.place(&route.to_place_id)?;
                Some(Exit {
                    direction: route.direction.clone(),
                    to_room_id: target.id.clone(),
                    to_room_name: target.name.clone(),
                    route_id: route.id.clone(),
                    mode: route.mode.clone(),
                    travel_cost: route.travel_cost,
                })
            })
            .collect::<Vec<_>>();
        (place, exits)
    };
    let mut objects = Vec::new();
    if let Some(pool) = crate::pg::pool()
        && let Some(source) = crate::pg::world_resource::get_at_place(&pool, place_id)?
    {
        let sockets = vec!["Look".to_string(), "Gather".to_string()];
        let state = if source.current_stock > 0 {
            format!("目前仍可取得約{}份。", source.current_stock)
        } else {
            "眼前能取用的部分已空，必須等它慢慢恢復。".to_string()
        };
        objects.push(RoomObject {
            id: format!("resource:{}:{}", source.place_id, source.item_id),
            name: format!("可取用的{}", source.name),
            owner: String::new(),
            sockets,
            responses: std::collections::HashMap::from([(
                "Look".to_string(),
                format!("{}；{}", source.origin_label, state),
            )]),
            move_to_room_id: String::new(),
        });
    }
    let entities = filter_room_entities(db::get_entities_in_room(place_id, game_hour)?);
    Ok(Some(RoomView {
        room: Room {
            id: place.id,
            name: place.name,
            tags: place.tags,
            zone: place.zone,
            description: place.description,
            objects: objects.clone(),
        },
        exits,
        entities,
        objects,
    }))
}

/// 若實體尚無房間則設為預設房間，並回傳房間 id。
pub fn ensure_entity_in_room(entity_id: &str, default_room_id: &str) -> anyhow::Result<String> {
    let mut room_id = db::get_entity_room(entity_id)?;
    if room_id.is_empty() {
        db::set_entity_room(entity_id, default_room_id)?;
        room_id = default_room_id.to_string();
    }
    Ok(room_id)
}

/// 依出口方向移動；回傳 `(新房間 id, 是否成功)`。
pub fn move_by_exit(entity_id: &str, direction: &str) -> anyhow::Result<(String, bool)> {
    let dir = direction.trim();
    if dir.is_empty() {
        return Ok((String::new(), false));
    }
    let room_id = db::get_entity_room(entity_id)?;
    if room_id.is_empty() {
        return Ok((String::new(), false));
    }
    let exits = db::get_exits_for_room(&room_id)?;
    for ex in &exits {
        if ex.direction.trim() == dir {
            db::set_entity_room(entity_id, &ex.to_room_id)?;
            return Ok((ex.to_room_id.clone(), true));
        }
    }
    let objs = db::get_objects_in_room(&room_id)?;
    for o in objs {
        if o.name.trim() != dir {
            continue;
        }
        if o.move_to_room_id.is_empty() || !object_has_socket(&o, "Move") {
            continue;
        }
        db::set_entity_room(entity_id, &o.move_to_room_id)?;
        return Ok((o.move_to_room_id.clone(), true));
    }
    Ok((String::new(), false))
}

// ─── 正方格地圖 ───────────────────────────────────────────────────

fn grid_objects_to_room_objects(objs: &[crate::grid::GridObject]) -> Vec<RoomObject> {
    objs.iter()
        .map(|o| RoomObject {
            id: o.id.clone(),
            name: o.name.clone(),
            owner: String::new(),
            sockets: if o.quantity > 0 {
                vec!["Look".into(), "Gather".into()]
            } else {
                vec!["Look".into()]
            },
            responses: Default::default(),
            move_to_room_id: String::new(),
        })
        .collect()
}

/// 依方位字在正方格上走一步。成功時已寫入 DB。
pub fn move_by_grid_direction(entity_id: &str, direction: &str) -> anyhow::Result<(String, bool)> {
    move_by_place_route(entity_id, direction, None)
}

/// 沿確切相鄰場所移動；未指定目標時才取該方向排序後的第一條路。
pub fn move_by_place_route(
    entity_id: &str,
    direction: &str,
    target_place_id: Option<&str>,
) -> anyhow::Result<(String, bool)> {
    let dir = direction.trim();
    if !matches!(dir, "東" | "西" | "南" | "北" | "上" | "下" | "進" | "出") {
        return Ok((String::new(), false));
    }
    let arc = store::get_store().ok_or_else(|| anyhow::anyhow!("no store"))?;
    let current_room = arc.read().get_entity_room(entity_id);
    let first_move =
        if let Some(target) = target_place_id.filter(|target| !target.trim().is_empty()) {
            let movement = arc
                .write()
                .move_entity_to_adjacent_place(entity_id, target.trim())?;
            if let Some(ref movement) = movement {
                anyhow::ensure!(movement.route.direction == dir, "目標場所與方向不相符");
            }
            movement
        } else {
            arc.write().move_entity_direction(entity_id, dir)?
        };
    if let Some(place_move) = first_move {
        record_command_move(entity_id, &place_move);
        let new_room = {
            let s = arc.read();
            let tz = s
                .get_place(&place_move.to_place_id)
                .map(|p| p.zone.clone())
                .unwrap_or_else(|| "wt-c-wenyao".to_string());
            let _ = s.step_npc_grid_coord(
                entity_id,
                &place_move.route.direction,
                &tz,
                super::now_unix() as u64,
            );
            s.get_entity_room(entity_id)
        };
        return Ok((new_room, true));
    }
    if let Some(gdir) = Direction::from_zh(dir)
        && reveal_missing_grid_route(entity_id, gdir)?
    {
        let second_move = { arc.write().move_entity_direction(entity_id, dir)? };
        if let Some(place_move) = second_move {
            record_command_move(entity_id, &place_move);
            let new_room = {
                let s = arc.read();
                let tz = s
                    .get_place(&place_move.to_place_id)
                    .map(|p| p.zone.clone())
                    .unwrap_or_else(|| "wt-c-wenyao".to_string());
                let _ = s.step_npc_grid_coord(
                    entity_id,
                    &place_move.route.direction,
                    &tz,
                    super::now_unix() as u64,
                );
                s.get_entity_room(entity_id)
            };
            return Ok((new_room, true));
        }
    }
    Ok((current_room, false))
}

fn record_command_move(entity_id: &str, movement: &store::PlaceMove) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let from_label = crate::db::get_room_name(&movement.from_place_id)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "原處".to_string());
    let to_label = crate::db::get_room_name(&movement.to_place_id)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "前方地界".to_string());
    db::set_entity_journey(crate::world::journey::EntityJourney {
        entity_id: entity_id.to_string(),
        reason: "player_command".to_string(),
        destination_place_id: Some(movement.to_place_id.clone()),
        planned_path: Vec::new(),
        consequence: format!(
            "自「{}」向{}行至「{}」",
            from_label,
            movement.route.direction,
            to_label
        ),
        updated_at: now,
    });
}


fn reveal_missing_grid_route(entity_id: &str, direction: Direction) -> anyhow::Result<bool> {
    let Some(store) = store::get_store() else {
        return Ok(false);
    };
    let Some(entity) = store.read().get_entity(entity_id) else {
        return Ok(false);
    };
    let (Some(x), Some(y)) = (entity.grid_x, entity.grid_y) else {
        return Ok(false);
    };
    let from = SquareCoord::new(x, y);
    let changed = crate::server::grid_manager::with_square_grid_mut(|grid| {
        let Some(cell) = grid.get(from) else {
            return false;
        };
        if !cell.explicit_exits.is_empty() {
            return false;
        }
        let target = from.neighbor(direction);
        let before = grid.len();
        crate::grid::reveal_grid_disk(grid, target, 2);
        grid.can_walk(from, target) && grid.len() > before
    })
    .unwrap_or(false);
    if changed {
        let grid = crate::server::grid_manager::get_runtime_square_grid()
            .ok_or_else(|| anyhow::anyhow!("grid not loaded"))?;
        store.write().sync_places_from_grid(&grid)?;
    }
    Ok(changed)
}

/// 組裝正方格 RoomView（當前格的實體、出口、物件）。
pub fn get_grid_room_view(
    _player_id: &str,
    x: i32,
    y: i32,
    game_hour: i32,
) -> anyhow::Result<Option<RoomView>> {
    let grid_opt = crate::server::grid_manager::get_runtime_square_grid();
    let coord = SquareCoord::new(x, y);
    let cell = if let Some(grid) = &grid_opt {
        let Some(c) = grid.get(coord) else {
            return Ok(None);
        };
        c.clone()
    } else {
        let (zone, sub_geo, terrain) = crate::grid::r#gen::classify_wilderness_cell(x, y);
        let mut c = crate::grid::GridCell::new(coord, terrain, format!("{zone}・{sub_geo}"));
        c.zone = zone.to_string();
        c.description = format!("{zone}的{sub_geo}地帶");
        c
    };
    let cell_objects = if cell.objects.is_empty() && !is_inside_settlement_urban_core(x, y) {
        crate::grid::r#gen::generate_cell_objects(coord, cell.terrain)
    } else {
        cell.objects.clone()
    };
    let mut room_objs = grid_objects_to_room_objects(&cell_objects);
    room_objs.extend(super::player_scene::player_layer_scene_objects_for_room(
        &cell.name,
    ));
    let mut room = Room {
        id: grid_room_id_from_coord(x, y),
        name: if cell.name.is_empty() {
            crate::grid::terrain_name_zh(cell.terrain).to_string()
        } else {
            cell.name.clone()
        },
        tags: cell.tags.clone(),
        zone: cell.zone.clone(),
        description: cell.description.clone(),
        objects: room_objs.clone(),
    };
    let mut exits: Vec<Exit> = Vec::new();
    if let Some(grid) = &grid_opt {
        if !cell.explicit_exits.is_empty() {
            for ex in &cell.explicit_exits {
                let target_name = grid
                    .get(ex.target)
                    .map(|tc| {
                        if tc.name.is_empty() {
                            crate::grid::terrain_name_zh(tc.terrain).to_string()
                        } else {
                            tc.name.clone()
                        }
                    })
                    .unwrap_or_else(|| "▒▒▒".to_string());
                exits.push(Exit {
                    direction: ex.dir.label_zh().to_string(),
                    to_room_id: grid_room_id_from_coord(ex.target.x, ex.target.y),
                    to_room_name: target_name,
                    route_id: String::new(),
                    mode: String::new(),
                    travel_cost: 0,
                });
            }
        } else {
            for n_coord in coord.neighbors() {
                let Some(nc) = grid.get(n_coord) else {
                    continue;
                };
                if !grid.can_walk(coord, n_coord) {
                    continue;
                }
                let Some(dir) = coord.direction_to(n_coord) else {
                    continue;
                };
                let target_name = if nc.name.is_empty() {
                    crate::grid::terrain_name_zh(nc.terrain).to_string()
                } else {
                    nc.name.clone()
                };
                exits.push(Exit {
                    direction: dir.label_zh().to_string(),
                    to_room_id: grid_room_id_from_coord(n_coord.x, n_coord.y),
                    to_room_name: target_name,
                    route_id: String::new(),
                    mode: String::new(),
                    travel_cost: 0,
                });
            }
        }
    }
    let mut entities = db::get_entities_at_grid(x, y, game_hour)
        .map(filter_room_entities)
        .unwrap_or_default();

    let is_road = room.name.contains("道路")
        || room.name.contains("木橋")
        || cell.terrain == crate::grid::Terrain::Road
        || cell.terrain == crate::grid::Terrain::Bridge;

    let time_bucket = (crate::game::now_unix() / 180) as i64;
    let mut matched_live_slot = false;

    if !is_inside_settlement_urban_core(x, y)
        && !matches!(
            cell.terrain,
            crate::grid::Terrain::Wall | crate::grid::Terrain::WaterDeep
        )
    {
        for z_idx in 0..CANONICAL_HUNTING_ZONES.len() {
            let slots = live_zone_shanhai_slots(z_idx, time_bucket);
            for sl in slots.iter().filter(|s| s.x == x && s.y == y) {
                if sl.tier == 1 && !is_road && !is_apex_beast_tracked_at(x, y) {
                    if !room_objs.iter().any(|o| o.id == "shanhai_apex_trace") {
                        room_objs.push(RoomObject {
                            id: "shanhai_apex_trace".to_string(),
                            name: "新鮮霸主獸跡".to_string(),
                            owner: String::new(),
                            sockets: vec!["Look".to_string()],
                            responses: std::collections::HashMap::new(),
                            move_to_room_id: String::new(),
                        });
                    }
                } else if sl.tier == 1 && is_road {
                    // 霸主獸跡不留於人工道路上
                } else if let Some(mut amb) = ensure_ambient_npc_at_grid(x, y, sl.tier_tag) {
                    amb.id = format!("shanhai_{}_{}_{}_{}", z_idx, sl.slot_idx, x, y);
                    if !entities.iter().any(|e| e.display_title == amb.display_title) {
                        entities.push(amb);
                    }
                }
                matched_live_slot = true;
            }
        }
    }

    if !matched_live_slot
        && !is_inside_settlement_urban_core(x, y)
        && !matches!(
            cell.terrain,
            crate::grid::Terrain::Wall | crate::grid::Terrain::WaterDeep
        )
        && (x.abs() > 18 || y.abs() > 18)
        && !entities.iter().any(|e| e.kind == crate::entity::EntityKind::Npc)
    {
        let z_idx = nearest_hunting_zone_idx(x, y);
        let st = get_zone_trophic_state(z_idx);
        let apex_present = st.as_ref().map(|s| s.apex_presence >= 0.95).unwrap_or(true);
        let t4_scale = st
            .as_ref()
            .map(|s| (s.n_tier4 / s.k_tier4.max(1.0)).clamp(0.20, 1.25))
            .unwrap_or(1.0);
        let roam_threshold = (72.0 * t4_scale) as u64;

        // 霸主威壓屏息格（約 14% 獵場腹地）：尋常小怪驚散匿跡，地上留有【新鮮霸主獸跡】，查探後方會破霧顯形
        let silence_hash = ((x as i64 * 31337) ^ (y as i64 * 65521)).unsigned_abs() % 7;
        if !is_road && is_apex_beast_tracked_at(x, y) && apex_present {
            if let Some(mut amb) = ensure_ambient_npc_at_grid(x, y, "__apex__") {
                amb.id = format!("apex_{z_idx}_{x}_{y}");
                entities.push(amb);
            }
        } else if !is_road && silence_hash == 0 && apex_present {
            if !room_objs.iter().any(|o| o.id == "shanhai_apex_trace") {
                room_objs.push(RoomObject {
                    id: "shanhai_apex_trace".to_string(),
                    name: "新鮮霸主獸跡".to_string(),
                    owner: String::new(),
                    sockets: vec!["Look".to_string()],
                    responses: std::collections::HashMap::new(),
                    move_to_room_id: String::new(),
                });
            }
        } else {
            // 尋常物種與山海小怪：隨時間片在格內動態游走顯形
            let roam_gate = ((x as i64).wrapping_mul(1103515245)
                ^ (y as i64).wrapping_mul(12345)
                ^ time_bucket.wrapping_mul(2654435761))
                .unsigned_abs()
                % 100;
            if is_road || roam_gate < roam_threshold {
                if let Some(amb) = ensure_ambient_npc_at_grid(x, y, &room.name) {
                    entities.push(amb);
                }
                // 獸群／小怪群聚地塊（約 28% 野外地塊有結群伴生小怪或同群獸類）
                if !is_road && roam_gate < 28 {
                    let flock_tag = if roam_gate % 2 == 0 { "__tier4a__" } else { "__tier4b__" };
                    if let Some(mut companion) = ensure_ambient_npc_at_grid(x, y, flock_tag) {
                        companion.id = format!("ambient_flock_{x}_{y}");
                        if !entities.iter().any(|e| e.display_title == companion.display_title) {
                            entities.push(companion);
                        }
                    }
                }
            }
        }
    }

    if let Some(site) = peek_recent_hunt_kill_site(x, y, crate::game::now_unix()) {
        room_objs.push(RoomObject {
            id: format!("hunt_kill_site_{x}_{y}"),
            name: format!("{}・圍獵遺骸", site.prey_name),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Harvest".to_string()],
            responses: std::collections::HashMap::new(),
            move_to_room_id: String::new(),
        });
    }
    if !is_inside_settlement_urban_core(x, y)
        && !matches!(
            cell.terrain,
            crate::grid::Terrain::Wall | crate::grid::Terrain::WaterDeep
        )
        && !is_road
    {
        let z_idx = nearest_hunting_zone_idx(x, y);
        if let Some(st) = &get_zone_trophic_state(z_idx) {
            let (weather, _) = current_zone_weather_and_tide(z_idx, time_bucket);
            let fungi_threshold = if weather == ZoneWeatherPhase::RainMist {
                105.0
            } else {
                120.0
            };
            if st.detritus_s >= fungi_threshold
                && ((x as i64 * 17389) ^ (y as i64 * 49157)).unsigned_abs() % 8 == 0
            {
                if !room_objs.iter().any(|o| o.id == format!("detritus_fungi_{x}_{y}")) {
                    room_objs.push(RoomObject {
                        id: format!("detritus_fungi_{x}_{y}"),
                        name: "腐木牛樟靈菇叢".to_string(),
                        owner: String::new(),
                        sockets: vec!["Look".to_string(), "Harvest".to_string()],
                        responses: std::collections::HashMap::new(),
                        move_to_room_id: String::new(),
                    });
                }
            }
        }
    }
     let vendor_objs = super::player_scene::dynamic_vendor_objects_for_entities(&entities);
     for v_obj in vendor_objs {
         if !room_objs.iter().any(|o| o.id == v_obj.id) {
             room_objs.push(v_obj);
         }
     }
     room.objects = room_objs.clone();
     let objects = room_objs;
     Ok(Some(RoomView {
         room,
         exits,
         entities,
         objects,
     }))
 }
 
 /// 確保方格座標 (x, y) 有駐場環境人物（若該處尚無活體人物且非水域、非城牆）。
pub fn ensure_ambient_npc_at_grid(x: i32, y: i32, room_name: &str) -> Option<Character> {
    let (zone, _sub_geo, terrain) = crate::grid::r#gen::classify_wilderness_cell(x, y);
    if terrain == crate::grid::Terrain::WaterDeep || terrain == crate::grid::Terrain::Wall {
        return None;
    }

    let time_bucket = (crate::game::now_unix() / 300) as i64;
    let seed = ((x as i64).wrapping_mul(374761393)
        ^ (y as i64).wrapping_mul(668265263)
        ^ time_bucket.wrapping_mul(1442695040888963407))
        .unsigned_abs();
    // 三層生態顯形分配：
    // - Tier 1 鎮場霸主（roll == 0）：預設隱匿不直接站樁，唯有尋跡觸發（room_name == "__apex__" 或已標記尋跡）始破霧顯形
    // - Tier 4 尋常小怪與靈禽（roll == 3, 4）：佔顯形遭遇 65%（野兔、穿山甲、獼猴、飛鼠、環頸雉、竹雞、招潮蟹、青毛蟹等直接顯形於格內）
    // - Tier 3 尋常中型獸群（roll == 2）：佔顯形遭遇 20%（梅花鹿、水鹿、山羌、長鬃山羊、山豬成群游走顯形）
    // - Tier 2 掠食者（roll == 1）：佔 7%
    let roll = if room_name == "__apex__" {
        0
    } else if room_name == "__tier2__" || room_name == "__tier2_overflow__" {
        1
    } else if room_name == "__tier3__" {
        2
    } else if room_name == "__tier4a__" {
        3
    } else if room_name == "__tier4b__" {
        4
    } else if is_apex_beast_tracked_at(x, y)
        && get_zone_trophic_state(nearest_hunting_zone_idx(x, y))
            .is_some_and(|s| s.apex_presence >= 0.95)
    {
        0
    } else {
        match seed % 100 {
            0..=34 => 3,  // 35% 尋常小型奇獸／甲殼小怪（格內顯形）
            35..=69 => 4, // 35% 尋常山海靈禽小怪（格內顯形）
            70..=89 => 2, // 20% 尋常中型主力獸群（格內顯形）
            _ => 1,       // 10% 掠食猛獸
        }
    };

    let (creature_name, creature_activity) = match zone {
            "長風草甸" => match roll {
                0 => ("長風四目水鹿王", "昂首聳立於草甸深處，雙叉巨角直指蒼穹，四目靈光威震四野"),
                1 => ("淺山錢紋石虎", "伏行於長風草甸灌叢陰影間，耳後白斑微動、伺機撲擊野兔"),
                2 => ("平原梅花鹿", "優雅地在草甸花叢間結群嚼草，白色梅花斑紋隨風微動"),
                3 => ("草甸野灰兔", "四足如飛，在齊膝芒草間迅疾掠過帶起陣陣草浪"),
                _ => ("草甸環頸雉", "披著絢麗五彩羽衣在芒草中昂首啼鳴，振翅破風"),
            },
            "古河巨木林" => match roll {
                0 => ("古河樟林雲豹", "無聲無息潛行於千年古樟巨枝之間，暗影中古玉雲紋隱現"),
                1 => ("錢紋石虎", "踏在千年古樟厚軟腐葉層上悄然潛行，雙耳白斑靈動轉動"),
                2 => ("四目水鹿", "生有雙重眶下靈腺，在巨木霧氣中結群踏水而行"),
                3 => ("古林獼猴", "攀附於千年楠木藤蔓間靈巧擺盪，凝神張望林下動靜"),
                _ => ("巨木領角鴞", "棲息於高枝樹洞旁，夜色中金黃雙瞳凝視林間落葉"),
            },
            "赤石坡" => match roll {
                0 => ("赤石鯪鯉穿山甲", "盤踞於赤紅土石礦坡，周身烏金厚鱗在烈日下泛著鐵光"),
                1 => ("淺山錢紋石虎", "伏行於赤石灌木叢陰影間，耳後白斑微動、目光如電"),
                2 => ("赤崖長鬃山羊", "腳踏垂直陡峭的赤褐色岩壁，如履平地凌空躍動"),
                3 => ("赤坡山羌", "在赤石坡蕨草間警覺昂首，發出清亮短促的吠鳴"),
                _ => ("赤崖大冠鷲", "展開黑白條紋巨翼在赤石坡高空乘熱氣流盤旋長鳴"),
            },
            "落日岬" => match roll {
                0 => ("落日黑鳶王", "逆著落日海風在岬角萬仞高空盤旋，尾羽如魚尾開合控風"),
                1 => ("海岬百年綠蠵龜", "伏於落日殘照的岬角礫灘，背甲印刻著百載海潮紋理"),
                2 => ("海崖長鬃山羊", "於狂暴海風撕扯的岬角懸崖邊緣穩健覓食耐鹽灌木"),
                3 => ("烏腳香白鼻心", "穿梭於海岬防風林與林投果叢間，鼻端白線分明"),
                _ => ("迎風海角雕", "逆著浩瀚西風在海角高空盤旋，神目如電巡視汪洋"),
            },
            "驚濤岩岸" => match roll {
                0 => ("驚濤遊隼王", "立於怒濤飛濺的玄武岩尖端，俯衝速度冠絕百禽、破浪裂風"),
                1 => ("裂浪灰海獺", "順著拍岸碎浪滑入急流，在礁石暗湧間靈動捕魚"),
                2 => ("斷崖長鬃山羊", "踏立於海浪拍擊的險峻岩岸峭壁，迎著鹹霧穩立不搖"),
                3 => ("潮間青岩巨蟹", "披著青黑重甲駐守於潮池裂罅，雙螯張合鏗鏘作響"),
                _ => ("驚濤黑岩鷺", "羽色深沉如玄武礁岩，單足凝立於碎浪礁石上伺機啄魚"),
            },
            "巨藻淺灣" => match roll {
                0 => ("淺灣百年綠蠵龜", "悠然浮游於隨潮搖曳的巨藻林間，背甲泛著翠綠海藻幽光"),
                1 => ("渡海黑面琵鷺", "在淺灣潮間帶優雅左右掃動黑長扁喙，成群涉水覓食"),
                2 => ("淺灣灰海獺", "胸口抱著青石在浮動巨藻叢中翻身仰游，悠閒自得"),
                3 => ("潮汐招潮蟹", "高舉鮮紅大螯在淺灣泥沙灘上整齊揮舞，隨潮水退湧進退"),
                _ => ("白鷺候鳥", "單足凝立於淺灣清流白浪之中，凝視游魚伺機啄擊"),
            },
            "黑石灣" => match roll {
                0 => ("黑石成雙海鱟", "身披青銅馬蹄硬甲成對伏於黑石潮池，劍尾如戟古意盎然"),
                1 => ("黑背玄武岩龜", "伏在黝黑巨石上吐納海霧，背甲深沉如萬年古墨"),
                2 => ("玄武岩大海蟹", "藏身於黑石灣潮間玄武岩裂隙，甲殼堅硬如鐵石"),
                3 => ("浪蝕九孔鮑", "如鐵盔般緊緊吸附於黑石浪痕之上，殼表珍珠層生輝"),
                _ => ("白腹鰹鳥", "自黑石海灣上空收翼垂直扎入浪花，精準捕食潮間游魚"),
            },
            "翠冠巨木林" => match roll {
                0 => ("千年樟雲豹王", "立於最高樹冠交疊處俯瞰整座山谷，周身雲紋隱隱發光"),
                1 => ("錢紋石虎", "踏在林下厚軟苔原上悄然潛行，雙耳白斑靈動轉動"),
                2 => ("巨角巡林水鹿", "頭戴分叉如戟的巨型長角，沉穩邁過翠綠灌木叢"),
                3 => ("烏腳香白鼻心", "順著老樟樹皮輕巧滑下，鼻端白線分明，自帶草果幽香"),
                _ => ("丹冠藍腹鷴", "寶藍色羽翼拖著雪白長尾，赤足朱冠在巨木林蔭下踱步"),
            },
            "清泉溪谷" => match roll {
                0 => ("清泉櫻花鉤吻鮭", "冰河孑遺靈魚逆著融雪清泉奮力躍水，身側胭脂梅花斑紋熠熠生輝"),
                1 => ("溪澗食蟹獴", "披著棕灰簑衣長毛在清泉卵石灘間敏捷穿梭，破殼取蟹"),
                2 => ("清泉山羌", "自溪邊蕨草叢探出小巧頭顱，低頭飲用冰涼泉水"),
                3 => ("清溪青毛蟹", "螯足生滿濃密青黑絨毛，在清澈見底的卵石溪流中側行"),
                _ => ("溪澗鉛色水鶇", "停在溪心濕石上輕巧展開栗紅尾羽，隨流水聲清脆鳴唱"),
            },
            "北坡雲杉林" => match roll {
                0 => ("胸白月熊王", "人立於百年雲杉之下發出渾厚咆哮，胸口一抹新月白毫如雪"),
                1 => ("雲杉霧林帝雉", "披著深藍光澤羽衣在雲杉冷霧間緩步巡行，氣度如王者"),
                2 => ("剛鬃黑山豬", "粗壯如石的獠牙向上挑起，前蹄刨掘著雲杉林下堅根"),
                3 => ("北坡山林獼猴", "掛在雲杉垂枝上抓耳撓腮，朝樹下警覺張望"),
                _ => ("高山角鷹", "展開寬闊黑褐花翎，傲視千山盤旋於雲杉密林之上"),
            },
            "西麓林道" => match roll {
                0 => ("西麓丹冠藍腹鷴", "朱紅肉冠配寶藍長羽，在西麓古道晨霧中從容踱步"),
                1 => ("長尾山娘", "拖著琉璃藍長尾成群掠過林道樹冠，彼此呼應守望"),
                2 => ("林道山羌", "在古道兩側闊葉林蔭下輕步覓食嫩葉，聞聲即躍入林"),
                3 => ("食蛇箱龜", "背甲金黃隆起如盾，從容不迫地在碎石落葉間挪行"),
                _ => ("山林竹雞", "在林道兩側密竹叢間結伴啄食草籽，拍翅低飛"),
            },
            "淺澤泥口" => match roll {
                0 => ("淺澤重角水牛王", "雙角寬闊如彎月，半身浸在涼爽泥澤中閉目反芻，氣息沉穩如山"),
                1 => ("沼澤食蟹獴", "咬著泥蟹在蘆葦泥渚間翻滾滑行，皮毛防濕油亮"),
                2 => ("泥口柴棺水龜", "背甲呈古銅褐色，成群靜伏於淺澤浮木上曬日吐納"),
                3 => ("紅螯泥沼蟹", "高舉血紅巨螯在泥孔外威嚇對手，吐著細密水泡"),
                _ => ("凌波水雉", "拖著修長飄逸尾羽輕踏菱角浮葉之上，宛若凌波仙禽"),
            },
            "白茫洲" => match roll {
                0 => ("黑沼百步蛇王", "盤踞於迷霧泥澤枯木之上，三角巨首與菱紋鱗甲透出凜然古靈威壓"),
                1 => ("黑沼飯匙倩", "在白茫洲水霧草叢中昂起前半身，頸部張開如飯匙般威懾四方"),
                2 => ("白茫四目水鹿", "通體沾滿晨霧水珠，頭頂雙叉長角踏過泥水無痕"),
                3 => ("翡翠綠樹蛙", "背脊閃爍翠玉光澤，靜靜蹲在濕潤水芋葉片上鳴叫"),
                _ => ("沼洲夜鷺", "縮頸佇立於迷霧淺灘枯木上，赤紅眼睛穿透重霧"),
            },
            "貼山深潭" => match roll {
                0 => ("寒潭千年花斑鱸鰻", "碗口粗的花斑巨軀潛伏於深碧潭底石洞，游動時捲起暗湧漩渦"),
                1 => ("高山苦花魚王", "銀光閃爍躍出寒潭激流，腹部金線在陽光下熠熠生輝"),
                2 => ("潭畔四目水鹿", "立於深潭岸邊青石上結伴俯身飲水，長角倒映於碧綠潭面"),
                3 => ("寒潭山椒魚", "冰河孑遺靈獸伏於寒潭水際冷石下，褐膚帶淺斑、喜極寒清泉"),
                _ => ("寒潭翠鳥", "披著寶石藍羽衣自潭邊蘆葦疾射入水，銜起銀魚破水而出"),
            },
            "青籠關" => match roll {
                0 => ("青籠大冠鷲王", "傲立於青籠關千仞險峰之巔，羽冠高聳、長鳴聲震動整條南境山隘"),
                1 => ("關隘錢紋石虎", "潛伏在關道荊棘石縫暗處，矯健斑紋身軀蓄勢待發"),
                2 => ("剛鬃黑山豬", "粗壯獠牙泛著冷光，在關隘山坳間拱土覓食"),
                3 => ("鯪鯉穿山甲", "捲成堅不可摧的烏金鱗球滾過山隘石坡，刀劍難傷"),
                _ => ("關隘長鬃山羊", "敏捷踏過險峻關石窄徑，灰色長鬃在山風中飛舞"),
            },
            "西嶺礦谷" => match roll {
                0 => ("裂石鯪鯉穿山甲王", "全身烏金鱗甲堅硬如百鍛精鋼，利爪一揮便刨開堅硬赤鐵礦岩"),
                1 => ("礦谷胸白月熊", "在西嶺礦谷深處岩洞前巡視領地，胸前新月白斑清晰奪目"),
                2 => ("青脊長鬃山羊", "攀行於近乎垂直的露頭礦脈岩壁上，蹄尖穩扣寸許石縫"),
                3 => ("礦谷黑野豬", "長吻帶泥在煤石堆旁衝撞覓食，渾身沾滿硬結礦泥"),
                _ => ("高山金翼白眉", "眉間白紋配金橙翼斑，在礦谷冷杉枝頭清脆吟唱"),
            },
            "東海峭壁" => match roll {
                0 => ("馭風角鷹王", "凌空盤旋俯瞰東海萬頃波浪，雙翼遮雲蔽日、頭頂雙簇角冠威凜"),
                1 => ("破浪遊隼", "自千仞海崖順風俯衝，雙翼收攏如精鋼利箭掠過海面"),
                2 => ("斷崖長鬃山羊", "腳踏險絕東海斷崖上的寸許石階，成群從容縱躍如飛"),
                3 => ("峭壁大赤飛鼠", "自海風呼嘯的懸崖樹梢縱身一躍，乘著上升氣流滑向對岸"),
                _ => ("峭壁岩鷚", "羽色如花崗岩斑駁，在臨海峭壁石隙中逆風跳躍覓食"),
            },
            "雲腰雪林" => match roll {
                0 => ("霧林黑長尾帝雉", "披著深藍曜黑羽衣拖著雪白橫紋長尾，漫步於高山圓柏與箭竹積雪中"),
                1 => ("雪線胸白月熊", "頂著嚴寒風雪在鐵杉雪林中巡狩，掌力能折斷百年冰枝"),
                2 => ("踏雪四目水鹿", "在寒霧積雪中安詳漫步，四目顧盼清明出塵"),
                3 => ("雪林山椒魚", "潛藏於雪林融冰石縫之下，耐受極寒冰泉"),
                _ => ("雪林酒紅朱雀", "全身羽色如陳年丹砂般鮮豔，在覆雪鐵杉枝頭宛若紅梅綻放"),
            },
            "白塔雪巔" => match roll {
                0 => ("雪巔霜冠大水鹿", "傲然立於冰封絕頂石柱之巔，霜白巨角如玉樹臨風、吐納天地至純清氣"),
                1 => ("極頂黑長尾帝雉", "在萬仞雪峰雲海邊緣漫步，長尾白紋與萬年玄冰交相輝映"),
                2 => ("雪脊長鬃山羊", "蹄踏玄冰雪脊，在萬丈虛空之上從容齧食高山冰苔"),
                3 => ("雪巔冰河山椒魚", "棲息於白塔極頂冰川融水石隙，見證萬載寒暑"),
                _ => ("雪峰岩鷚", "羽如灰石斑駁，在冰封岩頂上不懼酷寒歡快啄食細雪草籽"),
            },
            _ => match roll {
                0 => ("山林梅花鹿", "警覺地站在草丘上環顧四周風吹草動"),
                1 => ("烏腳香白鼻心", "在矮灌木叢中靈巧鑽動尋找甜美野果"),
                2 => ("雜林野灰兔", "在草葉間抽動鼻尖，後肢蹬地飛快奔竄"),
                3 => ("山林竹雞", "在路旁草葉間結隊咕咕低鳴覓食草籽"),
                _ => ("草叢龜殼花", "盤曲於厚軟草叢陰影間，冷眼凝視過往生靈"),
            },
    };

    let display_char = creature_name.chars().next().map(|c| c.to_string()).unwrap_or_default();

    let (vit, qi, dex) = match roll {
        0 => (120, 60, 30), // Tier 1 Apex
        1 => (55, 28, 26),  // Tier 2 Predator
        2 => (30, 15, 20),  // Tier 3 Medium Game
        3 => (16, 10, 24),  // Tier 4 Small Fauna
        _ => (18, 12, 28),  // Tier 4 Avian
    };
    Some(Character {
        id: format!("ambient_{}_{}", x, y),
        kind: EntityKind::Npc,
        display_char,
        x,
        y,
        move_state: crate::entity::MoveState::Idle,
        target_x: None,
        target_y: None,
        walk_or_run: crate::entity::WalkOrRun::Walk,
        move_started_at: None,
        vit,
        qi,
        dex,
        magnesium: 0,
        last_observed_at: Some(crate::game::now_unix()),
        created_at: crate::game::now_unix(),
        gender: None,
        soul_seed: None,
        display_title: creature_name.to_string(),
        activated_nodes: "[]".to_string(),
        equipment_slots: "{}".to_string(),
        inventory: "[]".to_string(),
        disposition: 0,
        thoughts: vec![],
        current_activity: {
            let gh = crate::store::current_game_hour();
            let mut act = creature_activity.to_string();
            if matches!(
                creature_name,
                "夜嘯大赤飛鼠" | "烏腳香白鼻心" | "錢紋石虎" | "石隙雪鴞" | "沼洲夜鷺" | "峭壁飛鼠"
            ) && !(6..18).contains(&gh)
            {
                act.push_str("（夜行時分：正乘著夜色幽暗無聲掠出，雙目在月華下爍爍生輝）");
            } else if matches!(
                creature_name,
                "逐日七叉白鹿" | "霧林黑長尾帝雉" | "丹冠藍腹鷴" | "白茫角鹿" | "踏雪四目水鹿"
            ) && (5..9).contains(&gh)
            {
                act.push_str("（晨霧瑞兆：正迎著破曉山嵐吐納朝露，神駿逼人）");
            } else if matches!(
                creature_name,
                "深海大王烏賊" | "深海皇帶龍魚" | "載嶼百年巨鼇" | "噴玉海翁巨鯨" | "海岬豆腐鯨鯊"
            ) && gh % 6 <= 1
            {
                act.push_str("（大潮現身：正乘著黑水潮汐退湧之際浮現海面，引動暗流轟鳴）");
            }
            act
        },
        grid_x: Some(x),
        grid_y: Some(y),
    })
}

/// 將山海環境生物名稱映射至千物總典物品 ID、名稱與《白塔山海誌》典籍描述。
pub fn ambient_creature_loot(creature_name: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if creature_name.ends_with('客')
        || creature_name.ends_with('人')
        || creature_name.ends_with('手')
        || creature_name.ends_with('民')
        || creature_name.ends_with('卒')
        || creature_name.ends_with('工')
        || creature_name.ends_with('農')
        || creature_name.ends_with('夫')
        || creature_name.ends_with('衛')
        || creature_name.ends_with('士')
        || creature_name.ends_with('員')
        || creature_name.ends_with('戶')
        || creature_name.ends_with('者')
        || creature_name.ends_with('商')
        || creature_name.ends_with('勇')
    {
        return None;
    }
    if creature_name.contains("帝雉") {
        Some(("wt_cat_0214", "霧林帝雉長尾翎", "《白塔山海誌》：烏雉生於雲霧鐵杉之巔，尾長三尺而橫白紋。"))
    } else if creature_name.contains("藍腹鷴") {
        Some(("wt_cat_0216", "丹冠藍腹鷴彩羽", "《白塔山海誌》：丹冠白尾鷴羽泛寶藍幽光，見之吉祥。"))
    } else if creature_name.contains("豹") {
        Some(("wt_cat_0172", "樟林雲豹軟毛皮", "《白塔山海誌》：雲豹身披古玉雲紋，佩其皮則山行不躓。"))
    } else if creature_name.contains("熊") {
        Some(("wt_cat_0205", "胸白月熊厚熊掌", "《白塔山海誌》：月熊胸生白毫如新月，其掌食之禦風寒。"))
    } else if creature_name.contains("穿山甲") || creature_name.contains("鯪鯉") {
        Some(("wt_cat_0188", "鯪鯉穿山甲鱗片", "《白塔山海誌》：鯪鯉遍體烏金硬鱗，炙之通經絡、綴甲禦刃。"))
    } else if creature_name.contains("龜") || creature_name.contains("鼇") {
        Some(("wt_cat_0252", "深潭野生老鱉", "《白塔山海誌》：靈龜玄甲堅實如盾，息於清泉林澗。"))
    } else if creature_name.contains("獴") || creature_name.contains("獺") || creature_name.contains("獾") {
        Some(("wt_cat_0183", "白鼻心烏腳香皮", "《白塔山海誌》：食蟹獴喜伏於溪澗泥澤捕食溪蟹水蟲，身形矯捷靈敏，毛皮厚實防潮。"))
    } else if creature_name.contains("蟹") || creature_name.contains("鱟") {
        Some(("wt_cat_0233", "紅螯大青蟹", "《白塔山海誌》：鐵甲巨螯伏於潮間泥灘，雙鉗力能夾石。"))
    } else if creature_name.contains("蛙") || creature_name.contains("蟾") || creature_name.contains("鯢") || creature_name.contains("山椒魚") {
        Some(("wt_cat_0275", "草叢野生牛蛙", "《白塔山海誌》：水澤兩棲靈物伏於寒潭草澗，喜清泉雨霧。"))
    } else if creature_name.contains("海膽") || creature_name.contains("螺") || creature_name.contains("鮑") {
        Some(("wt_cat_0223", "野生九孔鮑", "《白塔山海誌》：潮間岩穴甲殼海珍，味極甘鮮。"))
    } else if creature_name.contains("蛇") || creature_name.contains("蟒") || creature_name.contains("蝮") || creature_name.contains("百足") {
        Some(("wt_cat_0189", "百步蛇乾蛻", "《白塔山海誌》：百步大蛇首如三角鐵鏟，其蛻研粉可解瘴毒。"))
    } else if creature_name.contains("水鹿") || creature_name.contains("白鹿") || creature_name.contains("梅花鹿") || creature_name.contains("角鹿") || creature_name.contains("鹿") {
        Some(("wt_cat_0203", "高地水鹿厚鹿皮", "《白塔山海誌》：四目靈鹿生於高山溪谷，其皮堅韌耐水火。"))
    } else if creature_name.contains("山羌") || creature_name.contains("羌") {
        Some(("wt_cat_0209", "山羌柔軟麂皮", "《白塔山海誌》：山羌鳴聲如犬，其皮細膩溫軟。"))
    } else if creature_name.contains("羊") || creature_name.contains("羚") {
        Some(("wt_cat_0179", "長鬃山羊彎角", "《白塔山海誌》：長鬃岩羚善登千仞絕壁，其角堅韌可為號角。"))
    } else if creature_name.contains("地牛") || creature_name.contains("牛") {
        Some(("wt_cat_0187", "大角原牛厚蹄皮", "《白塔山海誌》：巨牛伏行草甸與地脈，蹄皮厚韌冠絕百獸。"))
    } else if creature_name.contains("豬") {
        Some(("wt_cat_0211", "海崖黑野豬牙", "《白塔山海誌》：剛鬃山豬力能拱石，其獠牙彎如新月。"))
    } else if creature_name.contains("飛鼠") || creature_name.contains("鼯") {
        Some(("wt_cat_0182", "大赤飛鼠厚絨毛", "《白塔山海誌》：赤鼯夜飛巨木之間，其絨輕暖勝狐裘。"))
    } else if creature_name.contains("白鼻心") || creature_name.contains("狐") || creature_name.contains("麝") || creature_name.contains("貂") || creature_name.contains("猴") {
        Some(("wt_cat_0183", "白鼻心烏腳香皮", "《白塔山海誌》：烏腳香與山海毛獸善棲林澗礁灣，毛皮細密隔水且帶草果清香。"))
    } else if creature_name.contains("石虎") || creature_name.contains("虎") || creature_name.contains("狼") || creature_name.contains("犬") {
        Some(("wt_cat_0171", "山林石虎斑皮", "《白塔山海誌》：石虎耳後生白斑，夜行山林迅捷如電。"))
    } else if creature_name.contains("兔") || creature_name.contains("鼠") {
        Some(("wt_cat_0185", "灰野兔整張絨皮", "《白塔山海誌》：野兔潛於芒草石縫，其絨護掌不畏寒霜。"))
    } else if creature_name.contains("鱷") || creature_name.contains("沼龍") || creature_name.contains("蜥") {
        Some(("wt_cat_0176", "泥甲巨鱷厚麟皮", "《白塔山海誌》：泥甲巨獸伏於腐葉深澤，背麟刀斧難入。"))
    } else if creature_name.contains("烏賊") || creature_name.contains("花枝") {
        Some(("wt_cat_0236", "大花枝鮮黑墨囊", "《白塔山海誌》：深海巨鰂眼如銅鑼，墨囊濃黑如漆。"))
    } else if creature_name.contains("鯨") || creature_name.contains("海翁") || creature_name.contains("鯊") {
        Some(("wt_cat_0220", "豆腐鯊厚砂魚皮", "《白塔山海誌》：海翁巨鯊浮海如嶼，其皮粗韌如砂石。"))
    } else if creature_name.contains("龍魚") || creature_name.contains("鬼頭刀") || creature_name.contains("苦花") || creature_name.contains("鯇") || creature_name.contains("鰻") || creature_name.contains("魚") {
        Some(("wt_cat_0224", "銀鱗土魠魚", "《白塔山海誌》：外海大魚銀鱗閃爍，食之益氣強身。"))
    } else if creature_name.contains("鷹") || creature_name.contains("雕") || creature_name.contains("隼") || creature_name.contains("鴞") {
        Some(("wt_cat_0192", "高山熊鷹翎羽", "《白塔山海誌》：高山角鷹盤旋絕壁，其翎破空不顫。"))
    } else if creature_name.contains("雉") || creature_name.contains("雞") || creature_name.contains("鳥") || creature_name.contains("鷗") || creature_name.contains("鷺") || creature_name.contains("燕") || creature_name.contains("鷚") || creature_name.contains("眉") || creature_name.contains("雀") || creature_name.contains("鳩") {
        Some(("wt_cat_0173", "水鳥蓬鬆絨羽", "《白塔山海誌》：山海飛禽絨羽輕柔，禦寒隔濕。"))
    } else {
        Some(("wt_cat_0185", "灰野兔整張絨皮", "《白塔山海誌》：山海林野走獸潛行草木岩壑之間，毛皮溫韌。"))
    }
}

/// 正典 19 座山海獵場與其深處地脈核心錨點。
pub const CANONICAL_HUNTING_ZONES: &[(&str, i32, i32)] = &[
    ("長風草甸", -180, -195),
    ("古河巨木林", -55, -80),
    ("赤石坡", -85, -180),
    ("落日岬", -320, -180),
    ("驚濤岩岸", -355, 115),
    ("巨藻淺灣", -280, 200),
    ("黑石灣", -280, 5),
    ("白茫洲", -180, -320),
    ("貼山深潭", 120, -260),
    ("淺澤泥口", 20, -180),
    ("青籠關", 0, -450),
    ("西麓林道", -120, 80),
    ("清泉溪谷", 60, 160),
    ("翠冠巨木林", -60, 220),
    ("北坡雲杉林", 80, 280),
    ("西嶺礦谷", 180, 40),
    ("東海峭壁", 320, 200),
    ("雲腰雪林", 180, 320),
    ("白塔雪巔", 0, 420),
];

/// 全島全域自然棲地生靈點位（消除人工同心圓，按 19 獵場真實疆域進行藍噪聲 Poisson-disc 與獸群聚落分布）。
#[derive(Debug, Clone)]
pub struct ShanhaiHabitatSlot {
    pub slot_idx: usize,
    pub base_x: i32,
    pub base_y: i32,
    pub x: i32,
    pub y: i32,
    pub sub_geo: &'static str,
    pub tier: u8,
    pub tier_tag: &'static str,
    pub kind_str: &'static str,
    pub tier_label: &'static str,
    pub act_prefix: &'static str,
}

pub fn is_inside_settlement_urban_core(x: i32, y: i32) -> bool {
    const SETTLEMENT_CORES: &[(i32, i32, i32)] = &[
        (0, 0, 20),         // 文曜 [-17..17]
        (-260, 160, 14),    // 風舶角 [-270..-250, 150..170]
        (-330, 160, 11),    // 風舶嶼 [-337..-323, 153..167]
        (-180, -140, 11),   // 大圳頭 [-187..-173, -147..-133]
        (160, 210, 11),     // 半嶺鎮 [153..167, 203..217]
        (0, -220, 11),      // 南哨 [-7..7, -227..-213]
        (0, 125, 9),        // 老堰頭 [-6..6, 119..131]
        (110, 115, 9),      // 北嶺腳 [104..116, 109..121]
        (190, 290, 9),      // 霜林口 [184..196, 284..296]
        (-360, 95, 9),      // 風櫃尾 [-366..-354, 89..101]
        (-260, 35, 9),      // 烏石鼻 [-266..-254, 29..41]
        (-180, -245, 9),    // 牛埔 [-186..-174, -250..-240]
        (-85, -140, 9),     // 紅土崁 [-91..-79, -146..-134]
        (-285, -140, 9),    // 三號坑口 [-291..-279, -146..-134]
        (-75, -280, 9),     // 水尾寮 [-81..-69, -286..-274]
    ];
    SETTLEMENT_CORES
        .iter()
        .any(|&(sx, sy, r)| (x - sx).abs() <= r && (y - sy).abs() <= r)
}

fn habitat_hash(a: i64, b: i64, c: i64) -> u64 {
    let mut h = (a as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((b as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add((c as u64).wrapping_mul(0x94D0_49BB_1331_11EB));
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^ (h >> 31)
}

fn is_valid_wilderness_habitat_cell(x: i32, y: i32, expected_zone: &str) -> Option<&'static str> {
    if !(-392..=392).contains(&x) || !(-492..=492).contains(&y) {
        return None;
    }
    if is_inside_settlement_urban_core(x, y) {
        return None;
    }
    let (zone, sub_geo, terrain) = crate::grid::r#gen::classify_wilderness_cell(x, y);
    if zone != expected_zone {
        return None;
    }
    if matches!(
        terrain,
        crate::grid::Terrain::WaterDeep | crate::grid::Terrain::Wall | crate::grid::Terrain::Water
    ) {
        return None;
    }
    Some(sub_geo)
}

type BaseHabitatEntry = (i32, i32, &'static str, u8, &'static str, usize);

static ZONE_HABITAT_LAYOUTS: std::sync::LazyLock<Vec<Vec<BaseHabitatEntry>>> =
    std::sync::LazyLock::new(|| {
        let mut zone_candidates: Vec<Vec<(i32, i32, &'static str)>> =
            vec![Vec::new(); CANONICAL_HUNTING_ZONES.len()];

        // 1. 掃描全島 800x1000 疆域（4 格抖動網格），按 classify_wilderness_cell 歸入 19 獵場真實版圖
        for by in (-488..=468).step_by(4) {
            for bx in (-388..=358).step_by(4) {
                let h = habitat_hash(bx as i64, by as i64, 17);
                let jx = ((h % 3) as i32) - 1;
                let jy = (((h >> 8) % 3) as i32) - 1;
                let gx = bx + jx;
                let gy = by + jy;
                if is_inside_settlement_urban_core(gx, gy) {
                    continue;
                }
                let (zone, sub_geo, terrain) = crate::grid::r#gen::classify_wilderness_cell(gx, gy);
                if matches!(
                    terrain,
                    crate::grid::Terrain::WaterDeep
                        | crate::grid::Terrain::Wall
                        | crate::grid::Terrain::Water
                ) {
                    continue;
                }
                if let Some(z_idx) = CANONICAL_HUNTING_ZONES
                    .iter()
                    .position(|(zn, _, _)| *zn == zone)
                {
                    zone_candidates[z_idx].push((gx, gy, sub_geo));
                }
            }
        }

        // 針對風舶外島「驚濤岩岸」與狹長海岸帶加密採樣（2 格步距），確保外島與岬角棲地充沛
        for by in (-270..=260).step_by(2) {
            for bx in (-376..=-225).step_by(2) {
                if is_inside_settlement_urban_core(bx, by) {
                    continue;
                }
                let (zone, sub_geo, terrain) = crate::grid::r#gen::classify_wilderness_cell(bx, by);
                if !matches!(zone, "驚濤岩岸" | "落日岬" | "巨藻淺灣" | "黑石灣") {
                    continue;
                }
                if matches!(
                    terrain,
                    crate::grid::Terrain::WaterDeep
                        | crate::grid::Terrain::Wall
                        | crate::grid::Terrain::Water
                ) {
                    continue;
                }
                if let Some(z_idx) = CANONICAL_HUNTING_ZONES
                    .iter()
                    .position(|(zn, _, _)| *zn == zone)
                {
                    if !zone_candidates[z_idx]
                        .iter()
                        .any(|&(ex, ey, _)| ex == bx && ey == by)
                    {
                        zone_candidates[z_idx].push((bx, by, sub_geo));
                    }
                }
            }
        }

        // 按 19 座獵場真實陸塊面積（候選棲地格數）動態配賦 Tier-3（共 380 隻）與 Tier-4（共 1,292 隻）名額，
        // 使廣袤大獵場（翠冠巨木林、西麓林道、西嶺礦谷、白茫洲等）腹地飽滿，狹小岬角不過度擁擠，總數嚴格維持 1,805 隻！
        let total_cands: usize = zone_candidates.iter().map(|v| v.len().max(1)).sum();
        let mut tier3_counts = vec![12usize; CANONICAL_HUNTING_ZONES.len()]; // 保底 12*19 = 228，餘 152 按面積分配
        let rem_t3 = 380usize.saturating_sub(12 * CANONICAL_HUNTING_ZONES.len());
        let mut assigned_t3 = 0usize;
        for z_idx in 0..CANONICAL_HUNTING_ZONES.len() {
            let add = (rem_t3 * zone_candidates[z_idx].len().max(1)) / total_cands.max(1);
            tier3_counts[z_idx] += add;
            assigned_t3 += add;
        }
        let mut order_by_size: Vec<usize> = (0..CANONICAL_HUNTING_ZONES.len()).collect();
        order_by_size.sort_by_key(|&i| std::cmp::Reverse(zone_candidates[i].len()));
        for i in 0..(rem_t3.saturating_sub(assigned_t3)) {
            tier3_counts[order_by_size[i % order_by_size.len()]] += 1;
        }

        let mut tier4_counts = vec![28usize; CANONICAL_HUNTING_ZONES.len()]; // 保底 28*19 = 532，餘 760 按面積分配
        let rem_t4 = 1292usize.saturating_sub(28 * CANONICAL_HUNTING_ZONES.len());
        let mut assigned_t4 = 0usize;
        for z_idx in 0..CANONICAL_HUNTING_ZONES.len() {
            let add = (rem_t4 * zone_candidates[z_idx].len().max(1)) / total_cands.max(1);
            tier4_counts[z_idx] += add;
            assigned_t4 += add;
        }
        for i in 0..(rem_t4.saturating_sub(assigned_t4)) {
            tier4_counts[order_by_size[i % order_by_size.len()]] += 1;
        }

        let mut layouts = Vec::with_capacity(CANONICAL_HUNTING_ZONES.len());
        let mut global_chosen: Vec<(i32, i32)> = Vec::with_capacity(1900);

        for (z_idx, &(zone_name, cx, cy)) in CANONICAL_HUNTING_ZONES.iter().enumerate() {
            let cands = &zone_candidates[z_idx];
            let t3_quota = tier3_counts[z_idx];
            let t4_quota = tier4_counts[z_idx];
            let mut entries: Vec<BaseHabitatEntry> = Vec::with_capacity(1 + 6 + t3_quota + t4_quota + 3);

            // 預先計算每個候選格距獵場邊界／海岸線／城鎮核心的邊界權重（3..=10），不封頂腹地距離，徹底消除大獵場中央空洞與邊界排隊
            let cand_boundary_weights: Vec<i64> = cands
                .iter()
                .map(|&(px, py, _)| {
                    const DIRS: &[(i32, i32)] = &[
                        (1, 0),
                        (-1, 0),
                        (0, 1),
                        (0, -1),
                        (1, 1),
                        (-1, 1),
                        (1, -1),
                        (-1, -1),
                    ];
                    for &(r, w) in &[(5i32, 3i64), (10, 6), (16, 8)] {
                        for &(dx, dy) in DIRS {
                            let tx = px + dx * r;
                            let ty = py + dy * r;
                            if is_inside_settlement_urban_core(tx, ty) {
                                return w;
                            }
                            let (tz, _, tt) =
                                crate::grid::r#gen::classify_wilderness_cell(tx, ty);
                            if tz != zone_name
                                || matches!(
                                    tt,
                                    crate::grid::Terrain::WaterDeep | crate::grid::Terrain::Wall
                                )
                            {
                                return w;
                            }
                        }
                    }
                    10i64
                })
                .collect();

            // (1) Tier 1 鎮場霸主（1 頭）：駐於獵場深處核心地脈，周圍 2 格設「百鳥屏息圈」
            let (apex_x, apex_y, apex_sub) = if let Some(sub) =
                is_valid_wilderness_habitat_cell(cx + 1, cy - 1, zone_name)
            {
                (cx + 1, cy - 1, sub)
            } else {
                cands
                    .iter()
                    .copied()
                    .min_by_key(|&(x, y, _)| (x - cx).pow(2) + (y - cy).pow(2))
                    .unwrap_or((cx + 1, cy - 1, "深谷"))
            };
            entries.push((apex_x, apex_y, apex_sub, 1, "__apex__", 0));

            let mut chosen_points: Vec<(i32, i32)> = vec![(apex_x, apex_y)];
            global_chosen.push((apex_x, apex_y));

            let pick_next_spread_point =
                |chosen: &mut Vec<(i32, i32)>,
                 all_chosen: &mut Vec<(i32, i32)>,
                 salt: i64|
                 -> (i32, i32, &'static str) {
                    let mut best = (cx + 5, cy - 5, "林地");
                    let mut best_score = -1i64;
                    for (c_idx, &(px, py, psub)) in cands.iter().enumerate() {
                        if (px - apex_x).abs() <= 2 && (py - apex_y).abs() <= 2 {
                            continue;
                        }
                        let min_local_d2 = chosen
                            .iter()
                            .map(|&(ox, oy)| {
                                let dx = (px - ox) as i64;
                                let dy = (py - oy) as i64;
                                dx * dx + dy * dy
                            })
                            .min()
                            .unwrap_or(0);
                        if min_local_d2 == 0 {
                            continue;
                        }
                        let min_global_d2 = all_chosen
                            .iter()
                            .map(|&(ox, oy)| {
                                let dx = (px - ox) as i64;
                                let dy = (py - oy) as i64;
                                dx * dx + dy * dy
                            })
                            .min()
                            .unwrap_or(min_local_d2);
                        let effective_d2 = min_local_d2.min(min_global_d2);
                        let bw = cand_boundary_weights[c_idx];
                        let jitter_w = 9 + (habitat_hash(px as i64, py as i64, salt) % 7) as i64;
                        let score = effective_d2 * bw * jitter_w
                            + (habitat_hash(py as i64, px as i64, salt + 99) % 31) as i64;
                        if score > best_score {
                            best_score = score;
                            best = (px, py, psub);
                        }
                    }
                    chosen.push((best.0, best.1));
                    all_chosen.push((best.0, best.1));
                    best
                };

            // (2) Tier 2 次級掠食猛獸／珍獸（6 隻 / 獵場，全島 114 隻）：分據獵場全境 6 大獨立巡獵領地
            for p_idx in 0..6usize {
                let (px, py, psub) = pick_next_spread_point(
                    &mut chosen_points,
                    &mut global_chosen,
                    100 + p_idx as i64,
                );
                entries.push((px, py, psub, 2, "__tier2__", 1 + p_idx));
            }

            // (3) Tier 3 尋常中型獸群（全島 380 隻，按獵場面積配賦 t3_quota）：以每群 2~3 隻結群放牧於全區草甸溪谷
            const HERD_MEMBER_DELTAS: &[(i32, i32)] = &[
                (0, 0),
                (2, 1),
                (-1, 2),
                (1, -2),
                (-2, -1),
                (2, -1),
                (-2, 2),
            ];
            let mut rem_herd_members = t3_quota;
            let mut herd_idx = 0usize;
            while rem_herd_members > 0 {
                let herd_size = if rem_herd_members >= 3 {
                    3
                } else {
                    rem_herd_members
                };
                rem_herd_members -= herd_size;
                let (hx, hy, hsub) = pick_next_spread_point(
                    &mut chosen_points,
                    &mut global_chosen,
                    300 + herd_idx as i64,
                );
                for m in 0..herd_size {
                    let (mut mx, mut my, mut msub) = (hx, hy, hsub);
                    if m > 0 {
                        for attempt in 0..HERD_MEMBER_DELTAS.len() {
                            let (dx, dy) = HERD_MEMBER_DELTAS
                                [(m + herd_idx + attempt) % HERD_MEMBER_DELTAS.len()];
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let tx = hx + dx;
                            let ty = hy + dy;
                            if ((tx - apex_x).abs() > 2 || (ty - apex_y).abs() > 2)
                                && !chosen_points.contains(&(tx, ty))
                                && let Some(valid_sub) =
                                    is_valid_wilderness_habitat_cell(tx, ty, zone_name)
                            {
                                mx = tx;
                                my = ty;
                                msub = valid_sub;
                                chosen_points.push((mx, my));
                                global_chosen.push((mx, my));
                                break;
                            }
                        }
                    }
                    entries.push((mx, my, msub, 3, "__tier3__", 7 + herd_idx));
                }
                herd_idx += 1;
            }

            // (4) Tier 4 尋常小怪與山海靈禽（全島 1,292 隻，按獵場面積配賦 t4_quota）：以藍噪聲漫山散佈於獵場全境腹地
            for k in 0..t4_quota {
                let (fx, fy, fsub) = pick_next_spread_point(
                    &mut chosen_points,
                    &mut global_chosen,
                    700 + k as i64,
                );
                let tier_tag = if k % 2 == 0 {
                    "__tier4a__"
                } else {
                    "__tier4b__"
                };
                entries.push((fx, fy, fsub, 4, tier_tag, 30 + k));
            }

            // (5) 次級掠食者過載溢出槽（3 隻 / 獵場）：供霸主退位時次級掠食者釋放（Mesopredator Release）湧現
            for ofs in 0..3usize {
                let (px, py, psub) = pick_next_spread_point(
                    &mut chosen_points,
                    &mut global_chosen,
                    900 + ofs as i64,
                );
                entries.push((px, py, psub, 2, "__tier2_overflow__", 1000 + ofs));
            }

            layouts.push(entries);
        }
        layouts
    });

fn resolve_displaced_habitat_cell(
    bx: i32,
    by: i32,
    base_sub: &'static str,
    target_x: i32,
    target_y: i32,
    max_step: i32,
    apex_x: i32,
    apex_y: i32,
    zone_name: &str,
) -> (i32, i32, &'static str) {
    let dx = target_x - bx;
    let dy = target_y - by;
    let dist = dx.abs() + dy.abs();
    if dist == 0 || max_step <= 0 {
        return (bx, by, base_sub);
    }
    let step = max_step.min(dist);
    let step_x = ((dx as f32 / dist as f32) * (step as f32)).round() as i32;
    let step_y = ((dy as f32 / dist as f32) * (step as f32)).round() as i32;
    let cand_x = bx + step_x;
    let cand_y = by + step_y;

    const PROBE_OFFSETS: &[(i32, i32)] = &[(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)];
    for &(ox, oy) in PROBE_OFFSETS {
        let tx = cand_x + ox;
        let ty = cand_y + oy;
        if ((tx - apex_x).abs() > 2 || (ty - apex_y).abs() > 2)
            && let Some(vsub) = is_valid_wilderness_habitat_cell(tx, ty, zone_name)
        {
            return (tx, ty, vsub);
        }
    }
    (bx, by, base_sub)
}

fn pick_zone_outskirt_road_spillover_cell(
    z_idx: usize,
    slot_idx: usize,
    time_tick: i64,
) -> Option<(i32, i32)> {
    let &(_, cx, cy) = CANONICAL_HUNTING_ZONES.get(z_idx)?;
    let milestones = get_island_road_milestones();
    if !milestones.is_empty() {
        let candidates: Vec<(i32, i32)> = milestones
            .iter()
            .copied()
            .filter_map(|(rx, ry, _)| {
                let dist = (rx - cx).abs() + (ry - cy).abs();
                if (15..=85).contains(&dist) && !is_inside_settlement_urban_core(rx, ry) {
                    Some((rx, ry))
                } else {
                    None
                }
            })
            .collect();
        if !candidates.is_empty() {
            let seed = (z_idx as i64 * 31)
                ^ (slot_idx as i64 * 71)
                ^ (time_tick * 101);
            let idx = (seed.unsigned_abs() as usize) % candidates.len();
            return Some(candidates[idx]);
        }
    }

    const SETTLEMENT_ANCHORS: &[(i32, i32, i32)] = &[
        (0, 0, 20),         // 文曜
        (-260, 160, 14),    // 風舶角
        (-330, 160, 11),    // 風舶嶼
        (-180, -140, 11),   // 大圳頭
        (160, 210, 11),     // 半嶺鎮
        (0, -220, 11),      // 南哨
        (0, 125, 9),        // 老堰頭
        (110, 115, 9),      // 北嶺腳
        (190, 290, 9),      // 霜林口
        (-360, 95, 9),      // 風櫃尾
        (-260, 35, 9),      // 烏石鼻
        (-180, -245, 9),    // 牛埔
        (-85, -140, 9),     // 紅土崁
        (-285, -140, 9),    // 三號坑口
        (-75, -280, 9),     // 水尾寮
    ];

    let nearest_anchor = SETTLEMENT_ANCHORS
        .iter()
        .min_by_key(|&(sx, sy, _)| (sx - cx).abs() + (sy - cy).abs())?;

    let (sx, sy, radius) = *nearest_anchor;
    let dx = sx - cx;
    let dy = sy - cy;
    let full_dist = (dx.abs() + dy.abs()).max(1);

    for t in (15..=85).step_by(3) {
        let step_ratio = (t as f32) / (full_dist as f32);
        let px = cx + ((dx as f32) * step_ratio).round() as i32;
        let py = cy + ((dy as f32) * step_ratio).round() as i32;
        let dist = (px - cx).abs() + (py - cy).abs();
        if (15..=85).contains(&dist)
            && !is_inside_settlement_urban_core(px, py)
            && (px - sx).abs() > radius + 1
            && (py - sy).abs() > radius + 1
        {
            return Some((px, py));
        }
    }

    let px = cx + 20;
    let py = cy + 20;
    if !is_inside_settlement_urban_core(px, py) {
        Some((px, py))
    } else {
        Some((cx + 25, cy + 25))
    }
}

/// 取得指定獵場（`z_idx in 0..19`）在當前時間片之《白塔山海誌》生靈即時棲地座標（個體與獸群各自獨立漫遊）。
pub fn live_zone_shanhai_slots(z_idx: usize, time_tick: i64) -> Vec<ShanhaiHabitatSlot> {
    step_all_zones_trophic_web_if_needed(time_tick);

    let Some(&(zone_name, _, _)) = CANONICAL_HUNTING_ZONES.get(z_idx) else {
        return Vec::new();
    };
    let base_list = &ZONE_HABITAT_LAYOUTS[z_idx];
    let (apex_x, apex_y) = base_list
        .first()
        .map(|&(x, y, ..)| (x, y))
        .unwrap_or((0, 0));

    let st = get_zone_trophic_state(z_idx);
    let apex_present = st.as_ref().map(|s| s.apex_presence >= 0.95).unwrap_or(true);
    let spillover_active = is_zone_predator_spillover_active(z_idx);
    let max_tier2 = st
        .as_ref()
        .map(|s| {
            let base = (s.n_tier2.round() as usize).clamp(1, 9);
            if spillover_active {
                base.max(7)
            } else {
                base
            }
        })
        .unwrap_or(6);
    let max_tier3 = st.as_ref().map(|s| (s.n_tier3.round() as usize).max(2)).unwrap_or(30);
    let base_max_tier4 = st.as_ref().map(|s| (s.n_tier4.round() as usize).max(4)).unwrap_or(70);

    let (_, tide) = current_zone_weather_and_tide(z_idx, time_tick);
    let max_tier4 = if is_coastal_hunting_zone(z_idx) && tide == ZoneTidePhase::EbbTide {
        ((base_max_tier4 as f32) * 1.15).round() as usize
    } else {
        base_max_tier4
    };

    let mut tier2_count = 0usize;
    let mut tier3_count = 0usize;
    let mut tier4_count = 0usize;

    let [n1, n2] = nearest_geographic_zones(z_idx);
    let rich_neighbor_idx = {
        let f1 = get_zone_trophic_state(n1).map(|s| s.flora_p).unwrap_or(1000.0);
        let f2 = get_zone_trophic_state(n2).map(|s| s.flora_p).unwrap_or(1000.0);
        if f1 >= f2 { n1 } else { n2 }
    };
    let &(_, rich_cx, rich_cy) = &CANONICAL_HUNTING_ZONES[rich_neighbor_idx];

    let mut out = Vec::with_capacity(base_list.len());
    let mut tier3_coords = Vec::new();

    // Pass 1: Tier 1, Tier 3, Tier 4
    for (slot_idx, &(bx, by, base_sub, tier, tier_tag, roam_group)) in
        base_list.iter().enumerate()
    {
        if tier == 2 {
            continue;
        }

        if tier == 1 {
            if !apex_present {
                continue;
            }
            out.push(ShanhaiHabitatSlot {
                slot_idx,
                base_x: bx,
                base_y: by,
                x: bx,
                y: by,
                sub_geo: base_sub,
                tier: 1,
                tier_tag,
                kind_str: "shanhai_apex",
                tier_label: "山海異獸",
                act_prefix: "潛伏於深林霧氣間，周遭百鳥屏息，正",
            });
            continue;
        }

        if tier == 3 {
            if tier3_count >= max_tier3 {
                continue;
            }
            tier3_count += 1;

            let respawn_gen = SHANHAI_SLOT_RESPAWN_GEN
                .read()
                .get(&(z_idx, slot_idx))
                .copied()
                .unwrap_or(0);

            let (gx, gy, sub_geo) = if st.as_ref().map(|s| s.flora_p < 500.0).unwrap_or(false) {
                resolve_displaced_habitat_cell(
                    bx,
                    by,
                    base_sub,
                    rich_cx,
                    rich_cy,
                    6,
                    apex_x,
                    apex_y,
                    zone_name,
                )
            } else {
                let effective_tick = time_tick.wrapping_add(respawn_gen.wrapping_mul(17) as i64);
                let rh = habitat_hash(
                    (z_idx as i64).wrapping_add(respawn_gen.wrapping_mul(7) as i64),
                    roam_group as i64,
                    effective_tick,
                );
                let rdx = ((rh % 5) as i32) - 2;
                let rdy = (((rh >> 8) % 5) as i32) - 2;
                let tx = bx + rdx;
                let ty = by + rdy;
                if ((tx - apex_x).abs() > 2 || (ty - apex_y).abs() > 2)
                    && let Some(vsub) = is_valid_wilderness_habitat_cell(tx, ty, zone_name)
                {
                    (tx, ty, vsub)
                } else {
                    (bx, by, base_sub)
                }
            };

            tier3_coords.push((gx, gy));

            let act = if let Some(s) = &st {
                if s.flora_p >= 800.0 {
                    "正低首悠然啃食肥美嫩草、"
                } else if s.flora_p < 500.0 {
                    "因周遭嫩草漸稀，正往草木豐茂處游走遷徙、"
                } else {
                    "正結伴於林間游走覓食、"
                }
            } else {
                "正結伴於林間游走覓食、"
            };

            out.push(ShanhaiHabitatSlot {
                slot_idx,
                base_x: bx,
                base_y: by,
                x: gx,
                y: gy,
                sub_geo,
                tier: 3,
                tier_tag,
                kind_str: "shanhai_herd",
                tier_label: "山林獸群",
                act_prefix: act,
            });
            continue;
        }

        if tier == 4 {
            if tier4_count >= max_tier4 {
                continue;
            }
            tier4_count += 1;

            let respawn_gen = SHANHAI_SLOT_RESPAWN_GEN
                .read()
                .get(&(z_idx, slot_idx))
                .copied()
                .unwrap_or(0);

            let effective_tick = time_tick.wrapping_add(respawn_gen.wrapping_mul(17) as i64);
            let rh = habitat_hash(
                (z_idx as i64).wrapping_add(respawn_gen.wrapping_mul(7) as i64),
                roam_group as i64,
                effective_tick,
            );
            let rdx = ((rh % 5) as i32) - 2;
            let rdy = (((rh >> 8) % 5) as i32) - 2;
            let tx = bx + rdx;
            let ty = by + rdy;
            let (gx, gy, sub_geo) = if ((tx - apex_x).abs() > 2 || (ty - apex_y).abs() > 2)
                && let Some(vsub) = is_valid_wilderness_habitat_cell(tx, ty, zone_name)
            {
                (tx, ty, vsub)
            } else {
                (bx, by, base_sub)
            };

            let lbl = if tier_tag == "__tier4a__" {
                "山野走獸"
            } else {
                "山海靈禽"
            };

            out.push(ShanhaiHabitatSlot {
                slot_idx,
                base_x: bx,
                base_y: by,
                x: gx,
                y: gy,
                sub_geo,
                tier: 4,
                tier_tag,
                kind_str: "shanhai_beast",
                tier_label: lbl,
                act_prefix: "",
            });
        }
    }

    // Pass 2: Tier 2 slots
    let now_unix = crate::game::now_unix();
    let sites_lock = RECENT_HUNT_KILL_SITES.read();
    let active_carcasses: Vec<(i32, i32)> = sites_lock
        .iter()
        .filter(|(_, site)| now_unix.saturating_sub(site.killed_at) <= 180)
        .map(|(&(cx, cy), _)| (cx, cy))
        .collect();
    drop(sites_lock);

    for (slot_idx, &(bx, by, base_sub, tier, tier_tag, roam_group)) in
        base_list.iter().enumerate()
    {
        if tier != 2 {
            continue;
        }

        if tier_tag == "__tier2_overflow__" {
            if !spillover_active {
                continue;
            }
            if tier2_count >= max_tier2 {
                continue;
            }
            tier2_count += 1;

            let (gx, gy, sub_geo) = if let Some((rx, ry)) =
                pick_zone_outskirt_road_spillover_cell(z_idx, slot_idx, time_tick)
            {
                (rx, ry, "古道外圍")
            } else {
                (bx, by, base_sub)
            };

            out.push(ShanhaiHabitatSlot {
                slot_idx,
                base_x: bx,
                base_y: by,
                x: gx,
                y: gy,
                sub_geo,
                tier: 2,
                tier_tag,
                kind_str: "shanhai_predator",
                tier_label: "山林猛獸",
                act_prefix: "因山中獵物匱乏而越界下山，正於古道草莽間伏行窺伺、",
            });
            continue;
        }

        if tier2_count >= max_tier2 {
            continue;
        }
        tier2_count += 1;

        let nearest_carcass = active_carcasses
            .iter()
            .filter_map(|&(cx, cy)| {
                let dist = (cx - bx).abs() + (cy - by).abs();
                if dist <= 65 {
                    Some((dist, cx, cy))
                } else {
                    None
                }
            })
            .min_by_key(|&(dist, ..)| dist);

        let (gx, gy, sub_geo, act_prefix) = if let Some((_, kx, ky)) = nearest_carcass {
            let (dx, dy, dsub) = resolve_displaced_habitat_cell(
                bx,
                by,
                base_sub,
                kx,
                ky,
                8,
                apex_x,
                apex_y,
                zone_name,
            );
            (dx, dy, dsub, "循著林間血氣聚攏於殘骸周遭爭食、")
        } else if (!apex_present || st.as_ref().map(|s| s.n_tier3 < s.k_tier3 * 0.70).unwrap_or(false))
            && !tier3_coords.is_empty()
        {
            let &(hx, hy) = tier3_coords
                .iter()
                .min_by_key(|&&(x, y)| (x - bx).abs() + (y - by).abs())
                .unwrap();
            let (dx, dy, dsub) = resolve_displaced_habitat_cell(
                bx,
                by,
                base_sub,
                hx,
                hy,
                7,
                apex_x,
                apex_y,
                zone_name,
            );
            let act = if !apex_present {
                "趁山中無巨獸出沒，正結伴四處巡獵、"
            } else {
                "腹中飢餓，正緊緊尾隨前方獸群伺機撲擊、"
            };
            (dx, dy, dsub, act)
        } else {
            let respawn_gen = SHANHAI_SLOT_RESPAWN_GEN
                .read()
                .get(&(z_idx, slot_idx))
                .copied()
                .unwrap_or(0);
            let effective_tick = time_tick.wrapping_add(respawn_gen.wrapping_mul(17) as i64);
            let rh = habitat_hash(
                (z_idx as i64).wrapping_add(respawn_gen.wrapping_mul(7) as i64),
                roam_group as i64,
                effective_tick,
            );
            let rdx = ((rh % 5) as i32) - 2;
            let rdy = (((rh >> 8) % 5) as i32) - 2;
            let tx = bx + rdx;
            let ty = by + rdy;
            let (dx, dy, dsub) = if ((tx - apex_x).abs() > 2 || (ty - apex_y).abs() > 2)
                && let Some(vsub) = is_valid_wilderness_habitat_cell(tx, ty, zone_name)
            {
                (tx, ty, vsub)
            } else {
                (bx, by, base_sub)
            };
            (dx, dy, dsub, "正壓低身形於草木間潛步巡獵、")
        };

        out.push(ShanhaiHabitatSlot {
            slot_idx,
            base_x: bx,
            base_y: by,
            x: gx,
            y: gy,
            sub_geo,
            tier: 2,
            tier_tag,
            kind_str: "shanhai_predator",
            tier_label: "山林猛獸",
            act_prefix,
        });
    }

    out.sort_by_key(|s| s.slot_idx);
    out
}

pub fn resolve_shanhai_or_ambient_entity_id(id: &str) -> Option<Character> {
    if let Some(rest) = id.strip_prefix("shanhai_") {
        let parts: Vec<&str> = rest.split('_').collect();
        if parts.len() == 4 {
            let z_idx: usize = parts[0].parse().ok()?;
            let slot_idx: usize = parts[1].parse().ok()?;
            let x: i32 = parts[2].parse().ok()?;
            let y: i32 = parts[3].parse().ok()?;
            let tier_tag = ZONE_HABITAT_LAYOUTS
                .get(z_idx)
                .and_then(|v| v.get(slot_idx))
                .map(|e| if e.4 == "__tier2_overflow__" { "__tier2__" } else { e.4 })
                .unwrap_or(match slot_idx {
                    0 => "__apex__",
                    1..=6 => "__tier2__",
                    _ => "__tier4b__",
                });
            if let Some(mut amb) = ensure_ambient_npc_at_grid(x, y, tier_tag) {
                amb.id = id.to_string();
                return Some(amb);
            }
        }
        return None;
    }

    if let Some(rest) = id.strip_prefix("ambient_flock_") {
        let parts: Vec<&str> = rest.split('_').collect();
        if parts.len() == 2 {
            let x: i32 = parts[0].parse().ok()?;
            let y: i32 = parts[1].parse().ok()?;
            let time_bucket = (crate::game::now_unix() / 180) as i64;
            let roam_gate = ((x as i64).wrapping_mul(1103515245)
                ^ (y as i64).wrapping_mul(12345)
                ^ time_bucket.wrapping_mul(2654435761))
                .unsigned_abs()
                % 100;
            let flock_tag = if roam_gate % 2 == 0 { "__tier4a__" } else { "__tier4b__" };
            if let Some(mut amb) = ensure_ambient_npc_at_grid(x, y, flock_tag) {
                amb.id = id.to_string();
                return Some(amb);
            }
        }
        return None;
    }

    if let Some(rest) = id.strip_prefix("ambient_") {
        let parts: Vec<&str> = rest.split('_').collect();
        if parts.len() == 2 {
            let x: i32 = parts[0].parse().ok()?;
            let y: i32 = parts[1].parse().ok()?;
            let tag = if is_apex_beast_tracked_at(x, y) { "__apex__" } else { "" };
            if let Some(mut amb) = ensure_ambient_npc_at_grid(x, y, tag) {
                amb.id = id.to_string();
                return Some(amb);
            }
        }
        return None;
    }

    None
}

/// 為「入山百工」挑選遍布該獵場全境真實疆域的採集／匠作航點，並回傳所在獵場名稱與子地形（支援休養期自動轉場）。
pub fn pick_zone_artisan_waypoint_with_zone(
    zone_name: &str,
    seed: u64,
    current: (i32, i32),
) -> ((i32, i32), &'static str, &'static str) {
    let z_idx = CANONICAL_HUNTING_ZONES
        .iter()
        .position(|(zn, _, _)| *zn == zone_name)
        .unwrap_or(0);

    let is_fallow = get_zone_trophic_state(z_idx)
        .map(|s| s.resource_fallow_mode)
        .unwrap_or(false);

    let mut target_z_idx = z_idx;
    if is_fallow && (seed % 2 == 0) {
        let neighbors = nearest_geographic_zones(z_idx);
        let richest = neighbors
            .iter()
            .copied()
            .filter_map(|nid| {
                let st = get_zone_trophic_state(nid)?;
                if st.timber_w >= 550.0 || st.mineral_m >= 550.0 || st.fishery_f >= 550.0 {
                    let total = st.timber_w + st.mineral_m + st.fishery_f;
                    Some((total, nid))
                } else {
                    None
                }
            })
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((_, best_nid)) = richest {
            target_z_idx = best_nid;
        }
    }

    let actual_zone_name: &'static str = CANONICAL_HUNTING_ZONES
        .get(target_z_idx)
        .map(|(zn, _, _)| *zn)
        .unwrap_or("長風草甸");

    let base_list = &ZONE_HABITAT_LAYOUTS[target_z_idx];
    if base_list.len() > 27 {
        let pool = &base_list[1..]; // 排除 slot 0 鎮場霸主屏息心
        let start = (seed as usize) % pool.len();
        for offset in 0..pool.len() {
            let &(px, py, sub_geo, ..) = &pool[(start + offset * 7) % pool.len()];
            if is_inside_settlement_urban_core(px, py) {
                continue;
            }
            let dist = (px - current.0).abs() + (py - current.1).abs();
            if dist >= 18 {
                return ((px, py), actual_zone_name, sub_geo);
            }
        }
        for &(px, py, sub_geo, ..) in pool {
            if !is_inside_settlement_urban_core(px, py) {
                return ((px, py), actual_zone_name, sub_geo);
            }
        }
        let &(px, py, sub_geo, ..) = &pool[start];
        return ((px, py), actual_zone_name, sub_geo);
    }
    let &(_, cx, cy) = &CANONICAL_HUNTING_ZONES[target_z_idx];
    ((cx, cy), actual_zone_name, "深山林地")
}

pub fn pick_zone_artisan_waypoint(
    zone_name: &str,
    seed: u64,
    current: (i32, i32),
) -> ((i32, i32), &'static str) {
    let (pos, _actual_zone, sub_geo) = pick_zone_artisan_waypoint_with_zone(zone_name, seed, current);
    (pos, sub_geo)
}

static ISLAND_ROAD_MILESTONES: std::sync::OnceLock<Vec<(i32, i32, &'static str)>> =
    std::sync::OnceLock::new();

fn get_island_road_milestones() -> &'static [(i32, i32, &'static str)] {
    if let Some(list) = ISLAND_ROAD_MILESTONES.get() {
        return list.as_slice();
    }
    if let Some(grid) = crate::server::grid_manager::get_runtime_square_grid() {
        let mut roads = Vec::new();
        for cell in grid.cells() {
            let coord = cell.coord;
            if (cell.terrain == crate::grid::Terrain::Road
                || cell.terrain == crate::grid::Terrain::Bridge)
                && !is_inside_settlement_urban_core(coord.x, coord.y)
            {
                let (zone, _, _) = crate::grid::r#gen::classify_wilderness_cell(coord.x, coord.y);
                roads.push((coord.x, coord.y, zone));
            }
        }
        roads.sort_by_key(|&(x, y, _)| (y, x));
        // 每隔 4 格取一處古道驛路里程點，確保均勻覆蓋全島 6,287 格驛道與古道網路
        let mut milestones = Vec::new();
        for (idx, item) in roads.into_iter().enumerate() {
            if idx % 4 == 0 {
                milestones.push(item);
            }
        }
        if !milestones.is_empty() {
            let _ = ISLAND_ROAD_MILESTONES.set(milestones);
            if let Some(list) = ISLAND_ROAD_MILESTONES.get() {
                return list.as_slice();
            }
        }
    }
    &[]
}

/// 為「古道行旅（挑夫頭、擔夫、押貨管事、刀牌手、巡隘銃手）」挑選全島驛道與聯外古道網上的真實道路航點。
pub fn pick_island_trail_waypoint(
    zone_name: &str,
    home: (i32, i32),
    errand: (i32, i32),
    seed: u64,
    current: (i32, i32),
) -> ((i32, i32), &'static str) {
    let milestones = get_island_road_milestones();
    if !milestones.is_empty() {
        // 優先挑選屬於該獵場或位於「居所 <-> 獵場 <-> 交割市集」走廊上的真實道路格
        let mid_x = (home.0 + errand.0) / 2;
        let mid_y = (home.1 + errand.1) / 2;
        let local: Vec<(i32, i32, &'static str)> = milestones
            .iter()
            .copied()
            .filter(|&(rx, ry, rzone)| {
                rzone == zone_name
                    || ((rx - mid_x).abs() <= 110 && (ry - mid_y).abs() <= 110)
                    || ((rx - home.0).abs() <= 95 && (ry - home.1).abs() <= 95)
            })
            .collect();
        let pool = if local.len() >= 6 {
            local.as_slice()
        } else {
            milestones
        };
        let start = (seed as usize) % pool.len();
        for step in 0..pool.len() {
            let (rx, ry, rzone) = pool[(start + step * 5) % pool.len()];
            let dist = (rx - current.0).abs() + (ry - current.1).abs();
            if (16..=140).contains(&dist) {
                return ((rx, ry), rzone);
            }
        }
        let (rx, ry, rzone) = pool[start];
        return ((rx, ry), rzone);
    }
    pick_zone_artisan_waypoint(zone_name, seed, current)
}

/// 19 大獵場首領異獸威壓與殘跡異兆（若該地帶或場景對應既定獵場）。
pub fn hunting_zone_apex_omen(zone_or_room: &str) -> Option<&'static str> {
    if zone_or_room.contains("長風草甸") {
        Some("草浪間隱現深深蹄印與磨角折斷的芒草叢，長風四目水鹿王與梅花鹿群奔踏而過的原野清氣猶在。")
    } else if zone_or_room.contains("古河巨木林") {
        Some("千年古樟樹幹三丈高處留有寸深的新鮮爪痕，古河樟林雲豹潛行於樹冠殘存的威壓令林間百鳥屏息。")
    } else if zone_or_room.contains("赤石坡") {
        Some("赤紅土石礦坡旁留有深邃掘洞與梅花狀貓科足印，赤石鯪鯉穿山甲與淺山錢紋石虎在此交錯出沒。")
    } else if zone_or_room.contains("落日岬") {
        Some("落日餘暉下的海岬高空盤旋著落日黑鳶王的魚尾剪影，礫灘上留有百年綠蠵龜爬行入海的寬闊軌跡。")
    } else if zone_or_room.contains("驚濤岩岸") {
        Some("怒濤拍擊的玄武岩尖端傳來破空銳嘯，驚濤遊隼王與裂浪灰海獺在狂風巨浪間各自稱雄。")
    } else if zone_or_room.contains("巨藻淺灣") {
        Some("隨潮搖曳的巨藻林間浮現百年綠蠵龜的翠綠背甲，淺灘上成群黑面琵鷺正迎著潮水展翅。")
    } else if zone_or_room.contains("黑石灣") {
        Some("玄武岩潮池中成雙成對的黑石海鱟披著青銅古甲緩步挪移，劍尾在黑石沙痕上劃出千年印記。")
    } else if zone_or_room.contains("翠冠巨木林") {
        Some("老樟樹幹三丈高處留有寸深的新鮮爪痕，千年樟雲豹王巡行殘存的淡淡樟木威壓令林間鳥雀不敢啼鳴。")
    } else if zone_or_room.contains("清泉溪谷") {
        Some("融雪清澈溪潭中銀光躍動，冰河孑遺清泉櫻花鉤吻鮭逆流躍上石階，溪畔卵石留有食蟹獴覓食足跡。")
    } else if zone_or_room.contains("北坡雲杉林") {
        Some("雲杉樹皮上留有高處抓痕與新鮮松脂，凍土雪泥上印著胸白月熊王如巨盆般的深印掌痕。")
    } else if zone_or_room.contains("西麓林道") {
        Some("晨霧瀰漫的闊葉林道間閃過一抹寶藍與朱紅光彩，西麓丹冠藍腹鷴與長尾山娘正掠過古道林隙。")
    } else if zone_or_room.contains("淺澤泥口") {
        Some("蘆葦泥澤水面浮現一對寬闊如彎月的巨角，淺澤重角水牛王靜浸泥水吐納，浮木上柴棺水龜安然曝背。")
    } else if zone_or_room.contains("白茫洲") {
        Some("腐葉與白瘴迷霧間留有碗口粗的三角菱紋滑痕，黑沼百步蛇王盤踞枯木，周遭蟲蛙盡數屏息。")
    } else if zone_or_room.contains("貼山深潭") {
        Some("碧綠如墨的深潭中央漩渦暗湧，寒潭千年花斑鱸鰻潛伏潭底石洞，水際冷石下隱現寒潭山椒魚蹤跡。")
    } else if zone_or_room.contains("青籠關") {
        Some("青籠關險峰高空迴盪著大冠鷲王清厲悠長的鳴聲，關道荊棘石縫間隱約得見錢紋石虎與剛鬃山豬的獸跡。")
    } else if zone_or_room.contains("西嶺礦谷") {
        Some("赤黑鐵礦岩壁下散落著被利爪刨開的碎礦石，裂石鯪鯉穿山甲王與青脊長鬃山羊在險峻礦脈間穿梭。")
    } else if zone_or_room.contains("東海峭壁") {
        Some("東海斷崖萬仞高空傳來撕裂長風的清厲長嘯，馭風角鷹王展開遮雲巨翼，斷崖長鬃山羊踏立絕壁。")
    } else if zone_or_room.contains("雲腰雪林") {
        Some("高山圓柏與冷杉積雪間留有修長三趾爪印，霧林黑長尾帝雉拖著雪白橫紋長尾自雪霧中優雅掠過。")
    } else if zone_or_room.contains("白塔雪巔") {
        Some("極寒冰峰石柱之上繚繞著純白清氣，雪巔霜冠大水鹿與極頂帝雉傲立雲海之巔，清氣貫通天地。")
    } else {
        None
    }
}

/// 地理環境吐納感知之天地來源與環境意象對應（供【觀境吐納】）。
/// 高山雪峰、流雲長風對應【氣源】，深潭、水澤、礁岸對應【水源】，其餘山野丘壑亦有微氣共鳴。
pub fn zone_ambient_word_source(zone_or_room: &str) -> Option<(crate::combat::wordplate::WordSource, &'static str)> {
    use crate::combat::wordplate::WordSource;

    if zone_or_room.contains("白塔雪巔")
        || zone_or_room.contains("雲腰雪林")
        || zone_or_room.contains("青籠關")
        || zone_or_room.contains("東海峭壁")
        || zone_or_room.contains("白茫洲")
        || zone_or_room.contains("風舶角")
    {
        Some((WordSource::Qi, "雲海罡風浩蕩，萬仞天風透體而過，天地至純清氣滌蕩心神。"))
    } else if zone_or_room.contains("貼山深潭")
        || zone_or_room.contains("清泉溪谷")
        || zone_or_room.contains("淺澤泥口")
        || zone_or_room.contains("驚濤岩岸")
        || zone_or_room.contains("巨藻淺灣")
        || zone_or_room.contains("落日岬")
        || zone_or_room.contains("黑石灣")
        || zone_or_room.contains("水尾寮")
        || zone_or_room.contains("烏石鼻")
    {
        Some((WordSource::Water, "碧波水精流轉，寒潭暗湧與潮聲拍岸共鳴，潤澤水靈沁潤心脾。"))
    } else if zone_or_room.contains("西嶺礦谷")
        || zone_or_room.contains("赤石坡")
        || zone_or_room.contains("紅土崁")
    {
        Some((WordSource::Mineral, "重岩地脈盤踞，金石沉著剛毅之質理厚重深邃，山骨厚土鎮守心神。"))
    } else if zone_or_room.contains("翠冠巨木林")
        || zone_or_room.contains("古河巨木林")
        || zone_or_room.contains("北坡雲杉林")
        || zone_or_room.contains("西麓林道")
        || zone_or_room.contains("長風草甸")
        || zone_or_room.contains("霜林口")
        || zone_or_room.contains("半嶺")
    {
        Some((WordSource::Plant, "草木葳蕤繁茂，萬木長青生發之氣綿綿不絕，蒼生靈機滋養百骸。"))
    } else {
        Some((WordSource::Qi, "天地吐納有節，微風徐徐流過四肢百骸，隱隱體察到周天游離清氣。"))
    }
}


/// 山海特產使用效果：
/// 回傳 `(category_tag, vit_gain, qi_gain, consume_item, effect_narrative)`。
pub fn shanhai_item_use_effect(item_id_or_name: &str) -> Option<(&'static str, i32, i32, bool, &'static str)> {
    match item_id_or_name {
        // 護身珍品／靈佩（不消耗道具）
        "wt_cat_0214" | "霧林帝雉長尾翎" => Some((
            "佩之・山海靈佩",
            0,
            10,
            false,
            "你將霧林帝雉長尾翎別於衣襟，周身湧起一陣清靈晨霧，心神寧靜如水。",
        )),
        "wt_cat_0216" | "丹冠藍腹鷴彩羽" => Some((
            "佩之・山海靈佩",
            5,
            5,
            false,
            "你撫摸著丹冠藍腹鷴的寶藍彩羽，清氣如朝霞瑞彩籠罩周身，祥瑞護持。",
        )),
        "wt_cat_0172" | "樟林雲豹軟毛皮" => Some((
            "佩之・山海靈佩",
            8,
            4,
            false,
            "你披上帶著古玉雲紋的雲豹軟毛皮，步履如風，身形輕靈如山林幽影。",
        )),
        "wt_cat_0171" | "山林石虎斑皮" => Some((
            "佩之・山海靈佩",
            6,
            3,
            false,
            "你將石虎斑皮繫於腰際，耳聰目明，黑夜中感知亦清晰入微。",
        )),
        "wt_cat_0188" | "鯪鯉穿山甲鱗片" => Some((
            "佩之・山海靈佩",
            10,
            0,
            false,
            "你手握金黑堅硬的鯪鯉麟片，筋骨經絡隱隱泛熱，肌理堅實如覆重甲。",
        )),
        "wt_cat_0176" | "泥甲巨鱷厚麟皮" => Some((
            "佩之・山海靈佩",
            12,
            0,
            false,
            "你撫過堅如頑石的巨鱷厚麟皮，如披重鎧，周身氣血穩固如山。",
        )),
        "wt_cat_0207" | "山豬王彎月獠牙" | "wt_cat_0211" | "海崖黑野豬牙" => Some((
            "佩之・山海靈佩",
            6,
            2,
            false,
            "你佩戴上如新月般剛硬的山豬尖牙，胸中湧起一股悍不畏死的山林勇氣。",
        )),
        "wt_cat_0183" | "白鼻心烏腳香皮" => Some((
            "佩之・山海靈佩",
            4,
            6,
            false,
            "你佩帶散發草果幽香的白鼻心皮，清香透鼻，令人百邪不侵、神清氣爽。",
        )),
        "wt_cat_0192" | "高山熊鷹翎羽" => Some((
            "佩之・山海靈佩",
            4,
            8,
            false,
            "你插上高山熊鷹的花翎羽毛，長風貫體，身輕體健仿若能乘風而起。",
        )),

        // 消耗性靈藥／珍餚（消耗道具）
        "wt_cat_0189" | "百步蛇乾蛻" => Some((
            "入藥・山海本草",
            10,
            15,
            true,
            "你將百步蛇乾蛻研粉調水服下，喉間微辛，一股清冽藥力順著經脈化開，深層瘴毒盡消。",
        )),
        "wt_cat_0205" | "胸白月熊厚熊掌" => Some((
            "食之・山海珍饈",
            25,
            10,
            true,
            "你細嚼慢嚥烹製的月熊厚掌，溫熱膏脂入口即化，全身暖流滾滾，四肢百骸力量沛然！",
        )),
        "wt_cat_0217" | "月熊凝白腹脂膏" => Some((
            "入藥・山海本草",
            20,
            8,
            true,
            "你將月熊凝白腹脂膏塗抹經絡或服食，肌膚溫潤生光，風寒暗傷頓時撫平。",
        )),
        "wt_cat_0064" | "深山牛樟芝" => Some((
            "入藥・山海本草",
            15,
            25,
            true,
            "你含化一片赤紅深山牛樟芝，苦盡甘來，精純元氣如瀑布般自氣海奔湧直上！",
        )),
        "wt_cat_0269" | "清溪活水黃金蜆" => Some((
            "食之・山海珍饈",
            12,
            12,
            true,
            "你烹食鮮美的清溪活水黃金蜆，湯鮮味美，五臟六腑備受滋潤，精氣神為之一振！",
        )),
        _ => None,
    }
}

/// 檢測玩家背包中是否佩戴山海護身靈佩珍品，提供戰鬥護持加成：
/// 回傳 `(talisman_name, vit_bonus, qi_bonus, bonus_narrative)`。
pub fn player_shanhai_talisman_bonus(inventory_json: &str) -> Option<(&'static str, i32, i32, &'static str)> {
    if inventory_json.contains("wt_cat_0176") {
        Some(("泥甲巨鱷厚麟皮", 12, 0, "泥甲護體，體魄固若金湯"))
    } else if inventory_json.contains("wt_cat_0188") {
        Some(("鯪鯉穿山甲鱗片", 10, 0, "烏金麟甲，氣血經絡通達無阻"))
    } else if inventory_json.contains("wt_cat_0172") {
        Some(("樟林雲豹軟毛皮", 8, 4, "古玉雲紋，身形敏捷化險為夷"))
    } else if inventory_json.contains("wt_cat_0171") {
        Some(("山林石虎斑皮", 6, 3, "石虎通靈，步法如電避實就虛"))
    } else if inventory_json.contains("wt_cat_0207") {
        Some(("山豬王彎月獠牙", 6, 2, "獠牙如月，雄渾剛毅撼動山川"))
    } else if inventory_json.contains("wt_cat_0211") {
        Some(("海崖黑野豬牙", 5, 2, "野豬尖牙，山林剛猛氣息加持"))
    } else if inventory_json.contains("wt_cat_0214") {
        Some(("霧林帝雉長尾翎", 0, 10, "帝雉白翎，清霧拂體元氣長存"))
    } else if inventory_json.contains("wt_cat_0216") {
        Some(("丹冠藍腹鷴彩羽", 5, 5, "藍鷴瑞羽，祥光普照體氣雙益"))
    } else if inventory_json.contains("wt_cat_0192") {
        Some(("高山熊鷹翎羽", 4, 8, "熊鷹花翎，神目如電長風護體"))
    } else {
        None
    }
}

/// 取得正方格附近格子的簡要視野（地圖渲染用）。
///
/// Tuple 欄位順序：`(x, y, kind, terrain, category, name, explored, walkable)`
/// - `kind`：Terrain enum 變體 snake_case（送前端切色/icon）
/// - `terrain`：中文標籤（向後相容）
/// - `category`：`"terrain"` / `"landmark"` / `"infra"`
#[allow(clippy::type_complexity)]
pub fn get_grid_cells_around(
    x: i32,
    y: i32,
    radius: i32,
) -> Vec<(i32, i32, String, String, String, String, bool, bool)> {
    let Some(grid) = crate::server::grid_manager::get_runtime_square_grid() else {
        return Vec::new();
    };
    let center = SquareCoord::new(x, y);
    let mut out = Vec::new();
    for dx in -radius..=radius {
        for dy in -radius..=radius {
            let c = SquareCoord::new(x + dx, y + dy);
            if center.chebyshev_distance(c) > radius {
                continue;
            }
            if let Some(cell) = grid.get(c) {
                out.push((
                    c.x,
                    c.y,
                    cell.terrain.kind_str().to_string(),
                    terrain_name_zh(cell.terrain).to_string(),
                    cell.terrain.category().to_string(),
                    cell.name.clone(),
                    cell.explored,
                    cell.terrain.walkable(),
                ));
            }
        }
    }
    out
}

/// 取得地形對應純符號（供前端或視圖查詢）
#[allow(dead_code)]
pub fn get_grid_cell_symbol(kind: &str) -> &'static str {
    crate::grid::Terrain::from_kind_str(kind)
        .map(|t| t.symbol())
        .unwrap_or("·")
}

#[must_use]
pub fn entity_kind_str(k: &EntityKind) -> &'static str {
    match k {
        EntityKind::Player => "player",
        EntityKind::Npc => "npc",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_TROPHIC_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_ensure_ambient_npc_at_grid() {
        let _guard = TEST_TROPHIC_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let west_gate = ensure_ambient_npc_at_grid(-18, 0, "西麓林道・道路");
        assert!(west_gate.is_some());
        let npc = west_gate.unwrap();
        assert_eq!(npc.kind, EntityKind::Npc);
        assert!(!npc.display_title.is_empty());
        assert!(!npc.current_activity.is_empty());
        assert!(npc.vit > 0);

        // 2. 獵場深處環境生物
        // 2. 獵場深處環境生物（預設顯形尋常物種／小怪，不直接刷出鎮場霸主）
        let hunting_ground = ensure_ambient_npc_at_grid(-180, -195, "長風草甸・向陽原野");
        assert!(hunting_ground.is_some());
        let creature = hunting_ground.unwrap();
        assert!(!creature.display_title.is_empty());
        assert_ne!(creature.display_title, "長風四目水鹿王");

        // 3. 尋跡顯形後鎮場霸主破霧現身
        mark_apex_beast_tracked_at(-180, -195);
        let tracked_apex = ensure_ambient_npc_at_grid(-180, -195, "長風草甸・向陽原野").unwrap();
        assert_eq!(tracked_apex.display_title, "長風四目水鹿王");

        // 4. 深水不應生成環境生物
        let deep_water = ensure_ambient_npc_at_grid(-390, 0, "北冕海・深水");
        assert!(deep_water.is_none());
    }

    #[test]
    fn test_hunting_zone_apex_omen() {
        let all_19_zones = [
            "長風草甸", "古河巨木林", "赤石坡", "落日岬", "驚濤岩岸",
            "巨藻淺灣", "黑石灣", "翠冠巨木林", "清泉溪谷", "北坡雲杉林",
            "西麓林道", "淺澤泥口", "白茫洲", "貼山深潭", "青籠關",
            "西嶺礦谷", "東海峭壁", "雲腰雪林", "白塔雪巔",
        ];
        for z in all_19_zones {
            let omen = hunting_zone_apex_omen(z);
            assert!(omen.is_some(), "Zone {z} must have an apex omen");
            let text = omen.unwrap();
            assert!(!text.contains("【山海異兆】"), "Omen must not contain bracketed system header");
            assert!(text.chars().count() >= 20);
        }
        assert!(hunting_zone_apex_omen("文曜門前大街").is_none());
    }

    #[test]
    fn test_shanhai_item_use_effect() {
        // 護身珍品：不消耗
        let (cat, vit, qi, consume, narr) = shanhai_item_use_effect("wt_cat_0214").unwrap();
        assert_eq!(cat, "佩之・山海靈佩");
        assert_eq!(vit, 0);
        assert_eq!(qi, 10);
        assert!(!consume);
        assert!(!narr.is_empty());

        let (_, _, _, consume, _) = shanhai_item_use_effect("丹冠藍腹鷴彩羽").unwrap();
        assert!(!consume);

        // 消耗性靈藥／珍餚：消耗
        let (cat, vit, qi, consume, narr) = shanhai_item_use_effect("wt_cat_0205").unwrap();
        assert_eq!(cat, "食之・山海珍饈");
        assert_eq!(vit, 25);
        assert_eq!(qi, 10);
        assert!(consume);
        assert!(!narr.is_empty());

        let (_, _, _, consume, _) = shanhai_item_use_effect("百步蛇乾蛻").unwrap();
        assert!(consume);
        let (_, _, _, consume, _) = shanhai_item_use_effect("深山牛樟芝").unwrap();
        assert!(consume);

        assert!(shanhai_item_use_effect("wt_cat_unknown").is_none());
    }

    #[test]
    fn test_player_shanhai_talisman_bonus() {
        let inv_with_croc = r#"[{"item_id":"wt_cat_0176","quantity":1}]"#;
        let bonus = player_shanhai_talisman_bonus(inv_with_croc);
        assert!(bonus.is_some());
        let (name, vit, qi, desc) = bonus.unwrap();
        assert_eq!(name, "泥甲巨鱷厚麟皮");
        assert_eq!(vit, 12);
        assert_eq!(qi, 0);
        assert!(!desc.is_empty());

        let inv_with_pheasant = r#"[{"item_id":"wt_cat_0214","quantity":1}]"#;
        let bonus = player_shanhai_talisman_bonus(inv_with_pheasant);
        assert!(bonus.is_some());
        let (name, vit, qi, _) = bonus.unwrap();
        assert_eq!(name, "霧林帝雉長尾翎");
        assert_eq!(vit, 0);
        assert_eq!(qi, 10);

        let inv_empty = r#"[]"#;
        assert!(player_shanhai_talisman_bonus(inv_empty).is_none());
    }

    #[test]
    fn test_zone_ambient_word_source() {
        use crate::combat::wordplate::WordSource;
        let qi_zone = zone_ambient_word_source("白塔雪巔").unwrap();
        assert_eq!(qi_zone.0, WordSource::Qi);
        let water_zone = zone_ambient_word_source("貼山深潭").unwrap();
        assert_eq!(water_zone.0, WordSource::Water);
        let plant_zone = zone_ambient_word_source("翠冠巨木林").unwrap();
        assert_eq!(plant_zone.0, WordSource::Plant);
        let min_zone = zone_ambient_word_source("西嶺礦谷").unwrap();
        assert_eq!(min_zone.0, WordSource::Mineral);
        let fallback_zone = zone_ambient_word_source("文曜街市").unwrap();
        assert_eq!(fallback_zone.0, WordSource::Qi);
    }

    #[test]
    fn test_trophic_cascade_on_apex_slain() {
        let _guard = TEST_TROPHIC_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let z_idx = 0; // 長風草甸
        let initial_st = get_zone_trophic_state(z_idx).expect("Zone state exists");
        assert!(initial_st.apex_presence >= 0.95);
        let slots_before = live_zone_shanhai_slots(z_idx, 100);
        assert!(slots_before.iter().any(|s| s.slot_idx == 0 && s.tier == 1));

        // Slay Tier 1 Apex
        let (_zone_name, cx, cy) = CANONICAL_HUNTING_ZONES[z_idx];
        record_wilderness_kill_at_coord(
            cx + 1,
            cy - 1,
            &format!("shanhai_{z_idx}_0_{}_{}", cx + 1, cy - 1),
            "長風四目水鹿王",
            "wt_cat_apex_loot",
            "水鹿王神角",
            "獵手",
            crate::game::now_unix(),
        );

        let st_slain = get_zone_trophic_state(z_idx).unwrap();
        assert_eq!(st_slain.apex_presence, 0.0);

        // Slot 0 should no longer be emitted in live_zone_shanhai_slots
        let slots_slain = live_zone_shanhai_slots(z_idx, 101);
        assert!(!slots_slain.iter().any(|s| s.slot_idx == 0));

        // Over 3 trophic steps: mesopredators surge (> 6.0), n_tier3 / n_tier4 dip but stay >= refuge
        for _ in 0..3 {
            step_zone_trophic_web_once(z_idx);
        }

        let st_after = get_zone_trophic_state(z_idx).unwrap();
        assert!(
            st_after.n_tier2 > 6.0,
            "Mesopredators should surge above 6.0, got {}",
            st_after.n_tier2
        );
        let ref4 = (st_after.k_tier4 * 0.18).max(6.0);
        let ref3 = (st_after.k_tier3 * 0.18).max(3.0);
        assert!(
            st_after.n_tier4 >= ref4,
            "Tier 4 count {} should stay above refuge floor {}",
            st_after.n_tier4, ref4
        );
        assert!(
            st_after.n_tier3 >= ref3,
            "Tier 3 count {} should stay above refuge floor {}",
            st_after.n_tier3, ref3
        );

        // Dynamic omen reflects apex vacancy in natural world prose
        let omen = dynamic_hunting_zone_ecology_omen("長風草甸");
        assert!(omen.is_some());
        assert!(omen.unwrap().contains("久未見巨獸蹤影"));
        // Test cleanup: restore apex_presence and n_tier2 for zone 0
        {
            RECENT_HUNT_KILL_SITES.write().remove(&(cx + 1, cy - 1));
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[z_idx].apex_presence = 1.0;
            web[z_idx].n_tier2 = 6.0;
            web[z_idx].n_tier3 = web[z_idx].k_tier3;
            web[z_idx].n_tier4 = web[z_idx].k_tier4;
        }
    }

    #[test]
    fn test_detritus_recycling_and_flora_grazing_loop() {
        let _guard = TEST_TROPHIC_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let z_idx = 1; // 古河巨木林
        let (_zone_name, cx, cy) = CANONICAL_HUNTING_ZONES[z_idx];
        let now = crate::game::now_unix();

        let old_time = now - 200;
        record_wilderness_kill_at_coord(
            cx,
            cy,
            &format!("shanhai_{z_idx}_1_{cx}_{cy}"),
            "錢紋石虎",
            "wt_cat_pelt",
            "石虎毛皮",
            "獵手",
            old_time,
        );

        {
            let sites = RECENT_HUNT_KILL_SITES.read();
            assert!(sites.contains_key(&(cx, cy)));
        }

        let detritus_before = get_zone_trophic_state(z_idx).unwrap().detritus_s;

        // Step trophic web: expired kill site should be swept and boost detritus_s by +18.0
        step_zone_trophic_web_once(z_idx);

        {
            let sites = RECENT_HUNT_KILL_SITES.read();
            assert!(!sites.contains_key(&(cx, cy)), "Expired kill site should be removed");
        }

        let st_stepped = get_zone_trophic_state(z_idx).unwrap();
        assert!(
            st_stepped.detritus_s > detritus_before,
            "Detritus should increase after kill site recycling"
        );

        // Set detritus_s >= 120.0 to verify fungi spawning and harvesting
        {
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[z_idx].detritus_s = 135.0;
        }

        let mut target_coord = None;
        for dy in -5..=5 {
            for dx in -5..=5 {
                let fx = cx + dx;
                let fy = cy + dy;
                if ((fx as i64 * 17389) ^ (fy as i64 * 49157)).unsigned_abs() % 8 == 0 {
                    target_coord = Some((fx, fy));
                    break;
                }
            }
            if target_coord.is_some() {
                break;
            }
        }
        let (fx, fy) = target_coord.expect("Found matching fungi coordinate");

        let view = get_grid_room_view("test_hunter", fx, fy, 12).unwrap().unwrap();
        let has_fungi = view.objects.iter().any(|o| o.id == format!("detritus_fungi_{fx}_{fy}"));
        assert!(has_fungi, "RoomView at ({fx}, {fy}) should contain detritus_fungi");

        let harvested = harvest_zone_detritus_fungi(fx, fy);
        assert!(harvested.is_some(), "Fungi should be harvestable when detritus_s >= 120.0");
        let (item_id, item_name) = harvested.unwrap();
        assert_eq!(item_id, "wt_cat_0064");
        assert_eq!(item_name, "深山牛樟芝");

        // Flora harvesting
        let harvest_units = harvest_hunting_zone_flora("古河巨木林", 25.0);
        assert_eq!(harvest_units, 25.0);

        // Test cleanup: restore detritus_s for zone 1
        {
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[z_idx].detritus_s = 100.0;
        }
    }

    #[test]
    fn test_spatial_taxis_and_starving_predator_road_spillover() {
        let _guard = TEST_TROPHIC_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let z_idx = 2; // 赤石坡
        let (_zone_name, cx, cy) = CANONICAL_HUNTING_ZONES[z_idx];

        // 1. Spillover activation predicate test
        {
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[z_idx].apex_presence = 1.0;
            web[z_idx].n_tier2 = 6.0;
            web[z_idx].n_tier3 = web[z_idx].k_tier3;
        }
        assert!(!is_zone_predator_spillover_active(z_idx));
        {
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[z_idx].apex_presence = 0.0;
            web[z_idx].n_tier2 = 7.0; // > 6.0 * 1.08 = 6.48
        }
        assert!(is_zone_predator_spillover_active(z_idx));

        // 2. Spillover slot appearance in live_zone_shanhai_slots
        let slots = live_zone_shanhai_slots(z_idx, 100);
        let spillover_slot = slots
            .iter()
            .find(|s| s.tier_tag == "__tier2_overflow__");
        assert!(
            spillover_slot.is_some(),
            "Overflow slot should appear when spillover is active"
        );
        let sp = spillover_slot.unwrap();
        assert_eq!(sp.sub_geo, "古道外圍");
        assert!(
            sp.act_prefix.contains("因山中獵物匱乏而越界下山"),
            "Expected outskirt spillover prefix, got: {}",
            sp.act_prefix
        );

        // 3. Carcass taxis
        let (pred_bx, pred_by) = slots
            .iter()
            .find(|s| s.tier == 2 && s.tier_tag == "__tier2__")
            .map(|s| (s.base_x, s.base_y))
            .unwrap_or((cx, cy));
        let kill_site_x = pred_bx + 4;
        let kill_site_y = pred_by + 4;
        let now = crate::game::now_unix();
        {
            let mut sites = RECENT_HUNT_KILL_SITES.write();
            sites.insert(
                (kill_site_x, kill_site_y),
                RecentHuntKillSite {
                    prey_name: "林間野鹿".to_string(),
                    loot_id: "wt_cat_0203".to_string(),
                    loot_name: "野鹿肉".to_string(),
                    hunter_label: "老獵人".to_string(),
                    killed_at: now,
                },
            );
            // Restore apex so spillover is inactive
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[z_idx].apex_presence = 1.0;
            web[z_idx].n_tier2 = 6.0;
        }

        let slots_carcass = live_zone_shanhai_slots(z_idx, 100);
        let carcass_feeder = slots_carcass
            .iter()
            .find(|s| s.tier == 2 && s.act_prefix.contains("循著林間血氣聚攏於殘骸周遭爭食"));
        assert!(
            carcass_feeder.is_some(),
            "Should have a predator displaced toward the carcass"
        );

        // Cleanup
        {
            RECENT_HUNT_KILL_SITES.write().remove(&(kill_site_x, kill_site_y));
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[z_idx].apex_presence = 1.0;
            web[z_idx].n_tier2 = 6.0;
            web[z_idx].n_tier3 = web[z_idx].k_tier3;
            web[z_idx].n_tier4 = web[z_idx].k_tier4;
        }
    }

    #[test]
    fn test_climate_tidal_coupling_and_downhill_migration() {
        let _guard = TEST_TROPHIC_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        // 1. Geographic nearest neighbors
        let neighbors_0 = nearest_geographic_zones(0);
        assert_ne!(neighbors_0[0], 0);
        assert_ne!(neighbors_0[1], 0);
        assert_ne!(neighbors_0[0], neighbors_0[1]);

        // 2. Coastal and alpine predicate tests
        assert!(is_coastal_hunting_zone(3));
        assert!(is_coastal_hunting_zone(4));
        assert!(is_coastal_hunting_zone(5));
        assert!(is_coastal_hunting_zone(6));
        assert!(is_coastal_hunting_zone(9));
        assert!(is_coastal_hunting_zone(16));
        assert!(!is_coastal_hunting_zone(0));
        assert!(!is_coastal_hunting_zone(18));

        assert!(is_alpine_hunting_zone(14));
        assert!(is_alpine_hunting_zone(17));
        assert!(is_alpine_hunting_zone(18));
        assert!(!is_alpine_hunting_zone(0));
        assert!(!is_alpine_hunting_zone(3));

        // 3. Deterministic weather & tide
        let (w0, t0) = current_zone_weather_and_tide(0, 0);
        assert_eq!(w0, ZoneWeatherPhase::RainMist);
        assert_eq!(t0, ZoneTidePhase::FloodTide);

        let (w1, t1) = current_zone_weather_and_tide(0, 1);
        assert_eq!(w1, ZoneWeatherPhase::ClearBreeze);
        assert_eq!(t1, ZoneTidePhase::EbbTide);

        let (w3, t3) = current_zone_weather_and_tide(0, 3);
        assert_eq!(w3, ZoneWeatherPhase::ColdFrost);
        assert_eq!(t3, ZoneTidePhase::FloodTide);

        // 4. Weather & Tide coupling in trophic step
        let alpine_idx = 18; // 白塔雪巔
        let mid_idx = 0; // 長風草甸
        {
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[alpine_idx].n_tier3 = web[alpine_idx].k_tier3;
            web[mid_idx].n_tier3 = web[mid_idx].k_tier3;
        }

        let alpine_tick = 1;
        assert_eq!(current_zone_weather_and_tide(alpine_idx, alpine_tick).0, ZoneWeatherPhase::ColdFrost);

        let pre_t3 = get_zone_trophic_state(alpine_idx).unwrap().n_tier3;
        {
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            let [left_idx, right_idx] = nearest_geographic_zones(alpine_idx);
            let n4_left = web[left_idx].n_tier4;
            let n3_left = web[left_idx].n_tier3;
            let n2_left = web[left_idx].n_tier2;
            let n4_right = web[right_idx].n_tier4;
            let n3_right = web[right_idx].n_tier3;
            let n2_right = web[right_idx].n_tier2;
            step_zone_equations_internal(
                &mut web[alpine_idx],
                n4_left,
                n3_left,
                n2_left,
                n4_right,
                n3_right,
                n2_right,
                alpine_tick,
            );
        }
        let post_t3 = get_zone_trophic_state(alpine_idx).unwrap().n_tier3;
        assert!(post_t3 <= pre_t3 * 1.05);

        // Cleanup
        {
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[alpine_idx].n_tier3 = web[alpine_idx].k_tier3;
            web[mid_idx].n_tier3 = web[mid_idx].k_tier3;
        }
    }

    #[test]
    fn test_gordon_schaefer_resource_pools_and_fallow_rotation() {
        let _guard = TEST_TROPHIC_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let z_idx = 0; // 長風草甸
        {
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[z_idx].timber_w = 1000.0;
            web[z_idx].mineral_m = 1000.0;
            web[z_idx].fishery_f = 1000.0;
            web[z_idx].resource_fallow_mode = false;
        }

        // 1. Harvest when abundant
        let (actual, abundant, is_fallow) = harvest_zone_natural_resource("長風草甸", "timber", 3.0);
        assert_eq!(actual, 3.0);
        assert!(abundant, "Pool was at 1000.0, so abundant bonus should be true");
        assert!(!is_fallow);

        // 2. Trigger fallow mode by depleting pool below 350.0
        {
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[z_idx].timber_w = 340.0;
            web[z_idx].resource_fallow_mode = true;
        }

        assert_eq!(is_zone_in_fallow_stewardship("長風草甸"), Some("林木"));

        // 3. Harvesting while in fallow mode returns (0.0, false, true) and restores pool by +6.0
        let (fallow_harvest, fallow_abundant, fallow_restored) =
            harvest_zone_natural_resource("長風草甸", "timber", 3.0);
        assert_eq!(fallow_harvest, 0.0);
        assert!(!fallow_abundant);
        assert!(fallow_restored);
        let restored_w = get_zone_trophic_state(z_idx).unwrap().timber_w;
        assert_eq!(restored_w, 346.0);

        // 4. Artisan waypoint rotation during fallow mode
        let ((_px, _py), rotated_zone, _sub_geo) =
            pick_zone_artisan_waypoint_with_zone("長風草甸", 0, (0, 0));
        assert_ne!(
            rotated_zone, "長風草甸",
            "Artisan waypoint should rotate to neighbor zone during fallow mode on even seed"
        );

        // 5. Recovery clears fallow mode when all pools >= 500.0
        {
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[z_idx].timber_w = 550.0;
            web[z_idx].mineral_m = 550.0;
            web[z_idx].fishery_f = 550.0;
            web[z_idx].resource_fallow_mode = false;
        }
        assert!(is_zone_in_fallow_stewardship("長風草甸").is_none());
    }

    #[test]
    fn no_design_jargon_in_hunting_zone_frontend_strings() {
        let _guard = TEST_TROPHIC_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        const FORBIDDEN: &[&str] = &[
            "五階平衡",
            "食物鏈運轉",
            "營養級聯",
            "霸主空窗",
            "霸主退位",
            "次級掠食",
            "鎮場霸主",
            "尋常中型獸群",
            "尋常小怪",
            "格內顯形",
            "尋跡顯形",
            "獵獲物質循環",
            "食腐分解閉環",
            "植被緊縮",
            "腐殖養分",
            "【山海異兆】",
            "\u{525b}\u{624d}",
            "\u{525b}\u{525b}",
            "\u{65b9}\u{624d}",
        ];
        for (z_idx, &(zone_name, _, _)) in CANONICAL_HUNTING_ZONES.iter().enumerate() {
            let omen = dynamic_hunting_zone_ecology_omen(zone_name).unwrap_or_default();
            for &bad in FORBIDDEN {
                assert!(
                    !omen.contains(bad),
                    "Zone {zone_name} dynamic omen leaked forbidden design term '{bad}': {omen}"
                );
            }
            let slots = live_zone_shanhai_slots(z_idx, 100);
            for sl in slots {
                for &bad in FORBIDDEN {
                    assert!(
                        !sl.tier_label.contains(bad),
                        "Zone {zone_name} slot tier_label leaked '{bad}': {}",
                        sl.tier_label
                    );
                    assert!(
                        !sl.act_prefix.contains(bad),
                        "Zone {zone_name} slot act_prefix leaked '{bad}': {}",
                        sl.act_prefix
                    );
                }
            }
        }

        for tick in 0..12 {
            let mut st = get_zone_trophic_state(0).unwrap();
            let [left_idx, right_idx] = nearest_geographic_zones(0);
            let l = get_zone_trophic_state(left_idx).unwrap();
            let r = get_zone_trophic_state(right_idx).unwrap();
            step_zone_equations_internal(
                &mut st,
                l.n_tier4,
                l.n_tier3,
                l.n_tier2,
                r.n_tier4,
                r.n_tier3,
                r.n_tier2,
                tick,
            );
            for &bad in FORBIDDEN {
                assert!(
                    !st.recent_trophic_event.contains(bad),
                    "Trophic event leaked '{bad}': {}",
                    st.recent_trophic_event
                );
            }
        }
    }
}
