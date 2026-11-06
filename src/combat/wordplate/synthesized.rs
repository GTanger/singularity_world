/// 合成後的招式心法特徵
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizedTechnique {
    pub canonical_name: String,
    pub custom_name: Option<String>,
    pub effective_name: String,
    pub length: usize,
    pub vector_12d: Vector12D,
    pub craft_dims: CraftDims4D,
    pub primary_craft: CraftAspect,
    pub primary_carrier: CarrierAffinity,
    pub primary_target: TargetPart,
    pub fluency_summary: OrbitFluency,
    #[serde(default)]
    pub word_chars: Vec<String>,
    #[serde(default)]
    pub motion_sequence: Vec<String>,
    #[serde(default)]
    pub meridian_detour_note: String,
    #[serde(default)]
    pub practice_feel: String,
    #[serde(default)]
    pub soul_seed: Option<i64>,
    #[serde(default)]
    pub practice_count: u32,
    #[serde(default)]
    pub anatomy_familiarity: u32,
    #[serde(default)]
    pub learned_delta: f64,
}

impl SynthesizedTechnique {
    /// 一字雙境總括描述（在戰陣為武相，在日常為藝相）
    pub fn dual_context_summary(&self) -> String {
        format!(
            "【交鋒武相】「{}」（{}／{}，偏向{}攻向{}，星軌{}）｜【百工藝相】主修{}（鍛鑄 {:.2}／丹膳 {:.2}／採獵 {:.2}／行旅 {:.2}）",
            self.effective_name,
            self.canonical_name,
            match self.length {
                3 => "三言心法",
                5 => "五言真訣",
                7 => "七言天篇",
                _ => "句式心法",
            },
            self.primary_carrier.label(),
            self.primary_target.label(),
            self.fluency_summary.label(),
            self.primary_craft.label(),
            self.craft_dims.forge,
            self.craft_dims.culinary,
            self.craft_dims.harvest,
            self.craft_dims.wayfare,
        )
    }
    /// 根據手頭實際器物載體，推導此招式的分解基礎碎招動作序列（如「沉腰蹲步 → 蹬地躍起 → 貼身上勾拳」或「踏步斜探 → 順紋上劈 → 反腕下砍」）
    pub fn derive_elementary_motions_for_carrier(
        &self,
        carrier_override: Option<CarrierAffinity>,
    ) -> Vec<String> {
        let carrier = carrier_override.unwrap_or(self.primary_carrier);
        if carrier == self.primary_carrier && !self.motion_sequence.is_empty() {
            return self.motion_sequence.clone();
        }
        derive_elementary_motions_with_fluency(
            &self.word_chars,
            carrier,
            self.primary_target,
            self.fluency_summary,
        )
    }

    /// 計算句法招式之共鳴階層（三階共鳴階梯）
    pub fn resonance_tier(&self) -> u8 {
        if self.length >= 5 && matches!(self.fluency_summary, OrbitFluency::ChangLiu | OrbitFluency::ShunTong) {
            3
        } else if self.length >= 3 && !matches!(self.fluency_summary, OrbitFluency::XianZu) {
            2
        } else {
            1
        }
    }

    /// 依共鳴階層提供之威力加成倍率
    pub fn resonance_multiplier(&self) -> f64 {
        match self.resonance_tier() {
            3 => 1.55,
            2 => 1.25,
            _ => 1.0,
        }
    }

    /// 檢視招式特徵與字印意象，推導施展後遺留於戰場之殘留氣場（BattlefieldResidue）
    pub fn emitted_residue(&self) -> Option<BattlefieldResidue> {
        if self.resonance_tier() < 2 {
            return None;
        }

        let has_char = |targets: &[&str]| -> bool {
            self.word_chars.iter().any(|wc| {
                targets.contains(&wc.as_str()) || targets.iter().any(|&t| wc.contains(t))
            })
        };

        if has_char(&["火", "烈", "炎", "焚", "灼", "陽", "熔"]) || self.vector_12d[4] > 0.35 {
            Some(BattlefieldResidue::ScorchingHeat)
        } else if has_char(&["冰", "寒", "霜", "雪", "凝", "冷", "陰"]) || self.vector_12d[5] > 0.35 {
            Some(BattlefieldResidue::FreezingFrost)
        } else if self.primary_carrier == CarrierAffinity::Blunt
            || self.vector_12d[1] > 0.45
            || self.vector_12d[6] > 0.4
        {
            Some(BattlefieldResidue::DeepTremor)
        } else if self.vector_12d[2] > 0.45 || self.vector_12d[0] > 0.45 {
            Some(BattlefieldResidue::GaleEdge)
        } else if self.vector_12d[7] > 0.35 {
            Some(BattlefieldResidue::VortexPull)
        } else {
            None
        }
    }

