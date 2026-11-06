//! 求職撮合（對齊既有 `npc/job_matching`）。

use std::collections::HashMap;

use rand::Rng;

use crate::db::{
    RoomGraph, get_all_venue_ids, get_assignment_count_by_venue, get_assignments_for_entity,
    get_entity, get_entity_room, get_first_occupation_id_for_venue, get_npc_ids_with_room,
    get_room_ids_for_venue, get_spawn_room_id, get_venue_max_staff, insert_assignment,
    insert_schedule,
};

/// 鎂閾值預設（config 為 0 時使用）。
pub const SEEK_JOB_MG_THRESHOLD_DEFAULT: i32 = 50;
/// 鎂≥閾值且開啟穩定撮合時，參與機率。
pub const JOB_MATCH_STABLE_PROBABILITY: f32 = 0.3;
/// 無既有職業時寫入的預設職業 id。
pub const DEFAULT_OCCUPATION_FOR_HIRE: &str = "服務生";
const DEFAULT_SHIFT_START: i32 = 6;
const DEFAULT_SHIFT_END: i32 = 19;

/// 撮合參數（對齊既有 `JobMatchParams`）。
#[derive(Debug, Clone)]
pub struct JobMatchParams {
    pub max_per_venue: i32,
    pub mg_threshold: i32,
    pub job_match_when_stable: bool,
}

/// 房間候選資訊（用於職能媒合與最佳工作房間評分）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoomCandidate {
    pub name: String,
    pub tags: Vec<String>,
}

fn strip_generic_role_affixes(s: &str) -> String {
    let mut out = s.trim().to_string();
    for affix in [
        "當值", "首席", "駐場", "匠師", "行首", "主事", "掌事", "師傅", "執事", "學徒", "工坊",
        "商鋪", "店鋪",
    ] {
        out = out.replace(affix, "");
    }
    out
}

fn shares_two_char_substring(a: &str, b: &str) -> bool {
    let clean_a = strip_generic_role_affixes(a);
    let clean_b = strip_generic_role_affixes(b);
    let chars_a: Vec<char> = clean_a.chars().collect();
    if chars_a.len() < 2 {
        return false;
    }
    for w in chars_a.windows(2) {
        let sub: String = w.iter().collect();
        if clean_b.contains(&sub) {
            return true;
        }
    }
    false
}

