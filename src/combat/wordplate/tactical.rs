
/// 田忌賽馬四戰術序位
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TacticalSlot {
    #[default]
    Opening,  // 起手式（爭奪先手與試探破綻）
    Midgame,  // 常態連攜（氣脈流轉與正面破甲）
    Finisher, // 破綻殺招（收尾絕殺與乾坤一擲）
    Defense,  // 守勢身法（卸勁固本與化解殺意）
}

impl TacticalSlot {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Opening => "起手式",
            Self::Midgame => "常態連攜",
            Self::Finisher => "破綻殺招",
            Self::Defense => "守勢身法",
        }
    }
}

/// 戰術配置方案（包含四戰術序位）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TacticalLoadout {
    pub opening: Option<StarSentence>,
    pub midgame: Option<StarSentence>,
    pub finisher: Option<StarSentence>,
    pub defense: Option<StarSentence>,
}

impl TacticalLoadout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_slot(&mut self, slot: TacticalSlot, sentence: StarSentence) {
        match slot {
            TacticalSlot::Opening => self.opening = Some(sentence),
            TacticalSlot::Midgame => self.midgame = Some(sentence),
            TacticalSlot::Finisher => self.finisher = Some(sentence),
            TacticalSlot::Defense => self.defense = Some(sentence),
        }
    }

    pub fn get_slot(&self, slot: TacticalSlot) -> Option<&StarSentence> {
        match slot {
            TacticalSlot::Opening => self.opening.as_ref(),
            TacticalSlot::Midgame => self.midgame.as_ref(),
            TacticalSlot::Finisher => self.finisher.as_ref(),
            TacticalSlot::Defense => self.defense.as_ref(),
        }
    }

    /// 計算活躍槽位之百工四維總和（若均未配置則回傳全 0 向量）
    pub fn total_craft_dims(&self, registry: &WordRegistry) -> CraftDims4D {
        let mut sum = CraftDims4D::default();
        let slots = [&self.opening, &self.midgame, &self.finisher, &self.defense];
        for slot in slots.into_iter().flatten() {
            if let Ok(tech) = slot.synthesize(registry) {
                sum.forge += tech.craft_dims.forge;
                sum.culinary += tech.craft_dims.culinary;
                sum.harvest += tech.craft_dims.harvest;
                sum.wayfare += tech.craft_dims.wayfare;
            }
        }
        sum
    }

    /// 計算詞匣中擁有之玄意詞元帶來的幸運加成
    /// 每擁有 1 種不同的「玄」語意詞元提供 +0.025，上限為 +0.30
    pub fn xuan_luck_bonus(&self, casket: &WordCasket, registry: &WordRegistry) -> f32 {
        let mut xuan_count = 0_usize;
        for (word_id, count) in casket.list() {
            if count > 0 {
                if let Some(elem) = registry.get_by_id(word_id) {
                    if elem.semantic == "玄" {
                        xuan_count += 1;
                    }
                }
            }
        }
        ((xuan_count as f32) * 0.025).min(0.30)
    }
}

