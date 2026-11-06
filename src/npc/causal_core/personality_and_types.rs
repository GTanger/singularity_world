fn splitmix64_next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// 先天心性六向單純形展開（Schwartz 圓周對偶偏置，總和恆守恆 600，各向 45..=165）
/// 順序：[Pride(傲骨), Care(護犢), Honor(重義), Profit(嗜利), Craft(匠心), Order(守序)]
pub fn innate_personality_simplex(seed: u64) -> [i32; 6] {
    let mut sm = seed ^ 0xa076_1d64_78bd_642f;
    let u1 = splitmix64_next(&mut sm);
    let u2 = splitmix64_next(&mut sm);
    let theta1 = (u1 as f64 / u64::MAX as f64) * std::f64::consts::TAU;
    let theta2 = (u2 as f64 / u64::MAX as f64) * std::f64::consts::TAU;

    let (s1, c1) = theta1.sin_cos();
    let c2 = theta2.cos();

    // Schwartz 圓周對偶偏置（Care/Honor 與 Profit/Pride 對蹠，Order 與 Craft 對蹠）
    let coupled = [
        -35.0 * c1 + 15.0 * s1, // Pride
         35.0 * c1 + 15.0 * s1, // Care
         35.0 * c1 - 15.0 * s1, // Honor
        -35.0 * c1 - 15.0 * s1, // Profit
         40.0 * c2,             // Craft
        -40.0 * c2,             // Order
    ];

    let mut vars = [0.0f64; 6];
    let mut var_sum = 0.0f64;
    for v in &mut vars {
        let uv = splitmix64_next(&mut sm);
        *v = ((uv % 1000) as f64 / 1000.0 - 0.5) * 30.0;
        var_sum += *v;
    }
    let mean_v = var_sum / 6.0;

    let mut cur = [0.0f64; 6];
    for k in 0..6 {
        cur[k] = (100.0 + coupled[k] + (vars[k] - mean_v)).clamp(45.0, 165.0);
    }

    // Bounded simplex water-filling 投影 onto 600.0
    for _ in 0..12 {
        let sum: f64 = cur.iter().sum();
        let diff = 600.0 - sum;
        if diff.abs() < 1e-9 {
            break;
        }
        let mut free_count = 0usize;
        for &v in &cur {
            if (diff > 0.0 && v < 165.0 - 1e-5) || (diff < 0.0 && v > 45.0 + 1e-5) {
                free_count += 1;
            }
        }
        if free_count == 0 {
            break;
        }
        let delta = diff / (free_count as f64);
        for v in &mut cur {
            if (diff > 0.0 && *v < 165.0 - 1e-5) || (diff < 0.0 && *v > 45.0 + 1e-5) {
                *v = (*v + delta).clamp(45.0, 165.0);
            }
        }
    }

    let mut int_b = [0i32; 6];
    for (i, v) in cur.iter().enumerate() {
        int_b[i] = (v.round() as i32).clamp(45, 165);
    }

    // 餘數校準至嚴格 600
    let mut rem = 600 - int_b.iter().sum::<i32>();
    while rem != 0 {
        let step = if rem > 0 { 1 } else { -1 };
        let mut best_i = None;
        let mut best_val = if rem > 0 { i32::MIN } else { i32::MAX };
        for (i, &val) in int_b.iter().enumerate() {
            if rem > 0 && val < 165 {
                if val > best_val {
                    best_val = val;
                    best_i = Some(i);
                }
            } else if rem < 0 && val > 45 {
                if val < best_val {
                    best_val = val;
                    best_i = Some(i);
                }
            }
        }
        let target_idx = best_i.unwrap_or_else(|| {
            int_b.iter().position(|&v| (rem > 0 && v < 165) || (rem < 0 && v > 45)).unwrap_or(0)
        });
        int_b[target_idx] += step;
        rem -= step;
    }

    int_b
}

/// 四大典型生活處境槽位（CAPS If-Then 處境簽名矩陣）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PsychSituationSlot {
    ScarcitySurvival = 0,
    VocationalDuty = 1,
    RelationalEncounter = 2,
    HighStressCrisis = 3,
}

