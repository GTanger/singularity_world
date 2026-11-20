//! 輕量版 RimWorld 平面大地圖即時模擬狀態、持久化與玩家指令介面。

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use super::rim_seed::{apply_outpost_architecture, ensure_outpost_residents, seed_outpost};
use super::rim_world::{GroundStack, MicroCellOverlay, RimItemKind, StructureKind};
use super::runtime;

const SAVE_REL_PATH: &str = "data/runtime/rim_sandbox_state.json";
static FLUSH_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

pub(crate) fn now_unix_sec() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorFaction {
    Settler,
    WildBeast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PendingTask {
    None,
    ChopAt {
        tx: i32,
        ty: i32,
    },
    FetchForBuild {
        src_x: i32,
        src_y: i32,
        dst_x: i32,
        dst_y: i32,
    },
    WorkBuild {
        tx: i32,
        ty: i32,
    },
    DeconstructAt {
        tx: i32,
        ty: i32,
    },
    HaulToStockpile {
        src_x: i32,
        src_y: i32,
        dst_x: i32,
        dst_y: i32,
    },
    CraftAt {
        tx: i32,
        ty: i32,
        item: RimItemKind,
    },
    PickupEquip {
        tx: i32,
        ty: i32,
    },
    RestAt {
        tx: i32,
        ty: i32,
    },
    DrinkAt {
        tx: i32,
        ty: i32,
    },
    CookAt {
        tx: i32,
        ty: i32,
        raw: RimItemKind,
    },
    ChaseTarget {
        target_id: usize,
    },
    RescueAlly {
        target_id: usize,
    },
    CaravanMove {
        tx: i32,
        ty: i32,
        leg: u8,
    },
    SleepBed {
        tx: i32,
        ty: i32,
    },
    FarmPlot {
        tx: i32,
        ty: i32,
        op: super::rim_ecology::FarmOp,
    },
    StorehouseTrade {
        tx: i32,
        ty: i32,
    },
    ShopPatron {
        tx: i32,
        ty: i32,
        op: super::rim_shops::ShopServiceOp,
    },
}

fn default_gender() -> String {
    "M".to_string()
}

fn default_need_val() -> u8 {
    88
}

fn default_magnesium() -> u32 {
    18
}

/// 場上所有人形生物與山海異獸共用之實體結構（含真實帳號綁定、性別與骨相種子）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RimActor {
    pub id: usize,
    pub code: String,
    #[serde(default)]
    pub actor_key: String,
    #[serde(default)]
    pub entity_id: Option<String>,
    pub glyph: String,
    pub name: String,
    #[serde(default = "default_gender")]
    pub gender: String,
    #[serde(default)]
    pub soul_seed: i64,
    pub role_tag: String,
    pub faction: ActorFaction,
    pub is_player: bool,
    pub autopilot: bool,
    #[serde(default)]
    pub running: bool,
    pub px: f32,
    pub py: f32,
    pub facing_rad: f32,
    pub hp: i32,
    pub max_hp: i32,
    pub equipped_tool: Option<RimItemKind>,
    pub carrying: Option<GroundStack>,
    #[serde(default)]
    pub inventory: Vec<GroundStack>,
    #[serde(default)]
    pub directive: String,
    pub path: VecDeque<(i32, i32)>,
    pub task: PendingTask,
    pub activity_tag: String,
    pub pop_text: Option<(String, u64)>,
    pub cooldown_ticks: u8,
    #[serde(default)]
    pub last_active_unix: i64,
    #[serde(default = "default_need_val")]
    pub hunger: u8,
    #[serde(default = "default_need_val")]
    pub thirst: u8,
    #[serde(default = "default_need_val")]
    pub stamina: u8,
    #[serde(default = "default_magnesium")]
    pub magnesium: u32,
    #[serde(default)]
    pub equipped_hat: bool,
    #[serde(default)]
    pub equipped_pack: bool,
    #[serde(default)]
    pub known_by: Vec<String>,
    #[serde(default)]
    pub practice_unarmed: u32,
    #[serde(default)]
    pub practice_blade: u32,
    #[serde(default)]
    pub practice_blunt: u32,
    #[serde(default)]
    pub craft_harvest: u32,
    #[serde(default)]
    pub craft_forge: u32,
    #[serde(default)]
    pub craft_culinary: u32,
    #[serde(default)]
    pub craft_wayfare: u32,
    #[serde(default)]
    pub anatomy_winged: u32,
    #[serde(default)]
    pub anatomy_hoofed: u32,
    #[serde(default)]
    pub injury_part: Option<String>,
    #[serde(default)]
    pub injury_ticks: u16,
    #[serde(default)]
    pub downed_ticks: u8,
    #[serde(default)]
    pub caravan_leg: u8,
    #[serde(default)]
    pub wet_ticks: u8,
    #[serde(default)]
    pub recent_diary: VecDeque<String>,
    #[serde(default)]
    pub civic: super::rim_shops::ActorCivicState,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActorPhaseBlob {
    #[serde(default)]
    pub practice_unarmed: u32,
    #[serde(default)]
    pub practice_blade: u32,
    #[serde(default)]
    pub practice_blunt: u32,
    #[serde(default)]
    pub craft_harvest: u32,
    #[serde(default)]
    pub craft_forge: u32,
    #[serde(default)]
    pub craft_culinary: u32,
    #[serde(default)]
    pub craft_wayfare: u32,
    #[serde(default)]
    pub anatomy_winged: u32,
    #[serde(default)]
    pub anatomy_hoofed: u32,
    #[serde(default)]
    pub injury_part: Option<String>,
    #[serde(default)]
    pub injury_ticks: u16,
    #[serde(default)]
    pub downed_ticks: u8,
    #[serde(default)]
    pub caravan_leg: u8,
    #[serde(default)]
    pub wet_ticks: u8,
    #[serde(default)]
    pub recent_diary: Vec<String>,
    #[serde(default)]
    pub civic: super::rim_shops::ActorCivicState,
}

impl RimActor {
    pub fn micro_pos(&self) -> (i32, i32) {
        (self.px.round() as i32, self.py.round() as i32)
    }

    /// 依觀看者視角回傳對方稱呼（未結識之聚落居民先顯示陌生身分，互通名姓後顯示真名）。
    pub fn display_name_for(&self, viewer_pid: Option<&str>) -> String {
        if self.faction == ActorFaction::WildBeast
            || viewer_pid.is_some_and(|pid| self.entity_id.as_deref() == Some(pid))
            || viewer_pid.is_some_and(|pid| self.known_by.iter().any(|k| k == pid))
        {
            return self.name.clone();
        }
        let r = self.role_tag.as_str();
        if r.contains("獵戶") {
            "陌生獵戶".into()
        } else if r.contains("鐵") {
            if r.contains("學徒") {
                "陌生鐵徒".into()
            } else {
                "陌生鐵匠".into()
            }
        } else if r.contains("醫") || r.contains("藥") {
            if self.gender == "F" {
                "陌生醫女".into()
            } else {
                "陌生醫師".into()
            }
        } else if r.contains("宿") {
            "陌生掌櫃".into()
        } else if r.contains("小吃") || r.contains("杓") || r.contains("廚") {
            if self.gender == "F" {
                "陌生廚娘".into()
            } else {
                "陌生幫廚".into()
            }
        } else if r.contains("茶") {
            if self.gender == "F" {
                "陌生茶娘".into()
            } else {
                "陌生焙師".into()
            }
        } else if r.contains("山民") || r.contains("農") {
            "陌生農戶".into()
        } else if r.contains("掌櫃") || r.contains("夥計") {
            "陌生店家".into()
        } else if r.contains("商") || r.contains("貨郎") {
            "陌生貨郎".into()
        } else {
            "陌生街坊".into()
        }
    }

    pub fn health_state_tag(&self) -> String {
        if self.downed_ticks > 0 {
            return "力竭昏迷".into();
        }
        let ratio = (self.hp * 100) / self.max_hp.max(1);
        let base = match ratio {
            85..=200 => "氣息勻暢",
            50..=84 => "皮肉擦傷",
            25..=49 => "氣血震盪",
            _ => "步履踉蹌",
        };
        if self.injury_ticks > 0
            && let Some(ref part) = self.injury_part
        {
            format!("{base}・{part}")
        } else {
            base.into()
        }
    }

    pub fn effective_seed(&self) -> i64 {
        super::rim_phase::effective_seed(self)
    }

    pub fn push_diary(&mut self, entry: impl Into<String>) {
        let text = entry.into();
        if text.trim().is_empty() {
            return;
        }
        if self.recent_diary.front().is_some_and(|f| f == &text) {
            return;
        }
        self.recent_diary.push_front(text);
        while self.recent_diary.len() > 8 {
            self.recent_diary.pop_back();
        }
    }

    pub fn phase_json(&self) -> String {
        let blob = ActorPhaseBlob {
            practice_unarmed: self.practice_unarmed,
            practice_blade: self.practice_blade,
            practice_blunt: self.practice_blunt,
            craft_harvest: self.craft_harvest,
            craft_forge: self.craft_forge,
            craft_culinary: self.craft_culinary,
            craft_wayfare: self.craft_wayfare,
            anatomy_winged: self.anatomy_winged,
            anatomy_hoofed: self.anatomy_hoofed,
            injury_part: self.injury_part.clone(),
            injury_ticks: self.injury_ticks,
            downed_ticks: self.downed_ticks,
            caravan_leg: self.caravan_leg,
            wet_ticks: self.wet_ticks,
            recent_diary: self.recent_diary.iter().cloned().collect(),
            civic: self.civic.clone(),
        };
        serde_json::to_string(&blob).unwrap_or_else(|_| "{}".into())
    }

    pub fn apply_phase_json(&mut self, raw: &str) {
        if raw.trim().is_empty() || raw.trim() == "{}" {
            return;
        }
        if let Ok(b) = serde_json::from_str::<ActorPhaseBlob>(raw) {
            self.practice_unarmed = b.practice_unarmed;
            self.practice_blade = b.practice_blade;
            self.practice_blunt = b.practice_blunt;
            self.craft_harvest = b.craft_harvest;
            self.craft_forge = b.craft_forge;
            self.craft_culinary = b.craft_culinary;
            self.craft_wayfare = b.craft_wayfare;
            self.anatomy_winged = b.anatomy_winged;
            self.anatomy_hoofed = b.anatomy_hoofed;
            self.injury_part = b.injury_part;
            self.injury_ticks = b.injury_ticks;
            self.downed_ticks = b.downed_ticks;
            self.caravan_leg = b.caravan_leg;
            self.wet_ticks = b.wet_ticks;
            self.recent_diary = b.recent_diary.into_iter().take(8).collect();
            self.civic = b.civic;
        }
    }

    pub fn add_inv(&mut self, item: RimItemKind, count: u16) {
        if count == 0 {
            return;
        }
        if let Some(slot) = self.inventory.iter_mut().find(|s| s.item == item) {
            slot.count = slot.count.saturating_add(count);
        } else {
            self.inventory.push(GroundStack { item, count });
        }
    }

    pub fn take_inv(&mut self, item: RimItemKind, max_take: u16) -> u16 {
        let Some(pos) = self
            .inventory
            .iter()
            .position(|s| s.item == item && s.count > 0)
        else {
            return 0;
        };
        let taken = self.inventory[pos].count.min(max_take);
        self.inventory[pos].count -= taken;
        if self.inventory[pos].count == 0 {
            self.inventory.remove(pos);
        }
        taken
    }

    pub fn natural_skills(&self) -> Vec<super::rim_phase::SkillChipDto> {
        super::rim_phase::synthesize_actor_skill_chips(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortEvent {
    pub tick: u64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RimWorldState {
    pub tick: u64,
    pub overlays: HashMap<(i32, i32), MicroCellOverlay>,
    pub actors: Vec<RimActor>,
    pub events: VecDeque<ShortEvent>,
    #[serde(default)]
    pub economy: super::rim_economy::RimEconomyBook,
    #[serde(skip)]
    pub dirty_unsaved_ticks: u32,
    #[serde(skip)]
    pub sub_step: u8,
    #[serde(skip)]
    pub stall_tracker: HashMap<usize, ((f32, f32), u8)>,
}

static GLOBAL_RIM: OnceLock<Mutex<RimWorldState>> = OnceLock::new();

pub fn global_rim() -> &'static Mutex<RimWorldState> {
    GLOBAL_RIM.get_or_init(|| Mutex::new(load_or_seed_world()))
}

fn save_path() -> PathBuf {
    PathBuf::from(SAVE_REL_PATH)
}

#[derive(Serialize, Deserialize)]
struct PersistedRim {
    tick: u64,
    cells: Vec<(i32, i32, MicroCellOverlay)>,
    actors: Vec<RimActor>,
    events: VecDeque<ShortEvent>,
    #[serde(default)]
    economy: super::rim_economy::RimEconomyBook,
}

fn load_or_seed_world() -> RimWorldState {
    let rt = runtime::global();
    if let Some(pool) = crate::pg::pool() {
        let _ = crate::pg::rim_persist::ensure_schema(&pool);
        let db_ov = crate::pg::rim_persist::load_all_overlays(&pool).unwrap_or_default();
        let mut db_actors = crate::pg::rim_persist::load_all_actors(&pool).unwrap_or_default();
        if !db_ov.is_empty() || !db_actors.is_empty() {
            let mut overlays = db_ov;
            let mut migrated = false;
            if overlays.is_empty() {
                apply_outpost_architecture(rt, &mut overlays);
                migrated = true;
            } else if !overlays
                .get(&(31, -7))
                .is_some_and(|o| o.structure == Some(StructureKind::Campfire))
            {
                super::rim_seed::apply_shuoweiliao_outpost(rt, &mut overlays);
                for a in &mut db_actors {
                    if a.name == "周阿茶" {
                        a.px = 30.0;
                        a.py = -7.0;
                    } else if a.name == "鄭石頭" {
                        a.px = 33.0;
                        a.py = -6.0;
                    }
                }
                migrated = true;
            }
            ensure_outpost_residents(&mut db_actors);
            if super::rim_life::recover_stranded_world_state(&mut overlays, &mut db_actors) {
                migrated = true;
            }
            if super::rim_shelter::upgrade_existing_shelters(&mut overlays) {
                migrated = true;
            }
            if super::rim_ecology::reconcile_real_scale_layout_and_clutter(rt, &mut overlays) {
                migrated = true;
            }
            let town_v24b =
                overlays.get(&(10, -1)).and_then(|o| o.structure) == Some(StructureKind::Workbench);
            super::rim_town::apply_town_architecture(rt, &mut overlays);
            if !town_v24b {
                migrated = true;
            }
            let economy = crate::pg::rim_persist::load_economy_book(&pool).unwrap_or_default();
            let mut events = VecDeque::new();
            events.push_back(ShortEvent {
                tick: 0,
                text: "山風拂過竹梢，湧泉水聲清淺".into(),
            });
            let mut st = RimWorldState {
                tick: 0,
                overlays,
                actors: db_actors,
                events,
                economy,
                dirty_unsaved_ticks: if migrated { 25 } else { 0 },
                sub_step: 0,
                stall_tracker: HashMap::new(),
            };
            st.resolve_solid_collisions(rt, 6);
            let prev_vault = st.economy.vault_mg;
            super::rim_economy::step_economy_tick(&mut st);
            if st.economy.vault_mg != prev_vault {
                migrated = true;
            }
            if migrated {
                st.persist_if_needed(true);
            }
            return st;
        }
    }

    let p = save_path();
    if let Ok(raw) = std::fs::read_to_string(&p)
        && let Ok(saved) = serde_json::from_str::<PersistedRim>(&raw)
        && !saved.actors.is_empty()
    {
        let mut overlays: HashMap<(i32, i32), MicroCellOverlay> = saved
            .cells
            .into_iter()
            .filter(|(_, _, ov)| !ov.is_pristine())
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(x, y, ov)| ((x, y), ov))
            .collect();
        apply_outpost_architecture(rt, &mut overlays);
        let _ = super::rim_shelter::upgrade_existing_shelters(&mut overlays);
        let _ = super::rim_ecology::reconcile_real_scale_layout_and_clutter(rt, &mut overlays);
        super::rim_town::apply_town_architecture(rt, &mut overlays);
        let mut actors = saved.actors;
        ensure_outpost_residents(&mut actors);
        let mut events: VecDeque<ShortEvent> = saved
            .events
            .into_iter()
            .filter(|e| !e.text.contains("資料庫") && !e.text.contains("前哨已就緒"))
            .collect();
        if events.is_empty() {
            events.push_back(ShortEvent {
                tick: 0,
                text: "山風拂過竹梢，湧泉水聲清淺".into(),
            });
        }
        let mut st = RimWorldState {
            tick: saved.tick,
            overlays,
            actors,
            events,
            economy: saved.economy,
            dirty_unsaved_ticks: 1,
            sub_step: 0,
            stall_tracker: HashMap::new(),
        };
        st.resolve_solid_collisions(rt, 6);
        st.persist_if_needed(true);
        return st;
    }

    let mut st = RimWorldState::fresh_seed();
    st.persist_if_needed(true);
    st
}

impl RimWorldState {
    pub fn fresh_seed() -> Self {
        let (overlays, actors, events) = seed_outpost(runtime::global());
        Self {
            tick: 0,
            overlays,
            actors,
            events,
            economy: Default::default(),
            dirty_unsaved_ticks: 1,
            sub_step: 0,
            stall_tracker: HashMap::new(),
        }
    }

    pub fn push_event(&mut self, text: impl Into<String>) {
        self.events.push_front(ShortEvent {
            tick: self.tick,
            text: text.into(),
        });
        while self.events.len() > 6 {
            self.events.pop_back();
        }
    }

    pub fn persist_if_needed(&mut self, force: bool) {
        if !force && self.dirty_unsaved_ticks < 20 {
            return;
        }
        self.dirty_unsaved_ticks = 0;
        if let Some(pool) = crate::pg::pool() {
            if FLUSH_IN_FLIGHT.swap(true, Ordering::AcqRel) && !force {
                return;
            }
            let overlays = self.overlays.clone();
            let actors = self.actors.clone();
            let economy = self.economy.clone();
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn_blocking(move || {
                    if let Err(e) =
                        crate::pg::rim_persist::flush_snapshot(&pool, &overlays, &actors, &economy)
                    {
                        tracing::warn!("rim_persist flush error: {e}");
                    }
                    FLUSH_IN_FLIGHT.store(false, Ordering::Release);
                });
            } else {
                let _ = crate::pg::rim_persist::flush_snapshot(&pool, &overlays, &actors, &economy);
                FLUSH_IN_FLIGHT.store(false, Ordering::Release);
            }
            return;
        }
        let payload = PersistedRim {
            tick: self.tick,
            cells: self
                .overlays
                .iter()
                .filter(|(_, ov)| !ov.is_pristine())
                .map(|(&(x, y), ov)| (x, y, ov.clone()))
                .collect(),
            actors: self.actors.clone(),
            events: self.events.clone(),
            economy: self.economy.clone(),
        };
        if let Ok(json) = serde_json::to_string(&payload) {
            if let Some(parent) = save_path().parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(save_path(), json);
        }
    }

    /// 依真實帳號登入結果取得或建立專屬 2D 角色實體（絕不與其他帳號混用）。
    pub fn ensure_player_actor(
        &mut self,
        pid: &str,
        glyph: &str,
        gender: &str,
        soul_seed: i64,
    ) -> usize {
        let now = now_unix_sec();
        let norm_gender = if gender == "F" || gender == "女" {
            "F".to_string()
        } else {
            "M".to_string()
        };
        let norm_glyph = if glyph.trim().is_empty() {
            pid.chars()
                .next()
                .map_or_else(|| "我".into(), |c| c.to_string())
        } else {
            glyph.trim().chars().take(1).collect()
        };
        if let Some(pos) = self
            .actors
            .iter()
            .position(|a| a.entity_id.as_deref() == Some(pid))
        {
            self.actors[pos].glyph = norm_glyph;
            self.actors[pos].name = pid.to_string();
            self.actors[pos].gender = norm_gender;
            if soul_seed != 0 {
                self.actors[pos].soul_seed = soul_seed;
            }
            self.actors[pos].last_active_unix = now;
            self.dirty_unsaved_ticks += 5;
            self.resolve_solid_collisions(runtime::global(), 6);
            return pos;
        }
        let next_id = self.actors.iter().map(|a| a.id).max().unwrap_or(10) + 1;
        let actor_key = format!("player:{pid}");
        self.actors.push(RimActor {
            id: next_id,
            code: actor_key.clone(),
            actor_key,
            entity_id: Some(pid.to_string()),
            glyph: norm_glyph,
            name: pid.to_string(),
            gender: norm_gender,
            soul_seed,
            role_tag: "親身行動".into(),
            faction: ActorFaction::Settler,
            is_player: false,
            autopilot: false,
            running: false,
            px: 0.0,
            py: -2.0,
            facing_rad: 1.57,
            hp: 100,
            max_hp: 100,
            equipped_tool: Some(RimItemKind::AlloyBlade),
            carrying: None,
            inventory: vec![
                GroundStack {
                    item: RimItemKind::BambooPole,
                    count: 2,
                },
                GroundStack {
                    item: RimItemKind::StonePick,
                    count: 1,
                },
            ],
            directive: "forage".into(),
            path: VecDeque::new(),
            task: PendingTask::None,
            activity_tag: "駐足觀望".into(),
            pop_text: None,
            cooldown_ticks: 0,
            last_active_unix: now,
            hunger: 88,
            thirst: 90,
            stamina: 92,
            magnesium: 18,
            equipped_hat: true,
            equipped_pack: true,
            known_by: Vec::new(),
            practice_unarmed: 0,
            practice_blade: 0,
            practice_blunt: 0,
            craft_harvest: 0,
            craft_forge: 0,
            craft_culinary: 0,
            craft_wayfare: 0,
            anatomy_winged: 0,
            anatomy_hoofed: 0,
            injury_part: None,
            injury_ticks: 0,
            downed_ticks: 0,
            caravan_leg: 0,
            wet_ticks: 0,
            recent_diary: VecDeque::new(),
            civic: super::rim_shops::ActorCivicState {
                tool_maker: Some("阿木鐵店・林阿木鍛造".into()),
                ..Default::default()
            },
        });
        self.dirty_unsaved_ticks += 5;
        let p_idx = self.actors.len() - 1;
        self.resolve_solid_collisions(runtime::global(), 6);
        p_idx
    }

    pub fn touch_player_activity(&mut self, pid: &str) {
        let now = now_unix_sec();
        if let Some(a) = self
            .actors
            .iter_mut()
            .find(|a| a.entity_id.as_deref() == Some(pid))
        {
            a.last_active_unix = now;
        }
    }

    pub fn reset_camp_for_player(&mut self, pid: &str) {
        if let Some(a) = self
            .actors
            .iter_mut()
            .find(|a| a.entity_id.as_deref() == Some(pid))
        {
            let aid = a.id;
            a.px = 0.0;
            a.py = -2.0;
            a.facing_rad = 1.57;
            a.running = false;
            a.path.clear();
            a.task = PendingTask::None;
            a.hp = a.max_hp;
            a.activity_tag = "駐足觀望".into();
            a.last_active_unix = now_unix_sec();
            self.stall_tracker.remove(&aid);
            self.economy.parcel_queues.remove(&aid);
        }
        apply_outpost_architecture(runtime::global(), &mut self.overlays);
        self.dirty_unsaved_ticks += 20;
    }

    pub fn stockpile_totals(&self) -> HashMap<RimItemKind, u32> {
        let mut totals = HashMap::new();
        for ov in self.overlays.values() {
            if let Some(g) = ov.ground_item {
                *totals.entry(g.item).or_insert(0) += u32::from(g.count);
            }
        }
        totals
    }

    /// 每 50ms（20Hz）連續平滑推進位移，每 5 微步（250ms）結算一次工單與拆招。
    pub fn step_physics_50ms(&mut self) {
        let rt = runtime::global();
        for idx in 0..self.actors.len() {
            if self.actors[idx].hp > 0 && self.actors[idx].downed_ticks == 0 {
                self.glide_actor_50ms(rt, idx);
            }
        }
        self.resolve_solid_collisions(rt, 4);
        self.sub_step = (self.sub_step + 1) % 5;
        if self.sub_step == 0 {
            self.step_tick();
        }
    }

    pub fn step_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        let rt = runtime::global();
        super::rim_life::step_settler_metabolism(self);
        super::rim_ecology::step_land_ecology(self, rt);
        super::rim_ecology::step_wildlife_carrying_capacity(self, rt);
        super::rim_shops::step_town_shops(self);
        super::rim_economy::step_economy_tick(self);

        for idx in 0..self.actors.len() {
            if self.actors[idx].hp <= 0 {
                continue;
            }
            if let Some((_, exp)) = self.actors[idx].pop_text
                && self.tick >= exp
            {
                self.actors[idx].pop_text = None;
            }
            if self.actors[idx].cooldown_ticks > 0 {
                self.actors[idx].cooldown_ticks -= 1;
            }
            if self.actors[idx].downed_ticks > 0 {
                self.actors[idx].downed_ticks -= 1;
                if self.actors[idx].downed_ticks == 0 {
                    let rec_hp = (self.actors[idx].max_hp / 3).max(18);
                    self.actors[idx].hp = self.actors[idx].hp.max(rec_hp);
                    self.actors[idx].activity_tag = "負傷苦撐".into();
                    self.actors[idx].pop_text = Some(("緩過一口氣起身".into(), self.tick + 10));
                }
                continue;
            }
            let (ax, ay) = self.actors[idx].micro_pos();
            if self.actors[idx].faction == ActorFaction::Settler && self.tick.is_multiple_of(12) {
                let near_fire = self.overlays.iter().any(|(&(fx, fy), ov)| {
                    ov.structure == Some(StructureKind::Campfire)
                        && (fx - ax).abs() <= 4
                        && (fy - ay).abs() <= 4
                });
                if near_fire && self.actors[idx].hp < self.actors[idx].max_hp {
                    self.actors[idx].hp = (self.actors[idx].hp + 3).min(self.actors[idx].max_hp);
                }
                if self.actors[idx].injury_ticks > 0 {
                    let heal_step = if near_fire { 6 } else { 1 };
                    self.actors[idx].injury_ticks =
                        self.actors[idx].injury_ticks.saturating_sub(heal_step);
                    if self.actors[idx].injury_ticks == 0 {
                        self.actors[idx].injury_part = None;
                        self.actors[idx].push_diary("爐畔調息，舊傷漸癒");
                        self.actors[idx].pop_text = Some(("傷勢漸癒".into(), self.tick + 8));
                    }
                }
            }
            let manual_player = self.actors[idx].entity_id.is_some() && !self.actors[idx].autopilot;
            if !manual_player && !self.actors[idx].path.is_empty() {
                let id = self.actors[idx].id;
                let (px, py) = (self.actors[idx].px, self.actors[idx].py);
                let entry = self.stall_tracker.entry(id).or_insert(((px, py), 0));
                if (px - entry.0.0).hypot(py - entry.0.1) < 0.08 {
                    entry.1 = entry.1.saturating_add(1);
                } else {
                    *entry = ((px, py), 0);
                }
                let stall_limit = 4 + ((id as u8) % 2) * 2;
                if entry.1 >= stall_limit {
                    *entry = ((px, py), 0);
                    self.try_yield_choke_point(rt, idx);
                }
            } else {
                self.stall_tracker.remove(&self.actors[idx].id);
            }
            if self.actors[idx].autopilot
                && self.actors[idx].task == PendingTask::None
                && self.actors[idx].path.is_empty()
            {
                self.assign_autonomous_task(rt, idx);
            }
        }

        for idx in 0..self.actors.len() {
            if self.actors[idx].hp <= 0 || self.actors[idx].downed_ticks > 0 {
                continue;
            }
            self.advance_actor(rt, idx);
        }

        self.resolve_collisions_and_combat();
        self.actors
            .retain(|a| a.faction == ActorFaction::Settler || a.hp > 0);
        let injury_sites = ["右臂筋絡震傷", "肋下鈍挫悶痛", "左腿擦裂微跛"];
        let mut downed_events: Vec<String> = Vec::new();
        for a in &mut self.actors {
            if a.faction == ActorFaction::Settler && a.hp <= 0 {
                let site = injury_sites[a.id % injury_sites.len()];
                a.hp = 1;
                a.downed_ticks = 18;
                a.running = false;
                a.injury_ticks = 90;
                a.injury_part = Some(site.into());
                a.path.clear();
                a.task = PendingTask::None;
                a.activity_tag = "力竭倒地".into();
                a.pop_text = Some((format!("力竭・{site}"), self.tick + 14));
                a.push_diary(format!("交鋒力竭倒地，添上{site}"));
                downed_events.push(format!("{} 力竭負傷倒地，尚待扶救", a.glyph));
            }
        }
        for ev in downed_events {
            self.push_event(ev);
        }

        self.dirty_unsaved_ticks += 1;
        self.persist_if_needed(false);
    }
}
