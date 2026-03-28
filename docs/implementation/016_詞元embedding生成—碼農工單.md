# 016 — 詞元 Embedding 生成（碼農工單）

> **目標**：用本地 Ollama bge-m3 模型，將 500 顆詞元的 desc 轉為 1024 維向量，存入 PostgreSQL（pgvector）。
>
> **一次性作業**，跑完後不再需要 LLM。未來新增詞元時，只需對新 desc 跑一次。

---

## 一、前置條件

- Ollama 已安裝且 `bge-m3` 模型可用（`ollama run bge-m3` 驗證）
- PostgreSQL 已啟用 pgvector 擴充（`CREATE EXTENSION IF NOT EXISTS vector` 已在 `sql.rs` 中）
- 詞元池：`data/config/word_elements.json`，500 顆，每顆有 `id`、`char`、`semantic`、`desc` 欄位

## 二、建表

在 `src/store/sql.rs` 的 `create_tables()` 中新增：

```sql
CREATE TABLE IF NOT EXISTS word_element_embeddings (
    id TEXT PRIMARY KEY,
    char TEXT NOT NULL,
    semantic TEXT NOT NULL,
    desc_text TEXT NOT NULL,
    embedding vector(1024) NOT NULL
);
```

欄位說明：
- `id`：詞元 ID，對應 word_elements.json 的 `id`
- `char`：單字，如「斬」
- `semantic`：語義類，如「動」
- `desc_text`：desc 原文，如「利刃橫過的一瞬」
- `embedding`：1024 維向量（bge-m3 輸出維度）

## 三、生成腳本

寫一個獨立腳本（Python 或 Rust 皆可，建議 Python 省事）：

### 路徑：`tools/generate_embeddings.py`

### 流程：

```python
import json
import requests
import psycopg2

# 1. 讀取詞元池
with open('data/config/word_elements.json') as f:
    elements = json.load(f)['elements']

# 2. 連接 PG
conn = psycopg2.connect("postgresql://localhost/singularity_world")
cur = conn.cursor()

# 3. 確保表存在
cur.execute("""
    CREATE TABLE IF NOT EXISTS word_element_embeddings (
        id TEXT PRIMARY KEY,
        char TEXT NOT NULL,
        semantic TEXT NOT NULL,
        desc_text TEXT NOT NULL,
        embedding vector(1024) NOT NULL
    )
""")

# 4. 逐筆跑 embedding
for e in elements:
    # 呼叫 Ollama embedding API
    resp = requests.post('http://localhost:11434/api/embed', json={
        'model': 'bge-m3',
        'input': e['desc']
    })
    vec = resp.json()['embeddings'][0]

    # 驗證維度
    assert len(vec) == 1024, f"期望 1024 維，實際 {len(vec)} 維，id={e['id']}"

    # Upsert
    cur.execute("""
        INSERT INTO word_element_embeddings (id, char, semantic, desc_text, embedding)
        VALUES (%s, %s, %s, %s, %s)
        ON CONFLICT (id) DO UPDATE SET
            char = EXCLUDED.char,
            semantic = EXCLUDED.semantic,
            desc_text = EXCLUDED.desc_text,
            embedding = EXCLUDED.embedding
    """, (e['id'], e['char'], e['semantic'], e['desc'], str(vec)))

conn.commit()
cur.close()
conn.close()
print(f"完成：{len(elements)} 顆詞元 embedding 已存入 PG")
```

### 執行：

```bash
cd ~/Projects/singularity_world
pip install psycopg2-binary  # 如果沒裝過
python3 tools/generate_embeddings.py
```

## 四、驗證

跑完後執行以下 SQL 驗證：

```sql
-- 確認數量
SELECT COUNT(*) FROM word_element_embeddings;
-- 應為 500

-- 確認維度（取一筆看長度）
SELECT id, char, vector_dims(embedding) FROM word_element_embeddings LIMIT 5;
-- vector_dims 應為 1024

-- 語義相似度測試：「斬」和「劈」應該很近，「斬」和「癒」應該很遠
SELECT a.char AS char_a, b.char AS char_b,
       1 - (a.embedding <=> b.embedding) AS cosine_similarity
FROM word_element_embeddings a, word_element_embeddings b
WHERE a.id = 'zhan' AND b.id IN ('pi', 'yu')
ORDER BY cosine_similarity DESC;
```

## 五、注意事項

1. **不要改 word_elements.json**。只讀取，不修改
2. **bge-m3 的 API 端點**是 `http://localhost:11434/api/embed`，不是 `/api/embeddings`（Ollama 新版 API）
3. **vector 格式**：pgvector 接受字串格式 `'[0.1, 0.2, ...]'`，Python 的 `str(list)` 即可
4. **500 筆大約跑 1~2 分鐘**，不會卡很久
5. **冪等**：用 `ON CONFLICT DO UPDATE`，重跑不會炸
6. **不需要動 Rust 代碼**。建表語句加到 `sql.rs` 的 `create_tables()` 裡即可，embedding 生成用 Python 腳本獨立跑

## 六、交付標準

- [ ] `word_element_embeddings` 表有 500 筆資料
- [ ] 每筆 embedding 維度為 1024
- [ ] 語義相似度測試通過（攻擊類詞元彼此接近、與防禦類遠離）
- [ ] Python 腳本放在 `tools/generate_embeddings.py`