/// 計算處境觸發相的零和位移量（各向 [-32, 32]，總和恆為 0）
pub fn situational_if_then_offset(seed: u64, slot: PsychSituationSlot) -> [i32; 6] {
    let slot_idx = slot as usize;
    // 6 facets: [Pride, Care, Honor, Profit, Craft, Order]
    let base: [i32; 6] = match slot {
        PsychSituationSlot::ScarcitySurvival => [2, 4, -4, 8, -4, -6],
        PsychSituationSlot::VocationalDuty => [2, -4, -6, -6, 8, 6],
        PsychSituationSlot::RelationalEncounter => [-4, 6, 8, -6, -2, -2],
        PsychSituationSlot::HighStressCrisis => [8, 6, -4, -2, -6, -2],
    };

    let mut sm = (seed.wrapping_mul(0x517c_c1b7_2722_0a95))
        ^ ((slot_idx as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15).wrapping_add(0xbf58_476d_1ce4_e5b9));

    let mut offsets = [0i32; 6];
    for k in 0..6 {
        let u = splitmix64_next(&mut sm);
        let var = ((u % 29) as i32) - 14;
        offsets[k] = (base[k] + var).clamp(-28, 28);
    }

    // 嚴格校準和至 0（各元素在 [-32, 32] 內）
    let mut sum: i32 = offsets.iter().sum();
    while sum != 0 {
        let step = if sum > 0 { -1 } else { 1 };
        let mut best_i = None;
        let mut best_dist = -1i32;
        for (i, &val) in offsets.iter().enumerate() {
            if step > 0 && val < 32 {
                let dist = 32 - val;
                if dist > best_dist {
                    best_dist = dist;
                    best_i = Some(i);
                }
            } else if step < 0 && val > -32 {
                let dist = val - (-32);
                if dist > best_dist {
                    best_dist = dist;
                    best_i = Some(i);
                }
            }
        }
        let target_idx = match best_i {
            Some(i) => i,
            None => break,
        };
        offsets[target_idx] += step;
        sum += step;
    }

    offsets
}

/// 計算後天「所學 × 所熟 × 所歷」性格塑造偏移量
pub fn compute_experiential_personality_shift(
    settlement: &str,
    same_task_repetitions: u32,
    hooks: &[RelationalHook],
    seed: u64,
) -> [i32; 6] {
    let mut shift = [0i32; 6];
    // 0: Pride, 1: Care, 2: Honor, 3: Profit, 4: Craft, 5: Order

    // 1. 所學（聚落風土道統，<= 8 分）
    if settlement.contains("文曜") {
        shift[5] += 6; // Order +6
        shift[4] += 4; // Craft +4
    } else if settlement.contains("聖塔") {
        shift[4] += 6; // Craft +6
        shift[0] += 4; // Pride +4
    } else if settlement.contains("南哨") {
        shift[2] += 6; // Honor +6
        shift[1] += 4; // Care +4
    } else if settlement.contains("風舶") {
        shift[3] += 6; // Profit +6
        shift[2] += 4; // Honor +4
    } else if settlement.contains("青林") {
        shift[1] += 6; // Care +6
        shift[4] += 4; // Craft +4
    } else if settlement.contains("白沙") {
        shift[0] += 6; // Pride +6
        shift[3] += 4; // Profit +4
    }

    // 2. 所熟（日課工序慣性冪律）
    let innate = innate_personality_simplex(seed);
    let mut top_idx = 0;
    let mut top_val = innate[0];
    for (i, &v) in innate.iter().enumerate().skip(1) {
        if v > top_val {
            top_val = v;
            top_idx = i;
        }
    }
    let habit_bonus = ((10.0 * ((same_task_repetitions.max(1) as f64) / 25.0).powf(0.45)) as i32).clamp(2, 28);
    shift[top_idx] += habit_bonus;

    // 3. 所歷（恩怨、償清與託付之身心刻痕）
    for h in hooks {
        let kl = &h.kind_label;
        let sm = &h.summary;
        let matches_any = |terms: &[&str]| terms.iter().any(|t| kl.contains(t) || sm.contains(t));

        if matches_any(&["恩", "守諾", "解囊", "馳援", "完修"]) {
            shift[1] += 8;  // Care += 8
            shift[2] += 10; // Honor += 10
            shift[3] -= 4;  // Profit -= 4
        }
        if matches_any(&["遭拒", "追討", "結怨", "嫌隙", "展期"]) {
            if (seed & 1) == 0 {
                shift[0] += 10; // Pride += 10
                shift[3] += 6;  // Profit += 6
                shift[1] -= 4;  // Care -= 4
            } else {
                shift[3] += 10; // Profit += 10
                shift[5] += 6;  // Order += 6
                shift[2] -= 4;  // Honor -= 4
            }
        }
        if matches_any(&["託付", "代辦", "立契"]) {
            shift[5] += 7; // Order += 7
            shift[4] += 7; // Craft += 7
        }
    }

    shift
}

