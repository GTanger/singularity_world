#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PersistedWordPlateState {
    #[serde(default)]
    caskets: HashMap<String, WordCasket>,
    #[serde(default)]
    loadouts: HashMap<String, TacticalLoadout>,
}

const PLAYER_WORDPLATES_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/runtime/player_wordplates.json"
);

fn player_wordplates_path() -> std::path::PathBuf {
    let p = std::path::Path::new("data/runtime/player_wordplates.json");
    if p.exists() || std::path::Path::new("data/runtime").exists() {
        p.to_path_buf()
    } else {
        std::path::PathBuf::from(PLAYER_WORDPLATES_FILE)
    }
}

fn is_persisted_wordplate_owner(pid: &str) -> bool {
    if pid.starts_with("npc_")
        || pid.starts_with("ambient_")
        || pid.starts_with("shanhai_")
        || pid.starts_with("wt_npc_")
        || pid.starts_with("wt_vendor_")
    {
        false
    } else {
        true
    }
}
fn load_persisted_wordplates() -> PersistedWordPlateState {
    let path = player_wordplates_path();
    if let Ok(data) = std::fs::read_to_string(&path) {
        if let Ok(mut state) = serde_json::from_str::<PersistedWordPlateState>(&data) {
            let registry = WordRegistry::global();
            let mut repaired = false;
            let caskets_before = state.caskets.len();
            state.caskets.retain(|k, _| is_persisted_wordplate_owner(k));
            if state.caskets.len() != caskets_before {
                repaired = true;
            }
            let loadouts_before = state.loadouts.len();
            state.loadouts.retain(|k, _| is_persisted_wordplate_owner(k));
            if state.loadouts.len() != loadouts_before {
                repaired = true;
            }
            for (pid, loadout) in state.loadouts.iter_mut() {
                if loadout.opening.is_none()
                    || loadout.midgame.is_none()
                    || loadout.finisher.is_none()
                    || loadout.defense.is_none()
                {
                    let seed = crate::combat::human_phase::resolve_entity_soul_seed(pid, None);
                    let casket_opt = state.caskets.get(pid);
                    let default_ld = crate::combat::wordplate::build_loadout_from_soul_seed(
                        seed, casket_opt, registry,
                    );
                    if loadout.opening.is_none() {
                        loadout.opening = default_ld.opening;
                        repaired = true;
                    }
                    if loadout.midgame.is_none() {
                        loadout.midgame = default_ld.midgame;
                        repaired = true;
                    }
                    if loadout.finisher.is_none() {
                        loadout.finisher = default_ld.finisher;
                        repaired = true;
                    }
                    if loadout.defense.is_none() {
                        loadout.defense = default_ld.defense;
                        repaired = true;
                    }
                }
            }
            if repaired {
                if let Ok(json) = serde_json::to_string_pretty(&state) {
                    let _ = std::fs::write(&path, json);
                }
            }
            return state;
        }
    }
    PersistedWordPlateState::default()
}

pub(crate) fn save_player_wordplates() {
    let path = player_wordplates_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let state = PersistedWordPlateState {
        caskets: PLAYER_CASKETS
            .read()
            .clone()
            .into_iter()
            .filter(|(k, _)| is_persisted_wordplate_owner(k))
            .collect(),
        loadouts: PLAYER_LOADOUTS
            .read()
            .clone()
            .into_iter()
            .filter(|(k, _)| is_persisted_wordplate_owner(k))
            .collect(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&state) {
        let tmp_path = path.with_extension("tmp");
        if std::fs::write(&tmp_path, json).is_ok() {
            let _ = std::fs::rename(tmp_path, path);
        }
    }
}

#[allow(dead_code)]
pub(crate) fn grant_crystal_or_word(
    pid: &str,
    word_id: &str,
    count: u32,
    quality: WordQuality,
    ratio: u8,
) {
    ensure_player_initialized(pid);
    {
        let mut caskets = PLAYER_CASKETS.write();
        if let Some(casket) = caskets.get_mut(pid) {
            casket.add_extracted(word_id, count, quality, ratio);
        }
    }
    save_player_wordplates();
}

/// 將多個單字字印一次性納入玩家詞匣，供手札參悟即刻就緒
pub fn grant_words_to_player_casket(pid: &str, word_ids: &[&str]) {
    ensure_player_initialized(pid);
    {
        let mut caskets = PLAYER_CASKETS.write();
        if let Some(casket) = caskets.get_mut(pid) {
            for &wid in word_ids {
                casket.add(wid, 1);
            }
        }
    }
    save_player_wordplates();
}

static PLAYER_CASKETS: LazyLock<RwLock<HashMap<String, WordCasket>>> = LazyLock::new(|| {
    let persisted = load_persisted_wordplates();
    RwLock::new(persisted.caskets)
});

static PLAYER_LOADOUTS: LazyLock<RwLock<HashMap<String, TacticalLoadout>>> = LazyLock::new(|| {
    let persisted = load_persisted_wordplates();
    RwLock::new(persisted.loadouts)
});

static STARTER_MANUAL_GRANTED: LazyLock<RwLock<HashSet<String>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));

