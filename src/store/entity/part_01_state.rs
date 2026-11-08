#[derive(Debug, Clone)]
pub struct NpcGridState {
    pub current: (i32, i32),
    pub dest: (i32, i32),
    pub dest_name: String,
    pub origin_name: String,
    pub stay_activity: String,
    pub purpose: String,
    pub stay_ticks_remaining: u8,
    pub work_errand_toggle: bool,
    pub consecutive_work_streaks: u8,
}

impl NpcGridState {
    pub fn current_activity(&self) -> String {
        let hour = crate::game::current_game_hour();
        if !(5..=19).contains(&hour) {
            let is_night_venue = [
                "旅社", "客舍", "行館", "驛館", "通鋪", "歇舍", "哨", "巡防", "守更", "鐘樓", "燈塔",
            ]
            .iter()
            .any(|k| self.dest_name.contains(k))
                || (hour == 20 && ["食堂", "茶", "浴", "湯"].iter().any(|k| self.dest_name.contains(k)));

            if !is_night_venue {
                let leaf = self.dest_name.rsplit('・').next().unwrap_or(&self.dest_name);
                if self.current != self.dest {
                    return format!("正趁夜色動身返回【{}】，準備洗漱休息", self.dest_name);
                } else if hour >= 21 || hour < 5 {
                    return format!("正在{}內室木榻上熟睡休息，門窗已妥善落栓", leaf);
                } else {
                    return if self.stay_activity.contains("安歇")
                        || self.stay_activity.contains("歇息")
                        || self.stay_activity.contains("家務")
                    {
                        self.stay_activity.clone()
                    } else {
                        format!("正在{}屋內燈下收拾器物與家務，準備就寢休息", leaf)
                    };
                }
            }
        }

        if self.current == self.dest {
            self.stay_activity.clone()
        } else {
            let purpose = self.purpose.as_str();
            if purpose.contains("安歇")
                || purpose.contains("睡")
                || purpose.contains("歇息")
                || purpose.contains("歇宿")
                || purpose.contains("返家")
                || purpose.contains("回屋舍")
                || purpose.contains("回居所")
            {
                if hour >= 20 || hour < 5 {
                    format!("正踏著夜色返回【{}】準備就寢休息", self.dest_name)
                } else {
                    format!("正返回【{}】稍作歇息", self.dest_name)
                }
            } else if purpose.contains("合圍")
                || purpose.contains("揪團")
                || purpose.contains("獵殺")
                || purpose.contains("運售")
            {
                if self.purpose.contains(&self.dest_name)
                    || self.purpose.contains("結伴前往")
                    || self.stay_activity.contains(&self.dest_name)
                    || self.stay_activity.contains("結伴前往")
                {
                    self.stay_activity.clone()
                } else {
                    format!("{}（推進目標：【{}】）", self.stay_activity, self.dest_name)
                }
            } else if purpose == "享用熱食"
                || purpose == "品茗用膳"
                || purpose == "洗沐放鬆"
            {
                format!("收工之餘，正動身前往【{}】{}", self.dest_name, purpose)
            } else if !purpose.is_empty() {
                let prefixes = [
                    "回屋舍", "回居所", "在攤前", "在旅社", "在茶屋", "在貨站", "在案頭", "去", "到", "在",
                ];
                let mut trimmed = purpose;
                let mut changed = true;
                while changed {
                    changed = false;
                    for p in prefixes {
                        if let Some(rest) = trimmed.strip_prefix(p) {
                            trimmed = rest.trim_start();
                            changed = true;
                            break;
                        }
                    }
                }
                if trimmed.is_empty() {
                    format!("正動身前往【{}】", self.dest_name)
                } else {
                    format!("正動身前往【{}】，準備{}", self.dest_name, trimmed)
                }
            } else {
                format!("正動身前往【{}】", self.dest_name)
            }
        }
    }

    pub fn spatial_temporal_sense(&self) -> String {
        if self.current != self.dest {
            let steps_left = ((self.current.0 - self.dest.0).abs()
                + (self.current.1 - self.dest.1).abs())
            .max(1);
            let est_game_mins = (steps_left * 6).max(6);
            let origin_clause =
                if !self.origin_name.trim().is_empty() && self.origin_name != self.dest_name {
                    format!("自【{}】動身，", self.origin_name)
                } else {
                    String::new()
                };
            format!(
                "【在途路程與時間感】你{}正前往【{}】（目前人在半路上，尚未抵達目的地鋪面或場館），距目的地尚有約 {} 步路程（約 {} 遊戲分鐘腳程）；若來客問「在哪裡」或「還要走多久」，請如實回答目前走在當前場所路上、距【{}】還有約 {} 步路，切勿說自己已經在目的地裡頭挑貨或辦事！",
                origin_clause, self.dest_name, steps_left, est_game_mins, self.dest_name, steps_left
            )
        } else {
            let ticks = self.stay_ticks_remaining.max(1) as i32;
            let remain_game_mins = ticks * 6;
            format!(
                "【駐留狀態與時間感】你目前已抵達【{}】（{}），預計在此處還會停留約 {} 遊戲分鐘才動身前往下一處。",
                self.dest_name,
                self.current_activity(),
                remain_game_mins
            )
        }
    }
}