/// 住民性格錨點（Cornerstone，決定高壓破防出口、不可妥協底線與行動剪枝）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Cornerstone {
    /// 傲骨：寧折不彎，不肯低聲賒欠或受人冷眼，遇壓價或吃閉門羹則轉頭自謀生路
    Pride,
    /// 護犢：先顧家眷與晚輩學徒溫飽，甘願替徒弟與家人扛下損耗
    Care,
    /// 重義：欠恩必償、見同袍有難必先馳援，合社獵團與街坊有難絕不退縮
    Honor,
    /// 嗜利：精算行情價差與囤貨時機，見市集急缺必搶先跨區套利
    Profit,
    /// 匠心：對器物火候與分寸極度執拗，器況稍有磨損必先校準妥貼才肯開工
    Craft,
    /// 守序：嚴守時刻、公所條例與巡防交接，帳目與差事一絲不苟
    Order,
}

impl Cornerstone {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pride => "傲骨",
            Self::Care => "護犢",
            Self::Honor => "重義",
            Self::Profit => "嗜利",
            Self::Craft => "匠心",
            Self::Order => "守序",
        }
    }

    pub fn from_role_and_seed(role_hint: &str, seed: u64) -> Self {
        let role_hint = crate::npc::causal_core::strip_trailing_person_name_from_role(role_hint);
        let mut b = innate_personality_simplex(seed);
        if role_hint.contains("家眷") || role_hint.contains("內當家") || role_hint.contains("持家") {
            b[1] += 8; // Care
            b[5] += 4; // Order
        } else if role_hint.contains("商")
            || role_hint.contains("主事")
            || role_hint.contains("執事")
            || role_hint.contains("帳房")
            || role_hint.contains("行旅")
        {
            b[3] += 8; // Profit
            b[5] += 4; // Order
        } else if role_hint.contains("鐵")
            || role_hint.contains("鍛")
            || role_hint.contains("木工")
            || role_hint.contains("木匠")
            || role_hint.contains("木器")
            || role_hint.contains("竹")
            || role_hint.contains("書")
            || role_hint.contains("醫")
        {
            b[4] += 8; // Craft
            b[0] += 4; // Pride
        } else if role_hint.contains("巡") || role_hint.contains("衛") || role_hint.contains("哨") {
            b[2] += 8; // Honor
            b[5] += 4; // Order
        } else if role_hint.contains("獵") || role_hint.contains("採") || role_hint.contains("樵") {
            b[0] += 6; // Pride
            b[4] += 6; // Craft
        }

        let mut max_idx = 0;
        let mut max_val = b[0];
        for (i, &val) in b.iter().enumerate().skip(1) {
            if val > max_val {
                max_val = val;
                max_idx = i;
            }
        }
        match max_idx {
            0 => Self::Pride,
            1 => Self::Care,
            2 => Self::Honor,
            3 => Self::Profit,
            4 => Self::Craft,
            _ => Self::Order,
        }
    }

    /// 當壓力超過閾值（`stress >= 72`）時，依性格錨點產出截然不同的破防轉折行動
    pub fn stress_break_directive(self, settlement: &str, vessel_name: &str) -> (&'static str, String, String) {
        let s = if settlement.trim().is_empty() { "文曜" } else { settlement.trim() };
        match self {
            Self::Pride => (
                "拂袖離場獨飲散悶",
                format!("連軸奔波又逢瑣事掣肘，傲骨難耐，索性收起「{vessel_name}」赴{s}茶寮獨飲散悶"),
                "眉宇微凜、將隨身器物往案上一頓，獨自斟茶平復胸中鬱氣".to_string(),
            ),
            Self::Care => (
                "攜糧歸家照看家小",
                format!("心頭牽掛家中老小與學徒溫飽，放下手頭瑣務先返{s}家宅安頓"),
                "把早先置辦的乾糧細細包好塞入懷中，快步趕回家宅照看家人".to_string(),
            ),
            Self::Honor => (
                "赴哨所尋同袍敘話",
                format!("胸中塊壘難消，轉赴{s}街坊同袍處對坐敘話、互通聲氣"),
                "拍去肩頭風塵，與熟識的同袍並肩坐下，痛快分飲壺中熱茶".to_string(),
            ),
            Self::Profit => (
                "轉赴外埠套利翻本",
                format!("見本地利薄費時，當機立斷盤算跨區行情，準備轉赴鄰埠市集尋覓急缺差價"),
                "指尖飛快撥算荷包成色，瞇眼打量市集貨擔，盤算著下一趟翻本買賣".to_string(),
            ),
            Self::Craft => (
                "閉門精校隨身器物",
                format!("自覺手頭分寸微亂，決意暫停外務，專心閉門精校「{vessel_name}」"),
                format!("屏息凝神，反覆摩挲並校準手中的「{vessel_name}」，不許分毫差池"),
            ),
            Self::Order => (
                "赴湯屋洗沐重整作息",
                format!("察覺自身作息與氣息失序，按規矩先赴{s}溫水浴堂洗沐滌慮"),
                "整肅衣冠、緩緩吐納，藉溫水熱氣將連日積攢的勞頓規整撫平".to_string(),
            ),
        }
    }
}