    /// 判定當前招式與上一序位交鋒遺留之戰場氣機殘留（BattlefieldResidue）之交互作用
    pub fn check_residue_interaction(&self, residue: BattlefieldResidue) -> Option<ResidueInteraction> {
        let emitted = self.emitted_residue();
        match residue {
            BattlefieldResidue::ScorchingHeat => {
                if emitted == Some(BattlefieldResidue::ScorchingHeat) || emitted == Some(BattlefieldResidue::GaleEdge) {
                    Some(ResidueInteraction::Resonance)
                } else if emitted == Some(BattlefieldResidue::FreezingFrost) {
                    Some(ResidueInteraction::Backlash)
                } else {
                    None
                }
            }
            BattlefieldResidue::FreezingFrost => {
                if emitted == Some(BattlefieldResidue::FreezingFrost) || self.primary_carrier == CarrierAffinity::Blade {
                    Some(ResidueInteraction::Resonance)
                } else if emitted == Some(BattlefieldResidue::ScorchingHeat) {
                    Some(ResidueInteraction::Backlash)
                } else {
                    None
                }
            }
            BattlefieldResidue::DeepTremor => {
                if self.primary_carrier == CarrierAffinity::Blunt
                    || emitted == Some(BattlefieldResidue::DeepTremor)
                    || self.primary_carrier == CarrierAffinity::Footwork
                {
                    Some(ResidueInteraction::Resonance)
                } else if self.primary_carrier == CarrierAffinity::Pole
                    && matches!(self.fluency_summary, OrbitFluency::ZhiSe | OrbitFluency::XianZu)
                {
                    Some(ResidueInteraction::Backlash)
                } else {
                    None
                }
            }
            BattlefieldResidue::GaleEdge => {
                if emitted == Some(BattlefieldResidue::GaleEdge)
                    || self.primary_carrier == CarrierAffinity::Footwork
                    || emitted == Some(BattlefieldResidue::ScorchingHeat)
                {
                    Some(ResidueInteraction::Resonance)
                } else if self.primary_carrier == CarrierAffinity::Blunt
                    && (self.vector_12d[2] < 0.35 || matches!(self.fluency_summary, OrbitFluency::ZhiSe | OrbitFluency::XianZu))
                {
                    Some(ResidueInteraction::Backlash)
                } else {
                    None
                }
            }
            BattlefieldResidue::VortexPull => {
                if emitted == Some(BattlefieldResidue::VortexPull)
                    || self.primary_carrier == CarrierAffinity::Shield
                    || self.primary_carrier == CarrierAffinity::Unarmed
                {
                    Some(ResidueInteraction::Resonance)
                } else {
                    None
                }
            }
        }
    }
}