pub fn describe_npc_spatial_temporal_state(entity_id: &str) -> Option<String> {
    LIVE_NPC_GRID_STATES
        .read()
        .get(entity_id)
        .map(NpcGridState::spatial_temporal_sense)
}

pub static LIVE_NPC_GRID_STATES: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, NpcGridState>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

pub static LIVE_NPC_OBLIGATIONS: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, Vec<(String, String)>>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

pub fn get_active_npc_obligation(entity_id: &str) -> Option<(String, String)> {
    let map = LIVE_NPC_OBLIGATIONS.read();
    let list = map.get(entity_id)?;
    if list.is_empty() {
        return None;
    }
    if let Some(item) = list.iter().find(|(k, _)| k == "repay_debt") {
        return Some(item.clone());
    }
    if let Some(item) = list.iter().find(|(k, _)| k == "stranded_cargo") {
        return Some(item.clone());
    }
    list.last().cloned()
}

pub fn get_all_active_npc_obligations(entity_id: &str) -> Vec<(String, String)> {
    LIVE_NPC_OBLIGATIONS
        .read()
        .get(entity_id)
        .cloned()
        .unwrap_or_default()
}

pub fn open_active_npc_obligation(entity_id: &str, obligation_kind: &str, subject_ref: &str) {
    {
        let mut map = LIVE_NPC_OBLIGATIONS.write();
        let list = map.entry(entity_id.to_string()).or_default();
        if let Some(pos) = list.iter().position(|(k, _)| k == obligation_kind) {
            list[pos].1 = subject_ref.to_string();
        } else {
            if list.len() >= 6 {
                if let Some(idx) = list
                    .iter()
                    .position(|(k, _)| k != "repay_debt" && k != "stranded_cargo")
                {
                    list.remove(idx);
                } else {
                    list.remove(0);
                }
            }
            list.push((obligation_kind.to_string(), subject_ref.to_string()));
        }
    }
    if let Some(pool) = crate::pg::pool() {
        let _ = crate::pg::living_world::open_obligation(
            &pool,
            entity_id,
            obligation_kind,
            subject_ref,
            None,
            "runtime:grid_flow",
            crate::game::now_unix(),
        );
    }
}

pub fn clear_active_npc_obligation(entity_id: &str, obligation_kind: &str) {
    let k = obligation_kind.trim();
    {
        let mut map = LIVE_NPC_OBLIGATIONS.write();
        if let Some(list) = map.get_mut(entity_id) {
            if k.is_empty() {
                list.retain(|(kind, _)| kind == "repay_debt" || kind == "stranded_cargo");
            } else {
                list.retain(|(kind, _)| kind != k);
            }
            if list.is_empty() {
                map.remove(entity_id);
            }
        }
    }
    if !k.is_empty()
        && let Some(pool) = crate::pg::pool()
    {
        let _ = crate::pg::living_world::resolve_obligations_by_kind(
            &pool,
            entity_id,
            k,
            crate::game::now_unix(),
        );
    }
}

static NPC_INTERACTION_PAUSE_UNTIL: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, i64>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

pub fn pause_npc_for_interaction(entity_id: &str, seconds: i64) {
    let until = crate::game::now_unix().saturating_add(seconds.max(1));
    NPC_INTERACTION_PAUSE_UNTIL
        .write()
        .insert(entity_id.to_string(), until);
}

pub fn unpause_npc_for_interaction(entity_id: &str) {
    NPC_INTERACTION_PAUSE_UNTIL.write().remove(entity_id);
}

pub fn is_npc_paused_for_interaction(entity_id: &str) -> bool {
    let now = crate::game::now_unix();
    if let Some(&until) = NPC_INTERACTION_PAUSE_UNTIL.read().get(entity_id) {
        return now < until;
    }
    false
}

