//! store 傳聞 — NPC rumors + digest 的讀寫、衰減、排序。
//! 目前純走 JSON 持久化（persist_npc_rumors*），未掛 PG writer queue。

use std::cmp::Ordering;
use std::collections::HashMap;

fn rumor_zone_matches(rumor_zone: &str, rumor_text: &str, query_zone: &str) -> bool {
    let r_zone = rumor_zone.trim();
    let q_zone = query_zone.trim();
    if r_zone.is_empty() || q_zone.is_empty() {
        return false;
    }
    if r_zone == q_zone {
        return true;
    }
    if q_zone == "白塔半島" || r_zone == "白塔半島" {
        return false;
    }
    let region_aliases: &[(&str, &[&str])] = &[
        ("wt-c-wenyao", &["文曜", "三水外坊", "三水口", "老堰頭"]),
        ("wt-c-sacred-tower", &["雲腰雪林", "白塔", "半嶺鎮", "半嶺"]),
        ("wt-n-sacred-mountain", &["神山", "白塔雪巔", "雲腰雪林", "北坡雲杉林", "霜林口", "北嶺腳"]),
        ("wt-n-north-crown-sea", &["北冕海"]),
        ("wt-w-windship-island", &["風舶嶼"]),
        ("wt-w-natural-harbor-channel", &["天然港灣道"]),
        ("wt-w-windship-cape", &["風舶角", "三號坑口", "紅土崁"]),
        ("wt-e-green-ridge", &["青脊山", "青竹坪", "風櫃尾", "烏石鼻"]),
        ("wt-e-east-sea", &["東海"]),
        ("wt-c-southwest-plain", &["大圳頭", "牛埔"]),
        ("wt-s-south-sentinel", &["南哨", "白茫洲", "落日岬"]),
        ("wt-s-black-marsh", &["水尾澤", "黑沼", "水尾寮"]),
        ("wt-s-qinglong-pass", &["青籠關", "下營"]),
    ];
    for (zid, names) in region_aliases {
        if r_zone == *zid {
            return names.iter().any(|n| q_zone == *n || q_zone.starts_with(n));
        }
        if q_zone == *zid {
            return names.iter().any(|n| r_zone == *n || r_zone.starts_with(n));
        }
        if names.iter().any(|n| r_zone == *n) && names.iter().any(|n| q_zone == *n) {
            return true;
        }
    }
    if !r_zone.starts_with("wt-") && !q_zone.starts_with("wt-") && !q_zone.starts_with("grid:") && rumor_text.contains(q_zone) {
        return true;
    }
    false
}

use super::{NpcRumor, NpcRumorDigest, Store};

impl Store {
    pub fn upsert_npc_rumor(&mut self, r: NpcRumor) -> anyhow::Result<()> {
        let id = r.id.trim().to_string();
        if id.is_empty() {
            return Ok(());
        }
        if let Some(old) = self.npc_rumors.get_mut(&id) {
            let text_changed = !r.text.trim().is_empty() && old.text != r.text;
            if !r.text.trim().is_empty() {
                old.text.clone_from(&r.text);
            }
            if !r.room_id.is_empty() {
                old.room_id.clone_from(&r.room_id);
            }
            if !r.zone.is_empty() {
                old.zone.clone_from(&r.zone);
            }
            if !r.source.is_empty() {
                old.source.clone_from(&r.source);
            }
            if r.source_score > 0 {
                old.source_score = r.source_score;
            }
            if text_changed {
                old.weight = r.weight.max(1);
                old.mention_count = 0;
                old.last_used_at = 0;
                old.blocked_until = 0;
            } else {
                old.weight = (old.weight + r.weight).clamp(1, 8);
            }
            if r.updated_at > 0 {
                old.updated_at = r.updated_at;
            }
            if text_changed || r.expires_at > old.expires_at {
                old.expires_at = r.expires_at;
            }
        } else {
            let mut cp = r;
            cp.id.clone_from(&id);
            if cp.weight <= 0 {
                cp.weight = 1;
            }
            if cp.source_score <= 0 {
                cp.source_score = 1;
            }
            self.npc_rumors.insert(id, cp);
        }
        self.persist_npc_rumors()
    }

    pub fn get_npc_rumor_digest(&self) -> Option<&NpcRumorDigest> {
        self.npc_rumor_digest.as_ref()
    }