/// 生命種子（soul_seed）＋ 361 星盤（760 條星軌阻力）＝ 萬人萬相戰術配置建構
pub fn build_loadout_from_soul_seed(
    seed: i64,
    casket: Option<&WordCasket>,
    registry: &WordRegistry,
) -> TacticalLoadout {
    let costs = crate::db::expand_soul_seed_to_topology_costs(seed);
    let (alpha, beta, gamma) = crate::db::expand_soul_seed_to_combat_axes(seed);

    // 1. 計算 20 主經脈樞紐（Hub 1..=20）之阻力綜合評價
    let mut hub_scores: Vec<(usize, f64)> = (1..=20)
        .map(|h| {
            let a_cost = costs.get(h - 1).copied().unwrap_or(13.0);
            let b_start = 20 + 5 * (h - 1);
            let b_avg: f64 = (0..5)
                .map(|j| costs.get(b_start + j).copied().unwrap_or(13.0))
                .sum::<f64>()
                / 5.0;
            let total_resistance = a_cost * 0.4 + b_avg * 0.6;
            (h, total_resistance)
        })
        .collect();

    // 依阻力由低到高（最順通）排序
    hub_scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // 取出最通暢的 4 條相異經脈樞紐，分別對應 [Opening, Midgame, Finisher, Defense]
    let selected_hubs = [
        hub_scores[0].0,
        hub_scores[1 % hub_scores.len()].0,
        hub_scores[2 % hub_scores.len()].0,
        hub_scores[3 % hub_scores.len()].0,
    ];

    // 2. 獲取備選詞元列表
    let casket_words: Vec<String> = casket
        .map(|c| {
            c.list()
                .into_iter()
                .filter(|(w, cnt)| {
                    *cnt > 0
                        && !matches!(
                            *w,
                            "qiao_shu" | "mu" | "he_tai" | "shui" | "huo_shou"
                        )
                })
                .map(|(w, _)| w.to_string())
                .collect()
        })
        .unwrap_or_default();

    let use_casket = casket_words.len() >= 4;

    let slots = [
        TacticalSlot::Opening,
        TacticalSlot::Midgame,
        TacticalSlot::Finisher,
        TacticalSlot::Defense,
    ];

    let mut loadout = TacticalLoadout::new();
    let mut used_word_ids = HashSet::new();

    for (slot_idx, &slot) in slots.iter().enumerate() {
        let hub = selected_hubs[slot_idx];

        // 在該經脈樞紐下選取 3 節點路徑：1 個 Hub 節點 + 1 個 Logic 節點 + 1 個 Peripheral 節點
        let node1 = format!("N{:03}", hub);
        let cost1 = costs.get(hub - 1).copied().unwrap_or(10.0);

        let b_base = 20 + 5 * (hub - 1);
        let mut best_j = 0;
        let mut min_b = f64::MAX;
        for j in 0..5 {
            let c = costs.get(b_base + j).copied().unwrap_or(13.0);
            if c < min_b {
                min_b = c;
                best_j = j;
            }
        }
        let node2 = format!("N{:03}", 20 + 5 * (hub - 1) + best_j + 1);
        let cost2 = min_b;

        let c_base = 120 + 12 * (hub - 1);
        let mut best_s = 0;
        let mut min_c = f64::MAX;
        for s in 0..12 {
            let c = costs.get(c_base + s).copied().unwrap_or(13.0);
            if c < min_c {
                min_c = c;
                best_s = s;
            }
        }
        let node3 = format!("N{:03}", 120 + 12 * (hub - 1) + best_s + 1);
        let cost3 = min_c;

        let nodes = [node1, node2, node3];
        let node_costs = [cost1, cost2, cost3];

        // 選取 3 個詞元
        let mut chosen_words = Vec::with_capacity(3);
        for pos in 0..3 {
            let wid = if use_casket {
                let mut picked = None;
                for offset in 0..casket_words.len() {
                    let idx = (slot_idx * 3 + pos + offset) % casket_words.len();
                    let candidate = &casket_words[idx];
                    if !used_word_ids.contains(candidate) || offset == casket_words.len() - 1 {
                        picked = Some(candidate.clone());
                        break;
                    }
                }
                picked.unwrap_or_else(|| casket_words[pos % casket_words.len()].clone())
            } else {
                let mut best_word = String::new();
                let mut best_score = -999999.0;
                let seed_u = seed as u64;

                for (elem_idx, elem) in registry.ordered_elements().enumerate() {
                    if used_word_ids.contains(&elem.id) {
                        continue;
                    }
                    let v = &elem.vector_12d;
                    let slot_score = match slot {
                        TacticalSlot::Opening => v.tempo_early * 2.2 + v.focus * 1.5 + beta * 1.8,
                        TacticalSlot::Midgame => v.tempo_mid * 2.0 + v.flow * 1.6 + v.penetration * 1.5,
                        TacticalSlot::Finisher => v.tempo_late * 2.4 + v.shock * 2.0 + alpha * 2.0,
                        TacticalSlot::Defense => v.sustain * 2.2 + v.vitality * 1.8 + gamma * 2.0,
                    };

                    let hash = (seed_u ^ (elem_idx as u64).wrapping_mul(1000000007))
                        .wrapping_add(((slot_idx * 3 + pos) as u64).wrapping_mul(2654435761))
                        .wrapping_mul(6364136223846793005);
                    let hash_f = (hash % 1000) as f64 / 1000.0 * 2.5;

                    let total = slot_score + hash_f;
                    if total > best_score {
                        best_score = total;
                        best_word = elem.id.clone();
                    }
                }

                if best_word.is_empty() {
                    let fallback_idx = (slot_idx * 3 + pos + (seed.abs() as usize % 100)) % registry.len().max(1);
                    registry.ordered_elements().nth(fallback_idx).map(|e| e.id.clone()).unwrap_or_else(|| "wu".to_string())
                } else {
                    best_word
                }
            };

            used_word_ids.insert(wid.clone());
            chosen_words.push(wid);
        }

        let inlays: Vec<StarInlay> = (0..3)
            .map(|i| {
                StarInlay::with_fluency(
                    nodes[i].clone(),
                    chosen_words[i].clone(),
                    OrbitFluency::from_cost(node_costs[i]),
                )
            })
            .collect();

        if let Ok(mut sentence) = StarSentence::new(inlays) {
            let ch_first = registry.get_by_id(&chosen_words[0]).map(|e| e.ch.as_str()).unwrap_or("勢");
            let ch_last = registry.get_by_id(&chosen_words[2]).map(|e| e.ch.as_str()).unwrap_or("決");

            let suffix_pool: &[&str] = match slot {
                TacticalSlot::Opening => &[
                    "探隙勢",
                    "奪人勢",
                    "截脈式",
                    "掠影勢",
                    "破空式",
                    "掣電勢",
                    "穿雲勢",
                    "破虛式",
                ],
                TacticalSlot::Midgame => &[
                    "截江連",
                    "奔湧連",
                    "流水擊",
                    "碎骨連",
                    "妙手連",
                    "拂柳連",
                    "驟雨連",
                    "崩濤連",
                ],
                TacticalSlot::Finisher => &[
                    "裂骨擊",
                    "一擲破",
                    "雷霆破",
                    "斷岳轟",
                    "穿隙破",
                    "透骨殺",
                    "開天轟",
                    "崩解殺",
                ],
                TacticalSlot::Defense => &[
                    "金湯身",
                    "卸勁步",
                    "換斗步",
                    "避實身",
                    "周天護",
                    "無轉身",
                    "潛水步",
                    "抱一身",
                ],
            };

            let s_idx = ((seed.abs() as usize) / 7 + slot_idx) % suffix_pool.len();
            let dyn_name = format!("{}{}{}", ch_first, ch_last, suffix_pool[s_idx]);
            sentence.set_custom_name(dyn_name);
            sentence.set_soul_seed(seed);
            loadout.set_slot(slot, sentence);
        }
    }

    loadout
}