/// 住民多元性格傾向權重（六大性格維度，由角色身分基底、種子微擾及當前處境動態調製）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PluralPersonalityWeights {
    pub pride: i32,
    pub care: i32,
    pub honor: i32,
    pub profit: i32,
    pub craft: i32,
    pub order: i32,
}

impl PluralPersonalityWeights {
    pub fn weight_for(self, cornerstone: Cornerstone) -> i32 {
        match cornerstone {
            Cornerstone::Pride => self.pride,
            Cornerstone::Care => self.care,
            Cornerstone::Honor => self.honor,
            Cornerstone::Profit => self.profit,
            Cornerstone::Craft => self.craft,
            Cornerstone::Order => self.order,
        }
    }
}

pub type PersonalityWeights = PluralPersonalityWeights;

/// 結合角色身分、先天單純形、處境If-Then位移與「所學×所熟×所歷」動態計算多元性格維度權重
pub fn compute_plural_personality_weights_with_experience(
    role_hint: &str,
    settlement: &str,
    seed: u64,
    stress: u8,
    magnesium: i32,
    same_task_repetitions: u32,
    hooks: &[RelationalHook],
    situation: Option<PsychSituationSlot>,
) -> PluralPersonalityWeights {
    let slot = match situation {
        Some(s) => s,
        None if stress >= 60 => PsychSituationSlot::HighStressCrisis,
        None if magnesium <= 4 => PsychSituationSlot::ScarcitySurvival,
        None => PsychSituationSlot::VocationalDuty,
    };

    let innate = innate_personality_simplex(seed);
    let sit = situational_if_then_offset(seed, slot);
    let exp = compute_experiential_personality_shift(settlement, same_task_repetitions, hooks, seed);

    // 溫和的職業性向引導（<= 8 分）
    let role_hint = crate::npc::causal_core::strip_trailing_person_name_from_role(role_hint);
    let mut role_nudge = [0i32; 6];
    if role_hint.contains("家眷") || role_hint.contains("內當家") || role_hint.contains("持家") {
        role_nudge[1] += 8; // Care
        role_nudge[5] += 4; // Order
    } else if role_hint.contains("商")
        || role_hint.contains("主事")
        || role_hint.contains("執事")
        || role_hint.contains("帳房")
        || role_hint.contains("行旅")
    {
        role_nudge[3] += 8; // Profit
        role_nudge[5] += 4; // Order
    } else if role_hint.contains("鐵")
        || role_hint.contains("鍛")
        || role_hint.contains("磨")
        || role_hint.contains("石")
        || role_hint.contains("木工")
        || role_hint.contains("木匠")
        || role_hint.contains("木器")
        || role_hint.contains("竹")
        || role_hint.contains("書")
        || role_hint.contains("醫")
    {
        role_nudge[4] += 8; // Craft
        role_nudge[0] += 4; // Pride
    } else if role_hint.contains("巡") || role_hint.contains("衛") || role_hint.contains("哨") {
        role_nudge[2] += 8; // Honor
        role_nudge[5] += 4; // Order
    } else if role_hint.contains("獵") || role_hint.contains("採") || role_hint.contains("樵") {
        role_nudge[0] += 6; // Pride
        role_nudge[4] += 6; // Craft
    }

    // 處境情勢急迫調節
    let mut acute = [0i32; 6];
    if stress >= 60 {
        acute[0] += 15; // Pride
        acute[1] += 15; // Care
        acute[3] += 15; // Profit
    }
    if magnesium <= 4 {
        acute[3] += 20; // Profit
        acute[1] += 15; // Care
    }
    if magnesium >= 25 {
        acute[2] += 20; // Honor
        acute[4] += 15; // Craft
        acute[0] += 15; // Pride
    }

    let val = |idx: usize| -> i32 {
        (innate[idx] + sit[idx] + exp[idx] + role_nudge[idx] + acute[idx]).clamp(0, 250)
    };

    PluralPersonalityWeights {
        pride: val(0),
        care: val(1),
        honor: val(2),
        profit: val(3),
        craft: val(4),
        order: val(5),
    }
}

