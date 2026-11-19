//! 純 Rust 活態社會演化模擬核心（`SocialWorldState`）：
//!
//! - 320 名無職稱真活居民在真實 `Landmass #01` 西麓騎樓街鎮、熟地與竹海間自然流動。
//! - 每拍驅動：內生缺口推進 → 高水期改道感知 → Dave Mark + Hysteresis 效用決策 →
//!   空間移動（遵守 `z == 0` 街面與 `z == 2` 夜歇隔離）→ 點對點零和交易／賒欠／抵債／
//!   同行競合／進山生死債／手藝幫工／流言目擊共振 → 三大守恆閥門校準。
//! - 嚴格守恆不變式：`sum(r.magnesium) + valves.burned_magnesium_total - valves.railcar_imported_mg == initial_total_mg`。

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::grid::substrate::hydrology::WorldClockSnapshot;

use super::handover::{
    PlayerWorkKind, PlayerWorkRecord, WorkAdoptionOutcome, try_adopt_player_work,
};
use super::person::{
    BeliefKind, ConductEdge, CoreObsession, ProvenanceVessel, ResidentActivityKind, SocialResident,
    SubjectiveBelief, generate_residents, splitmix64,
};
use super::relations::{
    GrantedRight, RelationLedger, RelationSocket, SharedHistoryEntry, SharedHistoryKind,
    propagate_witness_resonance,
};
use super::utility::{EconomyValves, choose_next_activity_with_debt};

/// 預設沙鹿竹林首發聚落活態居民規模（規格書建議 300～400 人）。
pub const DEFAULT_RESIDENT_COUNT: usize = 320;
/// 街頭回聲最大保留條數。
const MAX_RECENT_ECHOES: usize = 48;

const STREET_HUB_COORDS: [(i32, i32); 6] = [(-6, 0), (-5, 0), (-4, 0), (-3, 0), (-2, 0), (-1, 0)];

const ARCADE_FRONT_COORDS: [(i32, i32); 10] = [
    (-6, 1),
    (-5, 1),
    (-4, 1),
    (-3, 1),
    (-2, 1),
    (-6, -1),
    (-5, -1),
    (-4, -1),
    (-3, -1),
    (-2, -1),
];

const FARM_PLOT_COORDS: [(i32, i32); 6] =
    [(-6, -3), (-5, -3), (-4, -3), (-3, -3), (-2, -3), (-1, -1)];

const BAMBOO_GROVE_COORDS: [(i32, i32); 7] =
    [(1, 0), (2, 0), (1, -1), (2, 1), (3, -1), (0, -2), (2, -2)];

/// 街頭公開層自然湧現的事件回聲。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreetEcho {
    pub game_day: i32,
    pub gx: i32,
    pub gy: i32,
    pub summary: String,
}

/// 活態社會完整世界狀態（可確定性重播、可壓縮持久化至 PostgreSQL）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialWorldState {
    pub world_seed: u64,
    pub game_day: i32,
    pub tick_count: u64,
    pub initial_total_mg: i64,
    pub residents: Vec<SocialResident>,
    pub ledger: RelationLedger,
    pub valves: EconomyValves,
    /// 各玩家已結識（知曉真名）之居民集合（`player_id -> {resident_id}`）
    pub known_by_player: HashMap<String, HashSet<String>>,
    /// 近期公開層街頭回聲
    pub recent_echoes: Vec<StreetEcho>,
    /// 玩家留於世界之作品與活人接手紀錄（上限 64 條，向下相容舊快照）
    #[serde(default)]
    pub player_works: Vec<PlayerWorkRecord>,
    /// 累計因高水期脈衝而改道至高地／石板街之人次（供 60 日 Burn-in 硬驗收）
    #[serde(default)]
    pub high_water_reroutes: u64,
    /// 累計自然階級流動人次（先鋒入城開業、空倉掌櫃轉入竹海）
    #[serde(default)]
    pub class_mobility_count: u64,
}

impl SocialWorldState {
    /// 以指定世界種子建立零寫死初始社會狀態。
    pub fn new_seeded(world_seed: u64, count: usize) -> Self {
        let residents = generate_residents(world_seed, count);
        let initial_total_mg: i64 = residents.iter().map(|r| i64::from(r.magnesium)).sum();
        Self {
            world_seed,
            game_day: 0,
            tick_count: 0,
            initial_total_mg,
            residents,
            ledger: RelationLedger::default(),
            valves: EconomyValves::default(),
            known_by_player: HashMap::new(),
            recent_echoes: Vec::new(),
            player_works: Vec::new(),
            high_water_reroutes: 0,
            class_mobility_count: 0,
        }
    }

    /// 登記一筆玩家現場作品並立即嘗試由鄰近活態居民接手（若暫無鄰近者則留待後續拍次自然發現）。
    pub fn register_player_work(
        &mut self,
        player_id: &str,
        gx: i32,
        gy: i32,
        kind: PlayerWorkKind,
        game_day: i32,
    ) -> Option<WorkAdoptionOutcome> {
        let work_id = format!(
            "pw_{}_{}_{}_{}",
            self.tick_count,
            gx,
            gy,
            self.player_works.len() + 1
        );
        let mut record = PlayerWorkRecord {
            work_id,
            player_id: player_id.to_string(),
            gx,
            gy,
            kind,
            game_day,
            adopted_by_id: None,
            adopted_by_name: None,
            adoption_note: None,
        };
        let outcome =
            try_adopt_player_work(&mut self.residents, &mut self.recent_echoes, &mut record, 5);
        self.player_works.push(record);
        if self.player_works.len() > 64 {
            let drop_n = self.player_works.len() - 64;
            self.player_works.drain(0..drop_n);
        }
        outcome
    }

