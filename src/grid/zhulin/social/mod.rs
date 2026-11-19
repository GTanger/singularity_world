//! 沙鹿竹林活態居民與無職稱社會系統（Step 3）。
//!
//! - `person`：1 底板（`SoulSeed`）× 4 內在槽（10×12 正交脾氣、當下缺口、留痕器物、上限 8 條主觀認知）。
//! - `relations`：6 大關係插座 ＋ 不可洗白共同履歷 ＋ 具體權利移轉 ＋ 公開層（`z == 0`）目擊共振。
//! - `utility`：純 Rust Dave Mark 乘積補償效用 ＋ Hysteresis 防抖 ＋ 三大經濟守恆閥門。
//! - `sim`：320 名真活居民純 Rust 演化與嚴格零和守恆。
//! - 本模組提供 PostgreSQL `zhulin_social_state` 壓縮持久化、執行期單例、同格同層查詢、端詳／交談與診斷報表。

pub mod person;
pub mod relations;
pub mod sim;
pub mod utility;

use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::LazyLock;

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use parking_lot::RwLock;
use serde::Serialize;

use crate::grid::SHALU_ZHULIN_WORLD_SEED;
use crate::grid::substrate::hydrology::WorldClockSnapshot;
use crate::store::sql::DbPool;

use person::{BeliefKind, ConductEdge, CoreObsession, SocialResident};
use sim::{DEFAULT_RESIDENT_COUNT, SocialWorldState, StreetEcho};

const SOCIAL_STATE_ROW_ID: &str = "shalu_zhulin_v1";

const CREATE_SOCIAL_STATE_TABLE: &str = "
CREATE TABLE IF NOT EXISTS zhulin_social_state (
    id TEXT PRIMARY KEY,
    world_seed BIGINT NOT NULL,
    game_day INTEGER NOT NULL,
    tick_count BIGINT NOT NULL,
    payload_zlib BYTEA NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
)";

static SOCIAL_WORLD: LazyLock<RwLock<Option<SocialWorldState>>> =
    LazyLock::new(|| RwLock::new(None));

/// 供 `grid_view` / `room_enter` 渲染使用之活態居民視野卡（嚴格去數值化）。
#[derive(Debug, Clone, Serialize)]
pub struct SocialResidentView {
    pub id: String,
    pub display_name: String,
    pub card_state: &'static str,
    pub current_activity: String,
    pub hub_degree: usize,
}

/// 玩家端詳或交談 `zl_res_*` 居民之回傳結果。
#[derive(Debug, Clone)]
pub struct SocialInteractResult {
    pub target_name: String,
    pub narrative: String,
    pub actions: Vec<String>,
    pub unlocked_name: bool,
}

/// 單一風雲樞紐摘要（供診斷與驗證報表）。
#[derive(Debug, Clone, Serialize)]
pub struct HubResidentSummary {
    pub id: String,
    pub true_name: String,
    pub degree: usize,
    pub activity: String,
    pub gx: i32,
    pub gy: i32,
    pub gz: i16,
    pub magnesium: i32,
}

/// 活態社會診斷報表（`/api/substrate/social_summary` 與測試稽核專用）。
#[derive(Debug, Clone, Serialize)]
pub struct SocialSummaryReport {
    pub world_seed: u64,
    pub game_day: i32,
    pub tick_count: u64,
    pub resident_count: usize,
    pub town_count: usize,
    pub pioneer_count: usize,
    pub ground_visible_count: usize,
    pub upper_floor_count: usize,
    pub initial_total_mg: i64,
    pub current_total_mg: i64,
    pub burned_magnesium_total: i64,
    pub railcar_exported_bamboo: i64,
    pub railcar_imported_mg: i64,
    pub conservation_diff: i64,
    pub food_unit_mg: i32,
    pub bamboo_unit_mg: i32,
    pub shoot_unit_mg: i32,
    pub relation_edges: usize,
    pub socket_counts: HashMap<String, usize>,
    pub top_hubs: Vec<HubResidentSummary>,
    pub recent_echoes: Vec<StreetEcho>,
}

