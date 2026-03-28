use anyhow::Context;
use r2d2_postgres::postgres::{Client, NoTls};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Deserialize)]
struct WordPool {
    elements: Vec<Element>,
}

#[derive(Deserialize)]
struct Element {
    id: String,
    char: String,
    semantic: String,
    desc: String,
    #[serde(default)]
    embedding_desc: Option<String>,
}

#[derive(Serialize)]
struct EmbedReq {
    model: String,
    input: String,
}

#[derive(Deserialize)]
struct EmbedResp {
    embeddings: Vec<Vec<f64>>,
}

fn main() -> anyhow::Result<()> {
    // 1. 讀取詞元池
    let raw = fs::read_to_string("data/config/word_elements.json")
        .context("Failed to read data/config/word_elements.json")?;
    let pool: WordPool = serde_json::from_str(&raw).context("Failed to parse word_elements.json")?;

    // 2. 連接 PG（DATABASE_URL 環境變數或預設）
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:singularity@localhost:5432/singularity".to_string());
    let mut conn = Client::connect(&db_url, NoTls).context("Failed to connect to database")?;

    // 3. 確保表存在 (雖然 sql.rs 也有，但 binary 獨立跑時需要確保)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS word_element_embeddings (
            id TEXT PRIMARY KEY,
            char TEXT NOT NULL,
            semantic TEXT NOT NULL,
            desc_text TEXT NOT NULL,
            embedding vector(1024) NOT NULL
        )",
        &[],
    )?;

    let client = reqwest::blocking::Client::new();

    // 4. 逐筆跑 embedding
    println!("開始為 {} 顆詞元生成 embedding...", pool.elements.len());

    for (i, e) in pool.elements.iter().enumerate() {
        // 優先使用 enriched desc，沒有就用原始 desc
        let input_text = e.embedding_desc.as_deref().unwrap_or(&e.desc).to_string();
        let resp: EmbedResp = client
            .post("http://localhost:11434/api/embed")
            .json(&EmbedReq {
                model: "bge-m3".into(),
                input: input_text,
            })
            .send()
            .context("Ollama request failed")?
            .json()
            .context("Failed to parse Ollama response")?;

        if resp.embeddings.is_empty() {
             anyhow::bail!("Ollama returned empty embeddings for id={}", e.id);
        }

        let vec = &resp.embeddings[0];
        if vec.len() != 1024 {
            anyhow::bail!("期望 1024 維，實際 {} 維，id={}", vec.len(), e.id);
        }

        // 轉成 pgvector 字串格式 "[0.1,0.2,...]"
        let vec_str = format!(
            "[{}]",
            vec.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        let sql = format!(
            "INSERT INTO word_element_embeddings (id, char, semantic, desc_text, embedding)
             VALUES ($1, $2, $3, $4, '{}')
             ON CONFLICT (id) DO UPDATE SET
                char = EXCLUDED.char, 
                semantic = EXCLUDED.semantic,
                desc_text = EXCLUDED.desc_text, 
                embedding = EXCLUDED.embedding",
            vec_str
        );

        conn.execute(
            sql.as_str(),
            &[&e.id, &e.char, &e.semantic, &e.desc],
        )?;

        if (i + 1) % 50 == 0 {
            println!("進度：{}/{}", i + 1, pool.elements.len());
        }
    }

    println!("完成：{} 顆詞元 embedding 已存入 PG", pool.elements.len());
    Ok(())
}
