// NPC 建立、種子、池生成，對齊既有 db/npc。

use std::time::{SystemTime, UNIX_EPOCH};

use rand::Rng;

use crate::store::{self, Entity, Store};

use super::equip::starter_equipment;
use super::npc_names::{first_rune, generate_npc_name};
use super::{expand_soul_seed_to_base_stats, generate_soul_seed};
use super::ErrNoStore;

/// 預設 NPC 一筆（預設 NPC 列表為空，對齊保留結構）。
#[derive(Debug, Clone, Copy)]
pub struct NpcDef {
    pub id: &'static str,
    pub display_char: &'static str,
    pub gender: &'static str,
    pub work_room: &'static str,
    pub rest_room: &'static str,
    pub shift_start: i32,
    pub shift_end: i32,
    /// 指派職稱／occupation id
    pub title: &'static str,
}

/// 與預設種子列表 相同：啟動時可為空。
pub const DEFAULT_NPCS: &[NpcDef] = &[];

const VENUE_LIFE_INN: &str = "venue_life_inn";

fn format_base36(mut n: u128) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if n == 0 {
        return "0".to_string();
    }
    let mut v = Vec::new();
    while n > 0 {
        v.push(DIGITS[(n % 36) as usize] as char);
        n /= 36;
    }
    v.into_iter().rev().collect()
}

fn npc_gender_counts_locked(s: &Store) -> (i32, i32) {
    let mut male = 0i32;
    let mut female = 0i32;
    for eid in s.get_npc_ids_with_room() {
        let Some(e) = s.get_entity(&eid) else {
            continue;
        };
        match e.gender.as_str() {
            "M" => male += 1,
            "F" => female += 1,
            _ => {}
        }
    }
    (male, female)
}

/// 在已持有 `&mut Store` 時寫入新 NPC（對齊既有 `InsertNPC`）。
pub(crate) fn insert_npc_locked(
    s: &mut Store,
    id: &str,
    display_char: &str,
    gender: &str,
    display_title: &str,
) -> anyhow::Result<()> {
    let display_char = if display_char.is_empty() {
        let r: String = id.chars().take(1).collect();
        if r.is_empty() {
            "人".to_string()
        } else {
            r
        }
    } else {
        display_char.to_string()
    };
    let gender = if gender == "M" || gender == "F" {
        gender.to_string()
    } else {
        "M".to_string()
    };
    let seed = generate_soul_seed();
    let (vit, qi, dex) = expand_soul_seed_to_base_stats(seed);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let equip = starter_equipment(&gender);
    let e = Entity {
        id: id.to_string(),
        kind: "npc".to_string(),
        display_char,
        x: 0,
        y: 0,
        move_state: "idle".to_string(),
        target_x: None,
        target_y: None,
        walk_or_run: String::new(),
        move_started_at: None,
        vit,
        qi,
        dex,
        magnesium: 100,
        last_observed_at: None,
        created_at: now,
        gender,
        soul_seed: Some(seed),
        display_title: display_title.to_string(),
        activated_nodes: r#"["N000"]"#.to_string(),
        equipment_slots: equip,
        inventory: "[]".to_string(),
        disposition: 0,
        current_activity: String::new(),
        hex_q: None,
        hex_r: None,
        grid_x: None,
        grid_y: None,
    };
    s.put_entity(e)
}

/// 新增一筆 NPC 實體（對齊既有 `InsertNPC`）。
pub fn insert_npc(
    id: &str,
    display_char: &str,
    gender: &str,
    display_title: &str,
) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap_or_else(|e| e.into_inner());
    insert_npc_locked(&mut s, id, display_char, gender, display_title)
}

/// 逐一檢查預設 NPC（對齊既有 `SeedNPCs`）。
pub fn seed_npcs() -> anyhow::Result<()> {
    seed_npcs_for_store()
}