/// 依據造句詞元星位與角色生命種子，推導自動繞行經絡阻力之體感描述（同法不同相）
pub fn compute_auto_detour_meridian_note(_words: &[&WordEntry], soul_seed: i64) -> String {
    crate::combat::human_phase::InnateBonePhase::from_seed(soul_seed).describe_bone_phase_zh()
}

/// 依據生命種子體察周天經絡在靜修體悟中的自發分流與最暢通經脈
pub fn somatic_meridian_awareness(soul_seed: i64) -> String {
    crate::combat::human_phase::InnateBonePhase::from_seed(soul_seed).describe_bone_phase_zh()
}

/// 戰術交鋒每階段記錄（承載碎招、套路與成招技能狀態）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhaseClashRecord {
    pub phase: usize,
    pub slot: TacticalSlot,
    pub a_technique_name: String,
    pub b_technique_name: String,
    pub a_initiative_score: f64,
    pub b_initiative_score: f64,
    pub damage_to_a: i32,
    pub damage_to_b: i32,
    pub narrative: String,
    #[serde(default)]
    pub a_manifested_as_skill: bool,
    #[serde(default)]
    pub b_manifested_as_skill: bool,
    #[serde(default)]
    pub counter_reason: Option<String>,
    #[serde(default)]
    pub a_motions: Vec<String>,
    #[serde(default)]
    pub b_motions: Vec<String>,
    #[serde(default)]
    pub a_completed_motions: usize,
    #[serde(default)]
    pub b_completed_motions: usize,
    #[serde(default)]
    pub a_target_part: TargetPart,
    #[serde(default)]
    pub b_target_part: TargetPart,
    #[serde(default)]
    pub a_carrier: CarrierAffinity,
    #[serde(default)]
    pub b_carrier: CarrierAffinity,
    #[serde(default)]
    pub a_posture_after: PostureState,
    #[serde(default)]
    pub b_posture_after: PostureState,
    #[serde(default)]
    pub active_residue_before: Option<BattlefieldResidue>,
    #[serde(default)]
    pub residue_left_after: Option<BattlefieldResidue>,
    #[serde(default)]
    pub a_residue_interaction: Option<ResidueInteraction>,
    #[serde(default)]
    pub b_residue_interaction: Option<ResidueInteraction>,
    #[serde(default)]
    pub a_resonance_tier: u8,
    #[serde(default)]
    pub b_resonance_tier: u8,
}

