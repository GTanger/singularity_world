//! 個人探索迷霧稀疏點陣圖（`32×32` 分片 ＋ `zlib` 壓縮落盤）。
//!
//! 凍結全服共用 `cell_overlay.explored_day`，改將每位玩家的足跡與目及視野保存於獨立表
//! `player_exploration_fog`，既支援 307 萬格超低記憶體儲存，亦確保重啟不丟失足跡。

use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{LazyLock, OnceLock, RwLock};

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

use super::cache::{OVERLAY_CHUNK_SIDE, chunk_key};
use crate::store::sql::DbPool;

const MAGIC: &[u8; 4] = b"SWFG";
const VERSION: u8 = 1;
const CHUNK_BYTES: usize = (OVERLAY_CHUNK_SIDE as usize * OVERLAY_CHUNK_SIDE as usize) / 8; // 128 bytes

const CREATE_FOG_TABLE: &str = "
CREATE TABLE IF NOT EXISTS player_exploration_fog (
    player_id TEXT PRIMARY KEY,
    payload BYTEA NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
)";

static SCHEMA_READY: OnceLock<()> = OnceLock::new();

/// 單一玩家的稀疏 `32×32` 分片迷霧點陣圖。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerFogBitmap {
    /// `(chunk_x, chunk_y) -> [u8; 128]`（已實際踏足）
    visited: HashMap<(i32, i32), [u8; CHUNK_BYTES]>,
    /// `(chunk_x, chunk_y) -> [u8; 128]`（已目及）
    seen: HashMap<(i32, i32), [u8; CHUNK_BYTES]>,
}

impl PlayerFogBitmap {
    #[inline]
    fn bit_pos(gx: i32, gy: i32) -> ((i32, i32), usize, u8) {
        let ck = chunk_key(gx, gy);
        let lx = gx.rem_euclid(OVERLAY_CHUNK_SIDE) as usize;
        let ly = gy.rem_euclid(OVERLAY_CHUNK_SIDE) as usize;
        let idx = ly * OVERLAY_CHUNK_SIDE as usize + lx;
        (ck, idx / 8, 1u8 << (idx % 8))
    }

    fn set_bit(map: &mut HashMap<(i32, i32), [u8; CHUNK_BYTES]>, gx: i32, gy: i32) -> bool {
        let (ck, byte_idx, mask) = Self::bit_pos(gx, gy);
        let arr = map.entry(ck).or_insert([0u8; CHUNK_BYTES]);
        if (arr[byte_idx] & mask) != 0 {
            false
        } else {
            arr[byte_idx] |= mask;
            true
        }
    }

    fn get_bit(map: &HashMap<(i32, i32), [u8; CHUNK_BYTES]>, gx: i32, gy: i32) -> bool {
        let (ck, byte_idx, mask) = Self::bit_pos(gx, gy);
        map.get(&ck).is_some_and(|arr| (arr[byte_idx] & mask) != 0)
    }

    /// 標記 `(gx, gy)` 為已踏足，並將半徑 `seen_radius` 內格標記為已目及；若有任何新位元改變回傳 `true`。
    pub fn mark_visited_and_seen(&mut self, gx: i32, gy: i32, seen_radius: i32) -> bool {
        let mut changed = Self::set_bit(&mut self.visited, gx, gy);
        changed |= Self::set_bit(&mut self.seen, gx, gy);
        for dx in -seen_radius..=seen_radius {
            for dy in -seen_radius..=seen_radius {
                changed |= Self::set_bit(&mut self.seen, gx + dx, gy + dy);
            }
        }
        changed
    }

    pub fn is_visited(&self, gx: i32, gy: i32) -> bool {
        Self::get_bit(&self.visited, gx, gy)
    }

    pub fn is_seen(&self, gx: i32, gy: i32) -> bool {
        Self::get_bit(&self.seen, gx, gy) || Self::get_bit(&self.visited, gx, gy)
    }