/// 將 `SocialWorldState` 序列化並以 zlib 壓縮（落盤體積僅數十 KB）。
pub fn encode_state_zlib(state: &SocialWorldState) -> anyhow::Result<Vec<u8>> {
    let raw = serde_json::to_vec(state)?;
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::fast());
    enc.write_all(&raw)?;
    Ok(enc.finish()?)
}

/// 從 zlib 壓縮位元組還原 `SocialWorldState`。
pub fn decode_state_zlib(bytes: &[u8]) -> anyhow::Result<SocialWorldState> {
    let mut dec = ZlibDecoder::new(bytes);
    let mut raw = Vec::new();
    dec.read_to_end(&mut raw)?;
    Ok(serde_json::from_slice(&raw)?)
}

/// 確保 `zhulin_social_state` 表存在。
pub fn ensure_schema(pool: &DbPool) -> anyhow::Result<()> {
    let mut client = pool.get()?;
    client.batch_execute(CREATE_SOCIAL_STATE_TABLE)?;
    Ok(())
}

/// 從 PostgreSQL 載入已保存之活態社會狀態。
pub fn load_state_from_pg(pool: &DbPool) -> anyhow::Result<Option<SocialWorldState>> {
    ensure_schema(pool)?;
    let mut client = pool.get()?;
    let row = client.query_opt(
        "SELECT payload_zlib FROM zhulin_social_state WHERE id = $1",
        &[&SOCIAL_STATE_ROW_ID],
    )?;
    let Some(row) = row else {
        return Ok(None);
    };
    let bytes: Vec<u8> = row.get(0);
    let state = decode_state_zlib(&bytes)?;
    Ok(Some(state))
}

/// 將當前活態社會狀態壓縮覆寫進 PostgreSQL。
pub fn save_state_to_pg(pool: &DbPool, state: &SocialWorldState) -> anyhow::Result<()> {
    ensure_schema(pool)?;
    let payload = encode_state_zlib(state)?;
    let mut client = pool.get()?;
    client.execute(
        "INSERT INTO zhulin_social_state (id, world_seed, game_day, tick_count, payload_zlib, updated_at)
         VALUES ($1, $2, $3, $4, $5, now())
         ON CONFLICT (id) DO UPDATE SET
            world_seed = EXCLUDED.world_seed,
            game_day = EXCLUDED.game_day,
            tick_count = EXCLUDED.tick_count,
            payload_zlib = EXCLUDED.payload_zlib,
            updated_at = now()",
        &[
            &SOCIAL_STATE_ROW_ID,
            &(state.world_seed as i64),
            &state.game_day,
            &(state.tick_count as i64),
            &payload,
        ],
    )?;
    Ok(())
}

/// 確保記憶體單例已就緒：優先讀 PG；若尚無紀錄則以 `SHALU_ZHULIN_WORLD_SEED` 建立並暖機 24 拍後落盤。
pub fn ensure_initialized(pool: Option<&DbPool>) {
    if SOCIAL_WORLD.read().is_some() {
        return;
    }
    let loaded = pool.and_then(|p| match load_state_from_pg(p) {
        Ok(opt) => opt,
        Err(e) => {
            tracing::warn!("zhulin::social load_state_from_pg failed: {e}");
            None
        }
    });
    let state = if let Some(existing) = loaded {
        existing
    } else {
        let mut fresh =
            SocialWorldState::new_seeded(SHALU_ZHULIN_WORLD_SEED, DEFAULT_RESIDENT_COUNT);
        let base_clock = WorldClockSnapshot::current_live();
        // 首次開服暖機 24 拍（約 1 個遊戲日），讓西麓街鎮開門即有真實帳本、熟客契與街頭回聲
        for step in 0..24 {
            let mut c = base_clock;
            c.minute_of_day = ((step * 60) % 1440) as u16;
            fresh.step_world_tick(c);
        }
        if let Some(p) = pool
            && let Err(e) = save_state_to_pg(p, &fresh)
        {
            tracing::warn!("zhulin::social initial save_state_to_pg failed: {e}");
        }
        fresh
    };
    let mut guard = SOCIAL_WORLD.write();
    if guard.is_none() {
        *guard = Some(state);
    }
}

