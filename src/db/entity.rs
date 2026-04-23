//! 實體查詢與屬性更新：由 `store` 讀寫後轉成 `entity::Character`。
//!
//! 與 `db/mod.rs` 共享 `store_entity_to_character`、`ErrNoStore`。NPC display_title
//! cache 填充走 `npc_display` 子模組的 locked helpers。

use crate::entity::Character;
use crate::store::{self, Entity};

use super::{
    expand_soul_seed_to_personality, npc_display, room_hex, store_entity_to_character, ErrNoStore,
    Personality,
};

/// 依 id 查詢實體。
pub fn get_entity(id: &str) -> anyhow::Result<Option<Character>> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    // 快路徑：player/非 NPC 只需 read lock（多讀可並行，不卡 login 同時 writer）
    // 舊版全拿 write lock 是因為 NPC display_name cache update 的副作用，
    // 但 player 走不進那分支——不該為少數 NPC 情況讓所有 get_entity 排 writer 隊
    {
        let s = arc.read();
        match s.get_entity(id) {
            Some(se) if se.kind != "npc" => {
                return Ok(Some(store_entity_to_character(&se, "")));
            }
            None => return Ok(None),
            _ => {} // NPC case 落到下面拿 write lock
        }
    }
    // NPC 路徑：先用 read lock 確認是否需要 cache 填充
    // display_title 已有 → read 就夠，不升級 write lock
    {
        let s = arc.read();
        if let Some(se) = s.get_entity(id) {
            if !se.display_title.is_empty() {
                return Ok(Some(store_entity_to_character(&se, "")));
            }
        } else {
            return Ok(None);
        }
    }
    // display_title 為空才升級到 write lock 做 cache 填充
    let mut s = arc.write();
    let Some(se) = s.get_entity(id) else { return Ok(None) };
    if se.kind == "npc" {
        npc_display::npc_person_display_name_locked(&mut s, id);
    }
    let Some(se) = s.get_entity(id) else { return Ok(None) };
    Ok(Some(store_entity_to_character(&se, "")))
}

/// 回傳實體當前房間 id。
pub fn get_entity_room(entity_id: &str) -> anyhow::Result<String> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read();
    Ok(s.get_entity_room(entity_id))
}

/// 查詢座標落在 `[x_min,x_max]×[y_min,y_max]` 內的實體；`kind` 空字串表示不限種類。
pub fn get_entities_in_box(
    x_min: i32,
    x_max: i32,
    y_min: i32,
    y_max: i32,
    kind: &str,
) -> anyhow::Result<Vec<Character>> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let (mut entities, need_update): (Vec<Entity>, Vec<String>) = {
        let s = arc.read();
        let sel = s.get_entities_in_box(x_min, x_max, y_min, y_max, kind);
        let mut need = Vec::new();
        for e in &sel {
            if e.kind == "npc" && e.display_title.is_empty() {
                need.push(e.id.clone());
            }
        }
        (sel, need)
    };
    if !need_update.is_empty() {
        let mut s = arc.write();
        for id in &need_update {
            npc_display::npc_person_display_name_locked(&mut s, id);
        }
        for e in entities.iter_mut() {
            if need_update.iter().any(|id| id == &e.id)
                && let Some(updated) = s.get_entity(&e.id)
            {
                *e = updated;
            }
        }
    }
    Ok(entities.iter().map(|e| store_entity_to_character(e, "")).collect())
}

/// 回傳所有 `move_state == "moving"` 的實體。
pub fn get_moving_entities() -> anyhow::Result<Vec<Character>> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read();
    let ids = s.get_moving_entity_ids();
    let mut list = Vec::with_capacity(ids.len());
    for id in ids {
        let Some(se) = s.get_entity(&id) else {
            continue;
        };
        list.push(store_entity_to_character(&se, ""));
    }
    Ok(list)
}

/// 回傳指定房間內所有存活實體。
/// `game_hour` 0–23 供在職場顯示職稱；`-1` 不套用下班規則。
pub fn get_entities_in_room(room_id: &str, game_hour: i32) -> anyhow::Result<Vec<Character>> {
    use std::collections::HashSet;

    let arc = store::get_store().ok_or(ErrNoStore)?;
    let (id_set, need_update, label_room): (HashSet<String>, Vec<String>, String) = {
        let s = arc.read();
        let mut id_set: HashSet<String> = HashSet::new();
        for id in s.entity_ids_in_room(room_id) {
            id_set.insert(id);
        }
        if let Some((q, r)) = room_hex::resolve_room_to_hex(room_id) {
            for id in s.entity_ids_at_hex(q, r) {
                id_set.insert(id);
            }
        }
        let mut need = Vec::new();
        for id in &id_set {
            if let Some(e) = s.get_entity(id)
                && e.kind == "npc"
                && e.display_title.is_empty()
            {
                need.push(id.clone());
            }
        }
        let label_room = room_hex::canonical_location_key(room_id);
        (id_set, need, label_room)
    };
    if id_set.is_empty() {
        return Ok(Vec::new());
    }
    if !need_update.is_empty() {
        let mut s = arc.write();
        for id in &need_update {
            npc_display::npc_person_display_name_locked(&mut s, id);
        }
    }
    let mut s = arc.write();
    let mut list = Vec::new();
    for id in id_set {
        let Some(se) = s.get_entity(&id) else {
            continue;
        };
        if se.vit <= 0 {
            continue;
        }
        let label = if se.kind == "npc" {
            npc_display::npc_title_in_room_locked(&mut s, &id, &label_room, game_hour)
        } else {
            String::new()
        };
        let se = s.get_entity(&id).unwrap_or(se);
        list.push(store_entity_to_character(&se, &label));
    }
    Ok(list)
}