    /// 依時間移除過期傳聞並衰減權重／可信度（對齊既有 `DecayNpcRumors`）。
    pub fn decay_npc_rumors(&mut self, now_unix: i64) -> anyhow::Result<()> {
        let mut changed = false;
        let deleted_ids: Vec<String> = self
            .npc_rumors
            .iter()
            .filter(|(_, r)| {
                (r.expires_at > 0 && now_unix > r.expires_at)
                    || r.text.contains("npc_")
                    || r.text.chars().any(|c| c.is_ascii_alphabetic())
                    || r.text.contains("城外惡地")
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in &deleted_ids {
            self.npc_rumors.remove(id);
            changed = true;
        }
        for r in self.npc_rumors.values_mut() {
            if r.updated_at > 0 && now_unix - r.updated_at >= 900 && r.weight > 1 {
                r.weight -= 1;
                changed = true;
            }
            if r.last_used_at > 0 && now_unix - r.last_used_at >= 3600 {
                if r.source_score > 1 {
                    r.source_score -= 1;
                    changed = true;
                } else if r.weight > 1 {
                    r.weight -= 1;
                    changed = true;
                }
            }
        }
        if changed {
            self.persist_npc_rumors_with_deletes(&deleted_ids)?;
        }
        Ok(())
    }

    /// 由目前傳聞池 top 項組一句摘要並寫入 digest 檔（對齊既有 `BuildNpcRumorDigest`）。
    pub fn build_npc_rumor_digest(&mut self, now_unix: i64) -> anyhow::Result<()> {
        let top = self.top_npc_rumors("", "", now_unix, 5);
        if top.is_empty() {
            return Ok(());
        }
        let mut parts: Vec<String> = Vec::new();
        for t in top.iter() {
            if parts.len() >= 3 {
                break;
            }
            if t.source == "climate" || t.id.starts_with("weather|") {
                continue;
            }
            let x = t.text.trim();
            if x.is_empty()
                || x.contains("npc_")
                || x.chars().any(|c| c.is_ascii_alphabetic())
                || x.contains("城外惡地")
                || (t.source == "dialogue_anchor" && (x.contains('・') || x.chars().count() < 12))
            {
                continue;
            }
            parts.push(Self::trunc_to_runes(x, 90));
        }
        if parts.is_empty() {
            return Ok(());
        }
        let text = format!("近日鎮上：{}", parts.join("；"));
        self.npc_rumor_digest = Some(NpcRumorDigest {
            text,
            source_count: top.len() as i32,
            updated_at: now_unix,
        });
        self.persist_npc_rumor_digest()
    }

    /// 正規化傳聞文本鍵（對齊既有 `canonicalRumorText`）。
    fn canonical_rumor_text(text: &str) -> String {
        text.trim().to_lowercase()
    }

    fn trunc_to_runes(s: &str, max_runes: usize) -> String {
        let ch: Vec<char> = s.chars().collect();
        if ch.len() <= max_runes {
            return s.to_string();
        }
        let head: String = ch[..max_runes].iter().collect();
        head + "…"
    }

    /// 取指定 room/zone 最相關 topK 傳聞（對齊既有 `TopNpcRumors`）。
    #[must_use]
    pub fn top_npc_rumors(
        &self,
        room_id: &str,
        zone: &str,
        now_unix: i64,
        top_k: i32,
    ) -> Vec<NpcRumor> {
        if top_k <= 0 {
            return Vec::new();
        }
        let top_k = top_k as usize;
        let mut list: Vec<NpcRumor> = self
            .npc_rumors
            .values()
            .filter(|r| !r.text.trim().is_empty())
            .filter(|r| !r.text.contains("npc_") && !r.text.chars().any(|c| c.is_ascii_alphabetic()) && !r.text.contains("城外惡地"))
            .filter(|r| !(r.source == "dialogue_anchor" && (r.text.contains('・') || r.text.chars().count() < 12)))
            .filter(|r| r.last_used_at == 0 || now_unix - r.last_used_at >= 300)
            .filter(|r| {
                if r.source == "climate" || r.id.starts_with("weather|") {
                    !zone.is_empty() && rumor_zone_matches(&r.zone, &r.text, zone)
                } else if !r.zone.is_empty() && !zone.is_empty() {
                    rumor_zone_matches(&r.zone, &r.text, zone)
                } else {
                    true
                }
            })
            .filter(|r| r.expires_at == 0 || now_unix <= r.expires_at)
            .filter(|r| r.blocked_until == 0 || now_unix > r.blocked_until)
            .cloned()
            .map(|mut r| {
                let mut score = r.weight + r.source_score;
                match r.source.as_str() {
                    "job" => score += 3,
                    "room_event" => score += 2,
                    "spawn" => score += 1,
                    "economy" => {}
                    _ => {}
                }
                if !room_id.is_empty() && r.room_id == room_id {
                    score += 5;
                }
                if !zone.is_empty() && rumor_zone_matches(&r.zone, &r.text, zone) {
                    score += 2;
                }
                if r.last_used_at > 0 && now_unix - r.last_used_at < 240 {
                    score -= 4;
                }
                score -= (r.mention_count / 2).min(6);
                r.weight = score;
                r
            })
            .collect();
        list.sort_by(|a, b| match b.weight.cmp(&a.weight) {
            Ordering::Equal => b.updated_at.cmp(&a.updated_at),
            o => o,
        });
        let mut selected: Vec<NpcRumor> = Vec::new();
        let mut used_source: HashMap<String, i32> = HashMap::new();
        let mut used_text: HashMap<String, bool> = HashMap::new();
        let mut rest: Vec<NpcRumor> = Vec::new();
        for it in list {
            let key = Self::canonical_rumor_text(&it.text);
            if key.is_empty() || used_text.get(&key).copied().unwrap_or(false) {
                continue;
            }
            let src = it.source.trim().to_string();
            if !src.is_empty() && used_source.get(&src).copied().unwrap_or(0) >= 1 {
                rest.push(it);
                continue;
            }
            used_text.insert(key, true);
            if !src.is_empty() {
                *used_source.entry(src).or_default() += 1;
            }
            selected.push(it);
            if selected.len() >= top_k {
                return selected;
            }
        }
        for it in rest {
            let key = Self::canonical_rumor_text(&it.text);
            if key.is_empty() || used_text.get(&key).copied().unwrap_or(false) {
                continue;
            }
            used_text.insert(key, true);
            selected.push(it);
            if selected.len() >= top_k {
                break;
            }
        }
        selected
    }

    /// 標記傳聞被引用（對齊既有 `MarkRumorUsedByText`）。
    pub fn mark_rumor_used_by_text(&mut self, text: &str, now_unix: i64) -> anyhow::Result<()> {
        let key = Self::canonical_rumor_text(text);
        if key.is_empty() {
            return Ok(());
        }
        let mut changed = false;
        for r in self.npc_rumors.values_mut() {
            if Self::canonical_rumor_text(&r.text) != key {
                continue;
            }
            r.mention_count += 1;
            r.last_used_at = now_unix;
            if r.mention_count >= 4 && r.weight > 1 {
                r.weight -= 1;
            }
            if r.mention_count % 5 == 0 && r.source_score < 8 {
                r.source_score += 1;
            }
            changed = true;
            break;
        }
        if changed {
            self.persist_npc_rumors()?;
        }
        Ok(())
    }

    /// 衝突降權（對齊既有 `PenalizeRumorByText`）。
    pub fn penalize_rumor_by_text(
        &mut self,
        text: &str,
        now_unix: i64,
        reason: &str,
    ) -> anyhow::Result<()> {
        let key = Self::canonical_rumor_text(text);
        if key.is_empty() {
            return Ok(());
        }
        let mut changed = false;
        for r in self.npc_rumors.values_mut() {
            if Self::canonical_rumor_text(&r.text) != key {
                continue;
            }
            if r.weight > 1 {
                r.weight -= 2;
                if r.weight < 1 {
                    r.weight = 1;
                }
            }
            if r.source_score > 1 {
                r.source_score -= 1;
            }
            r.blocked_until = now_unix + 900;
            r.updated_at = now_unix;
            r.penalty_count += 1;
            r.last_penalty_at = now_unix;
            r.last_penalty_reason = reason.trim().to_string();
            changed = true;
            break;
        }
        if changed {
            self.persist_npc_rumors()?;
        }
        Ok(())
    }
}