/// 以全新狀態覆蓋執行期單例並寫入 PG（供 Step 4 60 日 Burn-in 寫盤使用）。
pub fn install_and_persist(pool: Option<&DbPool>, state: SocialWorldState) -> anyhow::Result<()> {
    if let Some(p) = pool {
        save_state_to_pg(p, &state)?;
    }
    *SOCIAL_WORLD.write() = Some(state);
    Ok(())
}

/// 執行一次活態社會即時節拍，並每 4 拍持久化至 PostgreSQL。
pub fn tick_live(pool: Option<&DbPool>, clock: WorldClockSnapshot) -> bool {
    ensure_initialized(pool);
    let snapshot_to_save = {
        let mut guard = SOCIAL_WORLD.write();
        let Some(state) = guard.as_mut() else {
            return false;
        };
        let prev_day = state.game_day;
        state.step_world_tick(clock);
        if state.tick_count.is_multiple_of(4) || state.game_day != prev_day {
            Some(state.clone())
        } else {
            None
        }
    };
    if let (Some(p), Some(st)) = (pool, snapshot_to_save)
        && let Err(e) = save_state_to_pg(p, &st)
    {
        tracing::warn!("zhulin::social periodic save failed: {e}");
    }
    true
}

/// 取得公開層（`gz == 0`）各座標格活態居民人數（供地圖格點疊加顯示，嚴格不含樓上私密層）。
pub fn ground_pop_counts() -> HashMap<(i32, i32), u16> {
    let guard = SOCIAL_WORLD.read();
    let Some(state) = guard.as_ref() else {
        return HashMap::new();
    };
    let mut counts: HashMap<(i32, i32), u16> = HashMap::new();
    for r in &state.residents {
        if r.gz == 0 {
            let entry = counts.entry((r.gx, r.gy)).or_insert(0);
            *entry = entry.saturating_add(1);
        }
    }
    counts
}

/// 查詢精確落在 `(gx, gy, gz)` 同格同層之活態居民（嚴格樓層隔離）。
pub fn residents_at_grid_z(
    gx: i32,
    gy: i32,
    gz: i16,
    viewer_player_id: &str,
) -> Vec<SocialResidentView> {
    let guard = SOCIAL_WORLD.read();
    let Some(state) = guard.as_ref() else {
        return Vec::new();
    };
    let known_set = state.known_by_player.get(viewer_player_id);
    let hub_deg = state.ledger.hub_degrees();

    let mut matched: Vec<SocialResidentView> = state
        .residents
        .iter()
        .filter(|r| r.gx == gx && r.gy == gy && r.gz == gz)
        .map(|r| {
            let known = known_set.is_some_and(|s| s.contains(&r.id));
            let deg = hub_deg.get(&r.id).copied().unwrap_or(0);
            SocialResidentView {
                id: r.id.clone(),
                display_name: r.observed_name(known).to_string(),
                card_state: if known { "named" } else { "stranger" },
                current_activity: r.current_activity.activity_phrase_zh().to_string(),
                hub_degree: deg,
            }
        })
        .collect();

    matched.sort_by(|a, b| {
        let a_known = a.card_state == "named";
        let b_known = b.card_state == "named";
        b_known
            .cmp(&a_known)
            .then_with(|| b.hub_degree.cmp(&a.hub_degree))
            .then_with(|| a.id.cmp(&b.id))
    });
    matched
}