/// 由句中字印、器物載體、目標部位與星軌通暢度，確定性推導出三節基礎碎招動作（同法不同相）
pub fn derive_elementary_motions_with_fluency(
    word_chars: &[String],
    carrier: CarrierAffinity,
    target: TargetPart,
    fluency: OrbitFluency,
) -> Vec<String> {
    let seed_hash: usize = word_chars
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            ch.chars()
                .next()
                .map(|c| (c as usize).wrapping_mul(i + 3))
                .unwrap_or(i + 1)
        })
        .sum::<usize>()
        .wrapping_add((fluency as usize).wrapping_mul(17));
    let target_label = match target {
        TargetPart::Head => "面門",
        TargetPart::Torso => "胸脅",
        TargetPart::Arm => "腕肘",
        TargetPart::Leg => "下盤",
        TargetPart::Weapon => "兵刃握處",
        TargetPart::SelfTarget => "周身氣門",
    };
    let (step1_pool, step2_pool, step3_pool): (&[&str], &[&str], &[&str]) = match carrier {
        CarrierAffinity::Blade => match fluency {
            OrbitFluency::ChangLiu | OrbitFluency::ShunTong => (
                &["行雲踏步", "流光抽刃", "側步掠鋒"],
                &["順紋上劈", "連環點刺", "靈動削脈"],
                &["旋刃卸骨", "順勢收鋒切入", "游刃截擊"],
            ),
            OrbitFluency::ZhiSe | OrbitFluency::XianZu => (
                &["凝息迴轉", "借勢繞隙", "沉腕咬刃"],
                &["硬折刀勢", "貼骨挫力", "遲滯挑筋"],
                &["反腕重劈", "力竭頓刃", "咬牙拖劈"],
            ),
            OrbitFluency::PingWen => (
                &["踏步斜探", "沉腕抽刃", "側步掠鋒"],
                &["順紋上劈", "貼骨挑筋", "斜削腕脈"],
                &["反腕下砍", "旋刃卸骨", "順勢收鋒切入"],
            ),
        },
        CarrierAffinity::Blunt => match fluency {
            OrbitFluency::ChangLiu | OrbitFluency::ShunTong => (
                &["借勢旋身", "行雲提柄", "順風掄臂"],
                &["連綿翻砸", "沉柄引力", "斜掄順勢擊"],
                &["連環壓落", "借轉崩錘", "流勢推砸"],
            ),
            OrbitFluency::ZhiSe | OrbitFluency::XianZu => (
                &["凝息迴轉", "借勢繞隙", "沉腰死紮"],
                &["硬拗力道", "沉柄頂推", "截氣橫掄"],
                &["傾力重砸", "震臂崩砸", "頓地回擊"],
            ),
            OrbitFluency::PingWen => (
                &["沉腰踏步", "雙手提柄", "擰腰蓄臂"],
                &["橫掄壓頂", "沉柄推撞", "斜掄掃擊"],
                &["傾力重砸", "順勢沉器壓落", "震臂崩砸"],
            ),
        },
        CarrierAffinity::Pole => match fluency {
            OrbitFluency::ChangLiu | OrbitFluency::ShunTong => (
                &["靈蛇探梢", "滑步流端", "隨風圈桿"],
                &["順桿遊撥", "如電連刺", "抖腕崩顫"],
                &["穿花點穴", "迴桿行雲劈", "順勢挑浪"],
            ),
            OrbitFluency::ZhiSe | OrbitFluency::XianZu => (
                &["凝息迴轉", "借勢繞隙", "頓步卡架"],
                &["硬別桿身", "拗力直刺", "粗重橫封"],
                &["透步攢刺", "咬牙下劈", "挫身頂桿"],
            ),
            OrbitFluency::PingWen => (
                &["撐步圈桿", "滑步探尖", "斂腕平端"],
                &["順桿攔撥", "抖腕直刺", "橫桿掃擊"],
                &["透步攢刺", "迴桿劈落", "順勢挑崩"],
            ),
        },
        CarrierAffinity::Shield => match fluency {
            OrbitFluency::ChangLiu | OrbitFluency::ShunTong => (
                &["滑步迎隙", "隨風偏卸", "巧轉掩身"],
                &["遊刃引偏", "如水化勁", "借勢挑架"],
                &["反身推擊", "順勁壓身", "巧震退敵"],
            ),
            OrbitFluency::ZhiSe | OrbitFluency::XianZu => (
                &["凝息迴轉", "借勢繞隙", "死紮步位"],
                &["硬抗重轟", "鈍挫卡鎖", "貼肘死抵"],
                &["合身頂撞", "震肩反擊", "頓步爆發"],
            ),
            OrbitFluency::PingWen => (
                &["斂步橫架", "沉肩偏卸", "側身掩肘"],
                &["格開來勢", "卸力引偏", "貼肘封門"],
                &["合身頂撞", "震肩反擊", "沉肘崩開"],
            ),
        },
        CarrierAffinity::Footwork => match fluency {
            OrbitFluency::ChangLiu | OrbitFluency::ShunTong => (
                &["踏波穿影", "借風旋身", "穿花繞隙"],
                &["行雲流轉", "騰挪如電", "落葉飄搖"],
                &["掠影切入死角", "凌空折向", "隨風旋踢"],
            ),
            OrbitFluency::ZhiSe | OrbitFluency::XianZu => (
                &["凝息迴轉", "借勢繞隙", "挫步重頓"],
                &["勉強側移", "踉蹌避鋒", "回旋沉紮"],
                &["回身掃腿", "借力飄退", "咬牙沉步切入"],
            ),
            OrbitFluency::PingWen => (
                &["側身滑步", "借勢旋身", "踏隙折轉"],
                &["繞行側翼", "縮步避鋒", "騰躍換位"],
                &["掠影切入死角", "借力飄退", "旋身掃擊下盤"],
            ),
        },
        CarrierAffinity::Unarmed => match fluency {
            OrbitFluency::ChangLiu | OrbitFluency::ShunTong => (
                &["流水推雲", "沾衣貼掌", "穿花走步"],
                &["順水推舟", "遊魚穿掌", "靈巧化摔"],
                &["寸勁疊爆", "游身拍按", "連環點穴"],
            ),
            OrbitFluency::ZhiSe | OrbitFluency::XianZu => (
                &["凝息迴轉", "借勢繞隙", "咬牙沉步"],
                &["死力鎖骨", "頂肘硬格", "挫腕擒拿"],
                &["貼身上勾拳", "雙掌吐勁", "沉肘重擊"],
            ),
            OrbitFluency::PingWen => (
                &["沉腰蹲步", "斂肘探掌", "錯步欺身"],
                &["蹬地躍起", "旋腕扣脈", "順勢格臂"],
                &["貼身上勾拳", "雙掌吐勁", "沉肘重擊"],
            ),
        },
    };
    let s1 = step1_pool[seed_hash % step1_pool.len()].to_string();
    let s2 = step2_pool[(seed_hash / 3) % step2_pool.len()].to_string();
    let s3_base = step3_pool[(seed_hash / 7) % step3_pool.len()];
    let s3 = format!("{s3_base}直取{target_label}");
    vec![s1, s2, s3]
}