/// 依住民身分角色、種子及當前處境（壓力、鎂幣）計算多元性格維度權重
pub fn compute_plural_personality_weights(
    role_hint: &str,
    seed: u64,
    stress: u8,
    magnesium: i32,
) -> PluralPersonalityWeights {
    compute_plural_personality_weights_with_experience(
        role_hint,
        "",
        seed,
        stress,
        magnesium,
        1,
        &[],
        None,
    )
}

/// 依先天心性、處境觸發相與歷練成相，產出給大語言模型或身份視圖之自然純粹繁中性格相描述
pub fn describe_npc_myriad_personality_phase(
    entity_id: &str,
    _role_hint: &str,
    seed: u64,
) -> String {
    let innate = innate_personality_simplex(seed);
    let mut facet_order = [0, 1, 2, 3, 4, 5];
    facet_order.sort_by(|&a, &b| innate[b].cmp(&innate[a]));
    let top1 = facet_order[0];
    let top2 = facet_order[1];

    let innate_str = match (top1, top2) {
        (0, 1) => "先天心性自持傲骨、外冷內熱，既有不肯折腰的風骨，心底亦深藏顧念身邊人的柔情",
        (0, 2) => "天生一派孤高傲骨且極重同道情義，行事寧折不彎、恥居人後",
        (0, 3) => "骨子裡傲骨凌厲又深諳市井利害，不屑平庸苟且，遇事講求手腕與成色",
        (0, 4) => "生就一副孤傲自許的脾性，醉心琢磨技藝器物，不喜隨波逐流",
        (0, 5) => "為人自持自重且嚴於律己，傲骨隱於端方規矩之中，言出必踐",

        (1, 0) => "生性慈和體恤、念舊顧後，看似溫厚卻內藏錚錚傲骨，護持親友時分毫不讓",
        (1, 2) => "天性古道熱腸、待人赤誠，最見不得身邊親友同袍受屈受苦",
        (1, 3) => "心裡記掛家小生計溫飽，為求老小安穩既懂操持生計亦能精打細算",
        (1, 4) => "秉性溫良純厚，善於體察細微，操持手藝活計總帶著照料後進的心思",
        (1, 5) => "為人溫和敦厚而循規蹈矩，處處以安頓家宅與照顧晚輩為第一要務",

        (2, 0) => "生來一身磊落俠氣與錚錚傲骨，看重然諾名聲重於身外之物",
        (2, 1) => "天性豪邁仗義、重情重義，同道有難必定傾囊相挺、鼎力周全",
        (2, 3) => "為人處事重信講義，卻也通達商道利害，深知無本難以行義的道理",
        (2, 4) => "性情剛正重諾，敬佩真才實學與精湛造詣，遇事極講道義與原則",
        (2, 5) => "極重然諾規矩，行事光明磊落，一言既出駟馬難追",

        (3, 0) => "心思靈動務實且自持自重，善於審時度勢把握進退，不甘居於人下",
        (3, 1) => "看似算盤精明、分文必較，實則所求皆是為家小後進求得一份安穩依靠",
        (3, 2) => "在商言商講求互利，骨子裡亦知守信踐諾才能長久立足",
        (3, 4) => "眼光敏銳務實，善於將精妙手藝轉化為實在生計進項，精益求精",
        (3, 5) => "凡事精打細算、帳目清晰，極度講究契據契約與條理分寸",

        (4, 0) => "天生一顆匠心、心無旁騖，骨子裡自負手藝成色，不願粗製濫造折辱手藝",
        (4, 1) => "沉潛於器物工序之美，待晚輩徒弟耐心溫厚，樂於傳承手藝心得",
        (4, 2) => "專注於工法規矩與百工精妙，行事方正重諾，以信義與實才立身",
        (4, 3) => "精研手頭造物活計，既追求工藝極致，也通達市集行情與生計價值",
        (4, 5) => "行事嚴謹細緻、分毫不差，在案頭工序間自有一套不可移易的方圓規矩",

        (5, 0) => "秉性方正端嚴、自持沉穩，恪守綱紀分寸，內心有不可逾越的尊嚴底線",
        (5, 1) => "作風循規蹈矩且兼顧人情溫暖，處事公正不偏，顧全大局與身邊眾人",
        (5, 2) => "恪守本分職責與信約法度，以誠信規矩為立身之本",
        (5, 3) => "崇尚條理秩序與契約規範，一分一毫皆依章程交割、童叟無欺",
        (5, 4) => "為人一絲不苟、嚴謹有序，操持營生如精密卡榫般條理分明",
        _ => "心性純樸沉穩，處事端方得體",
    };

    // 處境觸發相
    let voc_off = situational_if_then_offset(seed, PsychSituationSlot::VocationalDuty);
    let mut top_voc = 0;
    for i in 1..6 {
        if voc_off[i] > voc_off[top_voc] {
            top_voc = i;
        }
    }
    let voc_part = match top_voc {
        0 => "立足案頭當值自有一股不容輕慢的傲氣",
        1 => "操持差事時格外顧念同行學徒的辛勞",
        2 => "當差辦事時極重信義託付與同袍聲氣",
        3 => "操持生計時精打細算、寸步不讓",
        4 => "在崗位案頭專注於手上分寸、精益求精",
        _ => "平日當值處事講求條理法度、按部就班",
    };

    let rel_off = situational_if_then_offset(seed, PsychSituationSlot::RelationalEncounter);
    let mut top_rel = 0;
    for i in 1..6 {
        if rel_off[i] > rel_off[top_rel] {
            top_rel = i;
        }
    }
    let rel_part = match top_rel {
        0 => "遇上街坊往來依然端著自尊風骨",
        1 => "與街坊照面時口吻溫和、善解人意",
        2 => "一旦遇上同道交涉或託付則急公好義、重諾守信",
        3 => "與人打交道時算盤分明、利害清晰",
        4 => "遇同行交談則熱衷切磋手藝與門道",
        _ => "在人際往來中進退知分寸、守禮講矩",
    };

    let scarcity_off = situational_if_then_offset(seed, PsychSituationSlot::ScarcitySurvival);
    let mut top_scarcity = 0;
    for i in 1..6 {
        if scarcity_off[i] > scarcity_off[top_scarcity] {
            top_scarcity = i;
        }
    }
    let scarcity_part = match top_scarcity {
        0 => "身處困頓匱乏時更顯一身傲骨硬氣、絕不搖尾乞憐",
        1 => "即便身處難處也先緊著身邊晚輩的溫飽",
        2 => "落入窘迫之際仍信守承諾、不欠人情",
        3 => "遭遇匱乏拮据時則全力尋求變通變現、務實求生",
        4 => "生計吃緊時靠著自身手藝咬牙硬撐",
        _ => "遇上日子難過時愈發節約度日、規矩不亂",
    };

    let mut exp_parts = Vec::new();
    if let Ok(guard) = LIVE_NPC_CAUSAL_STATES.read() {
        if let Some(st) = guard.get(entity_id) {
            if st.same_task_repetitions >= 15 {
                exp_parts.push(format!("經年累月在崗位操持磨礪（累計歷練達 {} 回），手藝與心性愈發沉穩老練", st.same_task_repetitions));
            }
            let has_positive = st.hooks.iter().any(|h| {
                ["恩", "守諾", "解囊", "馳援", "完修"].iter().any(|&k| h.summary.contains(k) || h.kind_label.contains(k))
            });
            let has_negative = st.hooks.iter().any(|h| {
                ["遭拒", "追討", "結怨", "嫌隙", "展期"].iter().any(|&k| h.summary.contains(k) || h.kind_label.contains(k))
            });
            let has_subcontract = st.hooks.iter().any(|h| {
                ["託付", "代辦", "立契"].iter().any(|&k| h.summary.contains(k) || h.kind_label.contains(k))
            });
            if has_positive {
                exp_parts.push("近來承蒙鄉鄰相助與守諾結清，待人更添三分信賴與從容".to_string());
            }
            if has_negative {
                exp_parts.push("近來歷經生計周轉與債信波折，對外行事更添幾分審慎防備".to_string());
            }
            if has_subcontract {
                exp_parts.push("近來承接街坊差事託付，行事交割格外講求分寸".to_string());
            }
        }
    }

    let mut result = format!("其{}。平日{}；{}；{}。", innate_str, voc_part, rel_part, scarcity_part);
    if !exp_parts.is_empty() {
        result.push_str(&format!(" 歷練成相：{}。", exp_parts.join("；")));
    }
    result
}