/// 玩家端詳一名 `zl_res_*` 居民（去數值化敘事，僅同格同層可見）。
pub fn inspect_resident(
    resident_id: &str,
    viewer_player_id: &str,
    player_gx: i32,
    player_gy: i32,
    player_gz: i16,
) -> Option<SocialInteractResult> {
    let guard = SOCIAL_WORLD.read();
    let state = guard.as_ref()?;
    let r = state.residents.iter().find(|x| x.id == resident_id)?;
    let known = state
        .known_by_player
        .get(viewer_player_id)
        .is_some_and(|s| s.contains(&r.id));
    let target_name = r.observed_name(known).to_string();

    if r.gx != player_gx || r.gy != player_gy || r.gz != player_gz {
        return Some(SocialInteractResult {
            target_name,
            narrative: "你舉目張望，對方方才已轉身離開這一帶，往鄰街行去了。".to_string(),
            actions: Vec::new(),
            unlocked_name: false,
        });
    }

    let vessel_clause = r
        .vessels
        .first()
        .map(|v| format!("隨身帶著一件「{}」。", v.inspect_line()))
        .unwrap_or_default();
    let vibe_clause = temperament_vibe_clause(r);
    let body_vibe = r.deficit.body_vibe_zh();
    let act_phrase = r.current_activity.activity_phrase_zh();
    let (owed_out, owed_in) = state.ledger.debt_summary_of(&r.id);
    let ledger_hint = if owed_out > 0 {
        "腰際別著一紙折得發皺的欠單，神色略顯拘謹。"
    } else if owed_in > 0 {
        "袖口露出幾道紅墨折痕的帳摺，看來在街坊間頗有往來。"
    } else {
        ""
    };

    let narrative = format!(
        "你仔細打量眼前的{target_name}：此人{act_phrase}，{body_vibe}。{vibe_clause}{vessel_clause}{ledger_hint}"
    );
    Some(SocialInteractResult {
        target_name,
        narrative,
        actions: vec!["Look".to_string(), "Talk".to_string()],
        unlocked_name: false,
    })
}

/// 玩家與 `zl_res_*` 居民交談：記住真名落盤，依脾氣、當下缺口與近期目擊八卦自然回應。
pub fn talk_to_resident(
    resident_id: &str,
    viewer_player_id: &str,
    player_gx: i32,
    player_gy: i32,
    player_gz: i16,
    player_input: &str,
    pool: Option<&DbPool>,
) -> Option<SocialInteractResult> {
    let (result, persist_snapshot) = {
        let mut guard = SOCIAL_WORLD.write();
        let state = guard.as_mut()?;
        let r_idx = state.residents.iter().position(|x| x.id == resident_id)?;
        let r_pos = (
            state.residents[r_idx].gx,
            state.residents[r_idx].gy,
            state.residents[r_idx].gz,
        );
        let was_known = state
            .known_by_player
            .get(viewer_player_id)
            .is_some_and(|s| s.contains(resident_id));

        if r_pos != (player_gx, player_gy, player_gz) {
            let target_name = state.residents[r_idx].observed_name(was_known).to_string();
            return Some(SocialInteractResult {
                target_name,
                narrative: "你正欲搭話，對方已邁步離開此處，往鄰街走遠了。".to_string(),
                actions: Vec::new(),
                unlocked_name: false,
            });
        }

        let unlocked_name = state
            .known_by_player
            .entry(viewer_player_id.to_string())
            .or_default()
            .insert(resident_id.to_string());

        let r = &state.residents[r_idx];
        let name = r.true_name.clone();
        let intro = if unlocked_name {
            format!("對方抬眼打量了你一番，順手報了名號：「叫我{name}便成。」")
        } else {
            format!("{name}抬頭認出是你，點頭致意。")
        };
        let topic_line = compose_resident_speech(state, r, player_input);
        let narrative = format!("{intro}{topic_line}");
        let res = SocialInteractResult {
            target_name: name,
            narrative,
            actions: vec!["Look".to_string(), "Talk".to_string()],
            unlocked_name,
        };
        let snap = unlocked_name.then(|| state.clone());
        (res, snap)
    };

    if let (Some(p), Some(st)) = (pool, persist_snapshot)
        && let Err(e) = save_state_to_pg(p, &st)
    {
        tracing::warn!("zhulin::social save after talk failed: {e}");
    }
    Some(result)
}

/// 產出完整活態社會診斷報表（含守恆差、前 10 樞紐、六大插座計數、近期街頭回聲）。
pub fn summary_report() -> Option<SocialSummaryReport> {
    let guard = SOCIAL_WORLD.read();
    let state = guard.as_ref()?;
    Some(build_summary_report(state))
}