pub fn try_grace_latch_npc_to_player_cell(
    entity_id: &str,
    player_x: i32,
    player_y: i32,
    max_manhattan_dist: i32,
) -> bool {
    pause_npc_for_interaction(entity_id, 180);
    let mut states = LIVE_NPC_GRID_STATES.write();
    if let Some(state) = states.get_mut(entity_id) {
        let dist = (player_x - state.current.0).abs() + (player_y - state.current.1).abs();
        if dist <= max_manhattan_dist.max(1) {
            if dist > 0 {
                state.current = (player_x, player_y);
                state.stay_ticks_remaining = state.stay_ticks_remaining.max(2);
            }
            return true;
        }
        return false;
    }
    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HuntingPartyPhase {
    /// 階段一：於聚落獵寮／駐點吹角號召揪團集結
    Rallying,
    /// 階段二：尋跡引犬推進、鎖定活體山海獵物並展開四面戰術包抄合圍
    SurroundingPrey,
    /// 階段三：合圍收網、齊力搏殺獵物並按見者有份古禮分鹿割皮
    TakedownAndSplit,
    /// 階段四：結伴滿載凱旋、沿古道赴城鎮市集交割並均分鎂幣紅利
    ReturningToMarket,
}

#[derive(Debug, Clone)]
pub struct HuntingPartyState {
    pub zone_name: String,
    pub zone_idx: usize,
    pub center: (i32, i32),
    pub rally_coord: (i32, i32),
    pub market_coord: (i32, i32),
    pub market_name: String,
    pub leader_id: String,
    pub leader_name: String,
    pub core_member_ids: Vec<String>,
    pub member_ids: Vec<String>,
    pub phase: HuntingPartyPhase,
    pub prey_slot_idx: usize,
    pub prey_coord: (i32, i32),
    pub prey_name: String,
    pub prey_tier: u8,
    pub prey_loot_id: String,
    pub prey_loot_name: String,
    pub hunt_round: u32,
    pub last_phase_change_unix: i64,
}

pub static LIVE_HUNTING_PARTIES: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, HuntingPartyState>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

/// 玩家應募加入合社獵團之紀錄（`player_id -> zone_name`）。
pub static PLAYER_HUNTING_PARTY_PLEDGES: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, String>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

pub fn pledge_player_to_hunting_party(player_id: &str, zone_name: &str) {
    if !player_id.is_empty() && !zone_name.is_empty() {
        PLAYER_HUNTING_PARTY_PLEDGES
            .write()
            .insert(player_id.to_string(), zone_name.to_string());
    }
}

pub fn player_pledged_hunting_zone(player_id: &str) -> Option<String> {
    PLAYER_HUNTING_PARTY_PLEDGES.read().get(player_id).cloned()
}

const HUNTING_PARTY_ZONES: &[(&str, i32, i32)] = crate::game::CANONICAL_HUNTING_ZONES;

fn pick_live_shanhai_prey_for_zone(
    cx: i32,
    cy: i32,
    round: u32,
    now_unix: i64,
) -> ((i32, i32), usize, String, u8, String, String) {
    let time_tick = now_unix / 180;
    let z_idx = crate::game::CANONICAL_HUNTING_ZONES
        .iter()
        .enumerate()
        .min_by_key(|(_, (_, zx, zy))| (zx - cx).pow(2) + (zy - cy).pow(2))
        .map(|(idx, _)| idx)
        .unwrap_or(0);
    let is_aquatic = |tag: &str| {
        ["魚", "蝦", "蟹", "貝", "鰻", "鯇", "苦花", "龜", "鱉"]
            .iter()
            .any(|k| tag.contains(k))
    };
    let raw_slots = crate::game::live_zone_shanhai_slots(z_idx, time_tick);
    let slots: Vec<_> = raw_slots
        .into_iter()
        .filter(|s| !is_aquatic(s.tier_tag))
        .collect();

    let trophic_opt = crate::game::get_zone_trophic_state(z_idx);

    // 合社自適應獵律：見少留種、見害除猛獸
    let target_slot = if crate::game::is_zone_predator_spillover_active(z_idx) {
        let overflow_predators: Vec<_> = slots
            .iter()
            .filter(|s| s.tier == 2 && s.tier_tag == "__tier2_overflow__")
            .collect();
        if !overflow_predators.is_empty() {
            overflow_predators
                .get(((round as usize) + z_idx) % overflow_predators.len())
                .copied()
        } else {
            let predators: Vec<_> = slots.iter().filter(|s| s.tier == 2).collect();
            predators
                .get(((round as usize) + z_idx) % predators.len().max(1))
                .copied()
                .or_else(|| slots.first())
        }
    } else if trophic_opt
        .as_ref()
        .is_some_and(|st| st.n_tier2 > st.k_tier2 * 1.12)
    {
        // 次級掠食猛獸氾濫（除猛獸護鹿群）：優先獵捕 Tier 2 猛獸
        let predators: Vec<_> = slots.iter().filter(|s| s.tier == 2).collect();
        predators
            .get(((round as usize) + z_idx) % predators.len().max(1))
            .copied()
            .or_else(|| slots.first())
    } else if trophic_opt
        .as_ref()
        .is_some_and(|st| st.apex_presence >= 0.95 && st.n_tier3 >= st.k_tier3 * 0.72)
        && round % 10 == 9
    {
        // 鎮場霸主顯跡且草食獸群充裕：組織合社大獵挑戰 Tier 1 霸主
        slots.iter().find(|s| s.tier == 1).or_else(|| slots.first())
    } else if trophic_opt
        .as_ref()
        .is_some_and(|st| st.n_tier3 < st.k_tier3 * 0.45)
    {
        // 中型食草獸群匱乏（封山休獵留種）：避開 Tier 3，獵捕 Tier 2 掠食猛獸或 Tier 4 繁盛小型野獸
        let alt_prey: Vec<_> = slots.iter().filter(|s| s.tier == 2 || s.tier == 4).collect();
        alt_prey
            .get(((round as usize) + z_idx) % alt_prey.len().max(1))
            .copied()
            .or_else(|| slots.first())
    } else {
        // 永續輪獵：兩輪圍獵 Tier 3 尋常獸群、一輪圍獵 Tier 2 次級掠食猛獸
        if round % 3 != 2 {
            let herds: Vec<_> = slots.iter().filter(|s| s.tier == 3).collect();
            herds
                .get(((round as usize) * 3 + z_idx) % herds.len().max(1))
                .copied()
                .or_else(|| slots.first())
        } else {
            let predators: Vec<_> = slots.iter().filter(|s| s.tier == 2).collect();
            predators
                .get(((round as usize) + z_idx) % predators.len().max(1))
                .copied()
                .or_else(|| slots.first())
        }
    };

    if let Some(sl) = target_slot
        && let Some(amb) = crate::game::ensure_ambient_npc_at_grid(sl.x, sl.y, sl.tier_tag)
    {
        if !is_aquatic(&amb.display_title) {
            let (loot_id, loot_name) = crate::game::ambient_creature_loot(&amb.display_title)
                .map(|(id, name, _)| (id.to_string(), name.to_string()))
                .unwrap_or_else(|| ("mat_foraged_herb".to_string(), "山海毛皮獵物".to_string()));
            if !loot_name.contains("魚") && !is_aquatic(&loot_name) {
                return (
                    (sl.x, sl.y),
                    sl.slot_idx,
                    amb.display_title,
                    sl.tier,
                    loot_id,
                    loot_name,
                );
            }
        }
    }

    (
        (cx - 4, cy + 3),
        0,
        "高地水鹿".to_string(),
        3,
        "mat_hemp_rope_roll".to_string(),
        "厚實水鹿皮".to_string(),
    )
}

fn surround_tactical_slot(
    grid: &crate::grid::SquareGrid,
    prey_coord: (i32, i32),
    member_idx: usize,
) -> ((i32, i32), &'static str, &'static str) {
    const RING: &[(i32, i32, &str, &str)] = &[
        (
            0,
            1,
            "下風正面主伏位",
            "於下風處張滿百年山桑獵弓鎖定獵物咽喉、指揮收網",
        ),
        (
            1,
            0,
            "東翼持鏢截殺位",
            "手握倒鉤熟鐵長鏢封鎖東側退路，與隊友形成左右夾擊",
        ),
        (
            0,
            -1,
            "上風縱犬驅趕位",
            "縱放精悍山地獵犬自上風處吠叫逼壓，將獵物驅入伏擊圈",
        ),
        (
            -1,
            0,
            "西翼持盾封堵位",
            "手執精鍛闊葉山刀與藤盾死守西側林間缺口，防備獵物突圍",
        ),
        (
            1,
            1,
            "東南張網策應位",
            "於東南側拉緊粗麻索套與攔獸網，配合獵團四面收網",
        ),
        (
            -1,
            1,
            "西南後陣合圍位",
            "持鐵箍竹杖與麻索守住西南側後陣，協助獵團完成包抄",
        ),
        (
            1,
            -1,
            "東北側翼助陣位",
            "響應號召持械守住東北側翼，敲擊竹梆壓縮獵物逃竄空間",
        ),
        (
            -1,
            -1,
            "西北側翼助陣位",
            "響應號召於西北側翼布下絆索，與合社獵團合力圍困獵物",
        ),
    ];
    let (_, _, role_pos_label, role_act_desc) = RING[member_idx % RING.len()];
    for shift in 0..RING.len() {
        let (dx, dy, _, _) = RING[(member_idx + shift) % RING.len()];
        let tx = (prey_coord.0 + dx).clamp(-395, 395);
        let ty = (prey_coord.1 + dy).clamp(-495, 495);
        let walkable = grid
            .get(crate::grid::SquareCoord::new(tx, ty))
            .map(|c| {
                !matches!(
                    c.terrain,
                    crate::grid::Terrain::WaterDeep
                        | crate::grid::Terrain::Water
                        | crate::grid::Terrain::Wall
                        | crate::grid::Terrain::Fence
                )
            })
            .unwrap_or(false);
        if walkable {
            return ((tx, ty), role_pos_label, role_act_desc);
        }
    }
    (prey_coord, role_pos_label, role_act_desc)
}

fn apply_party_directive_to_state(
    party: &HuntingPartyState,
    member_idx: usize,
    grid: &crate::grid::SquareGrid,
    state: &mut NpcGridState,
) {
    let total_members = party.member_ids.len().max(1);
    let tier_tag = match party.prey_tier {
        1 => "山海異獸",
        2 => "山林猛獸",
        _ => "山林獸群",
    };
    match party.phase {
        HuntingPartyPhase::Rallying => {
            let ((rx, ry), _, _) = surround_tactical_slot(grid, party.rally_coord, member_idx);
            state.dest = (rx, ry);
            state.dest_name = format!("{}・合社獵團集結地", party.zone_name);
            state.purpose = format!("吹角揪團圍獵（{}人集結）", total_members);
            if member_idx == 0 {
                state.stay_activity = format!(
                    "正在{}吹響山豬牙號角揪團集結（目前已集結 {} 人），準備率隊進發合圍{}【{}】",
                    state.dest_name, total_members, tier_tag, party.prey_name
                );
            } else {
                state.stay_activity = format!(
                    "正響應領隊【{}】吹角號召於{}集結（共 {} 人），擦拭隨身獵具準備合圍{}【{}】",
                    party.leader_name, state.dest_name, total_members, tier_tag, party.prey_name
                );
            }
            state.stay_ticks_remaining = 3;
        }
        HuntingPartyPhase::SurroundingPrey => {
            let ((sx, sy), pos_label, act_desc) =
                surround_tactical_slot(grid, party.prey_coord, member_idx);
            state.dest = (sx, sy);
            state.dest_name = format!("{}・{}（合圍【{}】）", party.zone_name, pos_label, party.prey_name);
            state.purpose = format!("四面合圍【{}】", party.prey_name);
            state.stay_activity = format!(
                "正與領隊【{}】等 {} 名合社獵手四面合圍{}【{}】，居【{}】{}",
                party.leader_name, total_members, tier_tag, party.prey_name, pos_label, act_desc
            );
            state.stay_ticks_remaining = 3;
        }
        HuntingPartyPhase::TakedownAndSplit => {
            let ((sx, sy), pos_label, _) =
                surround_tactical_slot(grid, party.prey_coord, member_idx);
            state.dest = (sx, sy);
            state.dest_name = format!("{}・合圍收網處", party.zone_name);
            state.purpose = format!("齊力獵殺【{}】・分鹿割皮", party.prey_name);
            state.stay_activity = format!(
                "正與【{}】合社獵團（共 {} 人）於【{}】收攏包圍網齊力搏殺{}【{}】，並按見者有份古禮剝取【{}】",
                party.leader_name,
                total_members,
                pos_label,
                tier_tag,
                party.prey_name,
                party.prey_loot_name
            );
            state.stay_ticks_remaining = 3;
        }
        HuntingPartyPhase::ReturningToMarket => {
            // 返城交割時優先以市集建築本身或其門前街道為目標，避免誤指鄰近封閉民宅
            let mut mkt_dest = party.market_coord;
            if member_idx > 0 {
                const MKT_OFFSETS: &[(i32, i32)] = &[
                    (0, 1),
                    (0, -1),
                    (1, 0),
                    (-1, 0),
                    (1, 1),
                    (-1, 1),
                    (1, -1),
                    (-1, -1),
                ];
                for shift in 0..MKT_OFFSETS.len() {
                    let (dx, dy) = MKT_OFFSETS[(member_idx + shift) % MKT_OFFSETS.len()];
                    let tx = party.market_coord.0 + dx;
                    let ty = party.market_coord.1 + dy;
                    if grid
                        .get(crate::grid::SquareCoord::new(tx, ty))
                        .map(is_road_or_open_cell)
                        .unwrap_or(false)
                    {
                        mkt_dest = (tx, ty);
                        break;
                    }
                }
            }
            state.dest = mkt_dest;
            state.dest_name = party.market_name.clone();
            state.purpose = format!("結伴運售【{}】分成", party.prey_loot_name);
            state.stay_activity = format!(
                "正隨領隊【{}】等 {} 名合社獵團夥伴滿載圍獵所獲之【{}】，結伴前往{}過磅交割均分鎂幣紅利",
                party.leader_name, total_members, party.prey_loot_name, party.market_name
            );
            state.stay_ticks_remaining = 4;
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActiveNpcDialogue {
    pub room_id: String,
    pub npc_a: String,
    pub npc_b: String,
    pub text: String,
    pub updated_at: i64,
}

static ACTIVE_NPC_DIALOGUES: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, ActiveNpcDialogue>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

pub fn record_active_npc_dialogue(room_id: &str, npc_a: &str, npc_b: &str, text: &str) {
    let dialogue = ActiveNpcDialogue {
        room_id: room_id.to_string(),
        npc_a: npc_a.to_string(),
        npc_b: npc_b.to_string(),
        text: text.to_string(),
        updated_at: crate::game::now_unix(),
    };
    let mut map = ACTIVE_NPC_DIALOGUES.write();
    map.insert(npc_a.to_string(), dialogue.clone());
    map.insert(npc_b.to_string(), dialogue);
}

pub fn get_active_npc_dialogue(entity_id: &str) -> Option<ActiveNpcDialogue> {
    let now = crate::game::now_unix();
    let map = ACTIVE_NPC_DIALOGUES.read();
    if let Some(entry) = map.get(entity_id) {
        if now.saturating_sub(entry.updated_at) <= 180 {
            return Some(entry.clone());
        }
    }
    None
}

#[allow(dead_code)]
pub const PERSISTENT_NPC_ROLES: [&str; 24] = [
    "三水船塢搬運員",
    "西門車隊貨運員",
    "第一鍛造工坊技師",
    "文曜學院學生",
    "溫氏藥居採藥人",
    "三水商行押運夥計",
    "東橋茶屋常客",
    "海產乾貨商",
    "公議堂抄錄書生",
    "分家巷木工技師",
    "藏書閣校書生",
    "百工街鐵工",
    "北門巡守員",
    "三水食堂廚務員",
    "講堂街治學士",
    "織造坊裁縫師",
    "石橋攤商",
    "文曜蒙學堂助教",
    "南街藥材商",
    "鐘樓報時員",
    "泉香浴場侍者",
    "西門修配工",
    "三水川船務員",
    "外地旅客",
];

const SPECIFIC_ROLE_KEYWORDS: &[&str] = &[
    "主事", "師傅", "教習", "學徒", "醫師", "藥師", "舵手", "船夫", "鐵匠",
    "帳房", "巡衛", "茶博士", "庖師", "腳夫", "會長", "主編", "大匠", "匠師",
    "教務", "站長", "船長", "理事", "掌事", "隊長", "嚮導", "大廚", "場主",
    "木匠", "店員", "侍者", "工匠", "坊主", "教頭", "行首", "堂主", "館主",
    "衛士", "書生", "學子", "老師", "教師", "助教", "刻字", "雜役", "茶客", "行商", "貨郎", "散客",
    "員", "技師", "學生", "常客", "助理", "鐵工", "攤商", "裁縫師", "旅客", "商旅", "文吏", "客商",
];

pub fn npc_persistent_role_label(entity_id: &str, display_title: &str) -> String {
    let trimmed = display_title.trim();
    if !trimmed.is_empty() {
        if trimmed.contains('|') || trimmed.contains('｜') {
            return trimmed.to_string();
        }
        if SPECIFIC_ROLE_KEYWORDS.iter().any(|kw| trimmed.contains(kw)) {
            return trimmed.to_string();
        }
    }

    let slot = wenyao_household_slot_for(entity_id);
    let is_wenyao = slot.home_x.abs() <= 20 && slot.home_y.abs() <= 20;

    // 先剝離城市/外坊/聚落區域前綴，避免「講堂外坊・...」全數誤判為「文曜學院助教」
    let mut clean_work = slot.work_name.to_string();
    for prefix in &[
        "文曜・",
        "三水外坊・",
        "講堂外坊・",
        "百工外坊・",
        "西門外坊・",
        "三水口・",
        "西門・",
        "北門・",
        "南門・",
        "東門・",
        "風舶角・",
        "風舶嶼・",
        "大圳頭・",
        "半嶺鎮・",
        "南哨・",
        "老堰頭・",
        "北嶺腳・",
        "霜林口・",
        "風櫃尾・",
        "烏石鼻・",
        "牛埔・",
        "紅土崁・",
        "三號坑口・",
        "水尾寮・",
    ] {
        if let Some(stripped) = clean_work.strip_prefix(prefix) {
            clean_work = stripped.to_string();
        }
    }

    if slot.brief_tag.starts_with("合社獵團・")
        || slot.brief_tag.starts_with("入山百工・")
        || slot.brief_tag.starts_with("古道行旅・")
    {
        let zone_prefix = slot.work_name.split('・').next().unwrap_or("");
        if !zone_prefix.is_empty() {
            return format!("{zone_prefix}・{}", slot.brief_tag);
        }
        return slot.brief_tag.to_string();
    }
    if slot.work_x == slot.home_x && slot.work_y == slot.home_y && !slot.brief_tag.is_empty() {
        if !is_wenyao {
            return slot.brief_tag.to_string();
        }
    }

    if is_wenyao {
        if slot.attire_line.contains("磨刀") || slot.work_purpose.contains("廚刀") {
            return "三水流動磨刀匠".to_string();
        }
        if clean_work.contains("第一鐵坊") || clean_work.contains("岑氏鐵坊") {
            return "岑氏鐵坊鍛造匠師".to_string();
        }
        if clean_work.contains("巧木居所") || clean_work.contains("柯氏木作") {
            return "柯氏木作匠師".to_string();
        }
        if clean_work.contains("印行館") || clean_work.contains("印行書局") {
            return "印行館校勘與印務執事".to_string();
        }
        if clean_work == "文曜學院" || clean_work == "蒙學講堂" || clean_work.contains("學院講堂") {
            return "文曜學院助教".to_string();
        }
        if clean_work.contains("溫家藥居") || clean_work.contains("三水醫藥司") {
            return "三水醫藥司藥師".to_string();
        }
    }
    // 特定天然角色職銜判定
    if clean_work.contains("巡河") || clean_work.contains("哨所") || clean_work.contains("望樓") || clean_work.contains("瞭望") || clean_work.contains("燈塔") {
        if clean_work.ends_with("哨所") {
            let base = clean_work.trim_end_matches("哨所");
            if base.contains("巡河") {
                return format!("{base}哨官");
            } else {
                return format!("{base}巡河哨官");
            }
        }
        return format!("{clean_work}巡防執事");
    }
    if clean_work.contains("碼頭") || clean_work.contains("渡") || clean_work.contains("埠") || clean_work.contains("泊位") {
        return format!("{clean_work}船務執事");
    }
    if clean_work.contains("船塢") || clean_work.contains("工坊") || clean_work.contains("鐵器坊") || clean_work.contains("鋸木坊") || clean_work.contains("精製坊") || clean_work.contains("灰窯") {
        return format!("{clean_work}匠師");
    }
    if clean_work.contains("診所") {
        return format!("{clean_work}藥師");
    }
    if clean_work.contains("食堂") || clean_work.contains("茶") || clean_work.contains("酒") || clean_work.contains("麵") || clean_work.contains("浴") {
        return format!("{clean_work}掌事");
    }
    if clean_work.contains("商行") || clean_work.contains("帳房") || clean_work.contains("公議") || clean_work.contains("公所") || clean_work.contains("總社") || clean_work.contains("總會") || clean_work.contains("礦務所") {
        return format!("{clean_work}管事");
    }
    format!("{clean_work}執事")
}

pub static KNOWN_NPC_NAMES: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashSet<(String, String)>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashSet::new()));

pub static OBSERVED_NPCS: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashSet<(String, String)>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashSet::new()));