/// 由句中字印、器物載體與目標部位，確定性推導出三節基礎碎招動作（起勢、承轉、收合）
pub fn derive_elementary_motions_from_chars(
    word_chars: &[String],
    carrier: CarrierAffinity,
    target: TargetPart,
) -> Vec<String> {
    derive_elementary_motions_with_fluency(word_chars, carrier, target, OrbitFluency::PingWen)
}


/// 星盤造句成相（三言／五言／七言句法）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarSentence {
    pub inlays: Vec<StarInlay>,
    pub custom_name: Option<String>,
    #[serde(default)]
    pub soul_seed: Option<i64>,
    #[serde(default)]
    pub practice_count: Option<u32>,
    #[serde(default)]
    pub anatomy_familiarity: Option<u32>,
    #[serde(default)]
    pub learned_delta: Option<f64>,
}

impl StarSentence {
    /// 建立三言/五言/七言造句，嚴格校驗星盤節點與長度
    pub fn new(inlays: Vec<StarInlay>) -> Result<Self, WordPlateError> {
        let len = inlays.len();
        if len != 3 && len != 5 && len != 7 {
            return Err(WordPlateError::InvalidSentenceLength(len));
        }

        let mut seen_nodes = HashSet::with_capacity(len);
        for inlay in &inlays {
            let _ = parse_star_node(&inlay.node_id)?;
            if !seen_nodes.insert(&inlay.node_id) {
                return Err(WordPlateError::DuplicateNodeInlay(inlay.node_id.clone()));
            }
        }

        Ok(Self {
            inlays,
            custom_name: None,
            soul_seed: None,
            practice_count: None,
            anatomy_familiarity: None,
            learned_delta: None,
        })
    }
    /// 成相著譜定名
    pub fn with_custom_name(mut self, name: impl Into<String>) -> Self {
        self.custom_name = Some(name.into());
        self
    }

    pub fn set_custom_name(&mut self, name: impl Into<String>) {
        self.custom_name = Some(name.into());
    }