pub fn build_summary_report(state: &SocialWorldState) -> SocialSummaryReport {
    let resident_count = state.residents.len();
    let pioneer_count = state.residents.iter().filter(|r| r.pioneer_origin).count();
    let town_count = resident_count.saturating_sub(pioneer_count);
    let ground_visible_count = state.residents.iter().filter(|r| r.gz == 0).count();
    let upper_floor_count = resident_count.saturating_sub(ground_visible_count);
    let current_total_mg: i64 = state.residents.iter().map(|r| i64::from(r.magnesium)).sum();

    let mut socket_counts: HashMap<String, usize> = HashMap::from([
        ("zl_rel_debt".to_string(), 0),
        ("zl_rel_supply_pact".to_string(), 0),
        ("zl_rel_trade_rivalry".to_string(), 0),
        ("zl_rel_expedition_bond".to_string(), 0),
        ("zl_rel_craft_lineage".to_string(), 0),
        ("zl_rel_suspicion_hook".to_string(), 0),
    ]);
    for edge in state.ledger.edges.values() {
        for s in &edge.sockets {
            *socket_counts
                .entry(s.socket_code().to_string())
                .or_insert(0) += 1;
        }
    }

    let hub_deg = state.ledger.hub_degrees();
    let mut top_hubs: Vec<HubResidentSummary> = state
        .residents
        .iter()
        .map(|r| HubResidentSummary {
            id: r.id.clone(),
            true_name: r.true_name.clone(),
            degree: hub_deg.get(&r.id).copied().unwrap_or(0),
            activity: r.current_activity.observed_role_zh().to_string(),
            gx: r.gx,
            gy: r.gy,
            gz: r.gz,
            magnesium: r.magnesium,
        })
        .collect();
    top_hubs.sort_by(|a, b| b.degree.cmp(&a.degree).then_with(|| a.id.cmp(&b.id)));
    top_hubs.truncate(10);

    let recent_echoes = state.recent_echoes.iter().rev().take(10).cloned().collect();

    SocialSummaryReport {
        world_seed: state.world_seed,
        game_day: state.game_day,
        tick_count: state.tick_count,
        resident_count,
        town_count,
        pioneer_count,
        ground_visible_count,
        upper_floor_count,
        initial_total_mg: state.initial_total_mg,
        current_total_mg,
        burned_magnesium_total: state.valves.burned_magnesium_total,
        railcar_exported_bamboo: state.valves.railcar_exported_bamboo,
        railcar_imported_mg: state.valves.railcar_imported_mg,
        conservation_diff: state.conservation_diff(),
        food_unit_mg: state.valves.food_unit_mg,
        bamboo_unit_mg: state.valves.bamboo_unit_mg,
        shoot_unit_mg: state.valves.shoot_unit_mg,
        relation_edges: state.ledger.edges.len(),
        socket_counts,
        top_hubs,
        recent_echoes,
    }
}

fn temperament_vibe_clause(r: &SocialResident) -> &'static str {
    match r.temperament.edge {
        ConductEdge::Prideful => "脊背挺得筆直，言談間透著一股不肯落於人後的體面勁。",
        ConductEdge::PennyPincher => "指節時不時摩挲袖口，眼神精明，彷彿凡事都在心裡過一遍帳。",
        ConductEdge::GrudgeHolder => "眉梢微微下壓，看人時目光沉靜而帶幾分記性。",
        ConductEdge::TsundereSoft => "嘴角繃得有些硬，語氣沖人卻看不出多少惡意。",
        ConductEdge::Snobbish => "眼光在來客衣衫與行囊間快速掃過，頗看重對方來頭。",
        ConductEdge::LooseTongued => "眼珠子靈活轉動，一臉藏不住街頭新鮮事的模樣。",
        ConductEdge::Perfectionist => "衣襟與器具理得一絲不苟，容不得半點草率。",
        ConductEdge::RiskGambler => "神情爽利帶股狠勁，像是敢往險處搏一把的人。",
        ConductEdge::Suspicious => "說話前總先斜眼掃過左右街角，防備心頗重。",
        ConductEdge::BluntHothead => "步子邁得急促，眉宇間透著直來直往的急性子。",
        ConductEdge::QuietSlowWarm => "神色沉穩寡言，只安靜盯著手頭活計。",
        ConductEdge::PettyBargainer => "眼角帶著精打細算的笑意，處處想多討些便宜。",
    }
}