const DEFAULT_BASE_CHARS: &[&str] = &[
    "吾", "疾", "刺", "斬", "震", "卸", "盾", "步", "凝", "流", "破", "守",
];

fn resolve_word_id(registry: &WordRegistry, ch_or_id: &str) -> Option<String> {
    if registry.get_by_id(ch_or_id).is_some() {
        return Some(ch_or_id.to_string());
    }
    if let Some(ids) = registry.get_by_char(ch_or_id) {
        if let Some(first) = ids.first() {
            return Some(first.clone());
        }
    }
    // 若特定字不存在則選取最接近的基礎詞元
    let fallback = match ch_or_id {
        "疾" => "速",
        "卸" => "化",
        "步" => "避",
        _ => return registry.ordered_elements().next().map(|w| w.id.clone()),
    };
    if let Some(ids) = registry.get_by_char(fallback) {
        if let Some(first) = ids.first() {
            return Some(first.clone());
        }
    }
    registry.ordered_elements().next().map(|w| w.id.clone())
}

fn create_default_casket(registry: &WordRegistry) -> WordCasket {
    let mut casket = WordCasket::new();
    for &ch in DEFAULT_BASE_CHARS {
        if let Some(id) = resolve_word_id(registry, ch) {
            casket.add(id, 5);
        }
    }
    casket
}

fn create_default_loadout(registry: &WordRegistry) -> TacticalLoadout {
    crate::combat::wordplate::build_loadout_from_soul_seed(0, None, registry)
}