    /// 列出全部已踏足或已目及座標（最多回傳 `limit` 筆防封包過大）。
    pub fn known_coords(&self, limit: usize) -> Vec<(i32, i32, bool)> {
        let mut out = Vec::new();
        for (&(cx, cy), seen_bits) in &self.seen {
            let visited_bits = self.visited.get(&(cx, cy));
            let base_x = cx * OVERLAY_CHUNK_SIDE;
            let base_y = cy * OVERLAY_CHUNK_SIDE;
            for (byte_idx, &b) in seen_bits.iter().enumerate() {
                if b == 0 {
                    continue;
                }
                let vb = visited_bits.map_or(0, |v| v[byte_idx]);
                for bit in 0..8usize {
                    if (b & (1u8 << bit)) != 0 {
                        let idx = byte_idx * 8 + bit;
                        let lx = (idx % OVERLAY_CHUNK_SIDE as usize) as i32;
                        let ly = (idx / OVERLAY_CHUNK_SIDE as usize) as i32;
                        let is_v = (vb & (1u8 << bit)) != 0;
                        out.push((base_x + lx, base_y + ly, is_v));
                        if out.len() >= limit {
                            return out;
                        }
                    }
                }
            }
        }
        out
    }

    /// 編碼為 `zlib` 壓縮位元組。
    pub fn encode_zlib(&self) -> Result<Vec<u8>, String> {
        let mut raw = Vec::new();
        raw.extend_from_slice(MAGIC);
        raw.push(VERSION);
        let write_map = |buf: &mut Vec<u8>, m: &HashMap<(i32, i32), [u8; CHUNK_BYTES]>| {
            let mut entries: Vec<_> = m.iter().collect();
            entries.sort_unstable_by_key(|(k, _)| **k);
            buf.extend_from_slice(&(entries.len() as u32).to_le_bytes());
            for (&(cx, cy), bits) in entries {
                buf.extend_from_slice(&cx.to_le_bytes());
                buf.extend_from_slice(&cy.to_le_bytes());
                buf.extend_from_slice(bits);
            }
        };
        write_map(&mut raw, &self.visited);
        write_map(&mut raw, &self.seen);

        let mut enc = ZlibEncoder::new(Vec::new(), Compression::fast());
        enc.write_all(&raw).map_err(|e| e.to_string())?;
        enc.finish().map_err(|e| e.to_string())
    }

    /// 由 `zlib` 壓縮位元組還原。
    pub fn decode_zlib(bytes: &[u8]) -> Result<Self, String> {
        let mut dec = ZlibDecoder::new(bytes);
        let mut raw = Vec::new();
        dec.read_to_end(&mut raw).map_err(|e| e.to_string())?;
        if raw.len() < 5 || &raw[0..4] != MAGIC || raw[4] != VERSION {
            return Err("個人迷霧格式或版本不符".to_string());
        }
        let mut pos = 5usize;
        let read_map =
            |buf: &[u8], p: &mut usize| -> Result<HashMap<(i32, i32), [u8; CHUNK_BYTES]>, String> {
                if *p + 4 > buf.len() {
                    return Err("個人迷霧截斷".to_string());
                }
                let count = u32::from_le_bytes(
                    buf[*p..*p + 4]
                        .try_into()
                        .map_err(|_| "count parse".to_string())?,
                ) as usize;
                *p += 4;
                let mut map = HashMap::with_capacity(count.min(4096));
                for _ in 0..count {
                    if *p + 8 + CHUNK_BYTES > buf.len() {
                        return Err("個人迷霧分片截斷".to_string());
                    }
                    let cx = i32::from_le_bytes(
                        buf[*p..*p + 4]
                            .try_into()
                            .map_err(|_| "cx parse".to_string())?,
                    );
                    let cy = i32::from_le_bytes(
                        buf[*p + 4..*p + 8]
                            .try_into()
                            .map_err(|_| "cy parse".to_string())?,
                    );
                    *p += 8;
                    let mut bits = [0u8; CHUNK_BYTES];
                    bits.copy_from_slice(&buf[*p..*p + CHUNK_BYTES]);
                    *p += CHUNK_BYTES;
                    map.insert((cx, cy), bits);
                }
                Ok(map)
            };
        let visited = read_map(&raw, &mut pos)?;
        let seen = read_map(&raw, &mut pos)?;
        Ok(Self { visited, seen })
    }
}

static PLAYER_FOG_CACHE: LazyLock<RwLock<HashMap<String, PlayerFogBitmap>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 確保 `player_exploration_fog` 表已存在（冪等）。
pub fn ensure_schema(pool: &DbPool) -> Result<(), String> {
    if SCHEMA_READY.get().is_some() {
        return Ok(());
    }
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    conn.batch_execute(CREATE_FOG_TABLE)
        .map_err(|e| e.to_string())?;
    let _ = SCHEMA_READY.set(());
    Ok(())
}