/// 跨系統因果漣漪事實類型（蝴蝶效應傳播主軸）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RippleFactKind {
    PredatorSpillover,   // 猛獸下山擾道
    WeatherRouteHazard,  // 風雨霜寒阻道
    ResourceFallow,      // 山場休養輪作
    StrandedCargo,       // 行商受阻貨滯驛站
    MarketScarcity,      // 聚落物資奇缺看漲
    ApexSlainOrRescued,  // 除獸解圍／貨單搶通
    PlayerDeed,          // 玩家義舉／借還／大宗買賣／交鋒
}

/// 跨系統因果漣漪事實物件（在全島住民間如實傳導與衰減）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldRippleFact {
    pub fact_id: String,
    pub kind: RippleFactKind,
    pub subject_zone_or_settlement: String,
    pub item_or_actor: String,
    pub order_id: Option<String>,
    pub severity: f32,
    pub origin_tick: i64,
    pub hop_count: u8,
    pub max_hops: u8,
    pub prose_summary: String,
}

/// 滯留物資貨單（商路受阻與公庫懸賞託管）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrandedCargoOrder {
    pub order_id: String,
    pub carrier_id: String,
    pub carrier_name: String,
    pub item_id: String,
    pub item_name: String,
    pub qty: i32,
    pub origin_settlement: String,
    pub dest_settlement: String,
    pub waystation_name: String,
    pub waystation_coord: (i32, i32),
    pub escrow_bounty_mg: i32,
    pub hazard_prose: String,
    pub claimed_by: Option<String>,
    pub created_at_unix: i64,
    pub expires_at_unix: i64,
}

