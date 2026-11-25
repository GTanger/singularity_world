#![forbid(unsafe_code)]
//! 《奇點世界》手動修史工具（Manual Chronicler CLI）：
//! 配合「停止背景自動本機 LLM 修史，日常事蹟照常記錄，按需手動修史」架構：
//! - `list [--all] [--out <path>]`：匯出待修史（或全量）5D 事實卡與寫作規範 JSON；
//! - `reset-to-draft`：將非 `draft` 列退回乾淨台灣白話初稿（清除舊模型文稿）；
//! - `apply <authored.json>`：逐筆通過 `validate_polished_deed_prose` 機械防幻覺硬閘門後，原子寫入 PostgreSQL 並推進 `updated_seq` 觸發線上 `singularity.service` 免重啟熱載入與 Markdown 匯出。

use std::env;
use std::fs;

use r2d2_postgres::postgres::{Client, NoTls};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use singularity_world::grid::outer_island::living_chronicler::{
    ChronicleFactPacket, validate_polished_deed_prose,
};
use singularity_world::grid::outer_island::living_chronicler_worker::build_fact_sheet_prompt;

#[derive(Debug, Deserialize, Serialize)]
struct ManualPolishInputItem {
    deed_id: String,
    scope: String,
    target_id: String,
    prose_zh: String,
}