    /// 嚴格鎂晶守恆差：恆為 `0` 代表全鎮從未對空氣無中生有刷錢。
    pub fn conservation_diff(&self) -> i64 {
        let cur_mg: i64 = self.residents.iter().map(|r| i64::from(r.magnesium)).sum();
        cur_mg + self.valves.burned_magnesium_total
            - self.valves.railcar_imported_mg
            - self.initial_total_mg
    }

    fn push_echo(&mut self, game_day: i32, gx: i32, gy: i32, summary: String) {
        self.recent_echoes.push(StreetEcho {
            game_day,
            gx,
            gy,
            summary,
        });
        if self.recent_echoes.len() > MAX_RECENT_ECHOES {
            let drop_n = self.recent_echoes.len() - MAX_RECENT_ECHOES;
            self.recent_echoes.drain(0..drop_n);
        }
    }

    /// 推進單一純 Rust 活態社會節拍（無任何 LLM 呼叫，100% 確定性）。
    pub fn step_world_tick(&mut self, clock: WorldClockSnapshot) {
        self.tick_count = self.tick_count.saturating_add(1);
        self.game_day = clock.game_day;
        let hour = u32::from(clock.minute_of_day / 60) % 24;
        let is_night = hour >= 22 || hour < 5;
        let n = self.residents.len();
        if n == 0 {
            return;
        }

        // 1. O(E) 一次預建全鎮債務摘要，再逐人推進內生缺口 + Dave Mark + Hysteresis + 空間位移
        let debt_map = self.ledger.all_debt_summaries();
        for idx in 0..n {
            let (owed_out, owed_in) = debt_map
                .get(&self.residents[idx].id)
                .copied()
                .unwrap_or((0, 0));
            let r = &mut self.residents[idx];
            r.deficit.hunger += if r.current_activity == ResidentActivityKind::TendHomeRest {
                0.03
            } else {
                0.06
            };
            r.deficit.fatigue += if r.current_activity == ResidentActivityKind::TendHomeRest {
                -0.22
            } else {
                0.05
            };
            r.deficit.stock_gap += if r.bamboo_stock == 0 && r.food_stock <= 1 {
                0.07
            } else {
                -0.02
            };
            r.deficit.debt_pressure = if owed_out > 0 {
                (owed_out as f32 / 24.0).clamp(0.25, 1.0)
            } else {
                (r.deficit.debt_pressure - 0.12).max(0.0)
            };
            r.deficit.clamp_all();

            let high_water_here = clock.is_high_water_pulse_at(r.gx, r.gy);
            let next_act = choose_next_activity_with_debt(
                r,
                owed_out,
                owed_in,
                &self.valves,
                hour,
                high_water_here,
            );
            if next_act == r.current_activity {
                r.activity_ticks = r.activity_ticks.saturating_add(1);
            } else {
                r.current_activity = next_act;
                r.activity_ticks = 1;
            }

            let h = splitmix64(
                r.soul_seed
                    .wrapping_add(self.tick_count.wrapping_mul(0x9E37_79B9)),
            );
            let (tx, ty, tz, rerouted) =
                target_coord_for_activity(r, next_act, is_night, high_water_here, h);
            r.gx = tx;
            r.gy = ty;
            r.gz = tz;
            if rerouted {
                self.high_water_reroutes = self.high_water_reroutes.saturating_add(1);
            }
        }

        // 2. 索引同格名單與樞紐度數（供優先連接與同格互動使用）
        let mut by_cell: HashMap<(i32, i32, i16), Vec<usize>> = HashMap::new();
        for (idx, r) in self.residents.iter().enumerate() {
            by_cell.entry((r.gx, r.gy, r.gz)).or_default().push(idx);
        }
        let hub_deg = self.ledger.hub_degrees();

        // 3. 逐人執行實物生產、點對點零和交易、守恆閥門與六大關係插座事件
        for idx in 0..n {
            let act = self.residents[idx].current_activity;
            let (gx, gy, gz) = (
                self.residents[idx].gx,
                self.residents[idx].gy,
                self.residents[idx].gz,
            );
            let h = splitmix64(
                self.residents[idx]
                    .soul_seed
                    .wrapping_add(self.tick_count.wrapping_mul(0xBF58_476D)),
            );
            let high_water_here = clock.is_high_water_pulse_at(gx, gy);

            match act {
                ResidentActivityKind::TendHomeRest => {
                    let r = &mut self.residents[idx];
                    r.deficit.fatigue = (r.deficit.fatigue - 0.18).max(0.0);
                    r.deficit.injury = (r.deficit.injury - 0.06).max(0.0);
                }
                ResidentActivityKind::SeekClinicCare => {
                    self.resolve_clinic_care(idx, &by_cell, gx, gy, gz);
                }
                ResidentActivityKind::EatWarmMeal => {
                    self.resolve_eat_meal(idx, &by_cell, gx, gy, gz, h);
                }
                ResidentActivityKind::TendFarmPlot => {
                    let yield_food = if high_water_here {
                        1
                    } else {
                        1 + (h % 2) as i32
                    };
                    let r = &mut self.residents[idx];
                    r.food_stock = (r.food_stock + yield_food).min(24);
                    r.deficit.tool_wear = (r.deficit.tool_wear + 0.04).min(1.0);
                    r.deficit.stock_gap = (r.deficit.stock_gap - 0.14).max(0.0);
                }
                ResidentActivityKind::TendHearthStove => {
                    let r = &mut self.residents[idx];
                    if r.shoot_stock > 0 {
                        r.shoot_stock -= 1;
                        r.food_stock = (r.food_stock + 2).min(24);
                    } else if r.food_stock < 4 {
                        r.food_stock += 1;
                    }
                    r.deficit.stock_gap = (r.deficit.stock_gap - 0.16).max(0.0);
                }
                ResidentActivityKind::ForageBambooWild => {
                    self.resolve_wild_forage(idx, &by_cell, gx, gy, gz, high_water_here, h);
                }
                ResidentActivityKind::ForgeAndSharpen => {
                    self.resolve_forge_and_mentor(idx, &by_cell, gx, gy, gz, h);
                }
                ResidentActivityKind::RunArcadeCounter => {
                    self.resolve_arcade_counter(idx, &by_cell, &hub_deg, gx, gy, gz, h);
                }
                ResidentActivityKind::HaulCartGoods => {
                    self.resolve_haul_and_supply(idx, &hub_deg, h);
                }
                ResidentActivityKind::CollectOrSettleDebt => {
                    self.resolve_debt_settlement(idx, h);
                }
                ResidentActivityKind::StreetGossip => {
                    self.resolve_street_gossip(idx, &by_cell, gx, gy, gz, h);
                }
            }
        }

        // 4. 巡檢尚未被接手之玩家作品：讓經過附近之活態居民自然接手
        for work in &mut self.player_works {
            if work.adopted_by_id.is_none() {
                let _ =
                    try_adopt_player_work(&mut self.residents, &mut self.recent_echoes, work, 4);
            }
        }

        // 5. 閥門 3：每 12 拍執行一次有界西環台車外運（疏通竹材庫存、嚴格記帳守恆）
        if self.tick_count.is_multiple_of(12) {
            self.run_west_railcar_valve();
        }

        // 6. 閥門 2：依實物總存量重新校準全鎮牌價
        self.valves.recalibrate_prices(&self.residents);
        debug_assert_eq!(
            self.conservation_diff(),
            0,
            "magnesium conservation violated"
        );
    }