/// 確保 `DEFAULT_NPCS` 存在並設房間／排班／指派（對齊既有 `SeedNPCsForStore`）。
pub fn seed_npcs_for_store() -> anyhow::Result<()> {
    let Some(arc) = store::get_store() else {
        return Ok(());
    };
    let mut s = arc.write().unwrap_or_else(|e| e.into_inner());
    for npc in DEFAULT_NPCS {
        if s.get_entity(npc.id).is_some() {
            let _ = s.insert_schedule(
                npc.id,
                npc.work_room,
                npc.rest_room,
                npc.shift_start,
                npc.shift_end,
            );
            let _ = s.insert_assignment(npc.id, npc.title, VENUE_LIFE_INN, "");
            continue;
        }
        insert_npc_locked(&mut s, npc.id, npc.display_char, npc.gender, "")?;
        s.set_entity_room(npc.id, npc.work_room)?;
        s.insert_schedule(
            npc.id,
            npc.work_room,
            npc.rest_room,
            npc.shift_start,
            npc.shift_end,
        )?;
        s.insert_assignment(npc.id, npc.title, VENUE_LIFE_INN, "")?;
    }
    Ok(())
}

/// 為缺 `grid_x`/`grid_y` 的既存 NPC 補寫方格世界初始座標（grid 原點）。
/// 不改 vit/qi/dex 等其他欄位；hex_q/hex_r 也不動。
/// 啟動時可安全重複呼叫（已有座標的 NPC 不受影響）。
pub fn ensure_grid_coords() -> anyhow::Result<i32> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap_or_else(|e| e.into_inner());
    let missing: Vec<String> = s
        .npc_ids_missing_grid()
        .into_iter()
        .collect();
    let mut fixed = 0i32;
    for id in missing {
        if s.set_entity_grid(&id, 0, 0).is_ok() {
            fixed += 1;
        }
    }
    Ok(fixed)
}

/// 有房間的 NPC 男／女人數（對齊既有 `GetNPCGenderCounts`）。
#[must_use]
pub fn get_npc_gender_counts() -> (i32, i32) {
    let Some(arc) = store::get_store() else {
        return (0, 0);
    };
    let s = arc.read().unwrap_or_else(|e| e.into_inner());
    npc_gender_counts_locked(&s)
}

/// 獲取當前地圖總格數（房間數）。
#[must_use]
pub fn get_room_count() -> usize {
    let Some(arc) = store::get_store() else {
        return 0;
    };
    arc.read().unwrap_or_else(|e| e.into_inner()).rooms.len()
}

/// 為缺 `soul_seed` 的 NPC 補寫；不改 vit/qi/dex（對齊既有 `EnsureAllNPCsHaveSoulSeed`）。
pub fn ensure_all_npcs_have_soul_seed() -> anyhow::Result<i32> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap_or_else(|e| e.into_inner());
    let ids = s.npc_ids_with_missing_soul_seed();
    let mut fixed = 0i32;
    for id in ids {
        let seed = generate_soul_seed();
        if s.update_entity(&id, |e| {
            e.soul_seed = Some(seed);
        })
        .is_ok()
        {
            fixed += 1;
        }
    }
    Ok(fixed)
}

/// 自池生成一名 NPC 並放入指定房間（對齊既有 `SpawnOneNPCFromPool`）。
pub fn spawn_one_npc_from_pool(spawn_room_id: &str) -> anyhow::Result<String> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap_or_else(|e| e.into_inner());

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut id = format!("npc_{}", format_base36(nanos));
    while s.get_entity(&id).is_some() {
        let n2 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        id = format!("npc_{}_{}", n2, format_base36(n2 % 10_000));
    }

    let name = generate_npc_name("");
    let display_char = first_rune(&name);
    let (male, female) = npc_gender_counts_locked(&s);
    let mut rng = rand::rng();
    let gender = if male > female {
        "F"
    } else if female > male {
        "M"
    } else if rng.random_bool(0.5) {
        "F"
    } else {
        "M"
    };

    insert_npc_locked(&mut s, &id, &display_char, gender, &name)?;
    s.set_entity_room(&id, spawn_room_id)?;
    // 給新 NPC 方格世界初始座標（grid 原點），讓 grid_ai 能啟動。
    // 若 NPC 所在 spawn_room 已對應到特定 grid 格則後續 grid_ai 會自行更新。
    let _ = s.set_entity_grid(&id, 0, 0);
    Ok(id)
}
