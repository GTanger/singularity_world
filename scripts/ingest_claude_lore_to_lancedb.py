import os
import glob
import lancedb
import pyarrow as pa
from sentence_transformers import SentenceTransformer

# 1. 設置路徑與模型
source_dirs = [
    "/home/tanger/Desktop/Claude_Lore_Only",
    "/home/tanger/Projects/singularity_world/docs/stories"
]
db_path = "/home/tanger/.lancedb_tanger_singularity"
collection_name = "claude_lore_archive"

# 使用支援多語言的模型，避免 all-MiniLM-L6-v2 對繁體中文語意捕捉不佳的問題
print("正在載入多語言 Embedding 模型 (paraphrase-multilingual-MiniLM-L12-v2)...")
model = SentenceTransformer('paraphrase-multilingual-MiniLM-L12-v2')

db = lancedb.connect(db_path)

# 定義 Schema
schema = pa.schema([
    pa.field("vector", pa.list_(pa.float32(), 384)), # paraphrase-multilingual-MiniLM 也是 384 維
    pa.field("content", pa.string()),
    pa.field("source_file", pa.string()),
    pa.field("chunk_id", pa.int32())
])

# 若已存在則先刪除，確保資料乾淨
if collection_name in db.table_names():
    db.drop_table(collection_name)
table = db.create_table(collection_name, schema=schema)

md_files = []
for d in source_dirs:
    md_files.extend(glob.glob(os.path.join(d, "*.md")))
total_files = len(md_files)
print(f"找到 {total_files} 個過濾後的對話紀錄檔。開始進行語意切塊與向量化...")

def chunk_text(text, max_chars=1000, overlap=200):
    """將長文本切成帶有重疊區塊的片段，以保留上下文對話邏輯"""
    chunks = []
    start = 0
    while start < len(text):
        end = start + max_chars
        chunks.append(text[start:end])
        start += (max_chars - overlap)
    return chunks

batch_data = []
batch_size = 100  # 批次寫入資料庫
global_chunk_count = 0

for idx, file_path in enumerate(md_files):
    filename = os.path.basename(file_path)
    
    with open(file_path, "r", encoding="utf-8") as f:
        text = f.read()
        
    if not text.strip():
        continue
        
    chunks = chunk_text(text)
    
    for i, chunk in enumerate(chunks):
        embedding = model.encode(chunk).tolist()
        batch_data.append({
            "vector": embedding,
            "content": chunk,
            "source_file": filename,
            "chunk_id": i
        })
        global_chunk_count += 1
        
        # 批次寫入
        if len(batch_data) >= batch_size:
            table.add(batch_data)
            batch_data = []
            
    print(f"進度: {idx+1}/{total_files} ({filename}) - 累積 Chunk 數: {global_chunk_count}")

# 寫入剩餘的資料
if batch_data:
    table.add(batch_data)

print("=======================================")
print(f"✅ 語意庫建置完成！共切分了 {global_chunk_count} 個對話思考區塊。")
print(f"未來可以直接向 LanceDB 表 '{collection_name}' 進行語意查詢。")