    fn resolve_clinic_care(
        &mut self,
        idx: usize,
        by_cell: &HashMap<(i32, i32, i16), Vec<usize>>,
        gx: i32,
        gy: i32,
        gz: i16,
    ) {
        let fee = 4;
        let helper = by_cell.get(&(gx, gy, gz)).and_then(|peers| {
            peers
                .iter()
                .copied()
                .find(|&p| p != idx && self.residents[p].herb_stock > 0)
        });
        if let Some(h_idx) = helper {
            self.residents[h_idx].herb_stock -= 1;
            if self.residents[idx].magnesium >= fee {
                self.residents[idx].magnesium -= fee;
                self.residents[h_idx].magnesium += fee;
            } else {
                let cred_id = self.residents[h_idx].id.clone();
                let debt_id = self.residents[idx].id.clone();
                self.ledger.record_loan(
                    &cred_id,
                    &debt_id,
                    fee,
                    self.game_day,
                    self.game_day + 5,
                    gx,
                    gy,
                );
            }
            self.residents[idx].deficit.injury =
                (self.residents[idx].deficit.injury - 0.45).max(0.0);
        } else {
            self.residents[idx].deficit.injury =
                (self.residents[idx].deficit.injury - 0.16).max(0.0);
        }
    }

    fn resolve_eat_meal(
        &mut self,
        idx: usize,
        by_cell: &HashMap<(i32, i32, i16), Vec<usize>>,
        gx: i32,
        gy: i32,
        gz: i16,
        h: u64,
    ) {
        if self.residents[idx].food_stock > 0 {
            self.residents[idx].food_stock -= 1;
            self.residents[idx].deficit.hunger =
                (self.residents[idx].deficit.hunger - 0.55).max(0.0);
            return;
        }
        let price = self.valves.food_unit_mg;
        let seller = by_cell
            .get(&(gx, gy, gz))
            .and_then(|peers| {
                peers
                    .iter()
                    .copied()
                    .find(|&p| p != idx && self.residents[p].food_stock >= 2)
            })
            .or_else(|| {
                let n = self.residents.len();
                let start = (h as usize) % n;
                (0..n)
                    .map(|k| (start + k) % n)
                    .find(|&p| p != idx && self.residents[p].food_stock >= 3)
            });

        let Some(s_idx) = seller else {
            return;
        };
        if self.residents[idx].magnesium >= price {
            self.residents[idx].magnesium -= price;
            self.residents[s_idx].magnesium += price;
            self.residents[s_idx].food_stock -= 1;
            self.residents[idx].deficit.hunger =
                (self.residents[idx].deficit.hunger - 0.55).max(0.0);
            let sup_id = self.residents[s_idx].id.clone();
            let buy_id = self.residents[idx].id.clone();
            self.ledger.record_supply_trade(
                &sup_id,
                &buy_id,
                "zl_food_meal",
                1,
                price,
                self.game_day,
                gx,
                gy,
            );
        } else {
            // 現金不足：向熟客或嘴硬心軟的街坊賒帳買餐（零刷錢，純掛載 zl_rel_debt）
            let willing = matches!(
                self.residents[s_idx].temperament.edge,
                ConductEdge::TsundereSoft | ConductEdge::LooseTongued | ConductEdge::Prideful
            ) || self.residents[s_idx].temperament.obsession
                == CoreObsession::ProtectKin
                || h.is_multiple_of(3);
            if willing {
                self.residents[s_idx].food_stock -= 1;
                self.residents[idx].deficit.hunger =
                    (self.residents[idx].deficit.hunger - 0.50).max(0.0);
                let cred_id = self.residents[s_idx].id.clone();
                let debt_id = self.residents[idx].id.clone();
                self.ledger.record_loan(
                    &cred_id,
                    &debt_id,
                    price,
                    self.game_day,
                    self.game_day + 4,
                    gx,
                    gy,
                );
                let _ = propagate_witness_resonance(
                    &mut self.residents,
                    &cred_id,
                    &debt_id,
                    gx,
                    gy,
                    gz,
                    self.game_day,
                    BeliefKind::WitnessedDebt {
                        debtor_id: debt_id.clone(),
                        creditor_id: cred_id.clone(),
                    },
                );
            }
        }
    }