/// 指定六角座標上的存活實體。
pub fn get_entities_at_hex(q: i32, r: i32, game_hour: i32) -> anyhow::Result<Vec<Character>> {
    use crate::hex::hex_room_id_from_coord;

    let hex_room = hex_room_id_from_coord(q, r);
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    let ids = s.entity_ids_at_hex(q, r);
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut list = Vec::new();
    for id in ids {
        let Some(se) = s.get_entity(&id) else {
            continue;
        };
        if se.vit <= 0 {
            continue;
        }
        let label = if se.kind == "npc" {
            npc_display::npc_title_in_room_locked(&mut s, &id, &hex_room, game_hour)
        } else {
            String::new()
        };
        let se = s.get_entity(&id).unwrap_or(se);
        list.push(store_entity_to_character(&se, &label));
    }
    Ok(list)
}

/// 與指定正方格座標重合的活體實體。
pub fn get_entities_at_grid(x: i32, y: i32, game_hour: i32) -> anyhow::Result<Vec<Character>> {
    let grid_room = crate::grid::grid_room_id_from_coord(x, y);
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    let ids = s.entity_ids_at_grid(x, y);
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut list = Vec::new();
    for id in ids {
        let Some(se) = s.get_entity(&id) else {
            continue;
        };
        if se.vit <= 0 {
            continue;
        }
        let label = if se.kind == "npc" {
            npc_display::npc_title_in_room_locked(&mut s, &id, &grid_room, game_hour)
        } else {
            String::new()
        };
        let se = s.get_entity(&id).unwrap_or(se);
        list.push(store_entity_to_character(&se, &label));
    }
    Ok(list)
}

/// 依 id 查 soul_seed 並展開為 Personality。
pub fn get_personality_for_entity(entity_id: &str) -> Option<Personality> {
    let arc = store::get_store()?;
    let s = arc.read();
    let e = s.get_entity(entity_id)?;
    let seed = e.soul_seed?;
    Some(expand_soul_seed_to_personality(seed))
}

/// 更新 last_observed_at。
pub fn update_last_observed(id: &str, at: i64) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.update_entity(id, |e| {
        e.last_observed_at = Some(at);
    })
}

/// 清除 last_observed_at。
pub fn clear_last_observed(id: &str) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.update_entity(id, |e| {
        e.last_observed_at = None;
    })
}

/// 更新位置並設為 idle。
pub fn update_position(id: &str, x: i32, y: i32) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.update_entity(id, |e| {
        e.x = x;
        e.y = y;
        e.move_state = "idle".to_string();
        e.target_x = None;
        e.target_y = None;
        e.move_started_at = None;
    })
}

/// 設定移動目標。
pub fn set_move_target(
    id: &str,
    target_x: i32,
    target_y: i32,
    walk_or_run: &str,
    started_at: i64,
) -> anyhow::Result<()> {
    let wor = if walk_or_run.is_empty() { "walk" } else { walk_or_run };
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    let wor = wor.to_string();
    s.update_entity(id, move |e| {
        e.target_x = Some(target_x);
        e.target_y = Some(target_y);
        e.move_state = "moving".to_string();
        e.walk_or_run = wor;
        e.move_started_at = Some(started_at);
    })
}

/// 增減鎂（clamp >= 0）。
pub fn add_magnesium(entity_id: &str, delta: i32) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.update_entity(entity_id, |e| {
        e.magnesium += delta;
        if e.magnesium < 0 {
            e.magnesium = 0;
        }
    })
}

/// 將 `amount` 鎂自 `from_id` 轉至 `to_id`（餘額不足或實體不存在則錯）。
pub fn transfer_magnesium(from_id: &str, to_id: &str, amount: i32) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.transfer_magnesium(from_id, to_id, amount)
}

/// 更新體質（氣血）。
pub fn update_vit(entity_id: &str, new_vit: i32) -> anyhow::Result<()> {
    let v = new_vit.max(0);
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.update_entity(entity_id, move |e| {
        e.vit = v;
    })
}