fn compose_resident_speech(
    state: &SocialWorldState,
    r: &SocialResident,
    player_input: &str,
) -> String {
    let trimmed = player_input.trim();
    // 若居民最近目擊過公開層事件，優先吐露活態風聞
    if let Some(belief) = r.beliefs.last() {
        let gossip = match &belief.kind {
            BeliefKind::StreetRumor { summary } => {
                format!("「說起這條街的動靜，{summary}，大夥兒都在看後頭怎麼收場。」")
            }
            BeliefKind::WitnessedDebt {
                debtor_id,
                creditor_id,
            } => {
                let d_name = resident_name_or(state, debtor_id, "那位街坊");
                let c_name = resident_name_or(state, creditor_id, "騎樓掌櫃");
                format!(
                    "「前陣子才親眼見著{d_name}跟{c_name}在街口算帳，這年頭手頭緊的人真不少。」"
                )
            }
            BeliefKind::WitnessedRescue {
                rescuer_id,
                saved_id,
            } => {
                let a_name = resident_name_or(state, rescuer_id, "同行山客");
                let b_name = resident_name_or(state, saved_id, "同伴");
                format!("「東邊竹林坡濕滑得很，幸好上回{a_name}手快拉住{b_name}一把。」")
            }
            BeliefKind::WitnessedTrade { item_label, .. } => {
                format!("「這幾日騎樓裡頭{item_label}走得勤，行市起落看得出端倪。」")
            }
            BeliefKind::SuspectedPoach { .. } => {
                "「近來林子裡老有人趁黑摸進別家做過記號的筍窩，你進山可別踩進是非裡。」".to_string()
            }
            BeliefKind::SecretGroveTip { .. } => {
                "「雨後東側高坡竹根下常冒嫩筍，熟門熟路的人才找得到。」".to_string()
            }
        };
        if !trimmed.is_empty()
            || matches!(
                r.temperament.edge,
                ConductEdge::LooseTongued | ConductEdge::Suspicious
            )
        {
            return gossip;
        }
    }

    let obsession_line = match r.temperament.obsession {
        CoreObsession::CraftPerfection => {
            "「手頭傢伙若是鈍了一分，做出來的活計便差了一截，馬虎不得。」"
        }
        CoreObsession::HoardAndOwn => "「這西麓石板街一天比一天熱鬧，趁早攢下本錢才是實在。」",
        CoreObsession::ProtectKin => "「出門在外街坊互相照應，誰有個閃失都難過。」",
        CoreObsession::OutshinePeers => "「同一條騎樓下做買賣，不拿出真本事哪能壓得住旁人？」",
        CoreObsession::PlaySafe => "「夏漲一來低處溝水說漫就漫，走石板正街最穩妥。」",
        CoreObsession::SeekWonders => {
            "「東邊莿竹林深處聽說還藏著老山客沒走透的窪地，遲早得去探一遭。」"
        }
        CoreObsession::FineAppetite => "「忙活半天，能趕上騎樓灶頭剛起鍋的鮮筍熱湯才算不白幹。」",
        CoreObsession::HonorLedger => {
            "「欠債還錢、立約交貨，咱在這條街上站得住腳靠的就是這口信字。」"
        }
        CoreObsession::SelfMadeShop => {
            "「多跑幾趟貨、多攢些實底，早晚在騎樓前堂支起自個兒的門面。」"
        }
        CoreObsession::BoldVengeance => "「誰敢在背後耍陰招，我這筆帳可是記在心底絕不輕放。」",
    };

    if r.deficit.hunger >= 0.72 {
        format!("「忙得連口熱食都還沒沾著……」{obsession_line}")
    } else if r.deficit.injury >= 0.45 {
        format!("「方才在坡上蹭了道傷，正想尋些草藥敷上。」{obsession_line}")
    } else {
        obsession_line.to_string()
    }
}

fn resident_name_or<'a>(state: &'a SocialWorldState, id: &str, fallback: &'a str) -> &'a str {
    state
        .residents
        .iter()
        .find(|r| r.id == id)
        .map(|r| r.true_name.as_str())
        .unwrap_or(fallback)
}