pub fn mark_player_knows_npc(player_id: &str, npc_id: &str) {
    if player_id.is_empty() || npc_id.is_empty() {
        return;
    }
    KNOWN_NPC_NAMES
        .write()
        .insert((player_id.to_string(), npc_id.to_string()));
    OBSERVED_NPCS
        .write()
        .insert((player_id.to_string(), npc_id.to_string()));
}

pub fn mark_player_observed_npc(player_id: &str, npc_id: &str) {
    if player_id.is_empty() || npc_id.is_empty() {
        return;
    }
    OBSERVED_NPCS
        .write()
        .insert((player_id.to_string(), npc_id.to_string()));
}

pub fn player_knows_npc(player_id: &str, npc_id: &str) -> bool {
    if player_id.is_empty() || npc_id.is_empty() {
        return false;
    }
    KNOWN_NPC_NAMES
        .read()
        .contains(&(player_id.to_string(), npc_id.to_string()))
}

pub fn player_observed_npc(player_id: &str, npc_id: &str) -> bool {
    if player_id.is_empty() || npc_id.is_empty() {
        return false;
    }
    OBSERVED_NPCS
        .read()
        .contains(&(player_id.to_string(), npc_id.to_string()))
        || KNOWN_NPC_NAMES
            .read()
            .contains(&(player_id.to_string(), npc_id.to_string()))
}

