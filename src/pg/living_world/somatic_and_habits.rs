#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BreathHabitKind {
    /// 鍛爐沉息：常近爐火與重錘鍛打養成，吐納沉厚耐燥、蓄力剛猛
    ForgeHeavy,
    /// 山林長息：常年跋涉山林溪谷與狩獵採集養成，吐納綿長輕細、回氣輕快
    ForestLong,
    /// 海潮綿息：常處海濱潮汐、河堰水輪或舟楫勞作養成，氣息綿延不絕、耐受久戰
    TideContinuous,
    /// 靜室清息：常於靜室抄錄研讀、品茗調息或醫館搭脈養成，神思澄明、經脈繞路順暢
    ChamberClear,
    /// 百戰烈息：常歷生死交鋒與兵刃硬撼養成，氣血激盪如火、架勢重整迅疾
    BattleFierce,
}

impl BreathHabitKind {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ForgeHeavy => "forge_heavy",
            Self::ForestLong => "forest_long",
            Self::TideContinuous => "tide_continuous",
            Self::ChamberClear => "chamber_clear",
            Self::BattleFierce => "battle_fierce",
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code {
            "forge_heavy" => Self::ForgeHeavy,
            "tide_continuous" => Self::TideContinuous,
            "chamber_clear" => Self::ChamberClear,
            "battle_fierce" => Self::BattleFierce,
            _ => Self::ForestLong,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::ForgeHeavy => "鍛爐沉息",
            Self::ForestLong => "山林長息",
            Self::TideContinuous => "海潮綿息",
            Self::ChamberClear => "靜室清息",
            Self::BattleFierce => "百戰烈息",
        }
    }

    pub fn somatic_description(&self) -> &'static str {
        match self {
            Self::ForgeHeavy => "常年近火揮錘、運使沉重器物所養成的吐納習慣。吸氣短促入腹、呼氣沉厚如風箱鼓盪，耐受燥熱與重負荷，重擊發勁時氣息格外凝實。",
            Self::ForestLong => "常年穿行山林草甸、涉水追蹤所養成的吐納習慣。氣息細長綿遠、起落無聲，跋涉與閃轉後回氣輕快，耳目對風吹草動尤為敏銳。",
            Self::TideContinuous => "常年順應潮汐水勢、於河海舟楫與水磨間勞作所養成的吐納習慣。氣機如潮汐疊浪、後勁綿延不絕，久戰之下氣脈不易潰散。",
            Self::ChamberClear => "常年靜心著譜研讀、品茗調息與體察氣脈所養成的吐納習慣。吐納勻淨澄明，體內氣機遇阻時能自然尋隙繞行，行氣運勁更為省力。",
            Self::BattleFierce => "在連番貼身交鋒與兵刃硬撼中磨礪出的吐納習慣。臨敵時氣血如烘爐驟燃，受創或架勢動搖時能憑一口烈息迅速穩住下盤。",
        }
    }

    pub fn combat_rhythm_phrase(&self) -> &'static str {
        match self {
            Self::ForgeHeavy => "吐納沉如風箱鼓火，勁力自腰胯沉穩催發",
            Self::ForestLong => "呼吸綿長無聲，換氣間身形輕靈如林間游鹿",
            Self::TideContinuous => "內息如潮汐疊浪，一氣未盡、後勁已續",
            Self::ChamberClear => "神息澄明似水，氣機順支脈自然滑轉而不滯",
            Self::BattleFierce => "胸中一口烈息激盪，迎著鋒芒寸步不讓",
        }
    }
    pub fn effect_summary(&self) -> &'static str {
        match self {
            Self::ForgeHeavy => "實戰剛猛重擊，催發沉凝力道貫通關節以破剛硬守備；鍛打冶煉時能細察火候變化，器物精工成形更為純粹。",
            Self::ForestLong => "實戰輕靈游鬥，身法如林間游鹿靈動難測；山海崎嶇跋涉時能勻長換氣舒緩筋骨，百工淬煉亦顯得心應手。",
            Self::TideContinuous => "實戰綿延續航，內息如疊浪後勁綿綿；濕地水域跋涉時換氣自然順暢，庖廚熬煉與百工調配格外醇熟。",
            Self::ChamberClear => "實戰澄明貫脈，心神守一方寸自能透徹尋隙；細木卯合、煎藥研粉之際神意專注，精工品相極易生出巧思。",
            Self::BattleFierce => "實戰烘爐搏殺，胸中烈息激盪敢於近身硬撼重圍；兵刃搏殺與實戰磨合迅速入化，鍛淬器物自具凜冽鋒芒。",
        }
    }


    pub fn combat_modifiers(&self) -> (f64, f64, f64, &'static str) {
        match self {
            Self::ForgeHeavy => (
                1.10,
                1.03,
                3.0,
                "【內息催動・鍛爐沉息】你吐納沉如風箱鼓火，腰胯沉穩催發重勁，出招力道更添三分凝實。",
            ),
            Self::ForestLong => (
                1.04,
                1.09,
                2.0,
                "【內息催動・山林長息】你呼吸綿長無聲，換氣間步法輕靈，閃轉卸力後回氣極快。",
            ),
            Self::TideContinuous => (
                1.05,
                1.10,
                2.0,
                "【內息催動・海潮綿息】你內息如潮汐疊浪綿延不絕，纏鬥間氣脈深長、後勁不衰。",
            ),
            Self::ChamberClear => (
                1.06,
                1.04,
                4.5,
                "【內息催動・靜室清息】你神息澄明似水，體內氣機順支脈自然滑轉，招式落點分毫不差。",
            ),
            Self::BattleFierce => (
                1.12,
                1.02,
                3.5,
                "【內息催動・百戰烈息】你胸中一口烈息如烘爐驟燃，臨敵硬撼時氣勢暴漲、寸步不讓。",
            ),
        }
    }

    pub fn craft_mutation_bonus(&self) -> f64 {
        match self {
            Self::ForgeHeavy => 0.06,
            Self::ChamberClear => 0.06,
            Self::TideContinuous => 0.05,
            Self::ForestLong => 0.04,
            Self::BattleFierce => 0.03,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BreathHabitRecord {
    pub entity_id: String,
    pub dominant: BreathHabitKind,
    pub forge_score: i32,
    pub forest_score: i32,
    pub tide_score: i32,
    pub chamber_score: i32,
    pub battle_score: i32,
    pub updated_at: i64,
}

static BREATH_HABIT_CACHE: LazyLock<RwLock<HashMap<String, BreathHabitRecord>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn get_or_derive_breath_habit(
    entity_id: &str,
    vocation_hint: &str,
    soul_seed: i64,
) -> BreathHabitRecord {
    ensure_aux_pg_tables();
    if let Some(rec) = BREATH_HABIT_CACHE.read().unwrap().get(entity_id).cloned() {
        return rec;
    }
    let effective_seed = if soul_seed != 0 {
        soul_seed.unsigned_abs()
    } else {
        entity_id.bytes().fold(1u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
    };

    let mut forge_score = 4;
    let mut forest_score = 4;
    let mut tide_score = 4;
    let mut chamber_score = 4;
    let mut battle_score = 4;

    // Only apply random seed drift when vocation_hint does not specify a clear trade

    let is_guard_or_martial = vocation_hint.contains("衛")
        || vocation_hint.contains("武師")
        || vocation_hint.contains("教頭")
        || vocation_hint.contains("教練")
        || vocation_hint.contains("總教")
        || vocation_hint.contains("巡防")
        || vocation_hint.contains("哨")
        || vocation_hint.contains("百戰")
        || vocation_hint.contains("apex");

    let is_hunter_or_forager = vocation_hint.contains("獵")
        || vocation_hint.contains("採")
        || vocation_hint.contains("樵")
        || vocation_hint.contains("入山百工")
        || vocation_hint.contains("林道")
        || vocation_hint.contains("巡山");

    let is_dock_or_water = vocation_hint.contains("漁")
        || vocation_hint.contains("海")
        || vocation_hint.contains("港")
        || vocation_hint.contains("渡")
        || vocation_hint.contains("船")
        || vocation_hint.contains("舵")
        || vocation_hint.contains("網")
        || vocation_hint.contains("水")
        || vocation_hint.contains("潮")
        || vocation_hint.contains("泉")
        || vocation_hint.contains("堰")
        || vocation_hint.contains("渠");

    let is_artisan_forge = vocation_hint.contains("鍛")
        || vocation_hint.contains("鐵")
        || vocation_hint.contains("礦")
        || vocation_hint.contains("石")
        || vocation_hint.contains("鎬")
        || vocation_hint.contains("金")
        || vocation_hint.contains("木")
        || vocation_hint.contains("匠")
        || vocation_hint.contains("窯")
        || vocation_hint.contains("磚")
        || vocation_hint.contains("瓦")
        || vocation_hint.contains("泥")
        || vocation_hint.contains("刨")
        || vocation_hint.contains("修");

    let is_clerk_scholar_household = vocation_hint.contains("家眷")
        || vocation_hint.contains("內當家")
        || vocation_hint.contains("持家")
        || vocation_hint.contains("長輩")
        || vocation_hint.contains("晚輩")
        || vocation_hint.contains("掌事")
        || vocation_hint.contains("執事")
        || vocation_hint.contains("學徒")
        || vocation_hint.contains("東家")
        || vocation_hint.contains("帳房")
        || vocation_hint.contains("商行")
        || vocation_hint.contains("市集")
        || vocation_hint.contains("公所")
        || vocation_hint.contains("書")
        || vocation_hint.contains("學")
        || vocation_hint.contains("講")
        || vocation_hint.contains("印")
        || vocation_hint.contains("記")
        || vocation_hint.contains("醫")
        || vocation_hint.contains("藥")
        || vocation_hint.contains("診")
        || vocation_hint.contains("算")
        || vocation_hint.contains("食堂")
        || vocation_hint.contains("廚")
        || vocation_hint.contains("茶");

    if is_guard_or_martial {
        battle_score += 6;
    } else if is_hunter_or_forager {
        forest_score += 6;
    } else if is_artisan_forge {
        forge_score += 6;
    } else if is_dock_or_water {
        tide_score += 6;
    } else if is_clerk_scholar_household {
        chamber_score += 6;
    } else if soul_seed != 0 {
        match effective_seed % 5 {
            0 => forge_score += 3,
            1 => forest_score += 3,
            2 => tide_score += 3,
            3 => chamber_score += 3,
            _ => battle_score += 3,
        }
        match (effective_seed / 5) % 5 {
            0 => forge_score += 2,
            1 => forest_score += 2,
            2 => tide_score += 2,
            3 => chamber_score += 2,
            _ => battle_score += 2,
        }
    }

    let tiebreaker = |kind: BreathHabitKind| -> u64 {
        let idx = match kind {
            BreathHabitKind::ForgeHeavy => 0,
            BreathHabitKind::ForestLong => 1,
            BreathHabitKind::TideContinuous => 2,
            BreathHabitKind::ChamberClear => 3,
            BreathHabitKind::BattleFierce => 4,
        };
        effective_seed
            .wrapping_mul(0x9E3779B97F4A7C15)
            .wrapping_add((idx as u64).wrapping_mul(0xBF58476D1CE4E5B9))
    };

    let candidates = [
        (BreathHabitKind::ForgeHeavy, forge_score),
        (BreathHabitKind::ForestLong, forest_score),
        (BreathHabitKind::TideContinuous, tide_score),
        (BreathHabitKind::ChamberClear, chamber_score),
        (BreathHabitKind::BattleFierce, battle_score),
    ];
    let dominant = candidates
        .into_iter()
        .max_by(|(k_a, s_a), (k_b, s_b)| match s_a.cmp(s_b) {
            std::cmp::Ordering::Equal => tiebreaker(*k_a).cmp(&tiebreaker(*k_b)),
            ord => ord,
        })
        .map(|(k, _)| k)
        .unwrap_or(BreathHabitKind::ForestLong);

    let rec = BreathHabitRecord {
        entity_id: entity_id.to_string(),
        dominant,
        forge_score,
        forest_score,
        tide_score,
        chamber_score,
        battle_score,
        updated_at: 0,
    };
    BREATH_HABIT_CACHE
        .write()
        .unwrap()
        .insert(entity_id.to_string(), rec.clone());
    rec
}

pub fn record_breath_activity(
    entity_id: &str,
    habit: BreathHabitKind,
    weight: i32,
    now_unix: i64,
) -> BreathHabitRecord {
    ensure_aux_pg_tables();
    let delta = weight.max(1);
    let updated = {
        let mut guard = BREATH_HABIT_CACHE.write().unwrap();
        let entry = guard.entry(entity_id.to_string()).or_insert_with(|| BreathHabitRecord {
            entity_id: entity_id.to_string(),
            dominant: habit,
            forge_score: 4,
            forest_score: 4,
            tide_score: 4,
            chamber_score: 4,
            battle_score: 4,
            updated_at: now_unix,
        });
        match habit {
            BreathHabitKind::ForgeHeavy => entry.forge_score += delta,
            BreathHabitKind::ForestLong => entry.forest_score += delta,
            BreathHabitKind::TideContinuous => entry.tide_score += delta,
            BreathHabitKind::ChamberClear => entry.chamber_score += delta,
            BreathHabitKind::BattleFierce => entry.battle_score += delta,
        }
        entry.updated_at = now_unix;
        // 慣性法則：新習慣分數必須超越現有主導底色至少 4 分才會自然轉移，防止一秒切換
        let current_score = match entry.dominant {
            BreathHabitKind::ForgeHeavy => entry.forge_score,
            BreathHabitKind::ForestLong => entry.forest_score,
            BreathHabitKind::TideContinuous => entry.tide_score,
            BreathHabitKind::ChamberClear => entry.chamber_score,
            BreathHabitKind::BattleFierce => entry.battle_score,
        };
        let candidates = [
            (BreathHabitKind::ForgeHeavy, entry.forge_score),
            (BreathHabitKind::ForestLong, entry.forest_score),
            (BreathHabitKind::TideContinuous, entry.tide_score),
            (BreathHabitKind::ChamberClear, entry.chamber_score),
            (BreathHabitKind::BattleFierce, entry.battle_score),
        ];
        if let Some((best_kind, best_score)) = candidates.into_iter().max_by_key(|(_, s)| *s)
            && best_kind != entry.dominant
            && best_score >= current_score + 4
        {
            entry.dominant = best_kind;
        }
        entry.clone()
    };
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let _ = client.execute(
            "INSERT INTO entity_breath_habits
               (entity_id, dominant_code, forge_score, forest_score, tide_score, chamber_score, battle_score, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (entity_id) DO UPDATE SET
               dominant_code = EXCLUDED.dominant_code,
               forge_score = EXCLUDED.forge_score,
               forest_score = EXCLUDED.forest_score,
               tide_score = EXCLUDED.tide_score,
               chamber_score = EXCLUDED.chamber_score,
               battle_score = EXCLUDED.battle_score,
               updated_at = EXCLUDED.updated_at",
            &[
                &updated.entity_id,
                &updated.dominant.code(),
                &updated.forge_score,
                &updated.forest_score,
                &updated.tide_score,
                &updated.chamber_score,
                &updated.battle_score,
                &updated.updated_at,
            ],
        );
    }
    updated
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VesselFamiliarity {
    /// 初握生澀
    Unfamiliar,
    /// 漸趨順手
    GettingSmooth,
    /// 沉穩趁手
    WellBalanced,
    /// 如臂使指
    LikeOwnArm,
}

impl VesselFamiliarity {
    pub fn from_use_count(use_count: i32) -> Self {
        match use_count {
            0..=2 => Self::Unfamiliar,
            3..=8 => Self::GettingSmooth,
            9..=19 => Self::WellBalanced,
            _ => Self::LikeOwnArm,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Unfamiliar => "初握生澀",
            Self::GettingSmooth => "漸趨順手",
            Self::WellBalanced => "沉穩趁手",
            Self::LikeOwnArm => "如臂使指",
        }
    }

    pub fn feel_prose(&self) -> &'static str {
        match self {
            Self::Unfamiliar => "器物重心尚待摸索，運使時須多留一分力道收束",
            Self::GettingSmooth => "手掌已漸漸習慣握柄紋理，起落收發初見流暢",
            Self::WellBalanced => "分寸輕重皆在掌握，隨手一引便能順勢入勁",
            Self::LikeOwnArm => "人器氣機渾然相通，心念動處鋒芒已至，恍若肢體延伸",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VesselCondition {
    /// 鋒芒畢露（完好銳利）
    Sharp,
    /// 刃口微鈍（可於街邊磨刀石推磨開鋒）
    SlightlyDull,
    /// 捲刃待修（須於鍛造工坊借爐火鐵砧回火修整）
    RolledEdge,
}

impl VesselCondition {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Sharp => "sharp",
            Self::SlightlyDull => "slightly_dull",
            Self::RolledEdge => "rolled_edge",
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code {
            "slightly_dull" => Self::SlightlyDull,
            "rolled_edge" => Self::RolledEdge,
            _ => Self::Sharp,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Sharp => "鋒利",
            Self::SlightlyDull => "刃口微鈍",
            Self::RolledEdge => "捲刃待修",
        }
    }

    pub fn label_for_item(&self, item_id: &str, item_name: &str) -> &'static str {
        crate::game::object_ecology::ItemPhysicalClass::classify(item_id, item_name).condition_label(*self)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VesselStateRecord {
    pub entity_id: String,
    pub item_id: String,
    pub use_count: i32,
    pub wear_points: i32,
    pub condition: VesselCondition,
    pub updated_at: i64,
}

impl VesselStateRecord {
    pub fn familiarity(&self) -> VesselFamiliarity {
        VesselFamiliarity::from_use_count(self.use_count)
    }

    pub fn status_badge(&self) -> String {
        let item_name = crate::db::get_item_name(&self.item_id);
        let cond_lbl = self.condition.label_for_item(&self.item_id, &item_name);
        format!("{}・{}", self.familiarity().label(), cond_lbl)
    }
}

static VESSEL_STATE_CACHE: LazyLock<RwLock<HashMap<(String, String), VesselStateRecord>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn get_vessel_state(entity_id: &str, item_id: &str) -> VesselStateRecord {
    ensure_aux_pg_tables();
    if let Some(rec) = VESSEL_STATE_CACHE
        .read()
        .unwrap()
        .get(&(entity_id.to_string(), item_id.to_string()))
        .cloned()
    {
        return rec;
    }
    VesselStateRecord {
        entity_id: entity_id.to_string(),
        item_id: item_id.to_string(),
        use_count: 0,
        wear_points: 0,
        condition: VesselCondition::Sharp,
        updated_at: 0,
    }
}

pub fn get_entity_max_vessel_use_count(entity_id: &str) -> u32 {
    ensure_aux_pg_tables();
    if let Ok(guard) = VESSEL_STATE_CACHE.read() {
        let mut max_count = 0i32;
        for ((eid, _), rec) in guard.iter() {
            if eid == entity_id && rec.use_count > max_count {
                max_count = rec.use_count;
            }
        }
        if max_count > 0 {
            return max_count as u32;
        }
    }
    0
}

fn persist_vessel_state(rec: &VesselStateRecord) {
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let _ = client.execute(
            "INSERT INTO entity_vessel_states
               (entity_id, item_id, use_count, wear_points, condition_code, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (entity_id, item_id) DO UPDATE SET
               use_count = EXCLUDED.use_count,
               wear_points = EXCLUDED.wear_points,
               condition_code = EXCLUDED.condition_code,
               updated_at = EXCLUDED.updated_at",
            &[
                &rec.entity_id,
                &rec.item_id,
                &rec.use_count,
                &rec.wear_points,
                &rec.condition.code(),
                &rec.updated_at,
            ],
        );
    }
}

pub fn record_vessel_use(
    entity_id: &str,
    item_id: &str,
    wear_delta: i32,
    familiarity_delta: i32,
    now_unix: i64,
) -> VesselStateRecord {
    ensure_aux_pg_tables();
    let updated = {
        let mut guard = VESSEL_STATE_CACHE.write().unwrap();
        let entry = guard
            .entry((entity_id.to_string(), item_id.to_string()))
            .or_insert_with(|| VesselStateRecord {
                entity_id: entity_id.to_string(),
                item_id: item_id.to_string(),
                use_count: 0,
                wear_points: 0,
                condition: VesselCondition::Sharp,
                updated_at: now_unix,
            });
        entry.use_count = (entry.use_count + familiarity_delta.max(0)).min(9999);
        entry.wear_points = (entry.wear_points + wear_delta.max(0)).min(100);
        entry.condition = if entry.wear_points >= 8 {
            VesselCondition::RolledEdge
        } else if entry.wear_points >= 3 {
            VesselCondition::SlightlyDull
        } else {
            VesselCondition::Sharp
        };
        entry.updated_at = now_unix;
        entry.clone()
    };
    persist_vessel_state(&updated);
    updated
}

pub fn sharpen_vessel_at_grindstone(
    entity_id: &str,
    item_id: &str,
    now_unix: i64,
) -> Result<VesselStateRecord, &'static str> {
    ensure_aux_pg_tables();
    let current = get_vessel_state(entity_id, item_id);
    if current.condition == VesselCondition::RolledEdge {
        return Err("此器刃口已然捲曲崩缺，尋常青石磨刀槽難以推平，須前往鍛造工坊借爐火與鐵砧回火修整。");
    }
    let updated = {
        let mut guard = VESSEL_STATE_CACHE.write().unwrap();
        let entry = guard
            .entry((entity_id.to_string(), item_id.to_string()))
            .or_insert(current.clone());
        if current.wear_points > 0 || current.condition != VesselCondition::Sharp {
            entry.use_count = (entry.use_count + 2).min(9999);
        }
        entry.wear_points = 0;
        entry.condition = VesselCondition::Sharp;
        entry.updated_at = now_unix;
        entry.clone()
    };
    persist_vessel_state(&updated);
    Ok(updated)
}

pub fn temper_vessel_at_forge(
    entity_id: &str,
    item_id: &str,
    now_unix: i64,
) -> VesselStateRecord {
    ensure_aux_pg_tables();
    let current = get_vessel_state(entity_id, item_id);
    let updated = {
        let mut guard = VESSEL_STATE_CACHE.write().unwrap();
        let entry = guard
            .entry((entity_id.to_string(), item_id.to_string()))
            .or_insert(current);
        entry.use_count = (entry.use_count + 3).min(9999);
        entry.wear_points = 0;
        entry.condition = VesselCondition::Sharp;
        entry.updated_at = now_unix;
        entry.clone()
    };
    persist_vessel_state(&updated);
    updated
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LineageImprintRecord {
    pub entity_id: String,
    pub mentor_name: String,
    pub imprint_title: String,
    pub imprint_prose: String,
    pub interaction_count: i32,
    pub manual_gifted: bool,
    pub updated_at: i64,
}

static LINEAGE_IMPRINT_CACHE: LazyLock<RwLock<HashMap<String, Vec<LineageImprintRecord>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn record_mentor_imprint_progress(
    entity_id: &str,
    mentor_name: &str,
    imprint_title: &str,
    imprint_prose: &str,
    now_unix: i64,
) -> (LineageImprintRecord, bool) {
    ensure_aux_pg_tables();
    let (rec, newly_gifted) = {
        let mut guard = LINEAGE_IMPRINT_CACHE.write().unwrap();
        let list = guard.entry(entity_id.to_string()).or_default();
        if let Some(existing) = list.iter_mut().find(|r| r.mentor_name == mentor_name) {
            existing.interaction_count += 1;
            existing.updated_at = now_unix;
            let gift_now = existing.interaction_count >= 2 && !existing.manual_gifted;
            if gift_now {
                existing.manual_gifted = true;
            }
            (existing.clone(), gift_now)
        } else {
            let new_rec = LineageImprintRecord {
                entity_id: entity_id.to_string(),
                mentor_name: mentor_name.to_string(),
                imprint_title: imprint_title.to_string(),
                imprint_prose: imprint_prose.to_string(),
                interaction_count: 1,
                manual_gifted: false,
                updated_at: now_unix,
            };
            list.push(new_rec.clone());
            (new_rec, false)
        }
    };
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let _ = client.execute(
            "INSERT INTO entity_lineage_imprints
               (entity_id, mentor_name, imprint_title, imprint_prose, interaction_count, manual_gifted, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (entity_id, mentor_name) DO UPDATE SET
               imprint_title = EXCLUDED.imprint_title,
               imprint_prose = EXCLUDED.imprint_prose,
               interaction_count = EXCLUDED.interaction_count,
               manual_gifted = EXCLUDED.manual_gifted,
               updated_at = EXCLUDED.updated_at",
            &[
                &rec.entity_id,
                &rec.mentor_name,
                &rec.imprint_title,
                &rec.imprint_prose,
                &rec.interaction_count,
                &rec.manual_gifted,
                &rec.updated_at,
            ],
        );
    }
    (rec, newly_gifted)
}

pub fn get_entity_lineage_imprints(entity_id: &str) -> Vec<LineageImprintRecord> {
    ensure_aux_pg_tables();
    LINEAGE_IMPRINT_CACHE
        .read()
        .unwrap()
        .get(entity_id)
        .cloned()
        .unwrap_or_default()
}

pub fn record_entity_studied_manual(entity_id: &str, manual_title: &str) {
    ensure_aux_pg_tables();
    let already_has = {
        let mut guard = STUDIED_MANUALS_CACHE.write().unwrap();
        let list = guard.entry(entity_id.to_string()).or_default();
        if list.iter().any(|m| m == manual_title) {
            true
        } else {
            list.push(manual_title.to_string());
            false
        }
    };
    if !already_has {
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let _ = client.execute(
                "INSERT INTO entity_studied_manuals (entity_id, manual_title, studied_at)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (entity_id, manual_title) DO NOTHING",
                &[&entity_id, &manual_title, &now_unix],
            );
        }
    }
}

pub fn get_entity_studied_manuals(entity_id: &str) -> Vec<String> {
    ensure_aux_pg_tables();
    STUDIED_MANUALS_CACHE
        .read()
        .unwrap()
        .get(entity_id)
        .cloned()
        .unwrap_or_default()
}

pub fn record_entity_anatomy_familiarity(entity_id: &str, target_category: &str, delta: u32) -> u32 {
    ensure_aux_pg_tables();
    let new_count = {
        let mut guard = ANATOMY_FAMILIARITY_CACHE.write().unwrap();
        let map = guard.entry(entity_id.to_string()).or_default();
        let entry = map.entry(target_category.to_string()).or_insert(0);
        *entry = entry.saturating_add(delta.max(1));
        *entry
    };
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let count_i32 = new_count as i32;
        let _ = client.execute(
            "INSERT INTO entity_anatomy_familiarity (entity_id, target_category, familiarity_count, updated_at)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (entity_id, target_category) DO UPDATE SET
               familiarity_count = EXCLUDED.familiarity_count,
               updated_at = EXCLUDED.updated_at",
            &[&entity_id, &target_category, &count_i32, &now_unix],
        );
    }
    new_count
}

pub fn get_entity_anatomy_familiarity_total(entity_id: &str) -> u32 {
    ensure_aux_pg_tables();
    ANATOMY_FAMILIARITY_CACHE
        .read()
        .unwrap()
        .get(entity_id)
        .map(|map| map.values().copied().sum())
        .unwrap_or(0)
}

pub fn get_entity_anatomy_familiarity_summary(entity_id: &str) -> String {
    ensure_aux_pg_tables();
    let guard = ANATOMY_FAMILIARITY_CACHE.read().unwrap();
    let Some(map) = guard.get(entity_id) else {
        return "尚未累積深厚的山海生靈剖析或百工取材心得，仍以常理摸索。".to_string();
    };
    if map.is_empty() {
        return "尚未累積深厚的山海生靈剖析或百工取材心得，仍以常理摸索。".to_string();
    }
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by(|a, b| b.1.cmp(a.1));
    let items_str = entries
        .iter()
        .take(4)
        .map(|(cat, count)| {
            let depth = if **count >= 20 {
                "入微純熟"
            } else if **count >= 8 {
                "漸諳脈絡"
            } else {
                "初窺肌理"
            };
            format!("{}・{}", cat, depth)
        })
        .collect::<Vec<_>>()
        .join("、");
    format!(
        "常年接觸山海生靈與百工物材（如{}），熟稔筋骨脈絡與紋理走向，落刃順理而不傷器物鋒芒。",
        items_str
    )
}