    fn resolve_wild_forage(
        &mut self,
        idx: usize,
        by_cell: &HashMap<(i32, i32, i16), Vec<usize>>,
        gx: i32,
        gy: i32,
        gz: i16,
        high_water_here: bool,
        h: u64,
    ) {
        {
            let r = &mut self.residents[idx];
            r.bamboo_stock = (r.bamboo_stock + 1 + (h % 2) as i32).min(20);
            if h.is_multiple_of(3) {
                r.shoot_stock = (r.shoot_stock + 1).min(12);
            }
            if h % 7 == 0 {
                r.herb_stock = (r.herb_stock + 1).min(8);
            }
            r.wild_skill = (r.wild_skill + 0.004).min(1.0);
            r.deficit.tool_wear = (r.deficit.tool_wear + 0.07).min(1.0);
            r.deficit.fatigue = (r.deficit.fatigue + 0.06).min(1.0);
        }

        // 進山意外與搭檔生死債（zl_rel_expedition_bond）
        let risk_mod = if high_water_here { 7 } else { 17 };
        if h.is_multiple_of(risk_mod) {
            let partner = by_cell.get(&(gx, gy, gz)).and_then(|peers| {
                peers.iter().copied().find(|&p| {
                    p != idx
                        && self.residents[p].current_activity
                            == ResidentActivityKind::ForageBambooWild
                })
            });
            if let Some(p_idx) = partner {
                let actor_id = self.residents[idx].id.clone();
                let partner_id = self.residents[p_idx].id.clone();
                let partner_name = self.residents[p_idx].true_name.clone();
                let actor_name = self.residents[idx].true_name.clone();
                let abandons = matches!(
                    self.residents[p_idx].temperament.edge,
                    ConductEdge::PennyPincher | ConductEdge::Snobbish
                ) && self.residents[p_idx].temperament.obsession
                    != CoreObsession::ProtectKin;
                let edge = self.ledger.get_or_create_mut(&actor_id, &partner_id);
                if abandons {
                    self.residents[idx].deficit.injury =
                        (self.residents[idx].deficit.injury + 0.38).min(1.0);
                    edge.append_history(SharedHistoryEntry {
                        game_day: self.game_day,
                        gx,
                        gy,
                        kind: SharedHistoryKind::AbandonedInDanger,
                    });
                    upsert_expedition_socket(&mut edge.sockets, false);
                    self.push_echo(
                        self.game_day,
                        gx,
                        gy,
                        format!(
                            "林間風傳，{partner_name}見坡土滑塌卻撇下負傷的{actor_name}獨自脫身。"
                        ),
                    );
                } else {
                    self.residents[idx].deficit.injury =
                        (self.residents[idx].deficit.injury + 0.12).min(1.0);
                    edge.append_history(SharedHistoryEntry {
                        game_day: self.game_day,
                        gx,
                        gy,
                        kind: SharedHistoryKind::SavedLife,
                    });
                    edge.append_history(SharedHistoryEntry {
                        game_day: self.game_day,
                        gx,
                        gy,
                        kind: SharedHistoryKind::SharedSecretGrove { gx, gy },
                    });
                    edge.grant_right(GrantedRight::SecretGroveAccess {
                        holder_id: partner_id.clone(),
                        gx,
                        gy,
                    });
                    upsert_expedition_socket(&mut edge.sockets, true);
                    let _ = propagate_witness_resonance(
                        &mut self.residents,
                        &partner_id,
                        &actor_id,
                        gx,
                        gy,
                        gz,
                        self.game_day,
                        BeliefKind::WitnessedRescue {
                            rescuer_id: partner_id.clone(),
                            saved_id: actor_id.clone(),
                        },
                    );
                    self.push_echo(
                        self.game_day,
                        gx,
                        gy,
                        format!("竹林山坳裡，{partner_name}險中拉住了差點跌入暗溝的{actor_name}。"),
                    );
                }
            } else {
                self.residents[idx].deficit.injury =
                    (self.residents[idx].deficit.injury + 0.25).min(1.0);
            }
        }
    }