/// 單回合交鋒結算成果（落實「碎招連貫叫套路，成功叫技能，失敗被格擋就只是碎招」）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoutClashOutcome {
    pub winner_side: String, // "side_a", "side_b", or "draw"
    pub a_manifested_as_skill: bool,
    pub b_manifested_as_skill: bool,
    pub a_technique_name: String,
    pub b_technique_name: String,
    pub a_initiative_score: f64,
    pub b_initiative_score: f64,
    pub damage_to_a: i32,
    pub damage_to_b: i32,
    pub counter_reason: Option<String>,
    pub narrative: String,
    #[serde(default)]
    pub a_motions: Vec<String>,
    #[serde(default)]
    pub b_motions: Vec<String>,
    #[serde(default)]
    pub a_completed_motions: usize,
    #[serde(default)]
    pub b_completed_motions: usize,
    #[serde(default)]
    pub a_target_part: TargetPart,
    #[serde(default)]
    pub b_target_part: TargetPart,
    #[serde(default)]
    pub a_carrier: CarrierAffinity,
    #[serde(default)]
    pub b_carrier: CarrierAffinity,
    #[serde(default)]
    pub a_posture_after: PostureState,
    #[serde(default)]
    pub b_posture_after: PostureState,
    #[serde(default)]
    pub active_residue_before: Option<BattlefieldResidue>,
    #[serde(default)]
    pub residue_left_after: Option<BattlefieldResidue>,
    #[serde(default)]
    pub a_residue_interaction: Option<ResidueInteraction>,
    #[serde(default)]
    pub b_residue_interaction: Option<ResidueInteraction>,
    #[serde(default)]
    pub a_resonance_tier: u8,
    #[serde(default)]
    pub b_resonance_tier: u8,
}

/// 田忌賽馬交鋒總報告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticalClashReport {
    pub winner: String,
    pub rounds: Vec<PhaseClashRecord>,
    pub side_a_final_hp: i32,
    pub side_b_final_hp: i32,
    pub summary: String,
}