/// 依職業關鍵字匹配場域空間關鍵詞
#[must_use]
pub fn occupation_keywords(occupation: &str) -> Vec<&'static str> {
    let occ = occupation.trim();
    let mut kws = Vec::new();
    if occ.contains("磨") || occ.contains("石磨") || occ.contains("水輪") {
        kws.extend_from_slice(&["石磨", "磨坊", "水輪", "老堰頭"]);
    }
    if occ.contains("織")
        || occ.contains("染")
        || occ.contains("繡")
        || occ.contains("服")
        || occ.contains("布")
    {
        kws.extend_from_slice(&["織", "染坊", "雲錦", "服飾"]);
    }
    if occ.contains("茶") || occ.contains("旅") || occ.contains("宿") {
        kws.extend_from_slice(&["茶屋", "茶坊", "旅社", "歇宿"]);
    }
    if occ.contains("浴") || occ.contains("湯") || occ.contains("泉") {
        kws.extend_from_slice(&["浴場", "溫水", "湯池", "泉香"]);
    }
    if occ.contains("車") || occ.contains("聯運") || occ.contains("腳夫") || occ.contains("貨運") {
        kws.extend_from_slice(&["聯運", "車馬", "貨運", "倉", "碼頭"]);
    }
    if occ.contains("船") || occ.contains("渡") || occ.contains("港") || occ.contains("海") {
        kws.extend_from_slice(&["碼頭", "渡口", "船塢", "港", "泊位"]);
    }
    if occ.contains("牧") || occ.contains("牛") || occ.contains("乳酪") {
        kws.extend_from_slice(&["牛埔", "牧", "乳酪", "草場"]);
    }
    if occ.contains("窯") || occ.contains("陶") || occ.contains("磚") {
        kws.extend_from_slice(&["窯", "紅土崁", "陶"]);
    }
    if occ.contains("算") || occ.contains("契") || occ.contains("公議") || occ.contains("執事") {
        kws.extend_from_slice(&["公議", "契", "商行", "算"]);
    }
    if occ.contains("武") || occ.contains("巡") || occ.contains("哨") || occ.contains("衛") {
        kws.extend_from_slice(&["習武", "試招", "巡防", "哨", "門"]);
    }
    if occ.contains("鐵") || occ.contains("鍛") || occ.contains("鑄") {
        kws.extend_from_slice(&["鐵匠鋪", "鍛造", "爐"]);
    } else if occ.contains("匠") && kws.is_empty() {
        kws.extend_from_slice(&["鐵匠鋪", "鍛造", "工坊", "爐"]);
    }
    if occ.contains("醫") || occ.contains("藥") || occ.contains("診") {
        kws.extend_from_slice(&["醫館", "百草", "診所", "藥局", "藥材"]);
    }
    if occ.contains("食") || occ.contains("廚") || occ.contains("伙") || occ.contains("飯") {
        kws.extend_from_slice(&["大食堂", "食堂", "廚房", "食肆"]);
    }
    if occ.contains("商")
        || occ.contains("販")
        || occ.contains("店")
        || occ.contains("買")
        || occ.contains("賣")
        || occ.contains("掌事")
    {
        kws.extend_from_slice(&["商鋪", "市集", "商行", "店鋪"]);
    }
    if occ.contains("學") || occ.contains("書") || occ.contains("講") || occ.contains("生") {
        kws.extend_from_slice(&["書院", "講堂", "學堂", "蒙學"]);
    }
    if occ.contains("服務") || occ.contains("侍") {
        kws.extend_from_slice(&["茶屋", "食堂", "旅社"]);
    }

    let mut out = Vec::new();
    for kw in kws {
        if !out.contains(&kw) {
            out.push(kw);
        }
    }
    out
}

/// 依職業提取對應之標準英文房間分類標籤
#[must_use]
pub fn occupation_room_tags(occupation: &str) -> Vec<&'static str> {
    let occ = occupation.trim();
    let mut tags = Vec::new();
    if occ.contains("磨") || occ.contains("石磨") || occ.contains("水輪") {
        tags.extend_from_slice(&["grindstone", "workshop"]);
    }
    if occ.contains("織")
        || occ.contains("染")
        || occ.contains("繡")
        || occ.contains("服")
        || occ.contains("布")
    {
        tags.extend_from_slice(&["textile", "fabric", "workshop"]);
    }
    if occ.contains("茶") || occ.contains("旅") || occ.contains("宿") {
        tags.extend_from_slice(&["teahouse", "inn"]);
    }
    if occ.contains("浴") || occ.contains("湯") || occ.contains("泉") {
        tags.extend_from_slice(&["bathhouse"]);
    }
    if occ.contains("車") || occ.contains("聯運") || occ.contains("腳夫") || occ.contains("貨運") {
        tags.extend_from_slice(&["courier", "dock"]);
    }
    if occ.contains("船") || occ.contains("渡") || occ.contains("港") || occ.contains("海") {
        tags.extend_from_slice(&["dock", "fishing_port", "shipyard"]);
    }
    if occ.contains("牧") || occ.contains("牛") || occ.contains("乳酪") {
        tags.extend_from_slice(&["pasture", "ranch"]);
    }
    if occ.contains("窯") || occ.contains("陶") || occ.contains("磚") {
        tags.extend_from_slice(&["kiln", "workshop"]);
    }
    if occ.contains("算") || occ.contains("契") || occ.contains("公議") || occ.contains("執事") {
        tags.extend_from_slice(&["council_hall", "bank", "shop"]);
    }
    if occ.contains("武") || occ.contains("巡") || occ.contains("哨") || occ.contains("衛") {
        tags.extend_from_slice(&["guard_post", "gate"]);
    }
    if occ.contains("鐵") || occ.contains("鍛") || occ.contains("鑄") {
        tags.extend_from_slice(&["blacksmith", "forge", "workshop"]);
    } else if occ.contains("匠") && tags.is_empty() {
        tags.extend_from_slice(&["blacksmith", "forge", "workshop"]);
    }
    if occ.contains("醫") || occ.contains("藥") || occ.contains("診") {
        tags.extend_from_slice(&["clinic", "herb", "medicine"]);
    }
    if occ.contains("食") || occ.contains("廚") || occ.contains("伙") || occ.contains("飯") {
        tags.extend_from_slice(&["canteen", "hot_food", "fresh_drink"]);
    }
    if occ.contains("商")
        || occ.contains("販")
        || occ.contains("店")
        || occ.contains("買")
        || occ.contains("賣")
        || occ.contains("掌事")
    {
        tags.extend_from_slice(&["shop", "market", "bank"]);
    }
    if occ.contains("學") || occ.contains("書") || occ.contains("講") || occ.contains("生") {
        tags.extend_from_slice(&["academy", "library"]);
    }
    if occ.contains("服務") || occ.contains("侍") {
        tags.extend_from_slice(&["teahouse", "canteen", "inn"]);
    }
    tags
}

