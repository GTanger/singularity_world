import json
import glob
import re
import os
import time
import requests
import random

BANNED_WORDS = [
    '漢族','關族','大和','高麗','兀魯斯','德蘭尼','卡納提克','幼法拉底','柏柏爾',
    '突厥安','毛伊','歐拉比','人定居','人聚居','人口'
]

OLLAMA_URL = "http://localhost:11434/api/chat"
MODEL_NAME = "qwen3.5:9b"

def validate_desc(desc, seen):
    desc = desc.strip().strip('"').strip("'").strip()
    if len(desc) < 20 or len(desc) > 110:
        return False, f"長度異常: {len(desc)}"
    if re.search(r'[a-zA-Z]{3,}', desc):
        return False, "包含英文"
    for w in BANNED_WORDS:
        if w in desc:
            return False, f"包含禁用詞: {w}"
    if desc.startswith("據說"):
        return False, "以據說開頭"
    if desc in seen:
        return False, "描述重複"
    return True, desc

def get_desc(room_id, room_name, tags, biome, marker_type, meta, seen, attempt=1):
    tag_str = ", ".join(tags)
    marker_str = f"地標:{marker_type}" if marker_type else "無地標"
    
    # Large set of varieties to prevent duplicates
    angles = ["俯視全景", "平視街道", "入口特寫", "後巷深處", "市集中心", "城牆邊緣", "港口碼頭邊", "荒野遠眺", "局部光影特寫", "石階向下", "林間隙光"]
    times = ["蒙昽清晨", "烈日午後", "落日餘輝", "靜謐深夜", "微雨紛紛", "大霧籠罩", "狂風席捲", "星辰漫天", "陰雲密佈", "夏蟬鳴叫", "冬雪初落", "秋風肅殺"]
    moods = ["荒涼", "繁華", "神祕", "危險", "寧靜", "神聖", "破敗", "生機勃勃"]
    
    angle = random.choice(angles)
    time_of_day = random.choice(times)
    mood = random.choice(moods)

    # Use coordinates and IDs as "uniqueness sparks"
    cid = meta.get('cell_id', random.randint(1, 10000))
    bid = meta.get('burg_id', random.randint(1, 10000))

    messages = [
        {"role": "system", "content": "你是一位頂尖的奇幻遊戲場景描寫員。你專門編寫獨一無二、富有感染力的繁體中文場景描述。"},
        {"role": "user", "content": f"""請為地點「{room_name}」(ID:{cid}/{bid}) 寫一段獨一無二的場景描述。
規範：
1. 字數：40-80 字。
2. 重點：玩家踏入此地的第一景象（視覺、聽覺、嗅覺）。
3. 【禁忌】：人口、文化/種族/宗教名、英文、以「據說」開頭、枯燥列舉設施。
4. 設定：視角「{angle}」，氣氛「{time_of_day}」，語調「{mood}」。

數據：
- 地形：{biome}
- 特徵：{tag_str}
- 地標：{marker_str}

只回傳描述文字。每個地點都必須擁有完全不同的意境與詞藻，不可重複。"""}
    ]

    payload = {
        "model": MODEL_NAME,
        "messages": messages,
        "stream": False,
        "options": {
            "temperature": 1.1 + (attempt * 0.1),
            "num_predict": 200,
            "top_k": 50,
            "top_p": 0.9
        }
    }

    try:
        response = requests.post(OLLAMA_URL, json=payload, timeout=60)
        if response.status_code == 200:
            desc = response.json().get("message", {}).get("content", "").strip()
            is_valid, final_desc = validate_desc(desc, seen)
            if is_valid:
                return final_desc
            else:
                if attempt < 8:
                    return get_desc(room_id, room_name, tags, biome, marker_type, meta, seen, attempt + 1)
        return None
    except Exception:
        if attempt < 3:
            return get_desc(room_id, room_name, tags, biome, marker_type, meta, seen, attempt + 1)
        return None

def main():
    files = sorted(glob.glob('data/rooms/editor/world_*.json'))
    print(f"找到 {len(files)} 個檔案，執行 100% 重新生成中...")
    seen_descriptions = set()
    
    processed = 0
    updated = 0
    
    # 這裡我們強制重新生成所有描述
    for f in files:
        with open(f, 'r', encoding='utf-8') as fp:
            data = json.load(fp)
        
        room_id = data.get('id', '')
        room_name = data.get('name', '無名之地')
        
        new_desc = get_desc(room_id, room_name, data.get('tags', []), data.get('meta', {}).get('biome', ''), data.get('meta', {}).get('marker_type', ''), data.get('meta', {}), seen_descriptions)
        
        if new_desc:
            data['description'] = new_desc
            seen_descriptions.add(new_desc)
            with open(f, 'w', encoding='utf-8') as fp:
                json.dump(data, fp, ensure_ascii=False, indent=2)
                fp.write('\n')
            updated += 1
        else:
            print(f"  [Error] {room_id} 無法生成唯一內容。")

        processed += 1
        if processed % 10 == 0:
            print(f"進度: {processed}/{len(files)} (新生成 {updated} 個)")

    print(f"全面完成！共生成 {updated} 個不重複描述。")

if __name__ == "__main__":
    main()
