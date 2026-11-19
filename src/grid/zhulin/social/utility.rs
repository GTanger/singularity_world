//! 純 Rust Dave Mark 乘積補償效用決策 ＋ Hysteresis 遲滯防抖 ＋ 三大經濟守恆閥門。
//!
//! 鐵律：
//! 1. 100% 純 Rust 演算（0 LLM 延遲、絕對守恆）。
//! 2. 採用 Dave Mark Compensation Factor 修正多因子連乘衰減，並加上 Hysteresis 遲滯黏著避免每拍橫跳。
//! 3. 所有交易在活人之間點對點嚴格守恆（總鎂量除「鎂晶燃燒消耗池」外恆等守恆，絕不對空氣刷錢）。

use serde::{Deserialize, Serialize};

use super::person::{ConductEdge, CoreObsession, ResidentActivityKind, SocialResident};
use super::relations::RelationLedger;

/// Hysteresis 遲滯防抖加權：正在執行中的意圖需被更強候選明顯超越才切換。
pub const HYSTERESIS_BONUS: f32 = 0.14;
/// 最小持續拍數（避免連貫行為執行 1 拍就半途切換）。
pub const MIN_COMMIT_TICKS: u16 = 2;

/// Dave Mark 經典乘積補償公式（`factors` 各元素皆落在 `[0.0, 1.0]`）。
pub fn dave_mark_compensated_utility(factors: &[f32]) -> f32 {
    if factors.is_empty() {
        return 0.0;
    }
    let mut product = 1.0_f32;
    for &f in factors {
        let clamped = f.clamp(0.0, 1.0);
        if clamped <= 0.0 {
            return 0.0;
        }
        product *= clamped;
    }
    let k = factors.len() as f32;
    let mod_factor = 1.0 - (1.0 / k);
    let make_up_value = (1.0 - product) * mod_factor;
    (product + (make_up_value * product)).clamp(0.0, 1.0)
}

/// 三大經濟守恆閥門即時帳本（規格書 §5.2）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomyValves {
    /// 閥門 1：鎂晶真實燃燒銷毀總池（開爐校磨、台車發車消耗）
    pub burned_magnesium_total: i64,
    /// 閥門 2：當前全鎮彈性收購價（隨竹材／口糧總存量動態調節）
    pub food_unit_mg: i32,
    pub bamboo_unit_mg: i32,
    pub shoot_unit_mg: i32,
    /// 閥門 3：西環台車累計外運特產件數與匯回鎂錠（嚴格有界轉運）
    pub railcar_exported_bamboo: i64,
    pub railcar_imported_mg: i64,
}

impl Default for EconomyValves {
    fn default() -> Self {
        Self {
            burned_magnesium_total: 0,
            food_unit_mg: 3,
            bamboo_unit_mg: 4,
            shoot_unit_mg: 7,
            railcar_exported_bamboo: 0,
            railcar_imported_mg: 0,
        }
    }
}

impl EconomyValves {
    /// 依全鎮實有庫存與人口比例動態調節收購牌價（閥門 2）。
    pub fn recalibrate_prices(&mut self, residents: &[SocialResident]) {
        let n = residents.len().max(1) as f32;
        let total_food: i32 = residents.iter().map(|r| r.food_stock).sum();
        let total_bamboo: i32 = residents.iter().map(|r| r.bamboo_stock).sum();
        let total_shoot: i32 = residents.iter().map(|r| r.shoot_stock).sum();

        let food_per_cap = total_food as f32 / n;
        let bamboo_per_cap = total_bamboo as f32 / n;
        let shoot_per_cap = total_shoot as f32 / n;

        // 缺糧漲價吸引務農，糧多回落
        self.food_unit_mg = if food_per_cap < 1.8 {
            5
        } else if food_per_cap < 3.0 {
            4
        } else if food_per_cap > 5.0 {
            2
        } else {
            3
        };
        // 竹材積壓下調收購價，迫使採獵者轉深入尋奇物或回城幫工
        self.bamboo_unit_mg = if bamboo_per_cap > 3.5 {
            2
        } else if bamboo_per_cap > 2.2 {
            3
        } else if bamboo_per_cap < 1.0 {
            6
        } else {
            4
        };
        self.shoot_unit_mg = if shoot_per_cap < 0.6 {
            9
        } else if shoot_per_cap > 2.0 {
            5
        } else {
            7
        };
    }
}