pub fn populate_resident_vocational_casket(pid: &str, casket: &mut WordCasket, registry: &WordRegistry) -> bool {
    if crate::server::grid_manager::get_runtime_square_grid().is_some_and(|g| !g.has_peninsula_wilderness()) {
        return false;
    }
    let mut matched_clue: Option<String> = None;
    if pid == "秦水生" {
        matched_clue = Some("水輪石磨匠|老堰頭・水輪石磨工坊|秦水生".to_string());
    } else {
        for slot in crate::store::WENYAO_HOUSEHOLD_SLOTS.iter().chain(crate::store::OUTER_HOUSEHOLD_SLOTS.iter()) {
            let s_name = slot.name();
            if s_name == pid || pid.ends_with(&format!("|{}", s_name)) {
                matched_clue = Some(format!("{}|{}|{}", slot.brief_tag, slot.work_name, s_name));
                break;
            }
        }
        if matched_clue.is_none() {
            // 嘗試從全島住民作息排程與戶籍槽位比對其職銜與姓名
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for b in pid.as_bytes() {
                h ^= u64::from(*b);
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
            let target_settlement = crate::store::resident_macro_settlement(pid, "").unwrap_or("文曜");
            let matching: Vec<&'static crate::store::WenyaoHouseholdSlot> = if target_settlement == "文曜" {
                crate::store::WENYAO_HOUSEHOLD_SLOTS.iter().collect()
            } else {
                crate::store::OUTER_HOUSEHOLD_SLOTS
                    .iter()
                    .filter(|s| s.home_name.starts_with(target_settlement))
                    .collect()
            };
            if !matching.is_empty() {
                let slot = matching[(h >> 8) as usize % matching.len()];
                matched_clue = Some(format!("{}|{}|{}", slot.brief_tag, slot.work_name, pid));
            } else if let Some(dt) = crate::db::get_entity(pid)
                .ok()
                .flatten()
                .map(|e| e.display_title)
                .filter(|t| !t.trim().is_empty())
            {
                matched_clue = Some(dt);
            }
        }
    }
    let clue = match matched_clue {
        Some(c) => c,
        None => return false,
    };

    let mut mutated = false;

    // 1. 核心百工/工匠詞元（每人至少 3 個字印）
    for ch in ["磨", "流", "純", "巧", "堅", "定", "沉", "水"] {
        if let Some(wid) = resolve_word_id(registry, ch)
            && casket.count(&wid) < 3
        {
            casket.add(&wid, 3);
            mutated = true;
        }
    }

    // 2. 領域專屬詞元（依線索加成，嚴守字庫限制）
    let domain_chars: &[&str] = if clue.contains("鍛") || clue.contains("鐵") || clue.contains("礦") || clue.contains("窯") {
        &["火", "破", "堅", "定"]
    } else if clue.contains("食堂") || clue.contains("廚") || clue.contains("麵") || clue.contains("烤") || clue.contains("茶") {
        &["火", "溫", "和", "純"]
    } else if clue.contains("醫") || clue.contains("藥") || clue.contains("診") || clue.contains("百草") || clue.contains("浴") {
        &["清", "純", "生", "和"]
    } else if clue.contains("木") || clue.contains("竹") || clue.contains("筏") || clue.contains("造船") {
        &["巧", "固", "正", "合"]
    } else if clue.contains("織") || clue.contains("染") || clue.contains("服") || clue.contains("繡") {
        &["柔", "密", "巧", "純"]
    } else if clue.contains("港") || clue.contains("船") || clue.contains("海") || clue.contains("漁") || clue.contains("渡") {
        &["風", "潮", "流", "定"]
    } else {
        &[]
    };

    for &ch in domain_chars {
        if let Some(wid) = resolve_word_id(registry, ch)
            && casket.count(&wid) < 3
        {
            casket.add(&wid, 3);
            mutated = true;
        }
    }

    // 3. 解析本職隨身器物並刻銘
    let vessel = crate::npc::causal_core::resolve_vocational_vessel(&clue);
    let inscribe_pair: &[&str] = if vessel.is_bladed
        || clue.contains("鍛")
        || clue.contains("鐵")
        || clue.contains("木")
        || clue.contains("石")
    {
        &["巧", "堅"]
    } else {
        &["純", "流"]
    };
    if casket.inscribed_words_of(vessel.item_id).is_empty() {
        for &ch in inscribe_pair {
            if let Some(wid) = resolve_word_id(registry, ch) {
                casket.add(&wid, 1);
                let _ = casket.inscribe_item(vessel.item_id, &wid);
                mutated = true;
            }
        }
    }
    if !vessel.catalog_id.is_empty() && casket.inscribed_words_of(vessel.catalog_id).is_empty() {
        for &ch in inscribe_pair {
            if let Some(wid) = resolve_word_id(registry, ch) {
                casket.add(&wid, 1);
                let _ = casket.inscribe_item(vessel.catalog_id, &wid);
                mutated = true;
            }
        }
    }

    // 石磨器具固有刻印相容性
    if casket.inscribed_words_of("wt_cat_0984").is_empty() {
        for ch in ["巧", "堅"] {
            if let Some(wid) = resolve_word_id(registry, ch) {
                casket.add(&wid, 1);
                let _ = casket.inscribe_item("wt_cat_0984", &wid);
                mutated = true;
            }
        }
    }
    if casket.inscribed_words_of("wt_cat_0973").is_empty() {
        for ch in ["純", "流"] {
            if let Some(wid) = resolve_word_id(registry, ch) {
                casket.add(&wid, 1);
                let _ = casket.inscribe_item("wt_cat_0973", &wid);
                mutated = true;
            }
        }
    }

    mutated
}