pub fn is_role_or_placeholder_label(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty()
        || t.starts_with("npc_")
        || t.starts_with("wt_")
        || t.bytes().any(|b| b.is_ascii_alphabetic())
        || t.chars().all(|c| c.is_ascii())
        || t.contains("陌生人")
        || t.contains("行人")
        || t.contains("眼前之人")
        || t.contains('·')
        || t.contains('・')
    {
        return true;
    }
    const COMPOUND_SURNAMES: &[&str] = &[
        "歐陽", "司馬", "諸葛", "上官", "司徒", "司空", "皇甫", "慕容", "宇文", "長孫", "聞人",
        "東方", "獨孤", "南宮", "萬俟", "尉遲",
    ];
    let char_count = t.chars().count();
    if char_count >= 4 && !COMPOUND_SURNAMES.iter().any(|&cs| t.starts_with(cs)) {
        return true;
    }
    const ROLE_MARKERS: &[&str] = &[
        "主事", "師傅", "教習", "學徒", "醫師", "舵手", "船夫", "鐵匠", "帳房", "巡衛",
        "茶博士", "庖師", "腳夫", "會長", "主編", "大匠", "匠師", "教務", "站長", "船長", "理事",
        "掌事", "隊長", "嚮導", "大廚", "場主", "木匠", "店員", "侍者", "工匠", "坊主", "教頭",
        "行首", "堂主", "館主", "衛士", "書生", "學子", "老師", "教師", "助教", "刻字", "雜役", "茶客", "行商",
        "貨郎", "散客", "住戶", "儒生", "馱夫", "藥童", "巡丁", "居民", "旅人", "食客", "農夫",
        "漁夫", "信使", "錄事", "門生", "大嫂", "伴當", "門房", "船戶", "渡客", "候渡者", "搬魚人",
        "補網人", "補帆人", "舶司吏", "商館人", "跑堂", "幫廚", "醫師", "藥師", "醫者", "鏢師",
        "武師", "挑夫", "車夫", "馬夫", "樵夫", "女工", "記帳員", "手藝人", "採藥人",
        "員", "技師", "學生", "常客", "助理", "鐵工", "攤商", "裁縫師", "旅客", "商旅", "文吏", "客商",
        "補靴", "行頭", "一刀", "觀星", "大犁",
    ];
    ROLE_MARKERS.iter().any(|&kw| t.contains(kw))
}