/// 判斷場所房間是否契合特定職業
#[must_use]
pub fn room_matches_occupation(occupation: &str, room_name: &str, tags: &[String]) -> bool {
    let occ = occupation.trim();
    let keywords = occupation_keywords(occ);
    let expected_tags = occupation_room_tags(occ);

    for et in &expected_tags {
        if tags.iter().any(|t| t == et || crate::grid::canonical_room_tag(t) == *et) {
            return true;
        }
    }

    for kw in &keywords {
        if room_name.contains(kw) {
            return true;
        }
        for tag in tags {
            if tag == kw || tag.contains(kw) || kw.contains(tag.as_str()) {
                return true;
            }
        }
    }
    if !occ.is_empty() {
        if room_name.contains(occ) || occ.contains(room_name) {
            return true;
        }
        for tag in tags {
            if tag == occ || tag.contains(occ) || occ.contains(tag.as_str()) {
                return true;
            }
        }
    }

    if let Some(venue) = crate::grid::resolve_any_room_venue_object(room_name, "") {
        let venue_tags = venue.semantic_tags();
        for et in &expected_tags {
            if venue_tags.contains(et) {
                return true;
            }
        }
        for kw in &keywords {
            if venue.venue_kind.contains(kw)
                || kw.contains(venue.venue_kind)
                || venue.default_host_title.contains(kw)
                || kw.contains(venue.default_host_title)
                || venue_tags.iter().any(|vt| vt == kw || vt.contains(kw) || kw.contains(vt))
            {
                return true;
            }
        }
        if shares_two_char_substring(occ, venue.default_host_title)
            || shares_two_char_substring(occ, venue.venue_kind)
        {
            return true;
        }
    }

    false
}

/// 尋找最適合從事該職業之工作房間
#[must_use]
pub fn find_best_work_room(
    occupation: &str,
    home_room: &str,
    candidates: &[RoomCandidate],
) -> String {
    let mut best_score = -1;
    let mut best_room = home_room.to_string();
    let now_unix = crate::game::now_unix();
    let keywords = occupation_keywords(occupation);
    for r in candidates {
        if room_matches_occupation(occupation, &r.name, &r.tags) {
            let mut score = 1;
            if r.name == home_room {
                score += 1;
            }
            if keywords.iter().any(|kw| r.name.contains(kw)) {
                score += 4;
            }
            if let Some(venue) = crate::grid::resolve_any_room_venue_object(&r.name, occupation) {
                score += 2;
                if venue.default_host_title == occupation.trim() {
                    score += 6;
                } else if shares_two_char_substring(occupation, venue.default_host_title)
                    || shares_two_char_substring(occupation, venue.venue_kind)
                {
                    score += 4;
                }
            }
            if crate::game::peninsula_scripts::active_event_and_script_pack_for_room(
                &r.name,
                None,
                now_unix,
            )
            .is_some()
            {
                score += 2;
            }
            if score > best_score {
                best_score = score;
                best_room = r.name.clone();
            }
        }
    }

    if best_score < 0 && !home_room.is_empty() {
        return home_room.to_string();
    }
    best_room
}