pub fn ensure_player_initialized(pid: &str) {
    let registry = WordRegistry::global();
    let mut mutated = false;
    let casket_opt = {
        let mut caskets = PLAYER_CASKETS.write();
        let is_new = !caskets.contains_key(pid);
        let casket = caskets
            .entry(pid.to_string())
            .or_insert_with(|| create_default_casket(registry));
        if is_new {
            mutated = true;
        }
        let resident_mutated = populate_resident_vocational_casket(pid, casket, registry);
        if resident_mutated {
            mutated = true;
        }
        Some(casket.clone())
    };
    {
        let mut loadouts = PLAYER_LOADOUTS.write();
        let soul_seed = db::get_entity(pid).ok().flatten().and_then(|e| e.soul_seed);
        let seed = crate::combat::human_phase::resolve_entity_soul_seed(pid, soul_seed);
        let needs_reset = match loadouts.get(pid) {
            None => true,
            Some(loadout) => {
                let has_legacy_name = |slot: &Option<StarSentence>| {
                    slot.as_ref()
                        .and_then(|s| s.custom_name.as_deref())
                        .map(|name| matches!(name, "疾刺斬" | "流破斬" | "吾震破" | "凝流步") || name.contains('・'))
                        .unwrap_or(false)
                };
                has_legacy_name(&loadout.opening)
                    || has_legacy_name(&loadout.midgame)
                    || has_legacy_name(&loadout.finisher)
                    || has_legacy_name(&loadout.defense)
            }
        };
        if needs_reset {
            let loadout = crate::combat::wordplate::build_loadout_from_soul_seed(seed, casket_opt.as_ref(), registry);
            loadouts.insert(pid.to_string(), loadout);
            mutated = true;
        } else if let Some(loadout) = loadouts.get_mut(pid)
            && (loadout.opening.is_none()
                || loadout.midgame.is_none()
                || loadout.finisher.is_none()
                || loadout.defense.is_none())
        {
            let default_loadout =
                crate::combat::wordplate::build_loadout_from_soul_seed(seed, casket_opt.as_ref(), registry);
            if loadout.opening.is_none() {
                loadout.opening = default_loadout.opening;
                mutated = true;
            }
            if loadout.midgame.is_none() {
                loadout.midgame = default_loadout.midgame;
                mutated = true;
            }
            if loadout.finisher.is_none() {
                loadout.finisher = default_loadout.finisher;
                mutated = true;
            }
            if loadout.defense.is_none() {
                loadout.defense = default_loadout.defense;
                mutated = true;
            }
        }
    }
    if mutated && is_persisted_wordplate_owner(pid) {
        save_player_wordplates();
    }
    // 滅世後的新生：不再於登入時自動塞入舊世「空白手札（blank_manual）」至玩家行囊
    let _ = &STARTER_MANUAL_GRANTED;
}

/// 一次性建世種子：為全島工坊與住戶匠人初始化器物磨損與吐納基底（所有人皆為自己的主角）
pub fn ensure_flagship_specimen_qin_shuisheng_wordplate_initialized() {
    let pid = "秦水生";
    ensure_player_initialized(pid);
    let registry = WordRegistry::global();
    let mut mutated = false;
    let casket_snapshot = {
        let mut caskets = PLAYER_CASKETS.write();
        let casket = caskets
            .entry(pid.to_string())
            .or_insert_with(|| create_default_casket(registry));
        if populate_resident_vocational_casket(pid, casket, registry) {
            mutated = true;
        }
        casket.clone()
    };
    if mutated {
        let mut loadouts = PLAYER_LOADOUTS.write();
        let soul_seed = db::get_entity(pid).ok().flatten().and_then(|e| e.soul_seed);
        let seed = crate::combat::human_phase::resolve_entity_soul_seed(pid, soul_seed);
        let loadout = crate::combat::wordplate::build_loadout_from_soul_seed(
            seed,
            Some(&casket_snapshot),
            registry,
        );
        loadouts.insert(pid.to_string(), loadout);
        drop(loadouts);
        save_player_wordplates();
    }
}

/// 供戰鬥模組（combat.rs）呼叫以獲取玩家之四戰術序位配置
pub fn get_player_tactical_loadout(pid: &str) -> TacticalLoadout {
    ensure_player_initialized(pid);
    let guard = PLAYER_LOADOUTS.read();
    guard.get(pid).cloned().unwrap_or_default()
}

/// 供戰鬥模組（combat.rs）呼叫以獲取玩家之隨身詞匣
pub fn get_player_word_casket(pid: &str) -> WordCasket {
    ensure_player_initialized(pid);
    let guard = PLAYER_CASKETS.read();
    guard.get(pid).cloned().unwrap_or_default()
}

/// 供測試或系統設定特定角色之隨身詞匣
#[cfg(test)]
pub fn set_player_word_casket(pid: &str, casket: WordCasket) {
    ensure_player_initialized(pid);
    let mut guard = PLAYER_CASKETS.write();
    guard.insert(pid.to_string(), casket);
}