#[allow(dead_code)]
pub fn deterministic_wenyao_full_name(npc_id: &str) -> String {
    deterministic_wenyao_full_name_with_gender(npc_id, None)
}

pub fn npc_effective_is_female(entity_id: &str, gender_hint: Option<&str>) -> bool {
    let mut resolved_gender = gender_hint.map(str::to_string);
    if resolved_gender.is_none() {
        if let Some(arc) = super::get_store()
            && let Some(s) = arc.try_read()
            && let Some(ent) = s.get_entity(entity_id)
            && !ent.gender.is_empty()
        {
            resolved_gender = Some(ent.gender.clone());
        }
    }

    if !entity_id.starts_with("npc_") {
        return match resolved_gender.as_deref() {
            Some("F") | Some("f") | Some("女") | Some("female") | Some("女性") => true,
            Some("M") | Some("m") | Some("男") | Some("male") | Some("男性") => false,
            _ => false,
        };
    }

    let slot = wenyao_household_slot_for(entity_id);
    let attire_no_apron = slot.attire_line.replace("圍裙", "");
    let tag_no_apron = slot.brief_tag.replace("圍裙", "");
    let slot_female_clue = attire_no_apron.contains("裙")
        || slot.attire_line.contains("簪")
        || slot.attire_line.contains("髮髻")
        || tag_no_apron.contains("裙")
        || slot.work_purpose.contains("女")
        || slot.work_stay_activity.contains("女")
        || slot.home_stay_activity.contains("女")
        || slot.attire_line.contains("娘")
        || slot.attire_line.contains("小姐")
        || slot.attire_line.contains("婦");
    let slot_male_clue = slot.attire_line.contains("鬚")
        || slot.attire_line.contains("鬍")
        || slot.attire_line.contains("赤膊")
        || slot.attire_line.contains("雙臂赤裸")
        || slot.attire_line.contains("雄健")
        || slot.attire_line.contains("魁梧")
        || slot.attire_line.contains("老漢")
        || slot.attire_line.contains("壯漢");

    if slot_female_clue {
        true
    } else if slot_male_clue {
        false
    } else {
        match resolved_gender.as_deref() {
            Some("F") | Some("f") | Some("女") | Some("female") | Some("女性") => true,
            Some("M") | Some("m") | Some("男") | Some("male") | Some("男性") => false,
            _ => false,
        }
    }
}

