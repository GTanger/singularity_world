//! 「玩家作品被活人接手」閉環（規格書開機首屏 15 分鐘閉環 × 微步 3d 硬驗收 ④）。
//!
//! 玩家於真實底板完成施作（點燃竹葉營火、劈竹開路、採竹、採筍）後，作品留於世界並由
//! 鄰近最需要它的真實無職稱活態居民接手受惠：留存於居民主觀認知槽、隨身器物留痕與街頭回聲，
//! 嚴格零刷錢、零劇本寫死 NPC。

use serde::{Deserialize, Serialize};

use super::person::{BeliefKind, ResidentActivityKind, SocialResident, SubjectiveBelief};
use super::relations::propagate_witness_resonance;
use super::sim::StreetEcho;

/// 玩家在世界上留下的可接手作品種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlayerWorkKind {
    CampfireLit,
    TrailCleared,
    BambooHarvested,
    ShootHarvested,
}

impl PlayerWorkKind {
    pub fn label_zh(self) -> &'static str {
        match self {
            Self::CampfireLit => "林間竹葉營火",
            Self::TrailCleared => "新劈疏竹小徑",
            Self::BambooHarvested => "整理妥當的刺竹材",
            Self::ShootHarvested => "現挖帶土鮮春筍",
        }
    }
}

/// 單筆玩家作品與活人接手留痕。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerWorkRecord {
    pub work_id: String,
    pub player_id: String,
    pub gx: i32,
    pub gy: i32,
    pub kind: PlayerWorkKind,
    pub game_day: i32,
    pub adopted_by_id: Option<String>,
    pub adopted_by_name: Option<String>,
    pub adoption_note: Option<String>,
}

/// 單筆接手回執（供即時敘事與驗證稽核使用）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkAdoptionOutcome {
    pub resident_id: String,
    pub resident_name: String,
    pub observed_role: String,
    pub narrative: String,
}

