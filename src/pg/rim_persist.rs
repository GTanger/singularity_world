//! 輕量版 2D 開放世界 PostgreSQL 稀疏持久化模組（工作階段、`rim_actor_state`、`rim_micro_overlay`）。

use std::collections::{HashMap, HashSet};

use crate::grid::substrate::rim_sim::{ActorFaction, RimActor};
use crate::grid::substrate::rim_world::{
    BlueprintJob, GroundStack, MicroCellOverlay, RimItemKind, StructureKind, micro_to_macro,
};
use crate::store::sql::DbPool;

const CREATE_TABLES: &str = "
CREATE TABLE IF NOT EXISTS rim_http_sessions (
    sid TEXT PRIMARY KEY,
    player_id TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS rim_actor_state (
    actor_key TEXT PRIMARY KEY,
    actor_id INTEGER NOT NULL DEFAULT 0,
    entity_id TEXT,
    glyph TEXT NOT NULL,
    name TEXT NOT NULL,
    gender TEXT NOT NULL DEFAULT 'M',
    soul_seed BIGINT NOT NULL DEFAULT 0,
    role_tag TEXT NOT NULL,
    faction TEXT NOT NULL,
    px REAL NOT NULL,
    py REAL NOT NULL,
    facing_rad REAL NOT NULL,
    hp INTEGER NOT NULL,
    max_hp INTEGER NOT NULL,
    equipped_tool TEXT,
    carrying_json TEXT,
    inventory_json TEXT NOT NULL DEFAULT '[]',
    autopilot BOOLEAN NOT NULL DEFAULT false,
    directive TEXT NOT NULL DEFAULT '',
    activity_tag TEXT NOT NULL DEFAULT '',
    last_active_unix BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS rim_micro_overlay (
    mx INTEGER NOT NULL,
    my INTEGER NOT NULL,
    gx INTEGER NOT NULL,
    gy INTEGER NOT NULL,
    cleared_natural BOOLEAN NOT NULL DEFAULT false,
    structure TEXT,
    blueprint_json TEXT,
    ground_item_json TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (mx, my)
);
CREATE INDEX IF NOT EXISTS idx_rim_micro_overlay_macro ON rim_micro_overlay (gx, gy);
ALTER TABLE rim_actor_state ADD COLUMN IF NOT EXISTS hunger SMALLINT NOT NULL DEFAULT 86;
ALTER TABLE rim_actor_state ADD COLUMN IF NOT EXISTS thirst SMALLINT NOT NULL DEFAULT 88;
ALTER TABLE rim_actor_state ADD COLUMN IF NOT EXISTS stamina SMALLINT NOT NULL DEFAULT 90;
ALTER TABLE rim_actor_state ADD COLUMN IF NOT EXISTS magnesium INTEGER NOT NULL DEFAULT 18;
ALTER TABLE rim_actor_state ADD COLUMN IF NOT EXISTS equipped_hat BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE rim_actor_state ADD COLUMN IF NOT EXISTS equipped_pack BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE rim_actor_state ADD COLUMN IF NOT EXISTS known_by_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE rim_actor_state ADD COLUMN IF NOT EXISTS phase_json TEXT NOT NULL DEFAULT '{}';
ALTER TABLE rim_micro_overlay ADD COLUMN IF NOT EXISTS eco_ticks SMALLINT NOT NULL DEFAULT 0;
CREATE TABLE IF NOT EXISTS rim_settlement_vault (
    vault_id SMALLINT PRIMARY KEY,
    vault_mg BIGINT NOT NULL DEFAULT 120,
    hunting_pressure INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
";

/// 冪等建立 2D 世界工作階段、實體與微格覆寫資料表。
pub fn ensure_schema(pool: &DbPool) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    conn.batch_execute(CREATE_TABLES)
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn save_session(pool: &DbPool, sid: &str, player_id: &str) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO rim_http_sessions (sid, player_id, updated_at)
         VALUES ($1, $2, now())
         ON CONFLICT (sid) DO UPDATE SET player_id = EXCLUDED.player_id, updated_at = now()",
        &[&sid, &player_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn resolve_session(pool: &DbPool, sid: &str) -> Option<String> {
    let mut conn = pool.get().ok()?;
    let row = conn
        .query_opt(
            "SELECT player_id FROM rim_http_sessions WHERE sid = $1",
            &[&sid],
        )
        .ok()??;
    Some(row.get(0))
}

pub fn delete_session(pool: &DbPool, sid: &str) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM rim_http_sessions WHERE sid = $1", &[&sid])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_all_overlays(pool: &DbPool) -> Result<HashMap<(i32, i32), MicroCellOverlay>, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let rows = conn
        .query(
            "SELECT mx, my, cleared_natural, structure, blueprint_json, ground_item_json, eco_ticks
             FROM rim_micro_overlay",
            &[],
        )
        .map_err(|e| e.to_string())?;
    let mut out = HashMap::with_capacity(rows.len());
    for row in rows {
        let mx: i32 = row.get(0);
        let my: i32 = row.get(1);
        let cleared_natural: bool = row.get(2);
        let structure: Option<String> = row.get(3);
        let blueprint_json: Option<String> = row.get(4);
        let ground_item_json: Option<String> = row.get(5);
        let eco_ticks: i16 = row.get(6);
        let ov = MicroCellOverlay {
            cleared_natural,
            structure: structure.and_then(|s| serde_json::from_str::<StructureKind>(&s).ok()),
            blueprint: blueprint_json.and_then(|s| serde_json::from_str::<BlueprintJob>(&s).ok()),
            ground_item: ground_item_json
                .and_then(|s| serde_json::from_str::<GroundStack>(&s).ok()),
            eco_ticks: eco_ticks.max(0) as u16,
        };
        if !ov.is_pristine() {
            out.insert((mx, my), ov);
        }
    }
    Ok(out)
}

pub fn load_all_actors(pool: &DbPool) -> Result<Vec<RimActor>, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let rows = conn
        .query(
            "SELECT actor_key, actor_id, entity_id, glyph, name, gender, soul_seed, role_tag,
                    faction, px, py, facing_rad, hp, max_hp, equipped_tool, carrying_json,
                    inventory_json, autopilot, directive, activity_tag, last_active_unix,
                    hunger, thirst, stamina, magnesium, equipped_hat, equipped_pack, known_by_json, phase_json
             FROM rim_actor_state
             ORDER BY actor_id ASC, actor_key ASC",
            &[],
        )
        .map_err(|e| e.to_string())?;
    let mut out = Vec::with_capacity(rows.len());
    for (idx, row) in rows.into_iter().enumerate() {
        let actor_key: String = row.get(0);
        let raw_id: i32 = row.get(1);
        let entity_id: Option<String> = row.get(2);
        let faction_str: String = row.get(8);
        let faction =
            serde_json::from_str::<ActorFaction>(&faction_str).unwrap_or(ActorFaction::Settler);
        let equipped_tool = row
            .get::<_, Option<String>>(14)
            .and_then(|s| serde_json::from_str::<RimItemKind>(&s).ok());
        let carrying = row
            .get::<_, Option<String>>(15)
            .and_then(|s| serde_json::from_str::<GroundStack>(&s).ok());
        let inventory_raw: String = row.get(16);
        let inventory =
            serde_json::from_str::<Vec<GroundStack>>(&inventory_raw).unwrap_or_default();
        let actor_id = if raw_id > 0 { raw_id as usize } else { idx + 1 };
        let hunger = row.get::<_, i16>(21).clamp(0, 100) as u8;
        let thirst = row.get::<_, i16>(22).clamp(0, 100) as u8;
        let stamina = row.get::<_, i16>(23).clamp(0, 100) as u8;
        let magnesium = row.get::<_, i32>(24).max(0) as u32;
        let equipped_hat: bool = row.get(25);
        let equipped_pack: bool = row.get(26);
        let known_raw: String = row.get(27);
        let known_by = serde_json::from_str::<Vec<String>>(&known_raw).unwrap_or_default();
        let phase_raw: String = row.get(28);
        let mut actor = RimActor {
            id: actor_id,
            code: actor_key.clone(),
            actor_key,
            entity_id,
            glyph: row.get(3),
            name: row.get(4),
            gender: row.get(5),
            soul_seed: row.get(6),
            role_tag: row.get(7),
            faction,
            is_player: false,
            autopilot: row.get(17),
            running: false,
            px: row.get(9),
            py: row.get(10),
            facing_rad: row.get(11),
            hp: row.get(12),
            max_hp: row.get(13),
            equipped_tool,
            carrying,
            inventory,
            directive: row.get(18),
            path: std::collections::VecDeque::new(),
            task: crate::grid::substrate::rim_sim::PendingTask::None,
            activity_tag: row.get(19),
            pop_text: None,
            cooldown_ticks: 0,
            last_active_unix: row.get(20),
            hunger,
            thirst,
            stamina,
            magnesium,
            equipped_hat,
            equipped_pack,
            known_by,
            practice_unarmed: 0,
            practice_blade: 0,
            practice_blunt: 0,
            craft_harvest: 0,
            craft_forge: 0,
            craft_culinary: 0,
            craft_wayfare: 0,
            anatomy_winged: 0,
            anatomy_hoofed: 0,
            injury_part: None,
            injury_ticks: 0,
            downed_ticks: 0,
            caravan_leg: 0,
            wet_ticks: 0,
            recent_diary: std::collections::VecDeque::new(),
            civic: Default::default(),
        };
        actor.apply_phase_json(&phase_raw);
        out.push(actor);
    }
    Ok(out)
}

/// 原子交易寫入 2D 地圖髒格覆寫與人物狀態，並同步玩家 `entities.grid_x/grid_y/current_activity`。
pub fn load_economy_book(
    pool: &DbPool,
) -> Result<crate::grid::substrate::rim_economy::RimEconomyBook, String> {
    let mut book = crate::grid::substrate::rim_economy::RimEconomyBook::default();
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let rows = conn
        .query(
            "SELECT vault_id, vault_mg, hunting_pressure FROM rim_settlement_vault ORDER BY vault_id ASC",
            &[],
        )
        .map_err(|e| e.to_string())?;
    for r in rows {
        let vid: i16 = r.get(0);
        let mg: i64 = r.get(1);
        let hp: i32 = r.get(2);
        if (0..=1).contains(&vid) {
            book.vault_mg[vid as usize] = mg.clamp(0, i64::from(u32::MAX)) as u32;
            book.hunting_pressure = book.hunting_pressure.max(hp.clamp(0, i32::from(u16::MAX)) as u16);
        }
    }
    Ok(book)
}

pub fn flush_snapshot(
    pool: &DbPool,
    overlays: &HashMap<(i32, i32), MicroCellOverlay>,
    actors: &[RimActor],
    economy: &crate::grid::substrate::rim_economy::RimEconomyBook,
) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let mut tx = conn.transaction().map_err(|e| e.to_string())?;

    // 清理已回歸原生狀態之微格
    let existing_cells = tx
        .query("SELECT mx, my FROM rim_micro_overlay", &[])
        .map_err(|e| e.to_string())?;
    for r in existing_cells {
        let pos = (r.get::<_, i32>(0), r.get::<_, i32>(1));
        if !overlays.get(&pos).is_some_and(|ov| !ov.is_pristine()) {
            tx.execute(
                "DELETE FROM rim_micro_overlay WHERE mx = $1 AND my = $2",
                &[&pos.0, &pos.1],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    for (&(mx, my), ov) in overlays {
        if ov.is_pristine() {
            continue;
        }
        let ((gx, gy), _) = micro_to_macro(mx, my);
        let st_json = ov.structure.and_then(|s| serde_json::to_string(&s).ok());
        let bp_json = ov.blueprint.and_then(|b| serde_json::to_string(&b).ok());
        let it_json = ov.ground_item.and_then(|i| serde_json::to_string(&i).ok());
        let eco_i16 = ov.eco_ticks.min(i16::MAX as u16) as i16;
        tx.execute(
            "INSERT INTO rim_micro_overlay (mx, my, gx, gy, cleared_natural, structure, blueprint_json, ground_item_json, eco_ticks, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, now())
             ON CONFLICT (mx, my) DO UPDATE SET
               gx = EXCLUDED.gx,
               gy = EXCLUDED.gy,
               cleared_natural = EXCLUDED.cleared_natural,
               structure = EXCLUDED.structure,
               blueprint_json = EXCLUDED.blueprint_json,
               ground_item_json = EXCLUDED.ground_item_json,
               eco_ticks = EXCLUDED.eco_ticks,
               updated_at = now()",
            &[
                &mx,
                &my,
                &gx,
                &gy,
                &ov.cleared_natural,
                &st_json,
                &bp_json,
                &it_json,
                &eco_i16,
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    let mut active_keys: HashSet<&str> = HashSet::with_capacity(actors.len());
    for a in actors {
        let key = if a.actor_key.is_empty() {
            a.code.as_str()
        } else {
            a.actor_key.as_str()
        };
        active_keys.insert(key);
        let faction_json =
            serde_json::to_string(&a.faction).unwrap_or_else(|_| "\"settler\"".into());
        let tool_json = a.equipped_tool.and_then(|t| serde_json::to_string(&t).ok());
        let carry_json = a.carrying.and_then(|c| serde_json::to_string(&c).ok());
        let inv_json = serde_json::to_string(&a.inventory).unwrap_or_else(|_| "[]".into());
        let actor_id_i32 = i32::try_from(a.id).unwrap_or(0);
        let gender = if a.gender == "F" { "F" } else { "M" };
        let hunger_i16 = i16::from(a.hunger.min(100));
        let thirst_i16 = i16::from(a.thirst.min(100));
        let stamina_i16 = i16::from(a.stamina.min(100));
        let mag_i32 = i32::try_from(a.magnesium).unwrap_or(i32::MAX);
        let known_json = serde_json::to_string(&a.known_by).unwrap_or_else(|_| "[]".into());
        let phase_json = a.phase_json();
        tx.execute(
            "INSERT INTO rim_actor_state (
                actor_key, actor_id, entity_id, glyph, name, gender, soul_seed, role_tag,
                faction, px, py, facing_rad, hp, max_hp, equipped_tool, carrying_json,
                inventory_json, autopilot, directive, activity_tag, last_active_unix,
                hunger, thirst, stamina, magnesium, equipped_hat, equipped_pack, known_by_json,
                phase_json, updated_at
             ) VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,
                $22,$23,$24,$25,$26,$27,$28,$29, now()
             )
             ON CONFLICT (actor_key) DO UPDATE SET
                actor_id = EXCLUDED.actor_id,
                entity_id = EXCLUDED.entity_id,
                glyph = EXCLUDED.glyph,
                name = EXCLUDED.name,
                gender = EXCLUDED.gender,
                soul_seed = EXCLUDED.soul_seed,
                role_tag = EXCLUDED.role_tag,
                faction = EXCLUDED.faction,
                px = EXCLUDED.px,
                py = EXCLUDED.py,
                facing_rad = EXCLUDED.facing_rad,
                hp = EXCLUDED.hp,
                max_hp = EXCLUDED.max_hp,
                equipped_tool = EXCLUDED.equipped_tool,
                carrying_json = EXCLUDED.carrying_json,
                inventory_json = EXCLUDED.inventory_json,
                autopilot = EXCLUDED.autopilot,
                directive = EXCLUDED.directive,
                activity_tag = EXCLUDED.activity_tag,
                last_active_unix = EXCLUDED.last_active_unix,
                hunger = EXCLUDED.hunger,
                thirst = EXCLUDED.thirst,
                stamina = EXCLUDED.stamina,
                magnesium = EXCLUDED.magnesium,
                equipped_hat = EXCLUDED.equipped_hat,
                equipped_pack = EXCLUDED.equipped_pack,
                known_by_json = EXCLUDED.known_by_json,
                phase_json = EXCLUDED.phase_json,
                updated_at = now()",
            &[
                &key,
                &actor_id_i32,
                &a.entity_id,
                &a.glyph,
                &a.name,
                &gender,
                &a.soul_seed,
                &a.role_tag,
                &faction_json,
                &a.px,
                &a.py,
                &a.facing_rad,
                &a.hp,
                &a.max_hp,
                &tool_json,
                &carry_json,
                &inv_json,
                &a.autopilot,
                &a.directive,
                &a.activity_tag,
                &a.last_active_unix,
                &hunger_i16,
                &thirst_i16,
                &stamina_i16,
                &mag_i32,
                &a.equipped_hat,
                &a.equipped_pack,
                &known_json,
                &phase_json,
            ],
        )
        .map_err(|e| e.to_string())?;

        if let Some(pid) = a.entity_id.as_deref() {
            let (gx, gy) = a.micro_pos();
            tx.execute(
                "UPDATE entities SET grid_x = $2, grid_y = $3, current_activity = $4 WHERE id = $1",
                &[&pid, &gx, &gy, &a.activity_tag],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    let existing_actors = tx
        .query("SELECT actor_key FROM rim_actor_state", &[])
        .map_err(|e| e.to_string())?;
    for r in existing_actors {
        let k: String = r.get(0);
        if !active_keys.contains(k.as_str()) {
            tx.execute("DELETE FROM rim_actor_state WHERE actor_key = $1", &[&k])
                .map_err(|e| e.to_string())?;
        }
    }

    let pressure_i32 = i32::from(economy.hunting_pressure);
    for vid in 0_i16..=1_i16 {
        let vmg_i64 = i64::from(economy.vault_mg[vid as usize]);
        tx.execute(
            "INSERT INTO rim_settlement_vault (vault_id, vault_mg, hunting_pressure, updated_at)
             VALUES ($1, $2, $3, now())
             ON CONFLICT (vault_id) DO UPDATE SET
               vault_mg = EXCLUDED.vault_mg,
               hunting_pressure = EXCLUDED.hunting_pressure,
               updated_at = now()",
            &[&vid, &vmg_i64, &pressure_i32],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