pub const MALE_ONLY_NAME_CHARS: &[char] = &[
    '強', '偉', '\u{525b}', '勇', '彪', '豪', '傑', '杰', '俊', '雄', '建', '國', '軍', '磊', '濤', '鵬',
    '飛', '超', '康', '邦', '霸', '武', '猛', '龍', '虎', '志', '洋',
    '正', '明', '德', '興', '良', '貴', '富', '榮', '昌', '宏', '盛', '峰', '棟', '川', '山', '海',
    '軒', '宇', '翰', '廷', '翔', '維', '瑋', '凱', '鈞', '睿', '佑', '恩', '霖', '哲',
];

pub const FEMALE_ONLY_NAME_CHARS: &[char] = &[
    '淑', '娟', '婷', '慧', '敏', '麗', '美', '芳', '玲', '秀', '霞', '燕', '艷', '蘭', '萍', '梅',
    '琳', '琴', '娜', '芬', '靜', '妍', '嫣', '薇', '姿', '妤', '媛', '嬌', '婉', '嫺', '嫻', '蓉',
    '蕊', '惠', '琪', '紅', '雅', '晴', '雯', '馨', '佩', '碧', '素', '映', '月', '穎', '彤', '怡',
    '涵', '瑜', '珊', '瑄', '妮', '樺', '欣', '玥', '柔', '萱', '儀', '筑', '依', '荷', '翠', '姝',
    '韻', '微', '若',
];