    fn resolve_forge_and_mentor(
        &mut self,
        idx: usize,
        by_cell: &HashMap<(i32, i32, i16), Vec<usize>>,
        gx: i32,
        gy: i32,
        gz: i16,
        h: u64,
    ) {
        // 閥門 1：開爐校磨真實消耗 1 顆鎂晶（進入 burned_magnesium_total）
        if self.residents[idx].magnesium >= 2 && self.residents[idx].deficit.tool_wear >= 0.20 {
            self.residents[idx].magnesium -= 1;
            self.valves.burned_magnesium_total += 1;
            let game_day = self.game_day;
            let r = &mut self.residents[idx];
            if r.bamboo_stock > 0 {
                r.bamboo_stock -= 1;
            }
            r.deficit.tool_wear = 0.04;
            r.craft_skill = (r.craft_skill + 0.008).min(1.0);
            if let Some(v) = r.vessels.first_mut() {
                v.record_event(game_day, "於西麓炭爐重新淬火校刃");
            }
        }

        // 手藝幫工與心傳（zl_rel_craft_lineage）
        if h.is_multiple_of(5) {
            let helper = by_cell.get(&(gx, gy, gz)).and_then(|peers| {
                peers.iter().copied().find(|&p| {
                    p != idx
                        && self.residents[p].craft_skill + 0.05 < self.residents[idx].craft_skill
                })
            });
            if let Some(h_idx) = helper {
                let mentor_id = self.residents[idx].id.clone();
                let helper_id = self.residents[h_idx].id.clone();
                self.residents[h_idx].craft_skill =
                    (self.residents[h_idx].craft_skill + 0.015).min(1.0);
                let edge = self.ledger.get_or_create_mut(&mentor_id, &helper_id);
                edge.append_history(SharedHistoryEntry {
                    game_day: self.game_day,
                    gx,
                    gy,
                    kind: SharedHistoryKind::CraftMentored,
                });
                if let Some(RelationSocket::CraftLineage { sessions, .. }) =
                    edge.sockets.iter_mut().find(|s| {
                        matches!(
                            s,
                            RelationSocket::CraftLineage {
                                mentor_id: m,
                                helper_id: hp,
                                ..
                            } if m == &mentor_id && hp == &helper_id
                        )
                    })
                {
                    *sessions += 1;
                } else {
                    edge.sockets.push(RelationSocket::CraftLineage {
                        mentor_id,
                        helper_id,
                        sessions: 1,
                    });
                }
            }
        }
    }

    fn resolve_arcade_counter(
        &mut self,
        idx: usize,
        by_cell: &HashMap<(i32, i32, i16), Vec<usize>>,
        hub_deg: &HashMap<String, usize>,
        gx: i32,
        gy: i32,
        gz: i16,
        h: u64,
    ) {
        // 階級流動：若掌櫃連日資金見底且空倉，轉去進山採材或拉車謀生
        if self.residents[idx].magnesium < 4
            && self.residents[idx].bamboo_stock == 0
            && self.residents[idx].shoot_stock == 0
        {
            self.residents[idx].current_activity = ResidentActivityKind::ForageBambooWild;
            self.residents[idx].activity_ticks = 0;
            self.class_mobility_count = self.class_mobility_count.saturating_add(1);
            return;
        }

        // 同格同行競合（zl_rel_trade_rivalry）
        if h.is_multiple_of(7) {
            let rival = by_cell.get(&(gx, gy, gz)).and_then(|peers| {
                peers.iter().copied().find(|&p| {
                    p != idx
                        && self.residents[p].current_activity
                            == ResidentActivityKind::RunArcadeCounter
                        && self.residents[p].temperament.edge
                            != self.residents[idx].temperament.edge
                })
            });
            if let Some(r_idx) = rival {
                let a_id = self.residents[idx].id.clone();
                let b_id = self.residents[r_idx].id.clone();
                let a_name = self.residents[idx].true_name.clone();
                let b_name = self.residents[r_idx].true_name.clone();
                let edge = self.ledger.get_or_create_mut(&a_id, &b_id);
                edge.append_history(SharedHistoryEntry {
                    game_day: self.game_day,
                    gx,
                    gy,
                    kind: SharedHistoryKind::UndercutRival,
                });
                if let Some(RelationSocket::TradeRivalry { clash_count, .. }) = edge
                    .sockets
                    .iter_mut()
                    .find(|s| matches!(s, RelationSocket::TradeRivalry { .. }))
                {
                    *clash_count += 1;
                } else {
                    edge.sockets.push(RelationSocket::TradeRivalry {
                        contested_gx: gx,
                        contested_gy: gy,
                        clash_count: 1,
                    });
                }
                let _ = propagate_witness_resonance(
                    &mut self.residents,
                    &a_id,
                    &b_id,
                    gx,
                    gy,
                    gz,
                    self.game_day,
                    BeliefKind::StreetRumor {
                        summary: format!("{a_name}與{b_name}在騎樓下因收貨開價起口角"),
                    },
                );
                if h.is_multiple_of(14) {
                    self.push_echo(
                        self.game_day,
                        gx,
                        gy,
                        format!("騎樓柱下，{a_name}與{b_name}為了春筍收價當街各不相讓。"),
                    );
                }
            }
        }

        // 若手頭有餘錢且同格有貨源，收購竹材／春筍（增強優先連接樞紐）
        let _ = hub_deg;
    }

