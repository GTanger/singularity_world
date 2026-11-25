//! 白鷗港與銀鷗島家戶輩分、四族壽命推算、六棟空屋前塵台帳與口傳 `soul_seed` 折射算子。

use crate::combat::wordplate::splitmix64;

use super::mind::EvidenceTier;
use super::persona::{ValueAxis, derive_innate_persona};
use super::types::IslandWorldDelta;

pub const CURRENT_SOLAR_YEAR: i16 = 329;

/// 白鷗港 6 棟市政廳待售空屋之唯一前塵台帳（物權唯一，防一屋二賣，與隔壁鄰居及店鋪雙邊對帳）
#[derive(Debug, Clone, Copy)]
pub struct VacantHouseBackstory {
    pub house_node_id: &'static str,
    pub alley_node_id: &'static str,
    pub house_label: &'static str,
    pub former_head_zh: &'static str,
    pub former_race: &'static str,
    pub former_trade: &'static str,
    pub event_id: &'static str,
    pub solar_year: i16,
    pub cause_kind: VacantCauseKind,
    pub linked_shop_actor_id: &'static str,
    pub room_trace_zh: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VacantCauseKind {
    SouthCoastPlague,
    MainlandTariffBankruptcy,
}

pub const WHITE_GULL_VACANT_HOUSES: &[VacantHouseBackstory] = &[
    VacantHouseBackstory {
        house_node_id: "oi:white_gull_harbor:iron_anvil_ln:h10",
        alley_node_id: "oi:white_gull_harbor:iron_anvil_ln",
        house_label: "鐵砧巷10號",
        former_head_zh: "老石匠石錘葛恩",
        former_race: "矮人",
        former_trade: "砌牆石工",
        event_id: "hist:wgh:vacant:iron_anvil_10",
        solar_year: 326,
        cause_kind: VacantCauseKind::SouthCoastPlague,
        linked_shop_actor_id: "res:white_gull_harbor:shop_3_0",
        room_trace_zh: "冷透的石砌壁爐底還積著半撮燒剩的苦艾藥渣，門框內側留有船疫那年封門防疫時刮除的石灰白印。",
    },
    VacantHouseBackstory {
        house_node_id: "oi:white_gull_harbor:sail_ln:h10",
        alley_node_id: "oi:white_gull_harbor:sail_ln",
        house_label: "船帆巷10號",
        former_head_zh: "海商溫德",
        former_race: "人類",
        former_trade: "跨海布匹船商",
        event_id: "hist:wgh:vacant:sail_10",
        solar_year: 324,
        cause_kind: VacantCauseKind::MainlandTariffBankruptcy,
        linked_shop_actor_id: "res:white_gull_harbor:shop_1_0",
        room_trace_zh: "牆角立著半截斷裂的杉木船槳，門後木釘上還掛著幾枚蓋過海峽二成關稅紅印的廢棄木契牌。",
    },
    VacantHouseBackstory {
        house_node_id: "oi:white_gull_harbor:copper_ln:h9",
        alley_node_id: "oi:white_gull_harbor:copper_ln",
        house_label: "銅幣巷9號",
        former_head_zh: "計帳師霍爾",
        former_race: "人類",
        former_trade: "商行算手",
        event_id: "hist:wgh:vacant:copper_9",
        solar_year: 327,
        cause_kind: VacantCauseKind::SouthCoastPlague,
        linked_shop_actor_id: "res:white_gull_harbor:shop_3_0",
        room_trace_zh: "窗台石縫間壓著一隻乾癟的苦艾香囊，空木架上留著幾道搬空帳箱時匆忙拖出的刮痕。",
    },
    VacantHouseBackstory {
        house_node_id: "oi:white_gull_harbor:salted_fish_ln:h10",
        alley_node_id: "oi:white_gull_harbor:salted_fish_ln",
        house_label: "鹹魚巷10號",
        former_head_zh: "鹽戶老巴克",
        former_race: "獸人",
        former_trade: "鹽漬魚工",
        event_id: "hist:wgh:vacant:salted_fish_10",
        solar_year: 326,
        cause_kind: VacantCauseKind::SouthCoastPlague,
        linked_shop_actor_id: "res:white_gull_harbor:dock_porter",
        room_trace_zh: "後院石槽裡倒扣著兩口洗淨封存的粗鹽木桶，門栓上繫著獸人船工替亡故同鄉留下的麻繩結。",
    },
    VacantHouseBackstory {
        house_node_id: "oi:white_gull_harbor:cooper_ln:h9",
        alley_node_id: "oi:white_gull_harbor:cooper_ln",
        house_label: "桶匠巷9號",
        former_head_zh: "箍桶匠松木林恩",
        former_race: "精靈",
        former_trade: "船用箍桶匠",
        event_id: "hist:wgh:vacant:cooper_9",
        solar_year: 323,
        cause_kind: VacantCauseKind::MainlandTariffBankruptcy,
        linked_shop_actor_id: "res:white_gull_harbor:shop_2_0",
        room_trace_zh: "屋樑下懸著幾圈未上鐵箍的橡木彎板，地上刨花早已乾脆，顯見原屋主退租西遷時走得頗為從容。",
    },
    VacantHouseBackstory {
        house_node_id: "oi:white_gull_harbor:rope_ln:h9",
        alley_node_id: "oi:white_gull_harbor:rope_ln",
        house_label: "繩索巷9號",
        former_head_zh: "絞繩匠瑪格",
        former_race: "人類",
        former_trade: "船纜絞繩匠",
        event_id: "hist:wgh:vacant:rope_9",
        solar_year: 327,
        cause_kind: VacantCauseKind::SouthCoastPlague,
        linked_shop_actor_id: "res:white_gull_harbor:shop_3_0",
        room_trace_zh: "牆根木樁還繞著半截浸過桐油的麻纜，爐台邊留有一雙洗得發白卻無人再穿的粗布護掌。",
    },
];

pub fn vacant_house_trace_zh(node_id: &str) -> Option<&'static str> {
    WHITE_GULL_VACANT_HOUSES
        .iter()
        .find(|v| v.house_node_id == node_id)
        .map(|v| v.room_trace_zh)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EventLayer {
    PublicSkeleton,
    MicroBilateral,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum PartyStance {
    BenefactorOrCreditor,
    BeneficiaryOrDebtor,
    WitnessOrPeer,
}

pub(super) fn concrete_member_craft_role(
    head_role: &str,
    member_idx: usize,
    seed: u64,
) -> &'static str {
    let pick_b = (seed & 1) == 0;
    if head_role.contains("鍛鐵") {
        match member_idx {
            1 => {
                if pick_b {
                    "爐前司風手"
                } else {
                    "選炭理鐵匠"
                }
            }
            _ => {
                if pick_b {
                    "鍛坊幫錘"
                } else {
                    "淬火磨刃手"
                }
            }
        }
    } else if head_role.contains("石工") {
        match member_idx {
            1 => "刻石理縫工",
            _ => "鑿石搬料手",
        }
    } else if head_role.contains("木匠") {
        match member_idx {
            1 => {
                if pick_b {
                    "刨木榫接匠"
                } else {
                    "桐油上漆手"
                }
            }
            _ => {
                if pick_b {
                    "鋸木幫手"
                } else {
                    "箍桶學徒"
                }
            }
        }
    } else if head_role.contains("皮匠") {
        match member_idx {
            1 => "鞣革縫線匠",
            _ => "刮皮硝製手",
        }
    } else if head_role.contains("漁") {
        match member_idx {
            1 => {
                if pick_b {
                    "補網理線手"
                } else {
                    "鹽漬剖魚手"
                }
            }
            _ => {
                if pick_b {
                    "搖櫓隨船手"
                } else {
                    "曬魚收網工"
                }
            }
        }
    } else if head_role.contains("獵") {
        match member_idx {
            1 => "燻肉製革手",
            _ => "巡林削箭手",
        }
    } else if head_role.contains("草藥") {
        match member_idx {
            1 => "焙藥分揀師",
            _ => "採藥熬湯手",
        }
    } else if head_role.contains("農") {
        match member_idx {
            1 => "播種理畦農戶",
            _ => "挑水刈麥手",
        }
    } else if head_role.contains("苦力") || head_role.contains("水手") || head_role.contains("船")
    {
        match member_idx {
            1 => "絞繩補帆手",
            _ => "扛包理艙手",
        }
    } else {
        match member_idx {
            1 => "理貨記籌手",
            _ => "跑街送貨手",
        }
    }
}

/// 以當家屋主年齡為錨點，整戶一次推算所有成員之出生年與中性輩分稱謂，嚴格保證世代年齡單調合理！
pub(super) fn infer_household_birth_years_and_bonds(
    race: &str,
    is_shop: bool,
    is_porter: bool,
    fam_size: usize,
    hh_seed: u64,
) -> Vec<(i16, &'static str)> {
    let r = splitmix64(hh_seed);
    if is_porter {
        let age = 42 + (r % 14) as i16;
        return vec![(CURRENT_SOLAR_YEAR - age, "碼頭獨居老手")];
    }
    if is_shop {
        let (master_age, app_age, master_bond) = match race {
            "精靈" => (
                212 + (r % 36) as i16,
                68 + ((r >> 8) % 28) as i16,
                "堂主師父",
            ),
            "矮人" => (
                132 + (r % 34) as i16,
                46 + ((r >> 8) % 22) as i16,
                "鋪主師父",
            ),
            _ => (44 + (r % 14) as i16, 18 + ((r >> 8) % 6) as i16, "掌櫃師父"),
        };
        let mut out = vec![(CURRENT_SOLAR_YEAR - master_age, master_bond)];
        for k in 1..fam_size {
            let a_age = (app_age - (k as i16 - 1) * 2).max(16);
            out.push((CURRENT_SOLAR_YEAR - a_age, "入室學徒"));
        }
        return out;
    }

    let has_elder = fam_size >= 3 && ((r >> 6) % 3 == 0);
    let mut out = Vec::with_capacity(fam_size);
    match race {
        "精靈" => {
            let head_age = 175 + (r % 55) as i16; // 175..=229
            out.push((CURRENT_SOLAR_YEAR - head_age, "當家長輩"));
            if fam_size >= 2 {
                if has_elder {
                    let elder_age = head_age + 65 + ((r >> 10) % 25) as i16;
                    out.push((CURRENT_SOLAR_YEAR - elder_age, "家中長老"));
                } else {
                    let sp_age = head_age - 6 + ((r >> 10) % 12) as i16;
                    out.push((CURRENT_SOLAR_YEAR - sp_age, "同堂伴侶"));
                }
            }
            if fam_size >= 3 {
                let c1_age = 72 + ((r >> 16) % 24) as i16; // 比當家年輕至少 79 歲
                out.push((CURRENT_SOLAR_YEAR - c1_age, "隨業大輩"));
                if fam_size >= 4 {
                    let c2_age = c1_age - (12 + ((r >> 22) % 14) as i16);
                    out.push((CURRENT_SOLAR_YEAR - c2_age, "隨業小輩"));
                }
            }
        }
        "矮人" => {
            let head_age = 112 + (r % 40) as i16; // 112..=151
            out.push((CURRENT_SOLAR_YEAR - head_age, "當家匠師"));
            if fam_size >= 2 {
                if has_elder {
                    let elder_age = head_age + 52 + ((r >> 10) % 20) as i16;
                    out.push((CURRENT_SOLAR_YEAR - elder_age, "家中老輩"));
                } else {
                    let sp_age = head_age - 4 + ((r >> 10) % 8) as i16;
                    out.push((CURRENT_SOLAR_YEAR - sp_age, "同堂伴侶"));
                }
            }
            if fam_size >= 3 {
                let c1_age = 48 + ((r >> 16) % 18) as i16; // 比當家年輕至少 46 歲
                out.push((CURRENT_SOLAR_YEAR - c1_age, "隨業大輩"));
                if fam_size >= 4 {
                    let c2_age = c1_age - (6 + ((r >> 22) % 9) as i16);
                    out.push((CURRENT_SOLAR_YEAR - c2_age, "隨業小輩"));
                }
            }
        }
        _ => {
            // 人類與獸人：若有晚輩（fam_size >= 3），當家必 >= 38 歲，確保父母至少比長輩子女大 19 歲！
            let min_head = if fam_size >= 3 { 39 } else { 28 };
            let head_age = min_head + (r % 13) as i16; // 39..=51 或 28..=40
            out.push((CURRENT_SOLAR_YEAR - head_age, "當家屋主"));
            if fam_size >= 2 {
                if has_elder {
                    let elder_age = head_age + 21 + ((r >> 10) % 8) as i16; // 60..=79
                    out.push((CURRENT_SOLAR_YEAR - elder_age, "家中老輩"));
                } else {
                    let sp_age = head_age - 2 + ((r >> 10) % 5) as i16;
                    out.push((CURRENT_SOLAR_YEAR - sp_age, "同堂伴侶"));
                }
            }
            if fam_size >= 3 {
                let c1_age = (head_age - (20 + ((r >> 16) % 4) as i16)).clamp(18, 26);
                out.push((CURRENT_SOLAR_YEAR - c1_age, "隨業大輩"));
                if fam_size >= 4 {
                    let c2_age = (c1_age - (2 + ((r >> 22) % 3) as i16)).max(15);
                    out.push((CURRENT_SOLAR_YEAR - c2_age, "隨業小輩"));
                }
            }
        }
    }
    out
}

pub(super) fn natural_age_appearance(
    race: &str,
    role: &str,
    birth_year: i16,
    seed: u64,
    is_inland: bool,
) -> String {
    let lived = (CURRENT_SOLAR_YEAR - birth_year).max(14);
    let stage = match race {
        "精靈" => {
            if lived >= 210 {
                if is_inland {
                    "眼尾帶著見過百餘年山嵐雪線的沉靜細紋"
                } else {
                    "眼尾帶著見過百餘年潮汐的沉靜細紋"
                }
            } else if lived >= 110 {
                "神色清朗沉穩，指腹留著長年操持器物的薄繭"
            } else {
                "眉目清雋年輕，舉止還帶著幾分新奇銳氣"
            }
        }
        "矮人" => {
            if lived >= 145 {
                "濃密的鬢鬚已染灰霜，虎口老繭厚如鐵片"
            } else if lived >= 85 {
                "肩背敦實如石墩，雙臂筋肉結實有力"
            } else {
                "鬍鬚剛蓄不久，手背還留著學藝時燙出的淺疤"
            }
        }
        _ => {
            if lived >= 55 {
                "兩鬢斑白、背脊微駝，一雙老手佈滿風霜裂紋"
            } else if lived >= 30 {
                "正值壯年當家，衣袖挽起、手掌磨得粗厚油亮"
            } else {
                "眉眼尚帶少年氣，腳步輕快、袖口沾著活計碎屑"
            }
        }
    };
    let mud_cue = if is_inland {
        "鞋緣沾著山徑濕冷松泥"
    } else {
        "鞋緣沾著海港濕鹹泥印"
    };
    let detail_pool = [
        "腰間總別著順手小工具",
        mud_cue,
        "袖口縫補得乾淨俐落",
        "說話前習慣先打量來人手上的繭子",
    ];
    let pick = detail_pool[(splitmix64(seed ^ 0x9E37) as usize) % detail_pool.len()];
    format!("{}，{}（日常操持{}）", stage, pick, role)
}

fn racial_generation_span(race: &str) -> i16 {
    match race {
        "精靈" => 110,
        "矮人" => 65,
        _ => 26,
    }
}

fn ancestor_kin_prefix(
    race: &str,
    birth_year: i16,
    event_year: i16,
    is_shop: bool,
) -> &'static str {
    if birth_year <= event_year - 12 {
        "那年我在場親歷過"
    } else if birth_year <= event_year {
        "那年我年紀尚小、聽家裡大人常提"
    } else if is_shop {
        "師門老一輩傳下話來"
    } else {
        let gap = (birth_year - event_year).max(1);
        let span = racial_generation_span(race);
        if gap <= span {
            match race {
                "精靈" => "家中長輩親身經歷過",
                "矮人" => "我老輩生前常拍著鐵砧提過",
                _ => "長輩在飯桌上反覆唸叨過",
            }
        } else if gap <= span * 2 {
            "祖輩留下的老話總提"
        } else {
            "家裡傳了好幾代的老話說"
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "explicit stance and value axis refraction"
)]
fn refract_subjective_memory(
    axis: ValueAxis,
    stance: PartyStance,
    prefix: &str,
    year: i16,
    counterpart_name: &str,
    core_fact_creditor: &str,
    core_fact_debtor: &str,
    core_fact_witness: &str,
) -> String {
    let base = match stance {
        PartyStance::BenefactorOrCreditor => core_fact_creditor,
        PartyStance::BeneficiaryOrDebtor => core_fact_debtor,
        PartyStance::WitnessOrPeer => core_fact_witness,
    };
    let slant = match (stance, axis) {
        (PartyStance::BenefactorOrCreditor, ValueAxis::TangibleGain) => {
            format!(
                "當時墊出的工本與分寸，和{}心裡都有本明白帳",
                counterpart_name
            )
        }
        (PartyStance::BenefactorOrCreditor, ValueAxis::PactIntegrity) => {
            format!("既然跟{}當面應下，自當守住然諾辦到底", counterpart_name)
        }
        (PartyStance::BenefactorOrCreditor, ValueAxis::CraftPride) => {
            "憑自家本分行事，對得起門前手藝".to_string()
        }
        (PartyStance::BenefactorOrCreditor, _) => {
            format!("當年能幫襯{}一回，也是顧念同鎮情分", counterpart_name)
        }
        (PartyStance::BeneficiaryOrDebtor, ValueAxis::KinshipLoyalty) => {
            format!(
                "為了安頓屋裡自家人，{}當年的處置家裡一直記在心上",
                counterpart_name
            )
        }
        (PartyStance::BeneficiaryOrDebtor, ValueAxis::Autonomy) => {
            format!(
                "這份往來人情早晚得靠雙手還清，不教自家矮{}一頭",
                counterpart_name
            )
        }
        (PartyStance::BeneficiaryOrDebtor, ValueAxis::TangibleGain) => {
            format!(
                "時局緊迫之下，{}開出的條件分寸家裡心知肚明",
                counterpart_name
            )
        }
        (PartyStance::BeneficiaryOrDebtor, _) => {
            format!("這樁往事與{}牽連頗深，至今不敢忘懷", counterpart_name)
        }
        (PartyStance::WitnessOrPeer, ValueAxis::OrderHierarchy) => {
            "港裡許多老規矩便是從那時立穩的".to_string()
        }
        (PartyStance::WitnessOrPeer, ValueAxis::SurvivalSafety) => {
            "在海島風浪裡討生活，終歸以穩妥度日為重".to_string()
        }
        (PartyStance::WitnessOrPeer, ValueAxis::CuriosityVenture) => {
            "裡頭尚有幾分外人未必摸清的舊門道".to_string()
        }
        (PartyStance::WitnessOrPeer, _) => "老街坊心裡皆有分寸，平素不在外人面前多嘴".to_string(),
    };
    format!("太陽曆 {} 年前後，{}：{}，{}。", year, prefix, base, slant)
}

#[expect(
    clippy::too_many_arguments,
    reason = "explicit bilateral historical event injection"
)]
pub(super) fn inject_event_into_actor(
    delta: &mut IslandWorldDelta,
    actor_id: &str,
    event_id: &str,
    event_year: i16,
    node_id: &str,
    counterpart_id: &str,
    counterpart_name: &str,
    stance: PartyStance,
    layer: EventLayer,
    forgetting_seed: u64,
    fact_creditor: &str,
    fact_debtor: &str,
    fact_witness: &str,
    as_turning_point: bool,
) {
    let Some(actor_ro) = delta.actors.get(actor_id) else {
        return;
    };
    if actor_ro.controller_id.is_some() || !actor_ro.alive {
        return;
    }
    let birth_year = actor_ro.birth_year;
    let race = actor_ro.race.clone();
    let is_shop = actor_ro.household_id.contains(":shop_");
    let soul_seed = actor_ro.soul_seed.max(1);
    let drift = actor_ro.drift.clone();

    let direct_witness = birth_year <= event_year - 10;
    if !direct_witness
        && matches!(stance, PartyStance::BeneficiaryOrDebtor)
        && layer == EventLayer::MicroBilateral
        && splitmix64(forgetting_seed ^ soul_seed).is_multiple_of(5)
    {
        return;
    }

    let innate = derive_innate_persona(soul_seed);
    let top_axis = innate.top_value_axis(&drift);
    let prefix = ancestor_kin_prefix(&race, birth_year, event_year, is_shop);
    let (tier, credence, fidelity, rehearsals) = if direct_witness {
        (EvidenceTier::DirectWitness, 95u8, 92u8, 8u16)
    } else {
        let span = racial_generation_span(&race).max(20);
        let gen_hops = (((birth_year - event_year).max(span)) / span).clamp(1, 7) as u8;
        let fid = 82u8.saturating_sub(gen_hops.saturating_mul(8)).max(32);
        let cred = (86u8 + (gen_hops.min(3) * 3)).min(95);
        (
            EvidenceTier::FamilyOral,
            cred,
            fid,
            u16::from(gen_hops) * 3 + 2,
        )
    };

    let subj_zh = refract_subjective_memory(
        top_axis,
        stance,
        prefix,
        event_year,
        counterpart_name,
        fact_creditor,
        fact_debtor,
        fact_witness,
    );
    let clean_subject_id = if counterpart_id.starts_with("ghost:") || counterpart_id == actor_id {
        ""
    } else {
        counterpart_id
    };
    // 對齊 `clock_and_lang.rs`：太陽曆 312 年為 day 0，每年固定 364 遊戲日
    let origin_day = i64::from(event_year - 312) * 364;
    let salience = if as_turning_point { 88 } else { 74 };

    if let Some(actor) = delta.actors.get_mut(actor_id) {
        actor.mind.ingest_historical_episode(
            event_id,
            origin_day,
            node_id,
            clean_subject_id,
            counterpart_name,
            tier,
            credence,
            fidelity,
            salience,
            rehearsals,
            subj_zh,
        );
        if !clean_subject_id.is_empty() {
            let note = match stance {
                PartyStance::BenefactorOrCreditor => {
                    format!(
                        "太陽曆{}年我家曾對{}家有舊恩或未清人情",
                        event_year, counterpart_name
                    )
                }
                PartyStance::BeneficiaryOrDebtor => {
                    format!(
                        "太陽曆{}年舊事與{}家有過深厚牽連",
                        event_year, counterpart_name
                    )
                }
                PartyStance::WitnessOrPeer => {
                    format!(
                        "老街坊，知曉太陽曆{}年與{}有關的往事",
                        event_year, counterpart_name
                    )
                }
            };
            let w_delta = match (stance, top_axis) {
                (PartyStance::BenefactorOrCreditor, ValueAxis::TangibleGain) => -12,
                (PartyStance::BenefactorOrCreditor, _) => 16,
                (
                    PartyStance::BeneficiaryOrDebtor,
                    ValueAxis::Autonomy | ValueAxis::TangibleGain,
                ) => -14,
                (PartyStance::BeneficiaryOrDebtor, _) => 14,
                (PartyStance::WitnessOrPeer, _) => 6,
            };
            let fam_hint = if direct_witness { 46 } else { 22 };
            let _ = actor.mind.fold_social_memory_from_episode(
                event_id,
                origin_day,
                clean_subject_id,
                counterpart_name,
                "",
                0,
                "街坊舊識",
                direct_witness,
                false,
                fam_hint,
                w_delta,
                &note,
                "",
            );
        }
        if as_turning_point {
            let concise_tp = format!(
                "太陽曆 {} 年家門舊事（與{}相關）至今提醒全家守穩本行生計。",
                event_year, counterpart_name
            );
            actor
                .mind
                .push_turning_point(event_id, origin_day, &concise_tp, "");
        }
        actor.rev = actor.rev.saturating_add(1);
        actor.dirty = true;
    }
}