pub const WENYAO_SURNAMES: &[&str] = &[
    "陳", "林", "黃", "張", "李", "王", "吳", "劉", "蔡", "楊",
    "許", "鄭", "謝", "洪", "郭", "邱", "曾", "廖", "賴", "徐",
    "周", "葉", "蘇", "莊", "江", "呂", "何", "羅", "高", "蕭",
    "柯", "溫", "秦", "顧", "沈", "\u{65b9}", "岑", "陸", "白", "韓",
    "趙", "孫", "馬", "朱", "胡", "戴", "潘", "鄧", "馮", "曹",
    "彭", "范", "石", "姚", "余", "魏", "薛", "鍾", "宋", "段",
];

pub const WENYAO_FEMALE_GIVEN: &[&str] = &[
    "雅婷", "雅筑", "婷萱", "妤涵", "晴雯", "琳琪", "柔馨", "潔儀",
    "欣真", "宜佩", "慈惠", "秀娟", "雅竹", "婉清", "素心", "芷蘭",
    "佩玉", "碧雲", "映雪", "淑敏", "秋月", "秀英", "婉君", "佳穎",
    "子晴", "宥彤", "語晴", "品妤", "心怡", "筱涵", "芷瑜", "佩珊",
    "庭瑄", "雅柔", "靜雯", "碧華", "映辰", "彩妮", "青樺", "淑芬",
    "美玲", "淑惠", "詠晴", "若彤", "依晨", "彩薇", "涵玥", "芷嫣",
    "曼琳", "詩涵", "依婷", "佳欣",
    "語彤", "韻如", "知微", "若水", "靜姝", "曉萱", "雅雯", "淑娟",
    "玉蘭", "清荷", "秀芳", "翠玉",
];

pub const WENYAO_MALE_GIVEN: &[&str] = &[
    "冠廷", "宇翔", "志豪", "俊傑", "柏翰", "冠宇", "志明", "家豪",
    "宗翰", "承恩", "宇軒", "冠霖", "哲維", "振武", "建興", "致遠",
    "子傑", "明哲", "清和", "懷安", "文軒", "正明", "長青", "明德",
    "家瑋", "文華", "豐年", "志強", "忠義", "宏偉", "崇安", "毅宏",
    "銘凱", "駿宏", "皓天", "景耀", "啟銘", "承宏", "秉諺", "耀庭",
    "翔宇", "彥良", "凱鈞", "瀚睿", "睿佑", "辰威", "建宏", "冠良",
    "子豪", "政憲", "廷宇", "博文",
    "少華", "敬堯", "子軒", "定遠", "廷恩", "哲瑋", "承澤", "柏睿",
    "敬軒", "松年", "安邦", "紹齊",
];