/// 全島即時在線滯留貨單與公庫託管池
pub static LIVE_STRANDED_CARGO_ORDERS: LazyLock<RwLock<HashMap<String, StrandedCargoOrder>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 跨人物人情／恩怨／契約鉤子（Relational Hook）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationalHook {
    pub kind_label: String,
    pub target_name: String,
    pub summary: String,
    pub intensity: u8,
    #[serde(default)]
    pub target_id: String,
    #[serde(default)]
    pub updated_at_unix: i64,
}

/// 單一住民的因果大腦與體感狀態（$O(1)$ 常駐記憶體）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcCausalState {
    pub cornerstone: Cornerstone,
    pub stress: u8,
    pub stress_break_active: bool,
    #[serde(default)]
    pub hunger: u8,
    #[serde(default)]
    pub thirst: u8,
    #[serde(default)]
    pub fatigue: u8,
    pub plan_steps: [String; 3],
    pub plan_cursor: u8,
    pub belief_scarcity_item: String,
    pub last_surprise_delta: Option<String>,
    pub hooks: Vec<RelationalHook>,
    /// 不可逆之生命直線時間刻度（單調遞增，永不歸零重整）
    pub life_step_tick: u64,
    /// 同業日課與工序執行累計次數（即便每天做同樣的事，時間刻度與經驗仍持續累積）
    pub same_task_repetitions: u32,
    /// 昨夜延續至今晨之未竟狀態（每個人每天起床不是歸零重整，生命是一條直線）
    pub carryover_from_yesterday: String,
    /// 當前觸發之「決策意象」代碼（eat / drink / sleep_or_rest / heal_or_tend / mend_or_equip / work_or_trade / interact_or_unwind）
    #[serde(default)]
    pub decision_image_code: String,
    /// 當前條件（有錢沒錢、有多少錢、現況）下符合資格之預擬行為動作選項數量（體現「有餘裕的選擇多，匱乏的選擇少」）
    #[serde(default)]
    pub eligible_option_count: u8,
    /// 從符合條件之預擬選項中擇定的「行為動作」代碼
    #[serde(default)]
    pub chosen_action_code: String,
    /// 當前驅動住民之「需求」（由餓渴累傷、器損、荷包鎂幣與時刻派生）
    pub active_need: String,
    /// 「決策意象」（事先擬好選項，等待條件符合時，依有錢沒錢／有多少錢做出選擇）
    pub image_decision: String,
    /// 「行為動作」（決策意象在當前場所、器況與時間刻度下落實的具體行為動作）
    pub situated_action: String,
    /// 當前住民感知之物資供需與市價行情摘要
    #[serde(default)]
    pub belief_summary: String,
    /// 當前住民心頭縈繞之跨系統因果漣漪資訊（按突顯度遞減，至多 4 條）
    #[serde(default)]
    pub active_ripples: Vec<WorldRippleFact>,
    /// 住民自身對各漣漪 fact_id 之過濾與免疫時效清單（DAG 無環與 SIR 傳播免疫，至多 24 條）
    #[serde(default)]
    pub immune_fact_ids: Vec<(String, i64)>,
}

