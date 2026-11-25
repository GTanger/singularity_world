//! 背景活史官（Living Chronicler）PostgreSQL 正規化表、同交易事實卡落盤、
//! 手動修史熱同步（`updated_seq` 增量載入）與每日原子暫存目錄置換匯出。
//! （依設計者定案：已停用背景自動本機 LLM 修史；日常事蹟即時落盤台灣白話草稿與 5D 事實卡，由手動修史批次定稿。）

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use r2d2_postgres::postgres::types::ToSql;
use r2d2_postgres::postgres::{Client, Transaction};

use crate::pg;
use crate::store::sql::DbPool;

use super::causal_arcs::solar_year_zh;
use super::chronicle_export::export_markdown_chronicles_staged_atomic;
use super::clock_and_lang::{current_solar_clock, solar_clock_for_settled_bell};
use super::engine::global_island;
use super::gazetteer::REGIONAL_GAZETTEERS;
use super::living_chronicler::{
    ChronicleFactPacket, ChronicleProjectionRow, LivingChronicleState, num_zh,
    record_homestead_built, record_primordial_spider_slain, record_vacant_house_bought,
    record_witnessed_phase,
};
use super::types::{IslandWorldDelta, WorldStaticDoc};

const CHRONICLER_MD_OUT_DIR: &str = "docs/chronicles/銀鷗島人物列傳";

static CHRONICLER_WORKER_STARTED: AtomicBool = AtomicBool::new(false);