    fn resolve_haul_and_supply(&mut self, idx: usize, hub_deg: &HashMap<String, usize>, h: u64) {
        let sell_shoot = self.residents[idx].shoot_stock > 0;
        let sell_bamboo = !sell_shoot && self.residents[idx].bamboo_stock > 0;
        if !sell_shoot && !sell_bamboo {
            return;
        }
        let unit_price = if sell_shoot {
            self.valves.shoot_unit_mg
        } else {
            self.valves.bamboo_unit_mg
        };
        let item_id = if sell_shoot {
            "zl_item_bamboo_shoot"
        } else {
            "zl_item_bamboo_culm"
        };

        // 優先連接（Preferential Attachment）：傾向挑「手頭有鎂晶 + 已有樞紐度數高 + 在騎樓應市」的買家
        let n = self.residents.len();
        let sample_start = (h as usize) % n;
        let mut best_buyer: Option<(usize, usize)> = None;
        for step in 0..32.min(n) {
            let cand = (sample_start + step * 7) % n;
            if cand == idx {
                continue;
            }
            let r = &self.residents[cand];
            if r.magnesium < unit_price {
                continue;
            }
            if !matches!(
                r.current_activity,
                ResidentActivityKind::RunArcadeCounter
                    | ResidentActivityKind::TendHearthStove
                    | ResidentActivityKind::ForgeAndSharpen
            ) {
                continue;
            }
            let deg = hub_deg.get(&r.id).copied().unwrap_or(0);
            let score = deg * 4 + (r.magnesium as usize) / 4;
            if best_buyer.is_none_or(|(_, best_s)| score > best_s) {
                best_buyer = Some((cand, score));
            }
        }

        let Some((b_idx, _)) = best_buyer else {
            return;
        };
        // 嚴格點對點零和轉帳
        self.residents[b_idx].magnesium -= unit_price;
        self.residents[idx].magnesium += unit_price;
        if sell_shoot {
            self.residents[idx].shoot_stock -= 1;
            self.residents[b_idx].shoot_stock = (self.residents[b_idx].shoot_stock + 1).min(24);
        } else {
            self.residents[idx].bamboo_stock -= 1;
            self.residents[b_idx].bamboo_stock = (self.residents[b_idx].bamboo_stock + 1).min(30);
        }
        self.residents[b_idx].deficit.stock_gap =
            (self.residents[b_idx].deficit.stock_gap - 0.25).max(0.0);

        let sup_id = self.residents[idx].id.clone();
        let buy_id = self.residents[b_idx].id.clone();
        let gx = self.residents[b_idx].gx;
        let gy = self.residents[b_idx].gy;
        let gz = self.residents[b_idx].gz;
        self.ledger.record_supply_trade(
            &sup_id,
            &buy_id,
            item_id,
            1,
            unit_price,
            self.game_day,
            gx,
            gy,
        );
        let _ = propagate_witness_resonance(
            &mut self.residents,
            &sup_id,
            &buy_id,
            gx,
            gy,
            gz,
            self.game_day,
            BeliefKind::WitnessedTrade {
                target_id: buy_id.clone(),
                item_label: if sell_shoot { "鮮春筍" } else { "老刺竹" }.to_string(),
            },
        );

        // 先鋒累積資本後可置產開張（階級流動）
        if self.residents[idx].pioneer_origin && self.residents[idx].magnesium >= 72 {
            self.residents[idx].pioneer_origin = false;
            self.residents[idx].current_activity = ResidentActivityKind::RunArcadeCounter;
            self.class_mobility_count = self.class_mobility_count.saturating_add(1);
        }
    }