/// 檢查一個 `RelationSocket` 列表內是否包含指定代號（測試輔助）。
#[cfg(test)]
fn has_socket_code(report: &SocialSummaryReport, code: &str) -> bool {
    report.socket_counts.get(code).copied().unwrap_or(0) > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::substrate::hydrology::KaipuSeason;

    #[test]
    fn ten_day_simulation_preserves_strict_conservation_and_emerges_all_six_sockets() {
        let mut world =
            SocialWorldState::new_seeded(SHALU_ZHULIN_WORLD_SEED, DEFAULT_RESIDENT_COUNT);
        assert_eq!(world.conservation_diff(), 0);

        // 跑 10 遊戲日 × 每日 12 拍（每 2 遊戲小時 1 拍）= 120 拍
        for day in 1..=10 {
            for slot in 0..12 {
                let clock = WorldClockSnapshot {
                    game_day: day,
                    world_minute: day * 1440 + slot * 120,
                    minute_of_day: (slot * 120) as u16,
                    season: KaipuSeason::from_game_day(day),
                    weather_wetting: day % 4 == 0,
                };
                world.step_world_tick(clock);
                assert_eq!(
                    world.conservation_diff(),
                    0,
                    "day={day} slot={slot} conservation broke"
                );
            }
        }

        let report = build_summary_report(&world);
        assert_eq!(report.resident_count, 320);
        assert_eq!(report.conservation_diff, 0);
        assert!(
            report.burned_magnesium_total > 0,
            "valve 1 burn should occur"
        );
        assert!(
            report.relation_edges > 20,
            "edges={}",
            report.relation_edges
        );
        assert!(
            report.top_hubs.first().is_some_and(|h| h.degree >= 3),
            "top hub should emerge naturally: {:?}",
            report.top_hubs.first()
        );
        // 六大關係插座全數自然湧現（零硬編碼灌入）
        for code in [
            "zl_rel_debt",
            "zl_rel_supply_pact",
            "zl_rel_trade_rivalry",
            "zl_rel_expedition_bond",
            "zl_rel_craft_lineage",
            "zl_rel_suspicion_hook",
        ] {
            assert!(
                has_socket_code(&report, code),
                "missing emergent socket {code}; counts={:?}",
                report.socket_counts
            );
        }

        // zlib 壓縮與還原 100% 等價
        let blob = encode_state_zlib(&world).expect("encode zlib");
        let restored = decode_state_zlib(&blob).expect("decode zlib");
        assert_eq!(restored.tick_count, world.tick_count);
        assert_eq!(restored.conservation_diff(), 0);
        assert_eq!(restored.residents.len(), world.residents.len());
        assert_eq!(restored.ledger.edges.len(), world.ledger.edges.len());
    }

    #[test]
    fn restart_determinism_matches_bit_for_bit() {
        let run = || {
            let mut w = SocialWorldState::new_seeded(SHALU_ZHULIN_WORLD_SEED, 120);
            for step in 0..24 {
                let clock = WorldClockSnapshot {
                    game_day: 1 + (step / 12),
                    world_minute: step * 120,
                    minute_of_day: ((step * 120) % 1440) as u16,
                    season: KaipuSeason::SpringThaw,
                    weather_wetting: false,
                };
                w.step_world_tick(clock);
            }
            w
        };
        let a = run();
        let b = run();
        assert_eq!(a.conservation_diff(), 0);
        assert_eq!(
            a.valves.burned_magnesium_total,
            b.valves.burned_magnesium_total
        );
        assert_eq!(a.ledger.edges.len(), b.ledger.edges.len());
        for (ra, rb) in a.residents.iter().zip(b.residents.iter()) {
            assert_eq!(ra.id, rb.id);
            assert_eq!(ra.gx, rb.gx);
            assert_eq!(ra.gy, rb.gy);
            assert_eq!(ra.gz, rb.gz);
            assert_eq!(ra.magnesium, rb.magnesium);
            assert_eq!(ra.current_activity, rb.current_activity);
        }
    }
}