/// Fisher–Yates 洗牌。
fn shuffle_vec<T>(v: &mut [T], rng: &mut impl Rng) {
    for i in (1..v.len()).rev() {
        let j = rng.random_range(0..=i);
        v.swap(i, j);
    }
}

fn do_assign(entity_id: &str, venue_id: &str, spawn_room: &str) -> bool {
    let mut occupation = crate::pg::pool()
        .and_then(|pool| pool.get().ok())
        .and_then(|mut conn| {
            crate::pg::world_economy::occupation_for_venue(&mut *conn, venue_id)
                .ok()
                .flatten()
        })
        .unwrap_or_default();
    if occupation.is_empty() {
        occupation = get_first_occupation_id_for_venue(venue_id);
    }
    if occupation.is_empty() {
        occupation = DEFAULT_OCCUPATION_FOR_HIRE.to_string();
    }
    if insert_assignment(entity_id, &occupation, venue_id, "job_matching").is_err() {
        return false;
    }
    let room_ids = get_room_ids_for_venue(venue_id)
        .ok()
        .flatten()
        .unwrap_or_default();
    let work_room = if room_ids.len() > 1 {
        let candidates: Vec<RoomCandidate> = room_ids
            .iter()
            .map(|r_id| RoomCandidate {
                name: r_id.clone(),
                tags: Vec::new(),
            })
            .collect();
        find_best_work_room(&occupation, &room_ids[0], &candidates)
    } else {
        room_ids.first().cloned().unwrap_or_default()
    };
    let rest_room = get_entity_room(entity_id)
        .ok()
        .filter(|room_id| !room_id.is_empty())
        .unwrap_or_else(|| spawn_room.to_string());
    if !work_room.is_empty() && !rest_room.is_empty() {
        let _ = insert_schedule(
            entity_id,
            &work_room,
            &rest_room,
            DEFAULT_SHIFT_START,
            DEFAULT_SHIFT_END,
        );
    }
    true
}

