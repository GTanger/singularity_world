import argparse
import lancedb
from sentence_transformers import SentenceTransformer

def query_lore(query_text, limit=5):
    db_path = "/home/tanger/.lancedb_tanger_singularity"
    collection_name = "claude_lore_archive"
    
    # 載入模型
    model = SentenceTransformer('paraphrase-multilingual-MiniLM-L12-v2')
    db = lancedb.connect(db_path)
    
    try:
        table = db.open_table(collection_name)
    except Exception:
        print("資料庫或資料表尚未就緒，請確認 ingest_claude_lore_to_lancedb.py 是否已執行完畢。")
        return

    # 生成查詢向量
    query_vector = model.encode(query_text).tolist()
    
    # 進行語意搜尋
    results = table.search(query_vector).limit(limit).to_list()
    
    if not results:
        print("找不到相關的歷史紀錄。")
        return
        
    print(f"\n針對查詢「{query_text}」的檢索結果：\n" + "="*50)
    for row in results:
        print(f"\n[來源檔案: {row['source_file']} | 距離: {row.get('_distance', 0):.4f}]")
        print("-" * 30)
        print(row['content'].strip())
        print("="*50)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Query the Singularity World Lore LanceDB.")
    parser.add_argument("query", type=str, help="The semantic query string")
    parser.add_argument("--limit", type=int, default=5, help="Number of results to return")
    args = parser.parse_args()
    
    # Suppress verbose warnings
    import warnings
    warnings.filterwarnings("ignore")
    
    query_lore(args.query, args.limit)