/// 依居民當下 `4 內在槽 × 時辰 × 高水威脅 × 債務壓力` 選出本拍最高效用行動（含 Hysteresis 防抖）。
pub fn choose_next_activity(
    r: &SocialResident,
    ledger: &RelationLedger,
    valves: &EconomyValves,
    hour_of_day: u32,
    high_water_nearby: bool,
) -> ResidentActivityKind {
    let (owed_out, owed_in) = ledger.debt_summary_of(&r.id);
    choose_next_activity_with_debt(r, owed_out, owed_in, valves, hour_of_day, high_water_nearby)
}

/// 批次高速版效用決策（直接接收預計算之 `(owed_out, owed_in)`，避免 $O(N \cdot E)$ 重複掃邊）。
pub fn choose_next_activity_with_debt(
    r: &SocialResident,
    owed_out: i32,
    owed_in: i32,
    valves: &EconomyValves,
    hour_of_day: u32,
    high_water_nearby: bool,
) -> ResidentActivityKind {
    let is_night = hour_of_day >= 22 || hour_of_day < 5;

    // 急迫安全中斷：重傷或極飢不鎖定舊活動
    let urgent = r.deficit.injury >= 0.65 || r.deficit.hunger >= 0.85 || r.deficit.fatigue >= 0.88;
    if !urgent && r.activity_ticks < MIN_COMMIT_TICKS {
        return r.current_activity;
    }

    let temper_risk = match r.temperament.edge {
        ConductEdge::RiskGambler | ConductEdge::BluntHothead => 1.18_f32,
        ConductEdge::Suspicious | ConductEdge::QuietSlowWarm => 0.85_f32,
        _ => 1.0_f32,
    };
    let craft_bias = if matches!(
        r.temperament.obsession,
        CoreObsession::CraftPerfection | CoreObsession::SelfMadeShop
    ) {
        1.18_f32
    } else {
        1.0_f32
    };
    let wealth_bias = if matches!(
        r.temperament.obsession,
        CoreObsession::HoardAndOwn | CoreObsession::HonorLedger
    ) {
        1.18_f32
    } else {
        1.0_f32
    };
    let wild_bias = if matches!(
        r.temperament.obsession,
        CoreObsession::SeekWonders | CoreObsession::OutshinePeers | CoreObsession::BoldVengeance
    ) {
        1.20_f32
    } else if r.temperament.obsession == CoreObsession::PlaySafe {
        0.72_f32
    } else {
        1.0_f32
    };

    let candidates: [(ResidentActivityKind, f32); 11] = [
        (
            ResidentActivityKind::SeekClinicCare,
            if r.deficit.injury > 0.25 {
                dave_mark_compensated_utility(&[r.deficit.injury, 0.92])
            } else {
                0.0
            },
        ),
        (
            ResidentActivityKind::EatWarmMeal,
            dave_mark_compensated_utility(&[
                r.deficit.hunger,
                if r.food_stock > 0 || r.magnesium >= valves.food_unit_mg {
                    0.95
                } else {
                    0.35
                },
                if r.temperament.obsession == CoreObsession::FineAppetite {
                    1.0
                } else {
                    0.82
                },
            ]),
        ),
        (
            ResidentActivityKind::TendHomeRest,
            dave_mark_compensated_utility(&[
                r.deficit.fatigue.max(if is_night { 0.82 } else { 0.12 }),
                if is_night { 0.96 } else { 0.55 },
            ]),
        ),
        (
            ResidentActivityKind::CollectOrSettleDebt,
            if owed_in > 0 || (owed_out > 0 && r.magnesium >= 5) {
                dave_mark_compensated_utility(&[
                    (r.deficit.debt_pressure + if owed_in > 0 { 0.45 } else { 0.25 }).min(1.0),
                    (0.70 * wealth_bias).min(1.0),
                    if is_night { 0.25 } else { 0.88 },
                ])
            } else {
                0.0
            },
        ),
        (
            ResidentActivityKind::ForageBambooWild,
            dave_mark_compensated_utility(&[
                (0.45 + r.wild_skill * 0.45) * wild_bias * temper_risk,
                if high_water_nearby { 0.42 } else { 0.92 },
                (1.0 - r.deficit.injury).max(0.1),
                (valves.shoot_unit_mg as f32 / 9.0).clamp(0.4, 1.0),
                if is_night { 0.15 } else { 0.90 },
            ]),
        ),
        (
            ResidentActivityKind::ForgeAndSharpen,
            dave_mark_compensated_utility(&[
                (0.35 + r.craft_skill * 0.50 + r.deficit.tool_wear * 0.35) * craft_bias,
                if r.bamboo_stock > 0 || r.magnesium >= 4 {
                    0.88
                } else {
                    0.40
                },
                if is_night { 0.18 } else { 0.86 },
            ]),
        ),
        (
            ResidentActivityKind::TendFarmPlot,
            dave_mark_compensated_utility(&[
                0.42 + (valves.food_unit_mg as f32 / 6.0) * 0.42,
                if r.pioneer_origin { 0.45 } else { 0.86 },
                if high_water_nearby { 0.55 } else { 0.90 },
                if is_night { 0.12 } else { 0.88 },
            ]),
        ),
        (
            ResidentActivityKind::TendHearthStove,
            dave_mark_compensated_utility(&[
                0.38 + r.deficit.stock_gap * 0.40,
                if r.food_stock > 0 || r.shoot_stock > 0 {
                    0.90
                } else {
                    0.35
                },
                if is_night { 0.25 } else { 0.85 },
            ]),
        ),
        (
            ResidentActivityKind::RunArcadeCounter,
            dave_mark_compensated_utility(&[
                (0.44 + r.craft_skill * 0.32) * wealth_bias,
                if r.pioneer_origin && r.magnesium < 25 {
                    0.35
                } else {
                    0.85
                },
                if is_night { 0.20 } else { 0.88 },
            ]),
        ),
        (
            ResidentActivityKind::HaulCartGoods,
            dave_mark_compensated_utility(&[
                0.40 + if r.bamboo_stock >= 2 { 0.35 } else { 0.12 },
                if high_water_nearby { 0.55 } else { 0.88 },
                if is_night { 0.20 } else { 0.82 },
            ]),
        ),
        (
            ResidentActivityKind::StreetGossip,
            dave_mark_compensated_utility(&[
                if r.temperament.edge == ConductEdge::LooseTongued {
                    0.78
                } else {
                    0.35
                },
                if is_night { 0.22 } else { 0.72 },
            ]),
        ),
    ];

    let mut best_kind = r.current_activity;
    let mut best_score = -1.0_f32;
    for (kind, raw_score) in candidates {
        let score = if kind == r.current_activity {
            raw_score + HYSTERESIS_BONUS
        } else {
            raw_score
        };
        if score > best_score {
            best_score = score;
            best_kind = kind;
        }
    }
    best_kind
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::SHALU_ZHULIN_WORLD_SEED;
    use crate::grid::zhulin::social::person::generate_residents;

    #[test]
    fn dave_mark_compensation_lifts_multi_factor_without_breaking_zero_veto() {
        assert_eq!(dave_mark_compensated_utility(&[0.9, 0.0, 0.9]), 0.0);
        let raw_prod = 0.8_f32 * 0.8 * 0.8; // 0.512
        let comp = dave_mark_compensated_utility(&[0.8, 0.8, 0.8]);
        assert!(comp > raw_prod && comp < 0.85, "comp={comp}");
    }

    #[test]
    fn hysteresis_prevents_rapid_activity_thrashing() {
        let mut list = generate_residents(SHALU_ZHULIN_WORLD_SEED, 4);
        let r = &mut list[0];
        let ledger = RelationLedger::default();
        let valves = EconomyValves::default();
        r.current_activity = ResidentActivityKind::RunArcadeCounter;
        r.activity_ticks = 1; // 未達 MIN_COMMIT_TICKS 且無急迫危險
        assert_eq!(
            choose_next_activity(r, &ledger, &valves, 10, false),
            ResidentActivityKind::RunArcadeCounter
        );
        // 一旦重傷立刻觸發急迫中斷
        r.deficit.injury = 0.80;
        assert_eq!(
            choose_next_activity(r, &ledger, &valves, 10, false),
            ResidentActivityKind::SeekClinicCare
        );
    }
}
