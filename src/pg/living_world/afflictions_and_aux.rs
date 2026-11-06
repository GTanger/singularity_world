#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AfflictionKind {
    HeadConcussion, // 頭面震盪
    TorsoInternal,  // 胸脅內傷
    ArmSprain,      // 腕肘挫傷
    LegSprain,      // 下盤扭傷
    Miasma,         // 瘴毒入體（非部位環境傷患，須以藥圃草藥湯劑拔除）
}

impl AfflictionKind {
    pub fn code(self) -> &'static str {
        match self {
            Self::HeadConcussion => "head",
            Self::TorsoInternal => "torso",
            Self::ArmSprain => "arm",
            Self::LegSprain => "leg",
            Self::Miasma => "miasma",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "head" => Some(Self::HeadConcussion),
            "torso" => Some(Self::TorsoInternal),
            "arm" => Some(Self::ArmSprain),
            "leg" => Some(Self::LegSprain),
            "miasma" => Some(Self::Miasma),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::HeadConcussion => "頭面震盪",
            Self::TorsoInternal => "胸脅內傷",
            Self::ArmSprain => "腕肘挫傷",
            Self::LegSprain => "下盤扭傷",
            Self::Miasma => "瘴毒入體",
        }
    }

    pub fn symptom_prose(self) -> &'static str {
        match self {
            Self::HeadConcussion => "耳鳴目眩，凝神推演與識海專注略顯滯澀",
            Self::TorsoInternal => "胸脅氣血翻湧，周天運氣與發力受阻",
            Self::ArmSprain => "腕肘筋骨挫痛，運使手中器物與百工手藝略打折扣",
            Self::LegSprain => "下盤經絡扭傷，跋涉移步與閃轉騰挪較平時吃力",
            Self::Miasma => "濕熱瘴氣鬱結肺腑，須藉草藥湯劑或藥圃敷貼始得拔除",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscribedManualRecord {
    pub manual_hash: String,
    pub author_id: String,
    pub author_name: String,
    pub formal_name: String,
    pub slot_label: String,
    pub carrier_code: String,
    pub word_ids: Vec<String>,
    pub word_chars: Vec<String>,
    pub origin_basis: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeastCorpseRecord {
    pub corpse_item_id: String,
    pub creature_title: String,
    pub primary_loot_id: String,
    pub primary_loot_name: String,
    pub shanhai_note: String,
    pub tier: u8,
    pub fresh_until: i64,
}

static AFFLICTION_CACHE: LazyLock<RwLock<HashMap<String, Vec<AfflictionKind>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static GROUND_DROPS_CACHE: LazyLock<RwLock<HashMap<(i32, i32), Vec<(String, String, i32)>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static MANUALS_CACHE: LazyLock<RwLock<HashMap<String, TranscribedManualRecord>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static CORPSES_CACHE: LazyLock<RwLock<HashMap<String, BeastCorpseRecord>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static CORPSE_SEQ: AtomicU64 = AtomicU64::new(1000);
static STUDIED_MANUALS_CACHE: LazyLock<RwLock<HashMap<String, Vec<String>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static ANATOMY_FAMILIARITY_CACHE: LazyLock<RwLock<HashMap<String, HashMap<String, u32>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static PG_AUX_TABLES_READY: LazyLock<RwLock<bool>> = LazyLock::new(|| RwLock::new(false));

fn ensure_aux_pg_tables() {
    if *PG_AUX_TABLES_READY.read().unwrap() {
        return;
    }
    let mut ready = PG_AUX_TABLES_READY.write().unwrap();
    if *ready {
        return;
    }
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let _ = client.batch_execute(
            "SET client_min_messages = WARNING;
            CREATE TABLE IF NOT EXISTS entity_body_injuries (
                entity_id TEXT NOT NULL,
                affliction_code TEXT NOT NULL,
                source_ref TEXT NOT NULL DEFAULT '',
                created_at BIGINT NOT NULL DEFAULT 0,
                PRIMARY KEY (entity_id, affliction_code)
            );
            CREATE TABLE IF NOT EXISTS ground_dropped_items (
                drop_id TEXT PRIMARY KEY,
                gx INTEGER NOT NULL,
                gy INTEGER NOT NULL,
                owner_id TEXT NOT NULL,
                item_id TEXT NOT NULL,
                item_name TEXT NOT NULL,
                qty INTEGER NOT NULL,
                claimed BOOLEAN NOT NULL DEFAULT FALSE,
                created_at BIGINT NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS transcribed_manuals (
                manual_hash TEXT PRIMARY KEY,
                author_id TEXT NOT NULL,
                author_name TEXT NOT NULL,
                formal_name TEXT NOT NULL,
                slot_label TEXT NOT NULL,
                carrier_code TEXT NOT NULL,
                word_ids_json TEXT NOT NULL,
                word_chars_json TEXT NOT NULL,
                origin_basis TEXT NOT NULL,
                created_at BIGINT NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS beast_corpse_instances (
                corpse_item_id TEXT PRIMARY KEY,
                creature_title TEXT NOT NULL,
                primary_loot_id TEXT NOT NULL,
                primary_loot_name TEXT NOT NULL,
                shanhai_note TEXT NOT NULL,
                tier SMALLINT NOT NULL DEFAULT 1,
                fresh_until BIGINT NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS entity_breath_habits (
                entity_id TEXT PRIMARY KEY,
                dominant_code TEXT NOT NULL DEFAULT 'forest_long',
                forge_score INTEGER NOT NULL DEFAULT 0,
                forest_score INTEGER NOT NULL DEFAULT 0,
                tide_score INTEGER NOT NULL DEFAULT 0,
                chamber_score INTEGER NOT NULL DEFAULT 0,
                battle_score INTEGER NOT NULL DEFAULT 0,
                updated_at BIGINT NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS entity_vessel_states (
                entity_id TEXT NOT NULL,
                item_id TEXT NOT NULL,
                use_count INTEGER NOT NULL DEFAULT 0,
                wear_points INTEGER NOT NULL DEFAULT 0,
                condition_code TEXT NOT NULL DEFAULT 'sharp',
                updated_at BIGINT NOT NULL DEFAULT 0,
                PRIMARY KEY (entity_id, item_id)
            );
            CREATE TABLE IF NOT EXISTS entity_lineage_imprints (
                entity_id TEXT NOT NULL,
                mentor_name TEXT NOT NULL,
                imprint_title TEXT NOT NULL,
                imprint_prose TEXT NOT NULL,
                interaction_count INTEGER NOT NULL DEFAULT 1,
                manual_gifted BOOLEAN NOT NULL DEFAULT FALSE,
                updated_at BIGINT NOT NULL DEFAULT 0,
                PRIMARY KEY (entity_id, mentor_name)
            );
            CREATE TABLE IF NOT EXISTS character_vessel_mastery (
                entity_id TEXT NOT NULL,
                item_id TEXT NOT NULL,
                use_count INTEGER NOT NULL DEFAULT 0,
                wear_points INTEGER NOT NULL DEFAULT 0,
                condition_code TEXT NOT NULL DEFAULT 'sharp',
                updated_at BIGINT NOT NULL DEFAULT 0,
                PRIMARY KEY (entity_id, item_id)
            );
            CREATE TABLE IF NOT EXISTS character_lineage_imprints (
                entity_id TEXT NOT NULL,
                mentor_name TEXT NOT NULL,
                imprint_title TEXT NOT NULL,
                imprint_prose TEXT NOT NULL,
                interaction_count INTEGER NOT NULL DEFAULT 1,
                manual_gifted BOOLEAN NOT NULL DEFAULT FALSE,
                updated_at BIGINT NOT NULL DEFAULT 0,
                PRIMARY KEY (entity_id, mentor_name)
            );
            CREATE TABLE IF NOT EXISTS character_breath_habit (
                entity_id TEXT PRIMARY KEY,
                dominant_code TEXT NOT NULL DEFAULT 'forest_long',
                forge_score INTEGER NOT NULL DEFAULT 0,
                forest_score INTEGER NOT NULL DEFAULT 0,
                tide_score INTEGER NOT NULL DEFAULT 0,
                chamber_score INTEGER NOT NULL DEFAULT 0,
                battle_score INTEGER NOT NULL DEFAULT 0,
                updated_at BIGINT NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS entity_studied_manuals (
                entity_id TEXT NOT NULL,
                manual_title TEXT NOT NULL,
                studied_at BIGINT NOT NULL DEFAULT 0,
                PRIMARY KEY (entity_id, manual_title)
            );
            CREATE TABLE IF NOT EXISTS entity_anatomy_familiarity (
                entity_id TEXT NOT NULL,
                target_category TEXT NOT NULL,
                familiarity_count INTEGER NOT NULL DEFAULT 0,
                updated_at BIGINT NOT NULL DEFAULT 0,
                PRIMARY KEY (entity_id, target_category)
            );"
        );
        if let Ok(rows) = client.query(
            "SELECT manual_hash, author_id, author_name, formal_name, slot_label, carrier_code, word_ids_json, word_chars_json, origin_basis, created_at FROM transcribed_manuals",
            &[],
        ) {
            let mut guard = MANUALS_CACHE.write().unwrap();
            for r in rows {
                let manual_hash: String = r.get(0);
                let word_ids: Vec<String> = serde_json::from_str(r.get::<_, &str>(6)).unwrap_or_default();
                let word_chars: Vec<String> = serde_json::from_str(r.get::<_, &str>(7)).unwrap_or_default();
                guard.insert(
                    manual_hash.clone(),
                    TranscribedManualRecord {
                        manual_hash,
                        author_id: r.get(1),
                        author_name: r.get(2),
                        formal_name: r.get(3),
                        slot_label: r.get(4),
                        carrier_code: r.get(5),
                        word_ids,
                        word_chars,
                        origin_basis: r.get(8),
                        created_at: r.get(9),
                    },
                );
            }
        }
        if let Ok(rows) = client.query(
            "SELECT entity_id, affliction_code FROM entity_body_injuries",
            &[],
        ) {
            let mut guard = AFFLICTION_CACHE.write().unwrap();
            for r in rows {
                let eid: String = r.get(0);
                let code: String = r.get(1);
                if let Some(kind) = AfflictionKind::from_code(&code) {
                    let list = guard.entry(eid).or_default();
                    if !list.contains(&kind) {
                        list.push(kind);
                    }
                }
            }
        }
        if let Ok(rows) = client.query(
            "SELECT gx, gy, item_id, item_name, qty FROM ground_dropped_items WHERE claimed = FALSE",
            &[],
        ) {
            let mut guard = GROUND_DROPS_CACHE.write().unwrap();
            for r in rows {
                let gx: i32 = r.get(0);
                let gy: i32 = r.get(1);
                let item_id: String = r.get(2);
                let item_name: String = r.get(3);
                let qty: i32 = r.get(4);
                guard.entry((gx, gy)).or_default().push((item_id, item_name, qty));
            }
        }
        if let Ok(rows) = client.query(
            "SELECT corpse_item_id, creature_title, primary_loot_id, primary_loot_name, shanhai_note, tier, fresh_until FROM beast_corpse_instances",
            &[],
        ) {
            let mut guard = CORPSES_CACHE.write().unwrap();
            for r in rows {
                let cid: String = r.get(0);
                let tier_i16: i16 = r.get(5);
                guard.insert(
                    cid.clone(),
                    BeastCorpseRecord {
                        corpse_item_id: cid,
                        creature_title: r.get(1),
                        primary_loot_id: r.get(2),
                        primary_loot_name: r.get(3),
                        shanhai_note: r.get(4),
                        tier: tier_i16.max(1) as u8,
                        fresh_until: r.get(6),
                    },
                );
            }
        }
        if let Ok(rows) = client.query(
            "SELECT entity_id, dominant_code, forge_score, forest_score, tide_score, chamber_score, battle_score, updated_at FROM entity_breath_habits",
            &[],
        ) {
            let mut guard = BREATH_HABIT_CACHE.write().unwrap();
            for r in rows {
                let eid: String = r.get(0);
                let code: String = r.get(1);
                guard.insert(
                    eid.clone(),
                    BreathHabitRecord {
                        entity_id: eid,
                        dominant: BreathHabitKind::from_code(&code),
                        forge_score: r.get(2),
                        forest_score: r.get(3),
                        tide_score: r.get(4),
                        chamber_score: r.get(5),
                        battle_score: r.get(6),
                        updated_at: r.get(7),
                    },
                );
            }
        }
        if let Ok(rows) = client.query(
            "SELECT entity_id, item_id, use_count, wear_points, condition_code, updated_at FROM entity_vessel_states",
            &[],
        ) {
            let mut guard = VESSEL_STATE_CACHE.write().unwrap();
            for r in rows {
                let eid: String = r.get(0);
                let iid: String = r.get(1);
                let cond_code: String = r.get(4);
                guard.insert(
                    (eid.clone(), iid.clone()),
                    VesselStateRecord {
                        entity_id: eid,
                        item_id: iid,
                        use_count: r.get(2),
                        wear_points: r.get(3),
                        condition: VesselCondition::from_code(&cond_code),
                        updated_at: r.get(5),
                    },
                );
            }
        }
        if let Ok(rows) = client.query(
            "SELECT entity_id, mentor_name, imprint_title, imprint_prose, interaction_count, manual_gifted, updated_at FROM entity_lineage_imprints",
            &[],
        ) {
            let mut guard = LINEAGE_IMPRINT_CACHE.write().unwrap();
            for r in rows {
                let eid: String = r.get(0);
                let mentor: String = r.get(1);
                let list = guard.entry(eid.clone()).or_default();
                if !list.iter().any(|x| x.mentor_name == mentor) {
                    list.push(LineageImprintRecord {
                        entity_id: eid,
                        mentor_name: mentor,
                        imprint_title: r.get(2),
                        imprint_prose: r.get(3),
                        interaction_count: r.get(4),
                        manual_gifted: r.get(5),
                        updated_at: r.get(6),
                    });
                }
            }
        }
        if let Ok(rows) = client.query(
            "SELECT entity_id, manual_title FROM entity_studied_manuals ORDER BY studied_at ASC",
            &[],
        ) {
            let mut guard = STUDIED_MANUALS_CACHE.write().unwrap();
            for r in rows {
                let eid: String = r.get(0);
                let title: String = r.get(1);
                let list = guard.entry(eid).or_default();
                if !list.contains(&title) {
                    list.push(title);
                }
            }
        }
        if let Ok(rows) = client.query(
            "SELECT entity_id, target_category, familiarity_count FROM entity_anatomy_familiarity",
            &[],
        ) {
            let mut guard = ANATOMY_FAMILIARITY_CACHE.write().unwrap();
            for r in rows {
                let eid: String = r.get(0);
                let cat: String = r.get(1);
                let count: i32 = r.get(2);
                guard.entry(eid).or_default().insert(cat, count.max(0) as u32);
            }
        }
        *ready = true;
    }
}

pub fn record_entity_affliction(
    entity_id: &str,
    kind: AfflictionKind,
    source_ref: &str,
    now_unix: i64,
) {
    ensure_aux_pg_tables();
    {
        let mut guard = AFFLICTION_CACHE.write().unwrap();
        let list = guard.entry(entity_id.to_string()).or_default();
        if !list.contains(&kind) {
            list.push(kind);
        }
    }
    if let Some(pool) = crate::pg::pool() {
        let _ = open_obligation(
            &pool,
            entity_id,
            "treat_injury",
            kind.code(),
            Some(now_unix + 7200),
            source_ref,
            now_unix,
        );
        if let Ok(mut client) = pool.get() {
            let _ = client.execute(
                "INSERT INTO entity_body_injuries (entity_id, affliction_code, source_ref, created_at)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (entity_id, affliction_code) DO UPDATE SET source_ref = EXCLUDED.source_ref, created_at = EXCLUDED.created_at",
                &[&entity_id, &kind.code(), &source_ref, &now_unix],
            );
        }
    }
}

pub fn get_entity_afflictions(entity_id: &str) -> Vec<AfflictionKind> {
    ensure_aux_pg_tables();
    AFFLICTION_CACHE
        .read()
        .unwrap()
        .get(entity_id)
        .cloned()
        .unwrap_or_default()
}

pub fn clear_entity_afflictions(
    entity_id: &str,
    include_miasma: bool,
    now_unix: i64,
) -> Vec<AfflictionKind> {
    ensure_aux_pg_tables();
    let mut removed = Vec::new();
    let all_cleared = {
        let mut guard = AFFLICTION_CACHE.write().unwrap();
        if let Some(list) = guard.get_mut(entity_id) {
            let mut kept = Vec::new();
            for item in list.drain(..) {
                if item == AfflictionKind::Miasma && !include_miasma {
                    kept.push(item);
                } else {
                    removed.push(item);
                }
            }
            let has_remaining = !kept.is_empty();
            *list = kept;
            !has_remaining
        } else {
            true
        }
    };
    if !removed.is_empty()
        && let Some(pool) = crate::pg::pool()
    {
        if all_cleared {
            let _ = resolve_obligations_by_kind(&pool, entity_id, "treat_injury", now_unix);
        }
        if let Ok(mut client) = pool.get() {
            if include_miasma {
                let _ = client.execute(
                    "DELETE FROM entity_body_injuries WHERE entity_id = $1",
                    &[&entity_id],
                );
            } else {
                let _ = client.execute(
                    "DELETE FROM entity_body_injuries WHERE entity_id = $1 AND affliction_code <> 'miasma'",
                    &[&entity_id],
                );
            }
        }
    }
    removed
}

pub fn record_ground_dropped_items(
    gx: i32,
    gy: i32,
    owner_id: &str,
    items: &[(String, String, i32)],
    now_unix: i64,
) {
    if items.is_empty() {
        return;
    }
    ensure_aux_pg_tables();
    {
        let mut guard = GROUND_DROPS_CACHE.write().unwrap();
        let entry = guard.entry((gx, gy)).or_default();
        for (id, name, qty) in items {
            entry.push((id.clone(), name.clone(), *qty));
        }
    }
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        for (id, name, qty) in items {
            let drop_id = Uuid::new_v4().to_string();
            let _ = client.execute(
                "INSERT INTO ground_dropped_items (drop_id, gx, gy, owner_id, item_id, item_name, qty, claimed, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, FALSE, $8)",
                &[&drop_id, &gx, &gy, &owner_id, id, name, qty, &now_unix],
            );
        }
    }
}

pub fn claim_ground_dropped_items(gx: i32, gy: i32) -> Vec<(String, String, i32)> {
    ensure_aux_pg_tables();
    let claimed = {
        let mut guard = GROUND_DROPS_CACHE.write().unwrap();
        guard.remove(&(gx, gy)).unwrap_or_default()
    };
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let _ = client.execute(
            "UPDATE ground_dropped_items SET claimed = TRUE WHERE gx = $1 AND gy = $2 AND claimed = FALSE",
            &[&gx, &gy],
        );
    }
    claimed
}

pub fn peek_ground_dropped_items(gx: i32, gy: i32) -> Vec<(String, String, i32)> {
    ensure_aux_pg_tables();
    GROUND_DROPS_CACHE
        .read()
        .unwrap()
        .get(&(gx, gy))
        .cloned()
        .unwrap_or_default()
}

pub fn snapshot_all_ground_dropped_items() -> std::collections::HashMap<(i32, i32), Vec<(String, String, i32)>> {
    ensure_aux_pg_tables();
    GROUND_DROPS_CACHE.read().unwrap().clone()
}

pub fn save_transcribed_manual(record: &TranscribedManualRecord) {
    ensure_aux_pg_tables();
    {
        let mut guard = MANUALS_CACHE.write().unwrap();
        guard.insert(record.manual_hash.clone(), record.clone());
    }
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let word_ids_json = serde_json::to_string(&record.word_ids).unwrap_or_else(|_| "[]".into());
        let word_chars_json =
            serde_json::to_string(&record.word_chars).unwrap_or_else(|_| "[]".into());
        let _ = client.execute(
            "INSERT INTO transcribed_manuals
               (manual_hash, author_id, author_name, formal_name, slot_label, carrier_code, word_ids_json, word_chars_json, origin_basis, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (manual_hash) DO UPDATE SET
               formal_name = EXCLUDED.formal_name,
               origin_basis = EXCLUDED.origin_basis",
            &[
                &record.manual_hash,
                &record.author_id,
                &record.author_name,
                &record.formal_name,
                &record.slot_label,
                &record.carrier_code,
                &word_ids_json,
                &word_chars_json,
                &record.origin_basis,
                &record.created_at,
            ],
        );
    }
}

pub fn get_transcribed_manual(manual_hash_or_item_id: &str) -> Option<TranscribedManualRecord> {
    ensure_aux_pg_tables();
    let key = manual_hash_or_item_id
        .strip_prefix("manual_")
        .unwrap_or(manual_hash_or_item_id);
    MANUALS_CACHE.read().unwrap().get(key).cloned()
}

pub fn register_beast_corpse(
    creature_title: &str,
    primary_loot_id: &str,
    primary_loot_name: &str,
    shanhai_note: &str,
    tier: u8,
    now_unix: i64,
) -> BeastCorpseRecord {
    ensure_aux_pg_tables();
    let seq = CORPSE_SEQ.fetch_add(1, Ordering::Relaxed);
    let corpse_item_id = format!("corpse_{}_{}", primary_loot_id.trim_start_matches("wt_"), seq + (now_unix.unsigned_abs() % 10000));
    let fresh_until = now_unix + 1800; // 30 分鐘內靈韻未散可整軀剝名
    let rec = BeastCorpseRecord {
        corpse_item_id: corpse_item_id.clone(),
        creature_title: creature_title.to_string(),
        primary_loot_id: primary_loot_id.to_string(),
        primary_loot_name: primary_loot_name.to_string(),
        shanhai_note: shanhai_note.to_string(),
        tier: tier.max(1),
        fresh_until,
    };
    {
        let mut guard = CORPSES_CACHE.write().unwrap();
        guard.insert(corpse_item_id.clone(), rec.clone());
    }
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let tier_i16 = rec.tier as i16;
        let _ = client.execute(
            "INSERT INTO beast_corpse_instances
               (corpse_item_id, creature_title, primary_loot_id, primary_loot_name, shanhai_note, tier, fresh_until)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (corpse_item_id) DO NOTHING",
            &[
                &rec.corpse_item_id,
                &rec.creature_title,
                &rec.primary_loot_id,
                &rec.primary_loot_name,
                &rec.shanhai_note,
                &tier_i16,
                &rec.fresh_until,
            ],
        );
    }
    rec
}

pub fn get_beast_corpse(corpse_item_id: &str) -> Option<BeastCorpseRecord> {
    ensure_aux_pg_tables();
    CORPSES_CACHE.read().unwrap().get(corpse_item_id).cloned()
}

pub fn consume_beast_corpse(corpse_item_id: &str) {
    ensure_aux_pg_tables();
    CORPSES_CACHE.write().unwrap().remove(corpse_item_id);
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let _ = client.execute(
            "DELETE FROM beast_corpse_instances WHERE corpse_item_id = $1",
            &[&corpse_item_id],
        );
    }
}

// ============================================================================
// 5. 自然養成之「內息底色（吐納習慣）」與「人器磨合 × 鋒刃器況」與「師承烙印」
// ============================================================================

