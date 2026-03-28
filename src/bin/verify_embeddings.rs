use r2d2_postgres::postgres::{Client, NoTls};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct GameDimsFile { elements: Vec<GameDimsEntry> }
#[derive(Deserialize)]
struct GameDimsEntry { id: String, game_dims: HashMap<String, f64> }

fn main() -> anyhow::Result<()> {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:singularity@localhost:5432/singularity".to_string());
    let mut conn = Client::connect(&db_url, NoTls)?;

    // 0. 檢查是否需要拼接手標維度
    let row = conn.query_one("SELECT vector_dims(embedding) FROM word_element_embeddings LIMIT 1", &[])?;
    let current_dims: i32 = row.get(0);

    if current_dims == 1024 {
        println!("偵測到 1024 維，開始拼接 6 維手標維度...");

        // 讀手標維度
        let dims_raw = std::fs::read_to_string("data/config/word_elements_gamedims.json")?;
        let dims_file: GameDimsFile = serde_json::from_str(&dims_raw)?;
        let dims_map: HashMap<String, HashMap<String, f64>> = dims_file.elements.into_iter()
            .map(|e| (e.id, e.game_dims)).collect();

        let dim_order = ["atk", "def", "spd", "dot", "aoe", "heal"];

        // 先回到 1024（如果之前已改過但資料沒跟上）
        conn.execute("ALTER TABLE word_element_embeddings ALTER COLUMN embedding TYPE vector(1030)", &[]).ok();
        // 暫時放寬為無限制維度
        conn.execute("ALTER TABLE word_element_embeddings ALTER COLUMN embedding TYPE vector", &[])?;

        // 讀所有 embedding，拼接，更新
        let rows = conn.query("SELECT id, embedding::text FROM word_element_embeddings", &[])?;
        for row in &rows {
            let id: String = row.get(0);
            let emb_str: String = row.get(1);

            // Parse "[0.1,0.2,...]"
            let inner = emb_str.trim_start_matches('[').trim_end_matches(']');
            let mut vals: Vec<f64> = inner.split(',').filter_map(|s| s.trim().parse().ok()).collect();
            vals.truncate(1024);

            // 拼接 6 維
            let game_dims = dims_map.get(&id);
            for dk in &dim_order {
                let v = game_dims.map(|d| *d.get(*dk).unwrap_or(&0.0)).unwrap_or(0.0);
                vals.push(v);
            }

            let vec_str = format!("[{}]", vals.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","));
            let sql = format!(
                "UPDATE word_element_embeddings SET embedding = '{}'::vector WHERE id = $1",
                vec_str
            );
            conn.execute(sql.as_str(), &[&id])?;
        }
        // 資料都 1030 了，鎖定維度
        conn.execute("ALTER TABLE word_element_embeddings ALTER COLUMN embedding TYPE vector(1030)", &[])?;
        println!("完成：{} 顆已更新為 1030 維", rows.len());
    }

    // 1. 確認數量
    let row = conn.query_one("SELECT COUNT(*) FROM word_element_embeddings", &[])?;
    let count: i64 = row.get(0);
    println!("詞元總數: {}", count);

    // 2. 確認維度
    let row = conn.query_one("SELECT id, char, vector_dims(embedding) FROM word_element_embeddings LIMIT 1", &[])?;
    let id: String = row.get(0);
    let char: String = row.get(1);
    let dims: i32 = row.get(2);
    println!("樣本 [{} / {}] 維度: {}", id, char, dims);

    // 3. 廣泛語義相似度測試
    println!("\n語義相似度測試 (Cosine Similarity):");

    // 測試對：(id_a, id_b, 預期關係)
    let pairs = vec![
        // 應該近的
        ("zhan", "pi", "斬vs劈 (都是砍)"),
        ("zhan", "ci", "斬vs刺 (都是攻擊動作)"),
        ("dun_dong", "bi", "盾vs蔽 (都是防禦)"),
        ("jian", "rui", "堅vs銳 (都是物理屬性)"),
        ("ran", "chi", "燃vs熾 (都是火相關)"),
        ("dong", "ning", "凍vs凝 (都是冷/固化)"),
        // 應該遠的
        ("zhan", "yu", "斬vs癒 (攻擊vs治癒)"),
        ("zhan", "dun_dong", "斬vs盾 (攻擊vs防禦)"),
        ("ran", "dong", "燃vs凍 (火vs冰)"),
        ("jian", "rou", "堅vs柔 (硬vs軟)"),
        ("sheng_dong", "si_shu", "生vs死 (對立)"),
        ("xi_tai", "ai", "喜vs哀 (對立情緒)"),
        // 跨類
        ("wu", "zhan", "吾vs斬 (主詞vs動詞)"),
        ("ye", "zhan", "也vs斬 (助詞vs動詞)"),
    ];

    for (id_a, id_b, label) in &pairs {
        let rows = conn.query(
            "SELECT 1 - (a.embedding <=> b.embedding) AS sim
             FROM word_element_embeddings a, word_element_embeddings b
             WHERE a.id = $1 AND b.id = $2",
            &[id_a, id_b],
        )?;
        if let Some(row) = rows.first() {
            let sim: f64 = row.get(0);
            println!("  {:.4}  {}", sim, label);
        } else {
            println!("  N/A   {} (id not found)", label);
        }
    }

    // 4. 全局相似度分佈
    println!("\n全局相似度分佈（隨機抽 1000 對）:");
    let stats = conn.query_one(
        "WITH sample AS (
            SELECT 1 - (a.embedding <=> b.embedding) AS sim
            FROM word_element_embeddings a, word_element_embeddings b
            WHERE a.id < b.id
            ORDER BY random() LIMIT 1000
        )
        SELECT MIN(sim), AVG(sim), MAX(sim), STDDEV(sim) FROM sample",
        &[],
    )?;
    let min: f64 = stats.get(0);
    let avg: f64 = stats.get(1);
    let max: f64 = stats.get(2);
    let std: f64 = stats.get(3);
    println!("  min={:.4} avg={:.4} max={:.4} stddev={:.4}", min, avg, max, std);

    Ok(())
}