/// 為尚未被接手的玩家作品尋找鄰近（曼哈頓距離 `<= max_dist`）最需要它的活態居民接手。
///
/// 嚴格零刷錢：不增減任何居民鎂晶，只產生真實生理減乏、空間就近靠攏、器物留痕、主觀認知與公開層回聲。
pub fn try_adopt_player_work(
    residents: &mut [SocialResident],
    recent_echoes: &mut Vec<StreetEcho>,
    work: &mut PlayerWorkRecord,
    max_dist: i32,
) -> Option<WorkAdoptionOutcome> {
    if work.adopted_by_id.is_some() {
        return None;
    }

    let mut best: Option<(usize, i32)> = None;
    for (idx, r) in residents.iter().enumerate() {
        if r.gz != 0 {
            continue;
        }
        let dist = (r.gx - work.gx).abs() + (r.gy - work.gy).abs();
        if dist > max_dist {
            continue;
        }
        let need_score = match work.kind {
            PlayerWorkKind::CampfireLit => {
                let vit_need =
                    ((r.deficit.fatigue + r.deficit.hunger + r.deficit.injury) * 30.0) as i32;
                let wild_bonus = if matches!(
                    r.current_activity,
                    ResidentActivityKind::ForageBambooWild | ResidentActivityKind::SeekClinicCare
                ) {
                    25
                } else {
                    0
                };
                vit_need + wild_bonus - dist * 8
            }
            PlayerWorkKind::TrailCleared => {
                let role_bonus = if matches!(
                    r.current_activity,
                    ResidentActivityKind::HaulCartGoods | ResidentActivityKind::ForageBambooWild
                ) {
                    35
                } else {
                    10
                };
                role_bonus - dist * 7
            }
            PlayerWorkKind::BambooHarvested => {
                let role_bonus = if matches!(
                    r.current_activity,
                    ResidentActivityKind::ForgeAndSharpen
                        | ResidentActivityKind::RunArcadeCounter
                        | ResidentActivityKind::HaulCartGoods
                ) {
                    32
                } else {
                    8
                };
                let gap = (r.deficit.stock_gap * 25.0) as i32;
                role_bonus + gap - dist * 6
            }
            PlayerWorkKind::ShootHarvested => {
                let role_bonus = if matches!(
                    r.current_activity,
                    ResidentActivityKind::TendHearthStove
                        | ResidentActivityKind::RunArcadeCounter
                        | ResidentActivityKind::EatWarmMeal
                ) {
                    34
                } else {
                    10
                };
                let hunger = (r.deficit.hunger * 25.0) as i32;
                role_bonus + hunger - dist * 6
            }
        };
        if best.is_none_or(|(_, s)| need_score > s) {
            best = Some((idx, need_score));
        }
    }

    let (r_idx, _) = best?;
    let res_id = residents[r_idx].id.clone();
    let res_name = residents[r_idx].true_name.clone();
    let observed_role = residents[r_idx]
        .current_activity
        .observed_role_zh()
        .to_string();

    let (note, echo_text, vessel_note) = match work.kind {
        PlayerWorkKind::CampfireLit => {
            residents[r_idx].deficit.fatigue = (residents[r_idx].deficit.fatigue - 0.22).max(0.0);
            residents[r_idx].gx = work.gx;
            residents[r_idx].gy = work.gy;
            (
                format!(
                    "{observed_role}「{res_name}」聞到乾燥竹炭暖意，順步靠來烤暖濕袖，朝你點頭致謝。"
                ),
                format!(
                    "東坡竹影間，{res_name}趁著{}剛生起的竹葉營火烘乾衣衫，直說這把火生得正是時候。",
                    work.player_id
                ),
                "曾借過林間新燃竹火烘乾濕袖",
            )
        }
        PlayerWorkKind::TrailCleared => {
            residents[r_idx].gx = work.gx;
            residents[r_idx].gy = work.gy;
            (
                format!(
                    "{observed_role}「{res_name}」順著你剛劈出的疏竹通道省力穿行，回頭記住了這段好路。"
                ),
                format!(
                    "街坊口耳相傳，{res_name}挑貨走過{}剛劈開的疏竹小徑，比繞路省了半刻工夫。",
                    work.player_id
                ),
                "走過林緣新闢疏竹小徑省下腳力",
            )
        }
        PlayerWorkKind::BambooHarvested => {
            residents[r_idx].deficit.stock_gap =
                (residents[r_idx].deficit.stock_gap - 0.18).max(0.0);
            (
                format!(
                    "鄰近的{observed_role}「{res_name}」瞧見你斬下的厚壁老竹成色紮實，連忙上前相看記下貨路。"
                ),
                format!(
                    "{res_name}在竹林邊看中{}剛砍下的老刺竹，盤算著日後回騎樓照應收購。",
                    work.player_id
                ),
                "曾相看過林緣新伐老刺竹成色",
            )
        }
        PlayerWorkKind::ShootHarvested => {
            residents[r_idx].deficit.stock_gap =
                (residents[r_idx].deficit.stock_gap - 0.18).max(0.0);
            (
                format!(
                    "附近的{observed_role}「{res_name}」見你起出的春筍沾露鮮嫩，記下你的手路打算日後常來問貨。"
                ),
                format!(
                    "{res_name}逢人便誇{}挖出的春筍鮮甜帶露，西麓灶頭正缺這口鮮料。",
                    work.player_id
                ),
                "曾記下林間鮮春筍採點手路",
            )
        }
    };

    if let Some(v) = residents[r_idx].vessels.first_mut() {
        v.record_event(work.game_day, vessel_note);
    }
    residents[r_idx].push_belief(SubjectiveBelief {
        kind: BeliefKind::StreetRumor {
            summary: echo_text.clone(),
        },
        source_actor_id: work.player_id.clone(),
        game_day: work.game_day,
        confidence: 95,
    });

    // 同格公開層旁觀者同步聽聞這樁活人接手佳話
    let _ = propagate_witness_resonance(
        residents,
        &res_id,
        &work.player_id,
        work.gx,
        work.gy,
        0,
        work.game_day,
        BeliefKind::StreetRumor {
            summary: echo_text.clone(),
        },
    );

    recent_echoes.push(StreetEcho {
        game_day: work.game_day,
        gx: work.gx,
        gy: work.gy,
        summary: echo_text,
    });
    if recent_echoes.len() > 48 {
        let drop_n = recent_echoes.len() - 48;
        recent_echoes.drain(0..drop_n);
    }

    work.adopted_by_id = Some(res_id.clone());
    work.adopted_by_name = Some(res_name.clone());
    work.adoption_note = Some(note.clone());

    Some(WorkAdoptionOutcome {
        resident_id: res_id,
        resident_name: res_name,
        observed_role,
        narrative: note,
    })
}