fn resolve_db_url() -> String {
    if let Ok(u) = env::var("DATABASE_URL")
        && !u.trim().is_empty()
    {
        return u;
    }
    if let Ok(txt) = fs::read_to_string(".env") {
        for line in txt.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("DATABASE_URL=") {
                let unquoted = rest.trim().trim_matches('"').trim_matches('\'');
                if !unquoted.is_empty() {
                    return unquoted.to_string();
                }
            }
        }
    }
    "postgres://postgres:singularity@localhost:5432/singularity".to_string()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let cmd = args.first().map_or("list", String::as_str);
    let db_url = resolve_db_url();
    let mut conn = Client::connect(&db_url, NoTls)?;

    match cmd {
        "reset-to-draft" => {
            let updated = conn.execute(
                "UPDATE oi_chronicle_projections
                 SET prose_zh = draft_zh,
                     status = 'draft',
                     lease_until_ms = 0,
                     updated_seq = nextval('oi_chronicle_proj_seq'),
                     updated_at = now()
                 WHERE status <> 'draft' OR prose_zh <> draft_zh",
                &[],
            )?;
            println!(
                "[chronicler-manual] 已將 {updated} 筆紀錄退回乾淨台灣白話初稿（status='draft'）。"
            );
        }
        "list" => {
            let include_all = args.iter().any(|a| a == "--all");
            let out_path = args
                .windows(2)
                .find(|w| w[0] == "--out")
                .map(|w| w[1].clone());
            let sql = if include_all {
                "SELECT deed_id, scope, target_id, kind, title_zh, subtitle_zh, status, draft_zh, fact_json, solar_year, game_day
                 FROM oi_chronicle_projections
                 ORDER BY CASE WHEN scope = 'region' THEN 0 WHEN scope = 'annals' THEN 1 ELSE 2 END,
                          game_day ASC, updated_seq ASC"
            } else {
                "SELECT deed_id, scope, target_id, kind, title_zh, subtitle_zh, status, draft_zh, fact_json, solar_year, game_day
                 FROM oi_chronicle_projections
                 WHERE status = 'draft'
                 ORDER BY CASE WHEN scope = 'region' THEN 0 WHEN scope = 'annals' THEN 1 ELSE 2 END,
                          game_day ASC, updated_seq ASC"
            };
            let rows = conn.query(sql, &[])?;
            let mut items: Vec<Value> = Vec::with_capacity(rows.len());
            for (idx, r) in rows.into_iter().enumerate() {
                let deed_id: String = r.get(0);
                let scope: String = r.get(1);
                let target_id: String = r.get(2);
                let kind: String = r.get(3);
                let title_zh: String = r.get(4);
                let subtitle_zh: String = r.get(5);
                let status: String = r.get(6);
                let draft_zh: String = r.get(7);
                let fact_json: String = r.get(8);
                let solar_year: i16 = r.get(9);
                let game_day: i64 = r.get(10);
                let fact: ChronicleFactPacket =
                    serde_json::from_str(&fact_json).unwrap_or_default();
                let prompt =
                    build_fact_sheet_prompt(&scope, &target_id, &title_zh, &fact, &draft_zh);
                items.push(json!({
                    "index": idx,
                    "deed_id": deed_id,
                    "scope": scope,
                    "target_id": target_id,
                    "kind": kind,
                    "title_zh": title_zh,
                    "subtitle_zh": subtitle_zh,
                    "status": status,
                    "solar_year": solar_year,
                    "game_day": game_day,
                    "draft_zh": draft_zh,
                    "fact": fact,
                    "prompt": prompt,
                }));
            }
            let rendered = serde_json::to_string_pretty(&items)?;
            if let Some(p) = out_path {
                fs::write(&p, &rendered)?;
                println!(
                    "[chronicler-manual] 已匯出 {} 筆待修史條目至 {p}",
                    items.len()
                );
            } else {
                println!("{rendered}");
            }
        }
        "apply" => {
            let Some(in_path) = args.get(1) else {
                return Err("用法: chronicler-manual apply <authored.json>".into());
            };
            let raw = fs::read_to_string(in_path)?;
            let inputs: Vec<ManualPolishInputItem> = serde_json::from_str(&raw)?;
            let mut tx = conn.transaction()?;
            let mut errors = Vec::new();
            let mut validated = Vec::with_capacity(inputs.len());

            for (idx, item) in inputs.iter().enumerate() {
                let row_opt = tx.query_opt(
                    "SELECT draft_zh, fact_json FROM oi_chronicle_projections
                     WHERE deed_id = $1 AND scope = $2 AND target_id = $3",
                    &[&item.deed_id, &item.scope, &item.target_id],
                )?;
                let Some(r) = row_opt else {
                    errors.push(format!(
                        "#{idx} ({}/{}/{}) 資料庫查無對應投射列",
                        item.deed_id, item.scope, item.target_id
                    ));
                    continue;
                };
                let draft_zh: String = r.get(0);
                let fact_json: String = r.get(1);
                let fact: ChronicleFactPacket =
                    serde_json::from_str(&fact_json).unwrap_or_default();
                match validate_polished_deed_prose(&fact, &item.scope, &draft_zh, &item.prose_zh) {
                    Ok(clean) => validated.push((item, clean)),
                    Err(reason) => errors.push(format!(
                        "#{idx} ({}/{}/{}) 未過機械閘門: {reason}",
                        item.deed_id, item.scope, item.target_id
                    )),
                }
            }

            if !errors.is_empty() {
                let joined = errors.join("\n  - ");
                return Err(format!(
                    "共 {} 筆未通過防幻覺硬閘門，全批中止未寫入：\n  - {joined}",
                    errors.len()
                )
                .into());
            }

            for (item, clean_prose) in &validated {
                tx.execute(
                    "UPDATE oi_chronicle_projections
                     SET prose_zh = $4,
                         status = 'polished',
                         lease_until_ms = 0,
                         updated_seq = nextval('oi_chronicle_proj_seq'),
                         updated_at = now()
                     WHERE deed_id = $1 AND scope = $2 AND target_id = $3",
                    &[&item.deed_id, &item.scope, &item.target_id, clean_prose],
                )?;
            }
            tx.commit()?;
            println!(
                "[chronicler-manual] 全部 {} 筆史傳已通過機械防幻覺閘門並寫入 PostgreSQL（線上服務將於 6 秒內自動熱載入並匯出 Markdown）！",
                validated.len()
            );
        }
        other => {
            return Err(
                format!("未知子指令: {other}（支援 list | reset-to-draft | apply）").into(),
            );
        }
    }
    Ok(())
}