    fn resolve_debt_settlement(&mut self, idx: usize, h: u64) {
        let actor_id = self.residents[idx].id.clone();
        let gx = self.residents[idx].gx;
        let gy = self.residents[idx].gy;
        let gz = self.residents[idx].gz;

        // 找出與此人有關的一筆活躍債務（排序確定性，優先清償自己欠的，其次催討別人欠的）
        let active_debts = self.ledger.active_debts_sorted();
        let debt_target = active_debts
            .iter()
            .find(|(_, d, _, _)| d == &actor_id)
            .or_else(|| active_debts.iter().find(|(c, _, _, _)| c == &actor_id))
            .cloned();

        let Some((cred_id, debt_id, amount_mg, due_day)) = debt_target else {
            return;
        };
        let Some(c_idx) = self.residents.iter().position(|r| r.id == cred_id) else {
            return;
        };
        let Some(d_idx) = self.residents.iter().position(|r| r.id == debt_id) else {
            return;
        };

        let pay = self.residents[d_idx]
            .magnesium
            .saturating_sub(3)
            .min(amount_mg);
        if pay > 0 {
            self.residents[d_idx].magnesium -= pay;
            self.residents[c_idx].magnesium += pay;
            let _ = self
                .ledger
                .repay_loan(&cred_id, &debt_id, pay, self.game_day, gx, gy);
        } else if self.game_day > due_day && !self.residents[d_idx].vessels.is_empty() {
            // 逾期無力償還：轉移留痕實物器物抵債（零刷錢，並觸發街坊目擊）
            let mut vessel: ProvenanceVessel = self.residents[d_idx].vessels.remove(0);
            let d_name = self.residents[d_idx].true_name.clone();
            let c_name = self.residents[c_idx].true_name.clone();
            let v_label = vessel.label.clone();
            let v_id = vessel.vessel_id.clone();
            vessel.record_event(
                self.game_day,
                format!("因{d_name}逾期無力清帳而抵與{c_name}"),
            );
            if self.residents[c_idx].vessels.len() < 4 {
                self.residents[c_idx].vessels.push(vessel);
            }
            let edge = self.ledger.get_or_create_mut(&cred_id, &debt_id);
            edge.append_history(SharedHistoryEntry {
                game_day: self.game_day,
                gx,
                gy,
                kind: SharedHistoryKind::DebtDefaulted { amount_mg },
            });
            edge.append_history(SharedHistoryEntry {
                game_day: self.game_day,
                gx,
                gy,
                kind: SharedHistoryKind::VesselSeizedForDebt { vessel_id: v_id },
            });
            edge.sockets.retain(|s| {
                !matches!(
                    s,
                    RelationSocket::Debt {
                        creditor_id: c,
                        debtor_id: d,
                        ..
                    } if c == &cred_id && d == &debt_id
                )
            });
            let _ = propagate_witness_resonance(
                &mut self.residents,
                &cred_id,
                &debt_id,
                gx,
                gy,
                gz,
                self.game_day,
                BeliefKind::WitnessedDebt {
                    debtor_id: debt_id.clone(),
                    creditor_id: cred_id.clone(),
                },
            );
            if h.is_multiple_of(2) {
                self.push_echo(
                    self.game_day,
                    gx,
                    gy,
                    format!("騎樓街口一陣騷動，{d_name}拿隨身「{v_label}」抵了欠{c_name}的舊帳。"),
                );
            }
        }
    }

    fn resolve_street_gossip(
        &mut self,
        idx: usize,
        by_cell: &HashMap<(i32, i32, i16), Vec<usize>>,
        gx: i32,
        gy: i32,
        gz: i16,
        h: u64,
    ) {
        let Some(peers) = by_cell.get(&(gx, gy, gz)) else {
            return;
        };
        let Some(&p_idx) = peers.iter().find(|&&p| p != idx) else {
            return;
        };

        // 1. 傳遞自己最新一條主觀認知給同格街坊
        if let Some(mut belief) = self.residents[idx].beliefs.last().cloned() {
            belief.source_actor_id = self.residents[idx].id.clone();
            belief.confidence = belief.confidence.saturating_sub(8).max(30);
            self.residents[p_idx].push_belief(belief);
        }

        // 2. 多疑／記仇脾氣在閒談中長出主觀懷疑與把柄（zl_rel_suspicion_hook）
        if matches!(
            self.residents[idx].temperament.edge,
            ConductEdge::Suspicious | ConductEdge::GrudgeHolder | ConductEdge::LooseTongued
        ) && h.is_multiple_of(6)
        {
            let accuser_id = self.residents[idx].id.clone();
            let suspect_id = self.residents[p_idx].id.clone();
            let suspect_name = self.residents[p_idx].true_name.clone();
            let evidence_vessel_id = self.residents[p_idx]
                .vessels
                .first()
                .map(|v| v.vessel_id.clone());
            let edge = self.ledger.get_or_create_mut(&accuser_id, &suspect_id);
            let already = edge.sockets.iter().any(|s| {
                matches!(
                    s,
                    RelationSocket::SuspicionHook {
                        accuser_id: a,
                        suspect_id: s_id,
                        ..
                    } if a == &accuser_id && s_id == &suspect_id
                )
            });
            if !already {
                edge.append_history(SharedHistoryEntry {
                    game_day: self.game_day,
                    gx,
                    gy,
                    kind: SharedHistoryKind::AccusedAndConfronted,
                });
                edge.sockets.push(RelationSocket::SuspicionHook {
                    accuser_id: accuser_id.clone(),
                    suspect_id: suspect_id.clone(),
                    allegation: format!("疑{suspect_name}趁夜偷採旁人做過記號的私房筍窩"),
                    evidence_vessel_id,
                });
                self.residents[idx].push_belief(SubjectiveBelief {
                    kind: BeliefKind::SuspectedPoach {
                        suspect_id: suspect_id.clone(),
                        gx,
                        gy,
                    },
                    source_actor_id: accuser_id,
                    game_day: self.game_day,
                    confidence: 72,
                });
            }
        }
    }