    pub fn with_soul_seed(mut self, seed: i64) -> Self {
        self.soul_seed = Some(seed);
        self
    }

    pub fn set_soul_seed(&mut self, seed: i64) {
        self.soul_seed = Some(seed);
    }

    /// 結合 12D 向量與星軌通暢度合成招式心法
    pub fn synthesize(&self, registry: &WordRegistry) -> Result<SynthesizedTechnique, WordPlateError> {
        let length = self.inlays.len();
        let length_mult = match length {
            3 => 1.0,
            5 => 1.8,
            7 => 2.8,
            _ => return Err(WordPlateError::InvalidSentenceLength(length)),
        };

        let words: Vec<&WordElement> = self
            .inlays
            .iter()
            .map(|inlay| {
                registry
                    .get_by_id(&inlay.word_id)
                    .ok_or_else(|| WordPlateError::WordNotFound(inlay.word_id.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let sum_eff: f64 = self.inlays.iter().map(|i| i.fluency.efficiency_multiplier()).sum();
        let avg_eff = sum_eff / length as f64;

        let sum_recoil: f64 = self.inlays.iter().map(|i| i.fluency.recoil_mitigation()).sum();
        let avg_recoil = sum_recoil / length as f64;

        let sum_rank: usize = self.inlays.iter().map(|i| i.fluency as usize).sum();
        let avg_rank = (sum_rank as f64 / length as f64).round() as usize;
        let fluency_summary = match avg_rank {
            5 => OrbitFluency::ChangLiu,
            4 => OrbitFluency::ShunTong,
            3 => OrbitFluency::PingWen,
            2 => OrbitFluency::ZhiSe,
            _ => OrbitFluency::XianZu,
        };

        let mut focus = 0.0;
        let mut shock = 0.0;
        let mut flow = 0.0;
        let mut sustain = 0.0;
        let mut tempo_early = 0.0;
        let mut tempo_mid = 0.0;
        let mut tempo_late = 0.0;
        let mut penetration = 0.0;
        let mut reach = 0.0;
        let mut vitality = 0.0;
        let mut qi_cost = 0.0;
        let mut recoil = 0.0;

        let mut craft_forge = 0.0;
        let mut craft_culinary = 0.0;
        let mut craft_harvest = 0.0;
        let mut craft_wayfare = 0.0;

        for (idx, (inlay, word)) in self.inlays.iter().zip(words.iter()).enumerate() {
            let eff = inlay.fluency.efficiency_multiplier();
            let pos_ratio = idx as f64 / ((length.saturating_sub(1)).max(1) as f64);

            let (early_w, mid_w, late_w, focus_w, flow_w, shock_w, pen_w, reach_w, sustain_w) =
                if idx == 0 {
                    // 起／前段發端
                    (1.35, 0.90, 0.75, 1.25, 0.95, 0.85, 1.00, 0.90, 1.05)
                } else if idx + 1 == length {
                    // 合／末段收束釋放
                    (0.75, 0.95, 1.40, 0.90, 0.95, 1.30, 1.10, 1.25, 0.95)
                } else {
                    // 承轉／中段流轉變化
                    (0.90, 1.30, 0.95, 1.00, 1.25, 1.00, 1.20, 1.05, 1.15)
                };

            let v = &word.vector_12d;
            focus += v.focus * focus_w * eff;
            shock += v.shock * shock_w * eff;
            flow += v.flow * flow_w * eff;
            sustain += v.sustain * sustain_w * eff;
            tempo_early += v.tempo_early * early_w * eff;
            tempo_mid += v.tempo_mid * mid_w * eff;
            tempo_late += v.tempo_late * late_w * eff;
            penetration += v.penetration * pen_w * eff;
            reach += v.reach * reach_w * eff;
            vitality += v.vitality * eff;
            qi_cost += v.qi_cost;
            recoil += v.recoil * inlay.fluency.recoil_mitigation();

            // 累計百工與行旅維度（受時序與星軌通暢度影響）
            let cd = &word.craft_dims;
            let forge_w = 1.0 + 0.15 * pos_ratio;
            let culinary_w = 1.0 + 0.15 * pos_ratio;
            let harvest_w = 1.0 + 0.15 * (1.0 - pos_ratio);
            let wayfare_w = 1.0 + 0.15 * (1.0 - pos_ratio);

            craft_forge += cd.forge * forge_w * eff;
            craft_culinary += cd.culinary * culinary_w * eff;
            craft_harvest += cd.harvest * harvest_w * eff;
            craft_wayfare += cd.wayfare * wayfare_w * eff;
        }

        let combined_craft = CraftDims4D {
            forge: craft_forge * length_mult,
            culinary: craft_culinary * length_mult,
            harvest: craft_harvest * length_mult,
            wayfare: craft_wayfare * length_mult,
        };
        let primary_craft = combined_craft.primary_aspect();

        let combined_vector = Vector12D {
            focus: focus * length_mult,
            shock: shock * length_mult,
            flow: flow * length_mult,
            sustain: sustain * length_mult,
            tempo_early: tempo_early * length_mult,
            tempo_mid: tempo_mid * length_mult,
            tempo_late: tempo_late * length_mult,
            penetration: penetration * length_mult * avg_eff,
            reach: reach * length_mult * avg_eff,
            vitality: vitality * length_mult * avg_eff,
            qi_cost: qi_cost * length_mult,
            recoil: recoil * avg_recoil,
        };

        let mut carrier_counts = HashMap::new();
        for w in &words {
            *carrier_counts.entry(w.carrier_affinity).or_insert(0usize) += 1;
        }
        let primary_carrier = carrier_counts
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(c, _)| c)
            .unwrap_or(CarrierAffinity::Unarmed);

        let mut target_counts = HashMap::new();
        for w in &words {
            *target_counts.entry(w.target_part).or_insert(0usize) += 1;
        }
        let primary_target = target_counts
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(t, _)| t)
            .unwrap_or(TargetPart::Torso);

        fn semantic_weight(semantic: &str) -> i32 {
            match semantic {
                "動" => 10,
                "屬" => 9,
                "受" => 8,
                "構" => 7,
                "態" => 6,
                "補" => 5,
                "否" => 4,
                "主" => 3,
                "助" => 2,
                "玄" => 1,
                _ => 0,
            }
        }

        let mut indexed_weights: Vec<(usize, i32, &str)> = words
            .iter()
            .enumerate()
            .map(|(idx, w)| (idx, semantic_weight(&w.semantic), w.ch.as_str()))
            .collect();
        indexed_weights.sort_by(|a, b| b.1.cmp(&a.1));
        let keep_count = match length {
            3 => 2.min(indexed_weights.len()),
            5 => 3.min(indexed_weights.len()),
            _ => 4.min(indexed_weights.len()),
        };
        let mut selected = indexed_weights[..keep_count].to_vec();
        selected.sort_by_key(|(idx, _, _)| *idx);
        let canonical_name: String = selected.into_iter().map(|(_, _, ch)| ch).collect();

        let effective_name = self
            .custom_name
            .as_ref()
            .filter(|s| !s.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| canonical_name.clone());

        let word_chars: Vec<String> = words.iter().map(|w| w.ch.clone()).collect();
        let motion_sequence = derive_elementary_motions_with_fluency(
            &word_chars,
            primary_carrier,
            primary_target,
            fluency_summary,
        );

        let seed = self.soul_seed.unwrap_or(0);
        let meridian_detour_note = compute_auto_detour_meridian_note(&words, seed);
        let motions_feel = motion_sequence.join("、");
        let practice_feel = format!(
            "以{}之序演練，氣勢{}。{}",
            motions_feel,
            fluency_summary.label(),
            meridian_detour_note
        );

        Ok(SynthesizedTechnique {
            canonical_name,
            custom_name: self.custom_name.clone(),
            effective_name,
            length,
            vector_12d: combined_vector,
            craft_dims: combined_craft,
            primary_craft,
            primary_carrier,
            primary_target,
            fluency_summary,
            word_chars,
            motion_sequence,
            meridian_detour_note,
            practice_feel,
            soul_seed: self.soul_seed,
            practice_count: self.practice_count.unwrap_or(25),
            anatomy_familiarity: self.anatomy_familiarity.unwrap_or(20),
            learned_delta: self.learned_delta.unwrap_or(0.0),
        })
    }
}