pub const CREATE_LIVING_CHRONICLER_SCHEMA: &str = "
ALTER TABLE oi_world_globals ADD COLUMN IF NOT EXISTS living_chronicler_v4 BOOLEAN NOT NULL DEFAULT false;
CREATE SEQUENCE IF NOT EXISTS oi_chronicle_proj_seq;
CREATE TABLE IF NOT EXISTS oi_chronicle_projections (
    deed_id TEXT NOT NULL,
    scope TEXT NOT NULL,
    target_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    latch_key TEXT NOT NULL DEFAULT '',
    title_zh TEXT NOT NULL DEFAULT '',
    subtitle_zh TEXT NOT NULL DEFAULT '',
    era_zh TEXT NOT NULL DEFAULT '',
    carrier_zh TEXT NOT NULL DEFAULT '',
    prose_zh TEXT NOT NULL,
    draft_zh TEXT NOT NULL,
    fact_json TEXT NOT NULL,
    solar_year SMALLINT NOT NULL,
    game_day BIGINT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft',
    lease_until_ms BIGINT NOT NULL DEFAULT 0,
    updated_seq BIGINT NOT NULL DEFAULT nextval('oi_chronicle_proj_seq'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (deed_id, scope, target_id)
);
CREATE INDEX IF NOT EXISTS idx_oi_chronicle_proj_status ON oi_chronicle_projections (status, lease_until_ms);
CREATE INDEX IF NOT EXISTS idx_oi_chronicle_proj_target ON oi_chronicle_projections (scope, target_id, game_day, updated_seq);
";

pub(super) fn init_and_load_chronicle_pg(
    conn: &mut Client,
    lc: &mut LivingChronicleState,
) -> Result<(), String> {
    conn.batch_execute(CREATE_LIVING_CHRONICLER_SCHEMA)
        .map_err(|e| e.to_string())?;
    let rows = conn
        .query(
            "SELECT deed_id, scope, target_id, kind, latch_key, title_zh, subtitle_zh, era_zh,
                    carrier_zh, prose_zh, draft_zh, fact_json, solar_year, game_day, status, updated_seq
             FROM oi_chronicle_projections
             ORDER BY game_day ASC, updated_seq ASC",
            &[],
        )
        .map_err(|e| e.to_string())?;
    for r in rows {
        lc.apply_row_in_memory(ChronicleProjectionRow {
            deed_id: r.get(0),
            scope: r.get(1),
            target_id: r.get(2),
            kind: r.get(3),
            latch_key: r.get(4),
            title_zh: r.get(5),
            subtitle_zh: r.get(6),
            era_zh: r.get(7),
            carrier_zh: r.get(8),
            prose_zh: r.get(9),
            draft_zh: r.get(10),
            fact_json: r.get(11),
            solar_year: r.get(12),
            game_day: r.get(13),
            status: r.get(14),
            updated_seq: r.get(15),
        });
    }
    Ok(())
}

/// 在 `execute_commit_batch_pg` 同一筆交易中批量寫入；若單段就地方輿彙整（`deed:region_live:*`）內容演進則自動就地更新並重排撰稿！
pub(super) fn flush_chronicle_outbox_tx(
    tx: &mut Transaction<'_>,
    rows: &[ChronicleProjectionRow],
) -> Result<(), String> {
    if rows.is_empty() {
        return Ok(());
    }
    let mut deeds = Vec::with_capacity(rows.len());
    let mut scopes = Vec::with_capacity(rows.len());
    let mut targets = Vec::with_capacity(rows.len());
    let mut kinds = Vec::with_capacity(rows.len());
    let mut latches = Vec::with_capacity(rows.len());
    let mut titles = Vec::with_capacity(rows.len());
    let mut subtitles = Vec::with_capacity(rows.len());
    let mut eras = Vec::with_capacity(rows.len());
    let mut carriers = Vec::with_capacity(rows.len());
    let mut proses = Vec::with_capacity(rows.len());
    let mut drafts = Vec::with_capacity(rows.len());
    let mut facts = Vec::with_capacity(rows.len());
    let mut years = Vec::with_capacity(rows.len());
    let mut days = Vec::with_capacity(rows.len());
    let mut statuses = Vec::with_capacity(rows.len());

    for r in rows {
        deeds.push(r.deed_id.clone());
        scopes.push(r.scope.clone());
        targets.push(r.target_id.clone());
        kinds.push(r.kind.clone());
        latches.push(r.latch_key.clone());
        titles.push(r.title_zh.clone());
        subtitles.push(r.subtitle_zh.clone());
        eras.push(r.era_zh.clone());
        carriers.push(r.carrier_zh.clone());
        proses.push(r.prose_zh.clone());
        drafts.push(r.draft_zh.clone());
        facts.push(r.fact_json.clone());
        years.push(r.solar_year);
        days.push(r.game_day);
        statuses.push(r.status.clone());
    }

    tx.execute(
        "INSERT INTO oi_chronicle_projections (
            deed_id, scope, target_id, kind, latch_key, title_zh, subtitle_zh, era_zh,
            carrier_zh, prose_zh, draft_zh, fact_json, solar_year, game_day, status
         )
         SELECT u.d, u.sc, u.tg, u.kn, u.lk, u.ti, u.su, u.er, u.ca, u.pr, u.dr, u.fa, u.yr, u.gd, u.st
         FROM UNNEST(
            $1::text[], $2::text[], $3::text[], $4::text[], $5::text[],
            $6::text[], $7::text[], $8::text[], $9::text[], $10::text[],
            $11::text[], $12::text[], $13::smallint[], $14::bigint[], $15::text[]
         ) AS u(d, sc, tg, kn, lk, ti, su, er, ca, pr, dr, fa, yr, gd, st)
         ON CONFLICT (deed_id, scope, target_id) DO UPDATE SET
            kind = EXCLUDED.kind,
            latch_key = EXCLUDED.latch_key,
            title_zh = EXCLUDED.title_zh,
            subtitle_zh = EXCLUDED.subtitle_zh,
            era_zh = EXCLUDED.era_zh,
            carrier_zh = EXCLUDED.carrier_zh,
            prose_zh = EXCLUDED.prose_zh,
            draft_zh = EXCLUDED.draft_zh,
            fact_json = EXCLUDED.fact_json,
            solar_year = EXCLUDED.solar_year,
            game_day = EXCLUDED.game_day,
            status = EXCLUDED.status,
            lease_until_ms = 0,
            updated_seq = nextval('oi_chronicle_proj_seq'),
            updated_at = now()
         WHERE oi_chronicle_projections.draft_zh <> EXCLUDED.draft_zh",
        &[
            &deeds as &(dyn ToSql + Sync),
            &scopes,
            &targets,
            &kinds,
            &latches,
            &titles,
            &subtitles,
            &eras,
            &carriers,
            &proses,
            &drafts,
            &facts,
            &years,
            &days,
            &statuses,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub(super) fn apply_living_chronicler_v4_upgrade(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
) -> usize {
    let mut cleaned_actors = 0usize;
    for a in delta.actors.values_mut() {
        if a.controller_id.is_some() || a.actor_id.starts_with("pl:") {
            continue;
        }
        let mut filled_core = false;
        if a.bio_core_zh.trim().is_empty() {
            if let Some((core, _)) = a.personal_bio_zh.split_once("\n\n近歲新篇：") {
                a.bio_core_zh = core.trim().to_string();
            } else if !a.personal_bio_zh.trim().is_empty() {
                a.bio_core_zh = a.personal_bio_zh.trim().to_string();
            } else {
                a.bio_core_zh = super::causal_arcs::render_deterministic_shiji_bio(doc, a, &[]);
            }
            filled_core = true;
        }
        let had_polluted_post = filled_core
            || !a.bio_post_chapters_zh.is_empty()
            || a.personal_bio_zh.contains("近歲新篇：")
            || a.personal_bio_zh != a.bio_core_zh;
        if had_polluted_post {
            a.bio_post_chapters_zh.clear();
            a.personal_bio_zh = a.bio_core_zh.clone();
            a.mark_mutated();
            cleaned_actors += 1;
        }
    }

    let snap_clock = if delta.last_settled_bell > 0 {
        solar_clock_for_settled_bell(delta.last_settled_bell, 0)
    } else {
        current_solar_clock()
    };
    let sold_snapshot: Vec<(String, String)> = delta
        .sold_houses
        .iter()
        .map(|(nid, aid)| (nid.clone(), aid.clone()))
        .collect();
    for (house_nid, buyer_id) in sold_snapshot {
        let price = doc
            .households
            .iter()
            .find(|h| h.node_id == house_nid && h.vacant_for_sale)
            .map_or(360, |h| h.price_copper);
        record_vacant_house_bought(doc, delta, &snap_clock, &buyer_id, &house_nid, price);
    }

    let built_snapshot: Vec<(String, String, i64)> = delta
        .built_houses
        .values()
        .map(|bh| (bh.owner_id.clone(), bh.anchor_node_id.clone(), bh.built_day))
        .collect();
    for (owner_id, lot_nid, bday) in built_snapshot {
        let bclock = solar_clock_for_settled_bell(bday * 12 + 6, 0);
        let lot_name = doc
            .nodes
            .get(&lot_nid)
            .map_or_else(|| "鎮郊屋地".to_string(), |n| n.name.clone());
        record_homestead_built(doc, delta, &bclock, &owner_id, &lot_nid, &lot_name);
    }

    if let Some((slayer_id, raw_label, slain_day)) = delta.first_spider_slain.clone() {
        let sclock = solar_clock_for_settled_bell(slain_day * 12 + 6, 0);
        let wep = delta
            .actors
            .get(&slayer_id)
            .map_or_else(|| "隨身兵刃".to_string(), |a| a.bound_weapon.name.clone());
        let co_helper = raw_label.split_once('與').map(|(_, rhs)| {
            delta
                .actors
                .values()
                .find(|a| {
                    a.controller_id.is_none()
                        && !a.actor_id.starts_with("pl:")
                        && a.display_name == rhs
                })
                .map_or_else(
                    || "pl:__masked_co_helper__".to_string(),
                    |a| a.actor_id.clone(),
                )
        });
        record_primordial_spider_slain(
            doc,
            delta,
            &sclock,
            &slayer_id,
            co_helper.as_deref(),
            "oi:dng_pit:lair",
            &wep,
        );
    }

    let phase_snaps: Vec<(String, String, String, Option<String>, Option<String>)> = delta
        .actors
        .values()
        .filter(|a| {
            a.mind.ltm.mastery.witnessed_martial_phase.is_some()
                || a.mind.ltm.mastery.witnessed_craft_phase.is_some()
        })
        .map(|a| {
            (
                a.actor_id.clone(),
                a.node_id.clone(),
                a.bound_weapon.name.clone(),
                a.mind.ltm.mastery.witnessed_martial_phase.clone(),
                a.mind.ltm.mastery.witnessed_craft_phase.clone(),
            )
        })
        .collect();
    for (aid, nid, wep, m_opt, c_opt) in phase_snaps {
        if let Some(mp) = m_opt {
            record_witnessed_phase(doc, delta, &snap_clock, &aid, &nid, &wep, &mp, true);
        }
        if let Some(cp) = c_opt {
            record_witnessed_phase(doc, delta, &snap_clock, &aid, &nid, &wep, &cp, false);
        }
    }

    cleaned_actors
}

/// 組裝純結構化 5D 事實卡提示詞（供手動修史工具 `chronicler-manual` 匯出使用）
pub fn build_fact_sheet_prompt(
    scope: &str,
    target_id: &str,
    title_zh: &str,
    fact: &ChronicleFactPacket,
    draft_fallback: &str,
) -> String {
    let yr_zh = solar_year_zh(fact.solar_year);
    let inland_rule = if fact.tide_zh.is_empty() {
        format!(
            "{}屬內陸山谷／河谷地形，嚴禁出現海灣、潮汐、海港、船隻、碼頭、棧橋、漂木等濱海字眼；",
            fact.town_zh
        )
    } else {
        String::new()
    };
    let quote_rule = if !fact.phase_zh.is_empty() {
        format!(
            "除「{}」這個專有名號外，嚴禁編造任何人物對話引號；",
            fact.phase_zh
        )
    } else {
        "嚴禁編造任何人物對話引號（不得出現「」或『』）；".to_string()
    };

    let mut bullets = String::new();
    bullets.push_str(&format!(
        "- 時節與現場氣候：太陽曆{}年{}（{}，{}）\n",
        yr_zh,
        fact.season_zh,
        fact.bell_zh,
        if fact.tide_zh.is_empty() {
            fact.weather_zh.clone()
        } else {
            format!("{}・{}", fact.weather_zh, fact.tide_zh)
        }
    ));
    bullets.push_str(&format!(
        "- 所在聚落與地標：{}（{}）\n",
        fact.town_zh, fact.place_zh
    ));
    if fact.amount_copper > 0 {
        bullets.push_str(&format!(
            "- 涉及銅幣數額：{}枚銅幣（請保留此中文數額）\n",
            num_zh(fact.amount_copper)
        ));
    }
    for b in &fact.bullet_facts_zh {
        bullets.push_str(&format!("- {b}\n"));
    }
    if fact.bullet_facts_zh.is_empty() {
        bullets.push_str(&format!("- 核心經過摘要：{draft_fallback}\n"));
    }

    let prev_context = if scope == "region" {
        REGIONAL_GAZETTEERS
            .iter()
            .find(|g| g.id == target_id)
            .and_then(|g| g.paragraphs.last().copied())
            .map(|p| {
                format!("\n【前文尾段（請自然銜接其語氣與地景，勿重複前文已講過的事）】\n{p}\n")
            })
            .unwrap_or_default()
    } else {
        String::new()
    };

    let shape_hint = match scope {
        "annals" => format!(
            "請撰寫《{title_zh}》正文，分 3 個自然段（段間空一行），依「歷史前因 → 當代破局動作現場 → 城鎮迴響」展開，不必刻意算字數。"
        ),
        "region" => format!(
            "請撰寫《{}風土志》最末一段「當代地方近況」（單一自然段，不換行，不必刻意算字數），把所有近事自然融為同一個聚落的生活畫面，切勿寫成死板計數報告。",
            fact.town_zh
        ),
        _ => "請撰寫該角色《人物列傳》的新增近歲段落（單一自然段，不換行，不必刻意算字數），具體寫出現場氛圍、動作與人情餘韻。".to_string(),
    };

    format!(
        "你是《奇點世界》史官。請根據下方結構化事實卡，用「純粹現代台灣正體白話散文」直接撰寫正文。\n\
        【寫作規範】：\n\
        1. {shape_hint}\n\
        2. 必須完整保留事實卡中的人名（例如帶前綴的「獸夾布蘭」或「外來旅人」一字不可拆改）、聚落地標名與太陽曆{}年等中文數字，施事者與受事者因果關係絕不可顛倒。\n\
        3. {quote_rule}{inland_rule}不用文言虛詞（禁：乃、遂於、未嘗、莫不稱快、愈顯活絡、之乎者也），不用破折號與阿拉伯數字，不加任何標題或前後說明，直接輸出散文正文。\n\
        {prev_context}\n\
        【結構化事實卡】：\n{bullets}",
        yr_zh
    )
}

/// 增量同步外部手動修史（`chronicler-manual apply`）寫入 PostgreSQL 之最新定稿，免重啟 `singularity.service` 即時生效！
fn sync_external_chronicle_updates_from_pg(pool: &DbPool) -> usize {
    let cur_max_seq = {
        let eng = global_island();
        let Ok(guard) = eng.delta.read() else {
            return 0;
        };
        guard.living_chronicle.committed.max_updated_seq
    };
    let Ok(mut conn) = pool.get() else {
        return 0;
    };
    let Ok(rows) = conn.query(
        "SELECT deed_id, scope, target_id, kind, latch_key, title_zh, subtitle_zh, era_zh,
                carrier_zh, prose_zh, draft_zh, fact_json, solar_year, game_day, status, updated_seq
         FROM oi_chronicle_projections
         WHERE updated_seq > $1
         ORDER BY updated_seq ASC",
        &[&cur_max_seq],
    ) else {
        return 0;
    };
    if rows.is_empty() {
        return 0;
    }
    let mut fetched = Vec::with_capacity(rows.len());
    for r in rows {
        fetched.push(ChronicleProjectionRow {
            deed_id: r.get(0),
            scope: r.get(1),
            target_id: r.get(2),
            kind: r.get(3),
            latch_key: r.get(4),
            title_zh: r.get(5),
            subtitle_zh: r.get(6),
            era_zh: r.get(7),
            carrier_zh: r.get(8),
            prose_zh: r.get(9),
            draft_zh: r.get(10),
            fact_json: r.get(11),
            solar_year: r.get(12),
            game_day: r.get(13),
            status: r.get(14),
            updated_seq: r.get(15),
        });
    }
    let cnt = fetched.len();
    let eng = global_island();
    if let Ok(mut guard) = eng.delta.write() {
        for row in fetched {
            guard.living_chronicle.apply_row_in_memory(row);
        }
    }
    cnt
}

pub fn spawn_living_chronicler_worker() {
    if CHRONICLER_WORKER_STARTED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    tokio::spawn(async move {
        let mut last_export_wm: Option<(i64, u64)> = None;
        let mut last_export_game_day: i64 = -1;
        let mut interval = tokio::time::interval(Duration::from_secs(6));

        loop {
            interval.tick().await;
            let Some(pool) = pg::pool() else {
                continue;
            };
            let pool_clone = pool.clone();
            let synced_external = tokio::task::spawn_blocking(move || {
                sync_external_chronicle_updates_from_pg(&pool_clone)
            })
            .await
            .unwrap_or(0);

            let today = current_solar_clock().game_days;
            let cur_wm = {
                let eng = global_island();
                eng.delta
                    .read()
                    .map_or((0, 0), |g| g.living_chronicle.export_watermark())
            };
            let should_export = last_export_wm.is_none()
                || synced_external > 0
                || (today > last_export_game_day && Some(cur_wm) != last_export_wm);
            if should_export {
                let res = tokio::task::spawn_blocking(move || {
                    let eng = global_island();
                    let snap = eng.delta.read().ok().map(|g| g.clone())?;
                    let wm = snap.living_chronicle.export_watermark();
                    match export_markdown_chronicles_staged_atomic(
                        &eng.doc,
                        &snap,
                        Path::new(CHRONICLER_MD_OUT_DIR),
                    ) {
                        Ok(cnt) => {
                            tracing::info!(
                                "[living_chronicler] staged Markdown sync completed ({cnt} residents, wm={wm:?})"
                            );
                            Some(wm)
                        }
                        Err(err) => {
                            tracing::warn!(
                                "[living_chronicler] staged Markdown sync skipped: {err}"
                            );
                            None
                        }
                    }
                })
                .await
                .ok()
                .flatten();

                if let Some(done_wm) = res {
                    last_export_wm = Some(done_wm);
                    last_export_game_day = today;
                }
            }
        }
    });
}