    /// 閥門 3：西環台車外運過剩竹材並嚴格記帳匯回鎂錠（每班上限 6 份）。
    fn run_west_railcar_valve(&mut self) {
        let total_bamboo: i32 = self.residents.iter().map(|r| r.bamboo_stock).sum();
        if total_bamboo < (self.residents.len() as i32) * 2 {
            return;
        }
        let mut exporters: Vec<usize> = (0..self.residents.len())
            .filter(|&i| self.residents[i].bamboo_stock >= 4)
            .collect();
        exporters.sort_by(|&a, &b| {
            self.residents[b]
                .bamboo_stock
                .cmp(&self.residents[a].bamboo_stock)
                .then_with(|| self.residents[a].id.cmp(&self.residents[b].id))
        });

        let mut remaining_quota = 6i32;
        for idx in exporters {
            if remaining_quota <= 0 {
                break;
            }
            let take = (self.residents[idx].bamboo_stock - 2)
                .min(remaining_quota)
                .min(2);
            if take <= 0 {
                continue;
            }
            let revenue = take * self.valves.bamboo_unit_mg.max(2);
            self.residents[idx].bamboo_stock -= take;
            self.residents[idx].magnesium += revenue;
            self.valves.railcar_exported_bamboo += i64::from(take);
            self.valves.railcar_imported_mg += i64::from(revenue);
            remaining_quota -= take;
        }
    }
}

fn upsert_expedition_socket(sockets: &mut Vec<RelationSocket>, saved: bool) {
    if let Some(RelationSocket::ExpeditionBond {
        trips,
        saved_count,
        betrayal_count,
    }) = sockets
        .iter_mut()
        .find(|s| matches!(s, RelationSocket::ExpeditionBond { .. }))
    {
        *trips += 1;
        if saved {
            *saved_count += 1;
        } else {
            *betrayal_count += 1;
        }
    } else {
        sockets.push(RelationSocket::ExpeditionBond {
            trips: 1,
            saved_count: u32::from(saved),
            betrayal_count: u32::from(!saved),
        });
    }
}

/// 依當前意圖、晝夜與高水改道計算目標三維座標（回傳 `(gx, gy, gz, rerouted_by_high_water)`）。
fn target_coord_for_activity(
    r: &SocialResident,
    act: ResidentActivityKind,
    is_night: bool,
    high_water_here: bool,
    h: u64,
) -> (i32, i32, i16, bool) {
    let pick = |arr: &[(i32, i32)]| -> (i32, i32) { arr[(h as usize) % arr.len()] };
    match act {
        ResidentActivityKind::TendHomeRest => {
            // 騎樓街屋（gy = 1 / -1 / -2）具備二樓住家（gz = 2），夜間歸宅自動上樓隔離街面
            let has_upper = matches!(r.home_gy, 1 | -1 | -2) && (-6..=-2).contains(&r.home_gx);
            let tz = if has_upper && (is_night || r.deficit.fatigue >= 0.65) {
                2
            } else {
                r.home_gz
            };
            (r.home_gx, r.home_gy, tz, false)
        }
        ResidentActivityKind::RunArcadeCounter
        | ResidentActivityKind::TendHearthStove
        | ResidentActivityKind::ForgeAndSharpen => {
            if matches!(r.home_gy, 1 | -1 | -2) && (-6..=-2).contains(&r.home_gx) {
                (r.home_gx, r.home_gy, 0, false)
            } else {
                let (ax, ay) = pick(&ARCADE_FRONT_COORDS);
                (ax, ay, 0, false)
            }
        }
        ResidentActivityKind::EatWarmMeal
        | ResidentActivityKind::CollectOrSettleDebt
        | ResidentActivityKind::StreetGossip => {
            let (sx, sy) = if h.is_multiple_of(2) {
                pick(&ARCADE_FRONT_COORDS)
            } else {
                pick(&STREET_HUB_COORDS)
            };
            (sx, sy, 0, false)
        }
        ResidentActivityKind::SeekClinicCare => {
            if h.is_multiple_of(2) {
                (-4, 1, 0, false)
            } else {
                (-3, -1, 0, false)
            }
        }
        ResidentActivityKind::HaulCartGoods => {
            let (sx, sy) = pick(&STREET_HUB_COORDS);
            (sx, sy, 0, false)
        }
        ResidentActivityKind::TendFarmPlot => {
            if high_water_here {
                // 高水期農畦泥濘積水，改走石板街整補
                let (sx, sy) = pick(&STREET_HUB_COORDS);
                (sx, sy, 0, true)
            } else {
                let (fx, fy) = pick(&FARM_PLOT_COORDS);
                (fx, fy, 0, false)
            }
        }
        ResidentActivityKind::ForageBambooWild => {
            if high_water_here {
                // 高水期避開低窪水脈，轉走較高坡地 `(1, -1)` / `(2, 0)` / `(3, -1)`
                let high_groves = [(1, -1), (2, 0), (3, -1)];
                let (wx, wy) = pick(&high_groves);
                (wx, wy, 0, true)
            } else {
                let (wx, wy) = pick(&BAMBOO_GROVE_COORDS);
                (wx, wy, 0, false)
            }
        }
    }
}