fn load_from_pg(pool: &DbPool, player_id: &str) -> PlayerFogBitmap {
    if ensure_schema(pool).is_err() {
        return PlayerFogBitmap::default();
    }
    let Ok(mut conn) = pool.get() else {
        return PlayerFogBitmap::default();
    };
    let Ok(Some(row)) = conn.query_opt(
        "SELECT payload FROM player_exploration_fog WHERE player_id = $1",
        &[&player_id],
    ) else {
        return PlayerFogBitmap::default();
    };
    let bytes: Vec<u8> = row.get(0);
    PlayerFogBitmap::decode_zlib(&bytes).unwrap_or_default()
}

fn save_to_pg(pool: &DbPool, player_id: &str, fog: &PlayerFogBitmap) {
    if ensure_schema(pool).is_err() {
        return;
    }
    let Ok(payload) = fog.encode_zlib() else {
        return;
    };
    let Ok(mut conn) = pool.get() else {
        return;
    };
    let _ = conn.execute(
        "INSERT INTO player_exploration_fog (player_id, payload, updated_at)
         VALUES ($1, $2, now())
         ON CONFLICT (player_id) DO UPDATE SET payload = EXCLUDED.payload, updated_at = now()",
        &[&player_id, &payload],
    );
}

fn ensure_loaded(pool: Option<&DbPool>, player_id: &str) {
    if let Ok(guard) = PLAYER_FOG_CACHE.read()
        && guard.contains_key(player_id)
    {
        return;
    }
    let loaded = pool.map(|p| load_from_pg(p, player_id)).unwrap_or_default();
    if let Ok(mut guard) = PLAYER_FOG_CACHE.write() {
        guard.entry(player_id.to_string()).or_insert(loaded);
    }
}

/// 標記玩家踏足 `(gx, gy)` 與目及半徑 `seen_radius`，有新位元時同步壓縮落盤。
pub fn mark_player_explored(
    pool: Option<&DbPool>,
    player_id: &str,
    gx: i32,
    gy: i32,
    seen_radius: i32,
) {
    ensure_loaded(pool, player_id);
    let snapshot = if let Ok(mut guard) = PLAYER_FOG_CACHE.write() {
        let entry = guard.entry(player_id.to_string()).or_default();
        if entry.mark_visited_and_seen(gx, gy, seen_radius) {
            Some(entry.clone())
        } else {
            None
        }
    } else {
        None
    };
    if let (Some(p), Some(snap)) = (pool, snapshot) {
        save_to_pg(p, player_id, &snap);
    }
}

/// 查詢玩家是否已目及（或踏足）指定座標。
pub fn is_player_seen(pool: Option<&DbPool>, player_id: &str, gx: i32, gy: i32) -> bool {
    ensure_loaded(pool, player_id);
    PLAYER_FOG_CACHE
        .read()
        .ok()
        .and_then(|g| g.get(player_id).map(|f| f.is_seen(gx, gy)))
        .unwrap_or(false)
}

/// 取得玩家已踏足／目及座標清單（上限 `limit` 筆）。
pub fn player_known_coords(
    pool: Option<&DbPool>,
    player_id: &str,
    limit: usize,
) -> Vec<(i32, i32, bool)> {
    ensure_loaded(pool, player_id);
    PLAYER_FOG_CACHE
        .read()
        .ok()
        .and_then(|g| g.get(player_id).map(|f| f.known_coords(limit)))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fog_bitmap_roundtrip_and_negative_coords() {
        let mut fog = PlayerFogBitmap::default();
        assert!(fog.mark_visited_and_seen(0, 0, 2));
        assert!(fog.mark_visited_and_seen(-35, 17, 1));
        assert!(!fog.mark_visited_and_seen(0, 0, 2));

        assert!(fog.is_visited(0, 0));
        assert!(fog.is_seen(2, -2));
        assert!(!fog.is_visited(2, -2));
        assert!(fog.is_visited(-35, 17));
        assert!(fog.is_seen(-36, 18));
        assert!(!fog.is_seen(100, 100));

        let bytes = fog.encode_zlib().expect("zlib encode");
        assert!(bytes.len() < 120, "壓縮體積應極小，實際={}", bytes.len());
        let decoded = PlayerFogBitmap::decode_zlib(&bytes).expect("zlib decode");
        assert_eq!(fog, decoded);
        assert!(decoded.is_visited(-35, 17));
        assert!(decoded.is_seen(-36, 18));
    }
}