/// 一輪求職撮合；`graph` 為 `None` 時改隨機配對（與既有行為一致）。回傳本輪新入職 entity id。
#[must_use]
pub fn run_job_matching_tick(graph: Option<&RoomGraph>, mut p: JobMatchParams) -> Vec<String> {
    let mut added = Vec::new();
    if p.max_per_venue <= 0 {
        p.max_per_venue = 2;
    }
    let mut threshold = p.mg_threshold;
    if threshold <= 0 {
        threshold = SEEK_JOB_MG_THRESHOLD_DEFAULT;
    }

    let ids = get_npc_ids_with_room();
    if ids.is_empty() {
        return added;
    }

    let mut rng = rand::rng();
    let mut unassigned: Vec<String> = Vec::new();
    for entity_id in ids {
        let Ok(list) = get_assignments_for_entity(&entity_id) else {
            continue;
        };
        if !list.is_empty() {
            continue;
        }
        let Ok(Some(ch)) = get_entity(&entity_id) else {
            continue;
        };
        if ch.magnesium < threshold
            || (p.job_match_when_stable && rng.random_bool(f64::from(JOB_MATCH_STABLE_PROBABILITY)))
        {
            unassigned.push(entity_id);
        }
    }
    if unassigned.is_empty() {
        return added;
    }

    let Ok(venue_ids) = get_all_venue_ids() else {
        return added;
    };
    if venue_ids.is_empty() {
        return added;
    }

    let mut slots: Vec<String> = Vec::new();
    for vid in venue_ids {
        let Ok(n) = get_assignment_count_by_venue(&vid) else {
            continue;
        };
        let limit = get_venue_max_staff(&vid, p.max_per_venue);
        for _ in 0..limit.saturating_sub(n as i32) {
            slots.push(vid.clone());
        }
    }
    if slots.is_empty() {
        return added;
    }

    let spawn_room = get_spawn_room_id();
    let place_graph = crate::store::get_store().map(|store| store.read().place_graph_snapshot());
    let mut distance_cache = HashMap::<(String, String), i32>::new();

    if let Some(g) = graph {
        shuffle_vec(&mut slots, &mut rng);
        let mut un = unassigned;
        for venue_id in slots {
            let room_ids = get_room_ids_for_venue(&venue_id)
                .ok()
                .flatten()
                .unwrap_or_default();
            if room_ids.is_empty() {
                continue;
            }
            let venue_first = room_ids[0].clone();
            let mut best_idx: Option<usize> = None;
            let mut best_dist = 9999i32;
            for (i, entity_id) in un.iter().enumerate() {
                let Ok(current) = get_entity_room(entity_id) else {
                    continue;
                };
                if current.is_empty() {
                    continue;
                }
                let key = (current.clone(), venue_first.clone());
                let dist = *distance_cache.entry(key).or_insert_with(|| {
                    if current == venue_first {
                        return 0;
                    }
                    if let Some(path) = place_graph
                        .as_ref()
                        .and_then(|graph| graph.find_path(&current, &venue_first))
                    {
                        return i32::try_from(path.len().saturating_sub(1)).unwrap_or(i32::MAX);
                    }
                    g.find_path(&current, &venue_first)
                        .and_then(|path| i32::try_from(path.len()).ok())
                        .unwrap_or(9999)
                });
                if dist < best_dist {
                    best_dist = dist;
                    best_idx = Some(i);
                }
            }
            let Some(idx) = best_idx else {
                continue;
            };
            let entity_id = un.remove(idx);
            if do_assign(&entity_id, &venue_id, &spawn_room) {
                added.push(entity_id);
            }
            if un.is_empty() {
                break;
            }
        }
    } else {
        shuffle_vec(&mut unassigned, &mut rng);
        shuffle_vec(&mut slots, &mut rng);
        let n = unassigned.len().min(slots.len());
        for i in 0..n {
            if do_assign(&unassigned[i], &slots[i], &spawn_room) {
                added.push(unassigned[i].clone());
            }
        }
    }

    added
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_matches_occupation_peninsula_vocations() {
        assert!(room_matches_occupation(
            "當值石磨匠師",
            "老堰頭・水輪石磨工坊",
            &[]
        ));
        assert!(room_matches_occupation(
            "湯池掌事",
            "泉香溫水浴場",
            &[]
        ));
    }

    #[test]
    fn test_find_best_work_room() {
        let candidates = vec![
            RoomCandidate {
                name: "老堰頭・水輪石磨工坊".to_string(),
                tags: vec!["工坊".to_string()],
            },
            RoomCandidate {
                name: "泉香溫水浴場".to_string(),
                tags: vec!["浴場".to_string()],
            },
            RoomCandidate {
                name: "無關林道".to_string(),
                tags: vec![],
            },
        ];
        let chosen_mill = find_best_work_room("當值石磨匠師", "無關林道", &candidates);
        assert_eq!(chosen_mill, "老堰頭・水輪石磨工坊");

        let chosen_bath = find_best_work_room("湯池掌事", "無關林道", &candidates);
        assert_eq!(chosen_bath, "泉香溫水浴場");

        let fallback = find_best_work_room("未知流浪者", "無關林道", &candidates);
        assert_eq!(fallback, "無關林道");
    }
}