/// 七大日常「決策意象」（餓了就吃、渴了就喝、累了就睡、傷了就醫、器鈍就修、醒著幹活、事畢往來）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecisionImageKind {
    /// 餓了 ➔ 吃
    Eat,
    /// 渴了 ➔ 喝
    Drink,
    /// 累了 ➔ 睡／歇
    SleepOrRest,
    /// 傷了／病了 ➔ 醫／養
    HealOrTend,
    /// 器物鈍了／損了 ➔ 修／換
    MendOrEquip,
    /// 醒著要過日子 ➔ 幹活／交易
    WorkOrTrade,
    /// 手頭事畢／身邊有人 ➔ 往來／消遣
    InteractOrUnwind,
}

impl DecisionImageKind {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Eat => "eat",
            Self::Drink => "drink",
            Self::SleepOrRest => "sleep_or_rest",
            Self::HealOrTend => "heal_or_tend",
            Self::MendOrEquip => "mend_or_equip",
            Self::WorkOrTrade => "work_or_trade",
            Self::InteractOrUnwind => "interact_or_unwind",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Eat => "腹中飢餓，想尋口熱食果腹",
            Self::Drink => "口乾舌燥，想討碗茶水潤喉",
            Self::SleepOrRest => "筋骨困乏，想尋處歇腳養神",
            Self::HealOrTend => "身上帶傷不適，急需敷藥調理",
            Self::MendOrEquip => "隨身器物微鈍，得先修整稱手",
            Self::WorkOrTrade => "趁著精神尚可，忙活手頭生計與買賣",
            Self::InteractOrUnwind => "手頭差事告一段落，想與街坊同袍敘話散心",
        }
    }
}

/// 餘裕度分級（有錢沒錢？有錢，有多少錢？—— 有餘裕的選擇多，匱乏的選擇少）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarginBand {
    /// 匱乏（沒錢 0..=4 鎂）：僅餘極少數逼不得已的求生選項
    Scarcity,
    /// 溫飽（有小錢 5..=24 鎂）：算計荷包的務實日常選項
    Subsistence,
    /// 有餘裕（錢夠 >=25 鎂）：選擇大幅展開（自奉、請客、顧家、挑剔享受、提攜、精進）
    Abundance,
}

impl MarginBand {
    pub const fn from_magnesium(mg: i32) -> Self {
        if mg <= 4 {
            Self::Scarcity
        } else if mg <= 24 {
            Self::Subsistence
        } else {
            Self::Abundance
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Scarcity => "手頭拮据",
            Self::Subsistence => "小有進項、精打細算",
            Self::Abundance => "手頭寬裕",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VenueRequirement {
    CanteenOrStall,
    HomeOrKitchen,
    WildOrOutskirts,
    TeahouseOrTavern,
    WellOrStream,
    Bathhouse,
    InnOrHome,
    Clinic,
    ForgeOrWorkshop,
    DockOrStation,
    Market,
    OwnWorkplace,
    BridgeOrPlaza,
}

/// 事先擬好的「行為動作」選項（等待條件符合時，做出選擇）
#[derive(Debug, Clone, Copy)]
pub struct ActionOptionDef {
    pub code: &'static str,
    pub image: DecisionImageKind,
    pub margin_band: MarginBand,
    pub min_mg: i32,
    pub max_mg: Option<i32>,
    pub cost_mg: i32,
    pub earn_mg: i32,
    pub venue_req: VenueRequirement,
    pub preferred_cornerstone: Option<Cornerstone>,
    pub condition_gate: &'static str,
    pub action_title: &'static str,
    pub action_behavior: &'static str,
}

/// 預先擬好的完整「決策意象 × 條件閘門 × 行為動作」選項表：
/// 每個決策意象在「沒錢（0..=4 鎂）」時僅 2 項窄選項符合；
/// 在「有小錢（5..=24 鎂）」時有 3 項務實選項符合；
/// 在「有餘裕（>=25 鎂）」時展開為 5～6 項豐富選項（涵蓋原本被粗糙歸類為社交、尊重、自我實現的行為）。
pub fn find_predefined_action_by_title(title: &str) -> Option<&'static ActionOptionDef> {
    let t = title.trim();
    PREDEFINED_ACTION_CATALOG.iter().find(|opt| opt.action_title == t)
}

