// 主題遮罩、品質閘、錨點與情緒增量，對齊既有 npcnpc/helpers。

use crate::db::{self, rune_lcs_similarity};
use crate::gametext;
use crate::npc::NpcTopicMask;
use crate::store::NpcDyad;

use super::state::recent_room_events;
pub use crate::ai::sanitize::{replace_clause_initial_temporal_cai, strip_wt_catalog_codes};

const QUALITY_GATE_DEFAULT_MAX_RUNES: i32 = 52;

/// 依房間與時段建構主題遮罩（對齊既有 `TopicMaskForRoom`）。
pub fn topic_mask_for_room(room_id: &str, hour: i32) -> NpcTopicMask {
    let venue_ids = db::get_venue_ids_for_room(room_id).unwrap_or_default();
    let room_tags = db::get_room(room_id)
        .ok()
        .flatten()
        .map(|r| r.tags)
        .unwrap_or_default();
    NpcTopicMask {
        is_work_venue: !venue_ids.is_empty(),
        is_night_time: !(5..21).contains(&hour),
        has_room_event: !recent_room_events(room_id, 1).is_empty(),
        room_tags,
    }
}

/// 兩 NPC 是否有共用工作場所（對齊既有 `NpcPairSameVenue`）。
#[must_use]
pub fn npc_pair_same_venue(id_a: &str, id_b: &str) -> bool {
    let Ok(as_a) = db::get_assignments_for_entity(id_a) else {
        return false;
    };
    let Ok(as_b) = db::get_assignments_for_entity(id_b) else {
        return false;
    };
    if as_a.is_empty() || as_b.is_empty() {
        return false;
    }
    let mut vid = std::collections::HashSet::new();
    for a in &as_a {
        if !a.venue_id.is_empty() {
            vid.insert(a.venue_id.as_str());
        }
    }
    as_b.iter()
        .any(|b| !b.venue_id.is_empty() && vid.contains(b.venue_id.as_str()))
}

#[must_use]
pub fn has_tag(tags: &[String], t: &str) -> bool {
    tags.iter().any(|v| v == t)
}

pub fn add_tag(tags: &[String], t: &str) -> Vec<String> {
    if t.is_empty() || has_tag(tags, t) {
        return tags.to_vec();
    }
    let mut out = tags.to_vec();
    out.push(t.to_string());
    out
}

pub fn remove_tag(tags: &[String], t: &str) -> Vec<String> {
    if t.is_empty() || tags.is_empty() {
        return tags.to_vec();
    }
    tags.iter().filter(|v| *v != t).cloned().collect()
}

/// 雙元關係提示句（對齊既有 `dyadRelationHint`）。
#[must_use]
pub fn dyad_relation_hint(d: Option<&NpcDyad>) -> String {
    match d {
        None => gametext::dyad_relation_hint(true, 0, 0),
        Some(di) => gametext::dyad_relation_hint(false, di.sentiment, di.familiarity),
    }
}

/// 依台詞與主題推估情緒增量（對齊既有 `SentimentDeltaFromDialogue`）。
#[must_use]
pub fn sentiment_delta_from_dialogue(
    topic_hint: &str,
    line_a: &str,
    line_b: &str,
    familiarity: i32,
    topic_id: &str,
) -> i32 {
    let mut delta = 0;
    let sen = gametext::sentiment();
    if gametext::topic_contains_any(topic_hint, &sen.shift_or_chat_substrings) {
        delta += 1;
    }
    if !sen.pry_substring.is_empty()
        && topic_hint.contains(sen.pry_substring.as_str())
        && familiarity < 20
    {
        delta -= 1;
    }
    let outside = gametext::topic_id("outside_rumor");
    let weather = gametext::topic_id("weather");
    let empty_topic_outside_weather = topic_id.is_empty()
        && (gametext::topic_contains_any(topic_hint, &sen.outside_wild_substrings)
            || (!sen.weather_substring.is_empty()
                && topic_hint.contains(sen.weather_substring.as_str())));
    if topic_id == outside || topic_id == weather || empty_topic_outside_weather {
        delta += 1;
    }
    let joined = format!("{line_a} {line_b}");
    for w in &sen.neg_words {
        if !w.is_empty() && joined.contains(w.as_str()) {
            delta -= 2;
            break;
        }
    }
    for w in &sen.pos_words {
        if !w.is_empty() && joined.contains(w.as_str()) {
            delta += 1;
            break;
        }
    }
    delta
}

/// 清理 NPC 台詞（去除舞台提示 narration、簡化字/兒化音替換）
pub fn clean_npc_line(line: &str) -> String {
    let mut s = crate::ai::sanitize::normalize_simplified_to_traditional_zh(line.trim());
    for (bad, good) in [
        ("WT-1024", "六分儀"),
        ("-1024", "六分儀"),
        ("WT-0893北嶺", "北嶺"),
        ("-0893北嶺", "北嶺"),
        ("WT-0893", "硬木藤背架"),
        ("-0893", "硬木藤背架"),
    ] {
        if s.contains(bad) {
            s = s.replace(bad, good);
        }
    }
    s = strip_wt_catalog_codes(&s);
    for (open_pat, close_ch) in [("：『", '』'), ("：「", '」'), (":『", '』'), (":「", '」')] {
        if let Some(pos) = s.find(open_pat) {
            let prefix_chars = s[..pos].chars().count();
            if (1..=28).contains(&prefix_chars) {
                let rest = s[pos + open_pat.len()..].trim();
                let inner = rest.strip_suffix(close_ch).unwrap_or(rest).trim();
                if inner.chars().count() >= 4 {
                    s = inner.to_string();
                    break;
                }
            }
        }
    }
    for (bad, good) in [
        ("這样", "這樣"),
        ("那样", "那樣"),
        ("怎樣", "怎樣"),
        ("怎么样", "怎麼樣"),
        ("一样", "一樣"),
        ("模样", "模樣"),
        ("圖样", "圖樣"),
        ("花样", "花樣"),
        ("样品", "樣品"),
        ("样子", "樣子"),
        ("采草", "採草"),
        ("采藥", "採藥"),
        ("采茶", "採茶"),
        ("采礦", "採礦"),
        ("采石", "採石"),
        ("采集", "採集"),
        ("采摘", "採摘"),
        ("風采", "\u{E011}"),
        ("神采", "\u{E012}"),
        ("文采", "\u{E013}"),
        ("光采", "\u{E014}"),
        ("喝采", "\u{E015}"),
        ("豐采", "\u{E016}"),
        ("多采多姿", "\u{E017}"),
        ("興高采烈", "\u{E018}"),
        ("無精打采", "\u{E019}"),
        ("白采妮", "\u{E01A}"),
        ("采買", "採買"),
        ("采辦", "採辦"),
        ("再采", "再採"),
        ("去采", "去採"),
        ("來采", "來採"),
        ("多采", "多採"),
        ("少采", "少採"),
        ("新采", "新採"),
        ("才采", "才採"),
        ("剛采", "才採"),
        ("開采", "開採"),
        ("采幾", "採幾"),
        ("采一", "採一"),
        ("采兩", "採兩"),
        ("采三", "採三"),
        ("采些", "採些"),
        ("采點", "採點"),
        ("采了", "採了"),
        ("采過", "採過"),
        ("采完", "採完"),
        ("采回", "採回"),
        ("采來", "採來"),
        ("采下", "採下"),
        ("采到", "採到"),
        ("采花", "採花"),
        ("采果", "採果"),
        ("采蜜", "採蜜"),
        ("采菇", "採菇"),
        ("采筍", "採筍"),
        ("采蓮", "採蓮"),
        ("采桑", "採桑"),
        ("采砂", "採砂"),
        ("采購", "採購"),
        ("采納", "採納"),
        ("采用", "採用"),
        ("采取", "採取"),
        ("采訪", "採訪"),
        ("山裡采", "山裡採"),
        ("林裡采", "林裡採"),
        ("\u{E011}", "風采"),
        ("\u{E012}", "神采"),
        ("\u{E013}", "文采"),
        ("\u{E014}", "光采"),
        ("\u{E015}", "喝采"),
        ("\u{E016}", "豐采"),
        ("\u{E017}", "多采多姿"),
        ("\u{E018}", "興高采烈"),
        ("\u{E019}", "無精打采"),
        ("\u{E01A}", "白采妮"),
        ("咋樣", "怎樣"),
        ("咋辦", "怎麼辦"),
        ("咋回事", "怎麼回事"),
        ("咋說", "怎麼說"),
        ("咋走", "怎麼走"),
        ("咋個", "怎麼個"),
        ("咋這", "怎麼這"),
        ("咋那", "怎麼那"),
        ("咋不", "怎麼不"),
        ("咋還", "怎麼還"),
        ("咋又", "怎麼又"),
        ("咋沒", "怎麼沒"),
        ("咋會", "怎麼會"),
        ("咋能", "怎麼能"),
        ("咋就", "怎麼就"),
        ("咋看", "怎麼看"),
        ("咋想", "怎麼想"),
        ("咋整", "怎麼弄"),
        ("咋弄", "怎麼弄"),
        ("咋搞", "怎麼搞"),
        ("咋來", "怎麼來"),
        ("咋去", "怎麼去"),
        ("咋賣", "怎麼賣"),
        ("咋算", "怎麼算"),
        ("買肥皂", "買茶籽皂塊"),
        ("塊肥皂", "塊茶籽皂"),
        ("份肥皂", "份茶籽皂塊"),
        ("買葡萄酒", "買醇釀米酒"),
        ("份葡萄酒", "份醇釀米酒"),
        ("買橄欖", "買青梅果"),
        ("份橄欖", "份青梅果"),
        ("才窯溫下來", "窯溫才降下來"),
        ("剛窯溫下來", "窯溫才降下來"),
        ("晒乾", "曬乾"),
        ("晒太陽", "曬太陽"),
        ("晾晒", "晾曬"),
        ("暴晒", "暴曬"),
        ("日晒", "日曬"),
        ("晒鹽", "曬鹽"),
        ("晒魚", "曬魚"),
        ("晒穀", "曬穀"),
        ("老伴兒", "老伴"),
        ("昨兒", "昨日"),
        ("今兒", "今日"),
        ("明兒", "明日"),
        ("這兒", "這邊"),
        ("那兒", "那邊"),
        ("哪兒", "哪裡"),
        ("白開水", "溫茶水"),
        ("涼白開", "涼茶水"),
        ("喝完白開", "喝完涼茶水"),
        ("一口白開", "一口涼茶水"),
        ("白開", "涼茶水"),
        ("這味兒", "這滋味"),
        ("味兒", "滋味"),
        ("勁兒", "力道"),
        ("事兒", "事情"),
        ("話兒", "話"),
        ("門兒", "門路"),
        ("一點兒", "一點"),
        ("明兒個", "明日"),
        ("今兒個", "今日"),
        ("昨兒個", "昨日"),
        ("這會兒", "眼下"),
        ("那會兒", "那陣子"),
        ("待會兒", "待會"),
        ("今晚上", "今晚"),
        ("明晚上", "明晚"),
        ("昨晚上", "昨晚"),
        ("吃啥", "吃什麼"),
        ("幹啥", "做什麼"),
        ("做啥", "做什麼"),
        ("說啥", "說什麼"),
        ("買啥", "買什麼"),
        ("找啥", "找什麼"),
        ("看啥", "看什麼"),
        ("有啥", "有什麼"),
        ("沒啥", "沒什麼"),
        ("為啥", "為什麼"),
        ("刺猬", "刺蝟"),
        ("猬", "蝟"),
        ("做啥用", "做什麼用"),
        ("啥用", "做什麼用"),
        ("帶啥", "帶什麼"),
        ("備啥", "備什麼"),
        ("拿啥", "拿什麼"),
        ("要啥", "要什麼"),
        ("店里", "店裡"),
        ("屋里", "屋裡"),
        ("山里", "山裡"),
        ("水里", "水裡"),
        ("手里", "手裡"),
        ("心里", "心裡"),
        ("夜里", "夜裡"),
        ("城里", "城裡"),
        ("村里", "村裡"),
        ("院里", "院裡"),
        ("哪里", "哪裡"),
        ("這里", "這裡"),
        ("那里", "那裡"),
        ("咱們", "我們"),
        ("熱乎乎", "熱騰騰"),
        ("暖乎乎", "暖烘烘"),
        ("獅子", "石虎"),
        ("駱駝", "馱獸"),
        ("大象", "巨獸"),
        ("髮麻", "發麻"),
        ("髮抖", "發抖"),
        ("髮冷", "發冷"),
        ("髮熱", "發熱"),
        ("髮慌", "發慌"),
        ("髮愁", "發愁"),
        ("髮呆", "發呆"),
        ("髮現", "發現"),
        ("髮生", "發生"),
        ("出髮", "出發"),
        ("髮覺", "發覺"),
        ("髮揮", "發揮"),
        ("咱倆", "我們倆"),
        ("咱兩", "我們倆"),
        ("咱自己", "我們自己"),
        ("咱這邊", "我們這邊"),
        ("咱那邊", "我們那邊"),
        ("咱不", "我們不"),
        ("咱都", "我們都"),
        ("咱還", "我們還"),
        ("咱得", "我們得"),
        ("咱把", "我們把"),
        ("咱往", "我們往"),
        ("咱回", "我們回"),
        ("咱來", "我們來"),
        ("咱看", "我們看"),
        ("咱說", "我們說"),
        ("咱走", "我們走"),
        ("咱別", "我們別"),
        ("咱再", "我們再"),
        ("剛好", "正好"),
        ("剛巧", "湊巧"),
        ("天剛放晴", "天色初晴"),
        ("天才放晴", "天色初晴"),
        ("天剛亮", "天色初亮"),
        ("天才亮", "天色初亮"),
        ("天剛黑", "天色初暗"),
        ("天才黑", "天色初暗"),
        ("剛冒的霧", "新起的山霧"),
        ("才冒的霧", "新起的山霧"),
        ("剛冒出來的霜", "新結的寒霜"),
        ("才冒出來的霜", "新結的寒霜"),
        ("水缸裡的開水", "水缸裡的熱茶水"),
        ("冷開水", "涼茶水"),
        ("客官", "行客"),
        ("客倌", "行客"),
        ("那伙人", "那夥人"),
        ("這伙人", "這夥人"),
        ("一伙人", "一夥人"),
        ("同伙", "同夥"),
        ("合伙", "合夥"),
        ("入伙", "入夥"),
        ("散伙", "散夥"),
        ("大伙兒", "大夥"),
        ("大伙", "大夥"),
        ("伙計", "夥計"),
        ("小伙子", "年輕人"),
        ("小伙", "年輕人"),
        ("好家伙", "好傢伙"),
        ("老家伙", "老傢伙"),
        ("家伙", "傢伙"),
        ("加了班", "連夜巡守"),
        ("加個班", "連夜趕工"),
        ("加點班", "多趕一陣工"),
        ("上個班", "當值上工"),
        ("下個班", "收工歇息"),
        ("上下班", "上工收工"),
        ("都加班了", "都連夜加派人手了"),
        ("正加班", "正連夜巡守"),
        ("加班趕工", "連夜趕工"),
        ("加班", "連夜趕工"),
        ("上班", "上工"),
        ("下班", "收工"),
        ("大陽頭", "大日頭"),
        ("防爆防風", "防風"),
        ("陽春粗麵", "清湯粗麵"),
        ("陽春麵", "清湯粗麵"),
        ("喝口開水", "喝口熱茶水"),
        ("滾開水", "滾水"),
        ("熱開水", "熱茶水"),
        ("溫開水", "溫茶水"),
        ("開水", "熱茶水"),
        ("剛冒出來", "新凝出來"),
        ("才冒出來", "新凝出來"),
        ("剛冒出", "初冒出"),
        ("才冒出", "初冒出"),
        ("剛結的霜", "新結的霜"),
        ("才結的霜", "新結的霜"),
        ("剛結霜", "新結霜"),
        ("才結霜", "新結霜"),
        ("剛起的霧", "新起的霧"),
        ("才起的霧", "新起的霧"),
        ("剛起的風", "新起的風"),
        ("才起的風", "新起的風"),
        ("剛下的雪", "新落的雪"),
        ("才下的雪", "新落的雪"),
        ("剛下的雨", "新落的雨"),
        ("才下的雨", "新落的雨"),
        ("古河林礦脈封山", "古河巨木林封山休養"),
        ("半嶺避雪", "半嶺鎮避雪"),
        ("黑沼林澤", "水尾寮"),
        ("黑沼林", "水尾寮"),
        ("西南原", "大圳頭"),
        ("白塔山麓", "半嶺鎮"),
        ("北聖山", "北嶺腳"),
        ("西北礦帶", "紅土崁"),
        ("報備", "知會一聲"),
        ("報修", "送去修整"),
        ("這叫效率", "這叫手腳俐落"),
        ("講究效率", "講究手腳俐落"),
        ("效率高", "手腳俐落"),
        ("效率", "手腳俐落"),
        ("半個鐘頭", "半晌"),
        ("半個時辰", "半晌"),
        ("半個鐘", "半晌"),
        ("一個鐘頭", "一時半刻"),
        ("一個時辰", "一時半刻"),
        ("兩個鐘頭", "好一陣子"),
        ("兩個時辰", "好一陣子"),
        ("幾個鐘頭", "良久"),
        ("幾個時辰", "良久"),
        ("黑沼關隘", "下營"),
        ("鹽田澳", "烏石鼻"),
        ("青籠關", "下營"),
        ("從青籠回", "從下營回"),
        ("往青籠走", "往下營走"),
        ("去青籠", "去下營"),
        ("在青籠", "在下營"),
        ("巨藻淺灣林木封山", "巨藻淺灣休漁養息"),
        ("巨藻淺灣封山", "巨藻淺灣休漁養息"),
        ("淺澤泥口林木封山", "淺澤泥口休漁養息"),
        ("淺澤泥口封山", "淺澤泥口休漁養息"),
        ("黑石灣林木封山", "黑石灣休漁養息"),
        ("驚濤岩岸林木封山", "驚濤岩岸休漁養息"),
        ("人剛在", "人早先在"),
        ("人才在", "人早先在"),
        ("人剛從", "人早先從"),
        ("人才從", "人早先從"),
        ("人剛去", "人早先去"),
        ("人才去", "人早先去"),
        ("人剛到", "人早先到"),
        ("人才到", "人早先到"),
        ("人剛回", "人早先回"),
        ("人才回", "人早先回"),
        ("人剛把", "人早先把"),
        ("人才把", "人早先把"),
        ("剛那陣", "早先那陣"),
        ("才那陣", "早先那陣"),
        ("剛這陣", "眼下這陣"),
        ("才這陣", "眼下這陣"),
        ("剛還在", "早先還在"),
        ("才還在", "早先還在"),
        ("剛還說", "早先還說"),
        ("才還說", "早先還說"),
        ("剛還見", "早先還見"),
        ("才還見", "早先還見"),
        ("剛還看", "早先還看"),
        ("才還看", "早先還看"),
        ("剛還想", "早先還想"),
        ("才還想", "早先還想"),
        ("剛還聽", "早先還聽"),
        ("才還聽", "早先還聽"),
        ("才走過", "早先走過"),
        ("才路過", "早先路過"),
        ("才經過", "早先經過"),
        ("才見過", "早先見過"),
        ("才聽說", "早先聽說"),
        ("才看到", "早先看到"),
        ("才幫", "早先幫"),
        ("才收了", "早先收妥了"),
        ("才割了", "早先割了"),
        ("剛才", "早先"),
        ("剛剛", "早先"),
        ("方才", "早先"),
        ("適才", "早先"),
        ("剛拿", "早先拿"),
        ("剛從", "早先從"),
        ("剛帶", "新帶"),
        ("剛摘", "新摘"),
        ("剛採", "新採"),
        ("剛看", "早先看"),
        ("剛踩", "早先踩"),
        ("剛翻", "早先翻"),
        ("剛說", "早先說"),
        ("霧還繼著", "霧還續著"),
        ("還繼著", "還續著"),
        ("潮汐表", "潮信木牌"),
        ("正好，正好", "正好，可"),
        ("正好,正好", "正好,可"),
        ("剛在", "早先在"),
        ("剛買", "新買"),
        ("剛賣", "早先賣"),
        ("剛換", "新換"),
        ("剛修", "新修"),
        ("剛打", "新打"),
        ("剛熬", "新熬"),
        ("剛煮", "新煮"),
        ("剛烤", "新烤"),
        ("剛做", "新做"),
        ("剛搬", "早先搬"),
        ("剛扛", "早先扛"),
        ("剛挑", "早先挑"),
        ("剛送", "早先送"),
        ("剛交", "早先交"),
        ("剛接", "早先接"),
        ("剛取", "新取"),
        ("剛擦", "早先擦"),
        ("剛掃", "早先掃"),
        ("剛盤", "早先盤"),
        ("剛進", "初進"),
        ("剛出", "初出"),
        ("剛下", "初下"),
        ("剛上", "初上"),
        ("剛吃", "早先吃"),
        ("剛歇", "早先歇"),
        ("剛舀", "早先舀"),
        ("剛燉", "新燉"),
        ("剛檢", "早先檢"),
        ("剛繫", "新繫"),
        ("剛順", "早先順"),
        ("剛踏", "早先踏"),
        ("剛喘", "早先喘"),
        ("剛割", "早先割"),
        ("剛拾", "早先拾"),
        ("剛確", "早先確"),
        ("剛量", "早先量"),
        ("剛用", "早先用"),
        ("剛剝", "新剝"),
        ("剛瞧", "早先瞧"),
        ("剛磨", "新磨"),
        ("剛補", "新補"),
        ("剛見", "早先見"),
        ("剛摸", "早先摸"),
        ("剛晾", "早先晾"),
        ("剛巡", "早先巡"),
        ("剛曬", "早先曬"),
        ("剛冒", "初冒"),
        ("剛亮", "初亮"),
        ("剛醒", "初醒"),
        ("剛走過", "早先走過"),
        ("剛走", "早先走"),
        ("剛盯", "早先盯"),
        ("剛把", "早先把"),
        ("剛跟", "早先跟"),
        ("剛過", "早先過"),
        ("剛喝完", "早先喝完"),
        ("剛喝", "早先喝"),
        ("剛傳說", "早先傳說"),
        ("剛靠", "早先靠"),
        ("剛幫", "早先幫"),
        ("剛轉涼", "初轉涼"),
        ("吃完面", "吃完麵"),
        ("吃面", "吃麵"),
        ("煮面", "煮麵"),
        ("熱湯面", "熱湯麵"),
        ("一碗面", "一碗麵"),
    ] {
        if s.contains(bad) {
            s = s.replace(bad, good);
        }
    }
    if s.contains('剛') {
        s = crate::ai::sanitize::replace_temporal_gang_chars(&s);
    }
    s = crate::ai::sanitize::replace_clause_initial_temporal_cai(&s);
    while s.contains("早先在早先在") {
        s = s.replace("早先在早先在", "早先在");
    }
    for (bad, good) in [
        ("活兒才完", "活計方歇"),
        ("活計才完", "活計方歇"),
        ("差事才完", "差事方歇"),
        ("才盤貨", "早先盤貨"),
        ("真帶勁", "真猛烈"),
        ("太帶勁", "太凌厲"),
        ("跟活的似的", "如同活物一般"),
        ("正加強 ，", "正加強巡防，"),
        ("正加強,", "正加強巡防,"),
        ("這活兒", "這活計"),
        ("好活兒", "好活計"),
        ("等活兒", "等工"),
        ("接活兒", "接工"),
        ("大夥兒", "大夥"),
        ("老花鏡", "助視鏡"),
        ("黃銅眼鏡", "黃銅助視鏡"),
        ("眼鏡", "助視鏡"),
        ("水動鋸木廠", "水動鋸木坊"),
        ("連軸鋸木廠", "連軸鋸木坊"),
        ("鋸木廠", "鋸木坊"),
        ("內河造船廠", "內河造船塢"),
        ("避風灣造船廠", "避風灣造船塢"),
        ("造船廠", "造船塢"),
        ("器皿精煉廠", "器皿精煉坊"),
        ("精煉廠", "精煉坊"),
        ("我也早先", "我也先"),
        ("好一會兒", "好一陣子"),
        ("待會兒", "稍後"),
        ("等會兒", "稍後"),
        ("一會兒", "片刻"),
        ("這會兒", "眼下"),
        ("那會兒", "早先"),
        ("歇了會兒", "歇了片刻"),
        ("歇會兒", "歇息片刻"),
        ("坐了會兒", "坐了片刻"),
        ("坐會兒", "坐片刻"),
        ("等了會兒", "等了片刻"),
        ("聊了會兒", "聊了片刻"),
        ("喘了會兒", "喘了片刻"),
        ("曬了會兒", "曬了片刻"),
        ("說了會兒", "說了片刻"),
        ("過會兒", "稍後"),
        ("了會兒", "了片刻"),
        ("好一會兒", "好一陣子"),
        ("會兒", "片刻"),
        ("一塊兒", "一同"),
        ("大夥兒", "大夥"),
        ("傢伙事兒", "器物傢伙"),
        ("這事兒", "這事"),
        ("那事兒", "那事"),
        ("有事兒", "有事"),
        ("沒事兒", "沒事"),
        ("補補勁兒", "提提氣力"),
        ("門兒清", "一清二楚"),
        ("汗味兒", "汗氣"),
        ("這味兒", "這氣味"),
        ("玩意兒", "物件"),
        ("船工活兒", "船工活計"),
        ("手頭活兒", "手頭活計"),
        ("這活兒", "這活計"),
        ("幹活兒", "做工"),
        ("粗活兒", "粗活"),
        ("細活兒", "細活"),
        ("活兒", "活計"),
        ("攞貨", "提貨"),
        ("幫你攞", "幫你提"),
        ("去攞", "去提"),
        ("刮得真勁", "刮得真猛"),
        ("刮得勁", "刮得猛"),
        ("風勁得", "風猛得"),
        ("得勁", "痛快"),
        ("防水組件", "防浪油布"),
        ("組件", "構件"),
        ("拖鞋", "便鞋"),
        ("火柴", "火籤"),
        ("起子", "旋鑿"),
        ("鋼筆", "硬筆"),
        ("風衣", "防雨長衫"),
        ("黑板", "漆板"),
        ("分紅", "分成"),
        ("放大鏡", "助視鏡"),
        ("淋浴", "沖沐"),
        ("沐浴露", "沐浴膏"),
        ("牙刷", "潔齒刷"),
        ("濕巾", "濕布巾"),
        ("平底鍋", "平底煎鍋"),
        ("面膜", "敷面膏"),
        ("點點頭，說著也往林道走，", ""),
        ("點點頭，說著", ""),
        ("搖搖頭，說著", ""),
        ("湧得厲，", "湧得厲害，"),
        ("吹得厲，", "吹得厲害，"),
        ("刮得厲，", "刮得厲害，"),
        ("下得厲，", "下得厲害，"),
        ("被濕透了", "濕透了"),
        ("被濕透", "濕透了"),
        ("海風吹得碼頭發騷", "海風吹得碼頭呼呼作響"),
        ("發騷", "沙沙作響"),
        ("估摸著", "估量著"),
        ("估摸", "估量"),
        ("堵車", "車隊受阻"),
        ("門口松了", "門口鬆了"),
        ("松了", "鬆了"),
        ("這會子", "這陣子"),
        ("怕不是要", "怕是要"),
        ("真拼", "真賣力"),
        ("早先才", "早先"),
        ("護身符", "辟瘴香囊"),
        ("平安符", "祈安木牌"),
        ("符紙", "草藥帖"),
        ("買一送一", "買一贈一"),
        ("買二送一", "買二贈一"),
        ("概不退換", "離手不退"),
        ("怕氧化變色", "怕受潮走色"),
        ("怕氧化", "怕受潮走色"),
        ("氧化變色", "受潮走色"),
        ("防氧化", "防鏽走色"),
        ("氧化", "受潮走色"),
        ("過個週", "周轉幾日"),
        ("過這週", "過這幾日"),
        ("過個星期", "周轉幾日"),
        ("縮工時", "提早收工"),
        ("咱們", "我們"),
        ("咱就", "我們就"),
        ("咱也", "我們也"),
        ("咱這", "我們這"),
        ("咱那", "我們那"),
        ("咱先", "我們先"),
        ("咱去", "我們去"),
        ("咱倆", "我們倆"),
        ("咱", "我們"),
        ("缺啥", "缺什麼"),
        ("啥味", "什麼氣味"),
        ("啥異動", "什麼動靜"),
        ("啥大隻的", "什麼大條的"),
        ("啥動靜", "什麼動靜"),
        ("見啥", "見什麼"),
        ("見到啥", "見到什麼"),
        ("釣到啥", "釣到什麼"),
        ("嗅到啥", "嗅到什麼"),
        ("聞到啥", "聞到什麼"),
        ("啥", "什麼"),
        ("傘骨廠", "傘骨作坊"),
        ("釀酒廠", "釀酒坊"),
        ("酒廠", "酒坊"),
        ("鐵廠", "鐵作坊"),
        ("木廠", "木作坊"),
        ("船廠", "船塢"),
        ("磚廠", "磚窯"),
        ("染廠", "染坊"),
        ("織廠", "織坊"),
        ("嫩得能當柴燒", "長得肥嫩茂密"),
        ("你去得開嗎", "你走得開嗎"),
        ("去得開嗎", "走得開嗎"),
        ("昨兒", "昨日"),
        ("今兒個", "今日"),
        ("今兒", "今日"),
        ("麻利", "俐落"),
        ("天氣預報", "風候天色"),
        ("天氣走勢", "天候走向"),
        ("農業商行", "農產行"),
        ("戰備糧草總儲倉", "常平糧草總大庫"),
        ("雙輪大車廠", "雙輪大車坊"),
        ("大車廠", "大車坊"),
        ("車廠", "大車坊"),
        ("廠棚", "工棚"),
        ("太拼", "太操勞"),
        ("秦氏會計事務所", "秦氏算學居"),
        ("會計事務所", "算學舍"),
        ("商務會計教室", "百工商算學舍"),
        ("會計師", "商算大匠"),
        ("編輯助理", "校書生"),
        ("紀錄員", "抄錄書生"),
        ("護理員", "採藥人"),
        ("外務員", "押運夥計"),
        ("民事調解員", "民務通事"),
        ("陸老師宿舍", "蒙學陸先生舍"),
        ("老師宿舍", "先生舍"),
        ("寒水玉保鮮櫃", "寒水玉儲食石櫃"),
        ("保鮮櫃", "冰鑑櫃"),
        ("顯擺", "誇耀"),
        ("憋屈", "怨悶"),
        ("扯淡", "閒磕牙"),
        ("鍋爐房", "炭火氣釜房"),
        ("遣散費", "退役安家銀"),
        ("收費站", "關卡"),
        ("信用額度", "賒帳限額"),
        ("破罐破摔", "自暴自棄"),
        ("店長", "掌事"),
        ("保溫桶", "溫存桶"),
        ("急救折疊帆布擔架", "急救折疊帆布抬架"),
        ("擔架", "抬架"),
        ("重卡大車", "載重大車"),
        ("重卡", "載重大車"),
        ("門都沒有", "休想"),
        ("不行拉倒", "不成便罷"),
        ("拉倒", "作罷"),
        ("吃香喝辣", "錦衣玉食"),
        ("賭波", "博弈"),
        ("澡堂子", "浴堂"),
        ("套近乎", "套交情"),
        ("胡說八道", "信口胡謅"),
        ("瞎扯", "閒磕牙"),
        ("瞎折騰", "白費工夫"),
        ("吹牛", "誇口"),
        ("好勒", "好咧"),
        ("教室", "講舍"),
        ("八卦鏡", "避邪銅鏡"),
        ("發燒", "染寒熱"),
        ("防感冒草藥", "辟寒草藥"),
        ("系學生", "生員"),
        ("巡防課", "巡防處"),
        ("研究員", "治學士"),
        ("得瑟", "張揚"),
        ("嘚瑟", "張揚"),
        ("窩囊", "怯弱"),
        ("磨嘰", "拖延"),
        ("海防警衛狼犬幼崽", "海防哨衛狼犬幼崽"),
        ("警衛", "哨衛"),
        ("值班室", "當值房"),
        ("病床", "病榻"),
        ("瓷磚", "陶磚"),
        ("雙翼翻板日曆發條座鐘", "雙翼翻板星曆發條座鐘"),
        ("日曆", "星曆"),
        ("車廂", "車輿"),
        ("煞車", "制動"),
        ("明信片", "風物圖卷"),
        ("輪胎", "車輪箍"),
        ("井蓋", "渠石蓋"),
        ("幫浦", "唧筒"),
        ("藍圖", "工造圖樣"),
        ("糊弄", "矇混"),
        ("擦屁股", "充作揩拭紙"),
        ("背黑鍋", "代人受過"),
        ("窮光蛋", "赤貧窮漢"),
        ("搞清楚", "弄個明白"),
        ("清潔工", "淨街夫"),
        ("科研", "考工研習"),
        ("流體力學", "格物流水"),
        ("發條鐵青蛙", "發條木雀"),
        ("剪板機", "裁板機"),
        ("下課", "歇功"),
        ("非本公司員工禁止進入區", "非本號夥計禁止入內區"),
        ("非員工禁止進入", "非夥計禁止入內"),
        ("員工", "夥計"),
    ] {
        if s.contains(bad) {
            s = s.replace(bad, good);
        }
    }
    if s.contains('样') {
        s = s.replace('样', "樣");
    }
    if s.contains('晒') {
        s = s.replace('晒', "曬");
    }
    if s.contains("不？") || s.contains("不?") {
        s = rewrite_sentence_final_bu_question(&s);
    }
    s.trim().to_string()
}

fn rewrite_sentence_final_bu_question(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    for (i, &ch) in chars.iter().enumerate() {
        if ch == '不' {
            let next = chars.get(i + 1).copied();
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            if matches!(next, Some('？' | '?')) && prev != Some('豈') {
                out.push('嗎');
                continue;
            }
        }
        out.push(ch);
    }
    out
}

/// 單句品質閘（對齊既有 `qualityGateNpcLine`）；通過則 `(true, "")`。
pub fn quality_gate_npc_line(line: &str, max_runes: i32) -> (bool, &'static str) {
    let max = if max_runes <= 0 {
        QUALITY_GATE_DEFAULT_MAX_RUNES
    } else {
        max_runes
    };
    let s = line.trim();
    if s.is_empty() {
        return (false, "empty");
    }
    if s.contains('\u{fffd}') {
        return (false, "unicode_garbage");
    }
    let t = s.trim();
    let t_runes: Vec<char> = t.chars().collect();
    if t == "{" || t == "}" || (t.starts_with('{') && t_runes.len() <= 4) {
        return (false, "json_residue");
    }
    let low = s.to_lowercase();
    for leak in ["npc_", "entity_", "room_", "wt_", "mat_", "grid:", "scene_stub"] {
        if low.contains(leak) {
            return (false, "leaked_id");
        }
    }
    if low.contains("street") || low.contains("zone") {
        return (false, "leaked_id");
    }
    for forbidden in [
        "小市",
        "短市",
        "剛才",
        "剛剛",
        "還有",
        "資料庫",
        "聖塔",
        "110v",
        "110V",
        "念場",
        "富態",
        "城外惡地",
        "沃土風",
        "非核冬天",
        "低飽和微光",
        "玉女克全真",
        "庖丁解牛",
        "一力降十會",
        "十會解一力",
        "五常運動鏈",
        "換招破綻窗",
        "語意字根",
        "骨脈解構",
        "差官",
        "報表",
        "導能鎂晶",
        "銀票",
        "走你",
        "一趨",
        "松快",
        "回来",
        "這样",
        "晒乾",
        "采草",
        "老伴兒",
        "涼白開",
        "加班",
        "加了班",
        "加個班",
        "上班",
        "下班",
        "報備",
        "報修",
        "那伙人",
        "黑沼關隘",
        "鹽田澳",
        "半個鐘",
        "陽春麵",
        "開水",
        "防爆",
        "黑沼林",
        "西南原",
        "白塔山麓",
        "北聖山",
        "西北礦帶",
        "方才",
        "快遞",
        "外帶",
        "打卡",
        "時辰",
        "客官",
        "小二",
        "掌櫃",
        "銅錢",
        "文錢",
        "銀兩",
        "碎銀",
        "玄鐵",
        "骨相",
        "成相",
        "字印",
        "詞元",
        "書肆",
        "茶肆",
        "酒肆",
        "客棧",
        "保鏢",
        "鏢局",
        "老鏢頭",
        "排班",
        "發霉",
        "倒霉",
        "霉味",
        "老少咸宜",
        "衙門",
        "老夫",
        "哥們",
        "宗門",
        "神識",
        "適才",
        "丹田",
        "官衙",
        "差役",
        "住店",
        "銀幣",
        "銅幣",
        "百金",
        "十銀",
        "五十銅",
        "玉幣",
        "防暴",
        "效率",
        "數據",
        "系統",
        "體適能夏令營",
        "仰臥起坐板",
        "導彈",
        "水泥",
        "猎",
        "账",
        "调试",
        "靈氣",
        "靈力",
        "修煉",
        "神仙",
        "幫派",
        "門派",
        "江湖",
        "鏢資",
        "機器人",
        "瑜伽",
        "輻射",
        "法術",
        "晶片",
        "電池",
        "工廠",
        "兵工廠",
        "跟人說了會兒話",
        "螺絲",
        "扳手",
        "蒸汽鍛打機",
        "機油",
        "化學反應",
        "少林寺",
        "舍利子",
        "瘋人院",
        "訂書機",
        "奇點神教",
        "奇點世界",
        "魔法",
        "法杖",
        "牧師",
        "教堂",
        "鈔票",
        "五十金",
        "不鏽鋼",
        "不銹鋼",
        "尼龍",
        "塑膠",
        "塑料",
        "錢幣",
        "金幣",
        "鎂鈔",
        "氧化",
        "刺猬",
        "啥用",
        "過個週",
        "縮工時",
        "傘骨廠",
        "酒廠",
        "釀酒廠",
        "咱",
        "啥",
        "天氣預報",
        "待會兒",
        "一會兒",
        "這會兒",
        "那會兒",
        "玩意兒",
        "門兒清",
        "麻利",
        "車廠",
        "廠棚",
        "襯衫",
        "活兒",
        "大夥兒",
        "事兒",
        "勁兒",
        "門兒",
        "味兒",
        "昨兒",
        "今兒",
        "老花鏡",
        "眼鏡",
        "鋸木廠",
        "造船廠",
        "精煉廠",
        "會計事務所",
        "商務會計教室",
        "會計師",
        "編輯助理",
        "紀錄員",
        "護理員",
        "外務員",
        "民事調解員",
        "陸老師宿舍",
        "老師宿舍",
        "保鮮櫃",
        "顯擺",
        "憋屈",
        "扯淡",
        "八卦鏡",
        "發燒",
        "防感冒草藥",
        "系學生",
        "巡防課",
        "研究員",
        "得瑟",
        "嘚瑟",
        "窩囊",
        "磨嘰",
        "教室",
        "鍋爐房",
        "遣散費",
        "收費站",
        "信用額度",
        "破罐破摔",
        "店長",
        "保溫桶",
        "擔架",
        "重卡",
        "門都沒有",
        "拉倒",
        "吃香喝辣",
        "賭波",
        "澡堂子",
        "套近乎",
        "胡說八道",
        "瞎扯",
        "瞎折騰",
        "吹牛",
        "好勒",
        "警衛",
        "值班室",
        "病床",
        "瓷磚",
        "日曆",
        "車廂",
        "煞車",
        "明信片",
        "輪胎",
        "井蓋",
        "幫浦",
        "藍圖",
        "糊弄",
        "擦屁股",
        "背黑鍋",
        "窮光蛋",
        "搞清楚",
        "清潔工",
        "科研",
        "流體力學",
        "發條鐵青蛙",
        "剪板機",
        "下課",
        "員工",
    ] {
        if s.contains(forbidden) || low.contains(forbidden) {
            return (false, "forbidden_term");
        }
    }
    if s.chars().any(|c| c.is_ascii_alphabetic()) {
        return (false, "ascii_alpha_residue");
    }
    if s.contains("---") || s.contains("```") || s.contains("\n\n") {
        return (false, "markdown_residue");
    }
    let n = s.chars().count();
    if n <= 2 {
        return (false, "too_short");
    }
    if n > max as usize {
        return (false, "too_long");
    }
    for b in gametext::quality_gate_bad_meta() {
        if !b.is_empty() && s.contains(b.as_str()) {
            return (false, "meta_text");
        }
    }
    let gopen: char = '\u{300c}';
    let gclose: char = '\u{300d}';
    if s.matches(gopen).count() > 2 || s.matches(gclose).count() > 2 {
        return (false, "nested_quotes");
    }
    if has_long_repeat(s) {
        return (false, "repetition");
    }
    if looks_like_npc_npc_narration_line(s) {
        return (false, "suspicious_narration");
    }
    (true, "")
}

fn looks_like_npc_npc_narration_line(s: &str) -> bool {
    let r: Vec<char> = s.chars().collect();
    if r.len() < 8 || !s.ends_with('。') {
        return false;
    }
    let hints = gametext::dialogue_markers();
    for ch in r {
        if hints.contains(ch) {
            return false;
        }
    }
    true
}

#[must_use]
pub fn should_skip_npc_npc_summary_update(
    old_summary: &str,
    new_summary: &str,
    line_a: &str,
    line_b: &str,
) -> bool {
    let old_summary = old_summary.trim();
    let new_summary = new_summary.trim();
    if old_summary.is_empty() {
        return false;
    }
    if new_summary == old_summary {
        return true;
    }
    if rune_lcs_similarity(old_summary, new_summary) >= 0.7 {
        return true;
    }
    let la = line_a.trim();
    let lb = line_b.trim();
    if !la.is_empty() && rune_lcs_similarity(old_summary, la) >= 0.7 {
        return true;
    }
    if !lb.is_empty() && rune_lcs_similarity(old_summary, lb) >= 0.7 {
        return true;
    }
    false
}

#[must_use]
pub fn dedupe_social_context_lines(lines: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let key = line.to_lowercase();
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);
        out.push(line.to_string());
    }
    out
}

fn has_long_repeat(s: &str) -> bool {
    let r: Vec<char> = s.chars().collect();
    if r.len() < 8 {
        return false;
    }
    let mut i = 0;
    while i + 3 < r.len() {
        let ch = r[i];
        if ch == ' ' {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < r.len() && r[j] == ch {
            j += 1;
        }
        if j - i >= 4 {
            return true;
        }
        i += 1;
    }
    false
}

#[must_use]
pub fn merge_thread_anchors(old: &[String], fresh: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::<String>::new();
    let mut out = Vec::new();
    for v in old.iter().chain(fresh.iter()) {
        let v = v.trim();
        if v.is_empty() || !seen.insert(v.to_string()) {
            continue;
        }
        out.push(v.to_string());
    }
    if out.len() > 4 {
        out = out[out.len() - 4..].to_vec();
    }
    out
}

/// 錨與台詞是否矛盾（對齊既有 `anchorConsistencyCheck`）。
pub fn anchor_consistency_check(existing: &[String], line_a: &str, line_b: &str) -> (bool, String) {
    if existing.is_empty() {
        return (true, String::new());
    }
    let joined = format!("{} {}", line_a.trim(), line_b.trim())
        .trim()
        .to_string();
    if joined.is_empty() {
        return (true, String::new());
    }
    let negators = gametext::anchor_negators();
    for a in existing {
        let a = a.trim();
        if a.is_empty() {
            continue;
        }
        let runes: Vec<char> = a.chars().collect();
        for i in 0..runes.len().saturating_sub(1) {
            let k: String = runes[i..i + 2].iter().collect();
            if !joined.contains(&k) {
                continue;
            }
            for n in &negators {
                let nk = format!("{n}{k}");
                let kn = format!("{k}{n}");
                if joined.contains(&nk) || joined.contains(&kn) {
                    return (false, format!("anchor negated: {k}"));
                }
            }
        }
    }
    (true, String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_anchors_cap_four() {
        let old = vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()];
        let fresh = vec!["f".into()];
        let m = merge_thread_anchors(&old, &fresh);
        assert_eq!(m.len(), 4);
        assert_eq!(m.last().map(String::as_str), Some("f"));
    }

    #[test]
    fn dedupe_lines_casefold() {
        let v = vec!["Hi".into(), "hi".into(), "x".into()];
        assert_eq!(dedupe_social_context_lines(&v).len(), 2);
    }

    #[test]
    fn clean_npc_line_removes_stage_directions_and_substitutes() {
        let line = "靠在案頭喘了口氣，抬眼問：『羅藥師，天氣這样悶，草都晒乾了，你去那邊采草要留神！』";
        assert_eq!(
            clean_npc_line(line),
            "羅藥師，天氣這樣悶，草都曬乾了，你去那邊採草要留神！"
        );
        let line2 = "點點頭：『喝口涼白開，昨兒給老伴兒帶了點心。』";
        assert_eq!(
            clean_npc_line(line2),
            "喝口涼茶水，昨日給老伴帶了點心。"
        );
    }

    #[test]
    fn clean_npc_line_rewrites_sentence_final_bu_question() {
        let line = "你手裡有老蘆草綑不？我這邊缺點，能換點砥石不？";
        assert_eq!(
            clean_npc_line(line),
            "你手裡有老蘆草綑嗎？我這邊缺點，能換點砥石嗎？"
        );
        let keep_qi = "這豈不？那能行不？";
        assert_eq!(
            clean_npc_line(keep_qi),
            "這豈不？那能行嗎？"
        );
    }

    #[test]
    fn clean_npc_line_converts_simplified_and_dialect() {
        let line = "喝完白開，吃啥呢？這會兒店里沒啥事兒。";
        assert_eq!(
            clean_npc_line(line),
            "喝完涼茶水，吃什麼呢？眼下店裡沒什麼事情。"
        );
    }

    #[test]
    fn clean_npc_line_round28_replacements() {
        // 伙 group-noun fixes vs keep 伙食/伙夫/開伙/動伙
        assert_eq!(
            clean_npc_line("那伙人跟這伙人是一伙人，同伙合伙入伙散伙，大伙兒跟大伙還有伙計，小伙子與小伙，好家伙老家伙真家伙。"),
            "那夥人跟這夥人是一夥人，同夥合夥入夥散夥，大夥跟大夥還有夥計，年輕人與年輕人，好傢伙老傢伙真傢伙。"
        );
        assert_eq!(
            clean_npc_line("今晚伙食由伙夫張羅，廚房準備開伙，不可隨意動伙。"),
            "今晚伙食由伙夫張羅，廚房準備開伙，不可隨意動伙。"
        );

        // Modern/corporate slang & clock-hour replacements
        assert_eq!(
            clean_npc_line("都加班了，巡守隊正加班，碼頭加班趕工，工坊也在加班。"),
            "都連夜加派人手了，巡守隊正連夜巡守，碼頭連夜趕工，工坊也在連夜趕工。"
        );
        assert_eq!(
            clean_npc_line("這事得先報備，壞掉的石磨要報修。"),
            "這事得先知會一聲，壞掉的石磨要送去修整。"
        );
        assert_eq!(
            clean_npc_line("這叫效率，我們講究效率，做事效率高，講究效率。"),
            "這叫手腳俐落，我們講究手腳俐落，做事手腳俐落，講究手腳俐落。"
        );
        assert_eq!(
            clean_npc_line("等了半個鐘頭，歇了半個鐘，走了一個鐘頭，花了兩個鐘頭，耗了幾個鐘頭。"),
            "等了半晌，歇了半晌，走了一時半刻，花了好一陣子，耗了良久。"
        );
        assert_eq!(
            clean_npc_line("等了半個時辰，歇了半晌，走了一個時辰，花了兩個時辰，耗了幾個時辰。"),
            "等了半晌，歇了半晌，走了一時半刻，花了好一陣子，耗了良久。"
        );

        // Legacy non-canonical settlement/zone phrases
        assert_eq!(
            clean_npc_line("守在黑沼關隘，路過鹽田澳，走過青籠關，剛從青籠回，往青籠走，打算去青籠，還在青籠待著。"),
            "守在下營，路過烏石鼻，走過下營，早先從下營回，往下營走，打算去下營，還在下營待著。"
        );
        assert_eq!(
            clean_npc_line("巨藻淺灣林木封山，巨藻淺灣封山，淺澤泥口林木封山，淺澤泥口封山，黑石灣林木封山，驚濤岩岸林木封山。"),
            "巨藻淺灣休漁養息，巨藻淺灣休漁養息，淺澤泥口休漁養息，淺澤泥口休漁養息，黑石灣休漁養息，驚濤岩岸休漁養息。"
        );

        // 人 + 剛/才 and 剛/才 before temporal/verb phrases
        assert_eq!(
            clean_npc_line("人剛在屋裡，人才在屋裡，人剛從外頭來，人才從外頭來，人剛去海邊，人才去海邊，人剛到門口，人才到門口，人剛回收拾，人才回收拾，人剛把刀放下，人才把刀放下。"),
            "人早先在屋裡，人早先在屋裡，人早先從外頭來，人早先從外頭來，人早先去海邊，人早先去海邊，人早先到門口，人早先到門口，人早先回收拾，人早先回收拾，人早先把刀放下，人早先把刀放下。"
        );
        assert_eq!(
            clean_npc_line("剛那陣風大，才那陣浪急，剛這陣雨歇了，才這陣天青了。"),
            "早先那陣風大，早先那陣浪急，眼下這陣雨歇了，眼下這陣天青了。"
        );
        assert_eq!(
            clean_npc_line("剛還在說話，才還在喝茶，剛還說要走，才還說不急，剛還見著他，才還見著他，剛還看著呢，才還看著呢，剛還想問，才還想問，剛還聽見聲音，才還聽見聲音。"),
            "早先還在說話，早先還在喝茶，早先還說要走，早先還說不急，早先還見著他，早先還見著他，早先還看著呢，早先還看著呢，早先還想問，早先還想問，早先還聽見聲音，早先還聽見聲音。"
        );

        // Round 55 dialogue cleanup & regression tests
        assert_eq!(
            clean_npc_line("這桐油麻線新進的，乾得快，我才拿去修了兩把船鏟"),
            "這桐油麻線新進的，乾得快，我早先拿去修了兩把船鏟"
        );
        assert_eq!(
            clean_npc_line("嗯，我這邊才從霜林口帶回來一捆老蘆草，乾燥得剛好，正好補補藥房的料。"),
            "嗯，我這邊早先從霜林口帶回來一捆老蘆草，乾燥得正好，可補補藥房的料。"
        );
        assert_eq!(
            clean_npc_line("霧還継著呢，我才看過潮汐表"),
            "霧還續著呢，我早先看過潮信木牌"
        );
        assert_eq!(
            clean_npc_line("才摘了兩把，我才踩過，我才翻過那片密林，你才說黃冠廷"),
            "新摘了兩把，我早先踩過，我早先翻過那片密林，你早先說黃冠廷"
        );
        assert_eq!(
            clean_npc_line("省吃儉用才買下來的"),
            "省吃儉用才買下來的"
        );
        assert_eq!(
            clean_npc_line("花了大力氣才打出來的"),
            "花了大力氣才打出來的"
        );
        assert_eq!(
            clean_npc_line("天才買主吧"),
            "天才買主吧"
        );
        assert_eq!(
            clean_npc_line("走半晌才到，這才拿正眼瞧你，這才看向你，人才出眾，患難之中，才見真情"),
            "走半晌才到，這才拿正眼瞧你，這才看向你，人才出眾，患難之中，才見真情"
        );
    }

    #[test]
    fn quality_gate_round28_forbidden_terms() {
        assert_eq!(quality_gate_npc_line("今晚加班趕工", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("這事要向隊長報備", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("壞掉的水車送去報修", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("那伙人不知道哪來的", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("守在黑沼關隘前", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("走到鹽田澳歇腳", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("歇了半個鐘再走", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("大家手腳麻利點", 40), (false, "forbidden_term"));
    }

    #[test]
    fn clean_npc_line_round29_replacements() {
        // Work shift and overtime terms
        assert_eq!(
            clean_npc_line("昨夜加了班，今晚加個班，再加點班，準時上個班，等等下個班，上下班辛苦，準時上班，準時下班。"),
            "昨夜連夜巡守，今晚連夜趕工，再多趕一陣工，準時當值上工，等等收工歇息，上工收工辛苦，準時上工，準時收工。"
        );

        // Dialect sun, armor, noodles, water, places
        assert_eq!(
            clean_npc_line("大陽頭曬得慌，防爆防風斗笠，來碗陽春粗麵或陽春麵，水缸裡的開水，喝口開水，來碗滾開水、熱開水、溫開水、冷開水、白開水或開水。"),
            "大日頭曬得慌，防風斗笠，來碗清湯粗麵或清湯粗麵，水缸裡的熱茶水，喝口熱茶水，來碗滾水、熱茶水、溫茶水、涼茶水、溫茶水或熱茶水。"
        );

        // Emergence, frost, fog, wind, snow, rain
        assert_eq!(
            clean_npc_line("草芽剛冒出來，才冒出來，剛冒出，才冒出，剛結的霜，才結的霜，剛結霜，才結霜，剛起的霧，才起的霧，剛起的風，才起的風，剛下的雪，才下的雪，剛下的雨，才下的雨。"),
            "草芽新凝出來，新凝出來，初冒出，初冒出，新結的霜，新結的霜，新結霜，新結霜，新起的霧，新起的霧，新起的風，新起的風，新落的雪，新落的雪，新落的雨，新落的雨。"
        );

        // Mountain closure and zone name replacements
        assert_eq!(
            clean_npc_line("聽說古河林礦脈封山了，去半嶺避雪，走入黑沼林澤。"),
            "聽說古河巨木林封山休養了，去半嶺鎮避雪，走入水尾寮。"
        );
    }

    #[test]
    fn quality_gate_round29_forbidden_terms() {
        assert_eq!(quality_gate_npc_line("昨夜加了班巡守", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("今晚加個班趕工", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("準時上班當差", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("何時下班歇息", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("吃碗陽春麵暖身", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("舀一碗開水解渴", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("帶著防爆防風斗笠", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("穿越黑沼林去鎮上", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("從西南原出發", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("白塔山麓風勢強勁", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("北聖山雪積很深", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("前往西北礦帶探採", 40), (false, "forbidden_term"));
    }

    #[test]
    fn quality_gate_round41_forbidden_terms() {
        assert_eq!(quality_gate_npc_line("這事要向隊長報備", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("壞掉的水車送去報修", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("明日排班如何調配", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("木架上的乾糧發霉了", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("今日出門真是倒霉", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("庫房角落有股霉味", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("這道甜湯老少咸宜", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("這事得報給衙門處置", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("老夫今日有些乏了", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("幾位哥們一塊走吧", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("他是哪個宗門的弟子", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("動用神識探查周圍", 40), (false, "forbidden_term"));
    }

    #[test]
    fn quality_gate_round42_forbidden_terms() {
        assert_eq!(quality_gate_npc_line("天地至純清氣奔湧成靈氣", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("體內靈力充盈激盪", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("閉關修煉數月方成", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("莫非是天上神仙下凡", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("島上各大幫派互不相讓", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("這事被捲入門派紛爭", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("行走江湖全憑義氣", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("這次押送的鏢資甚巨", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("不知何處來的機器人", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("在此處練習瑜伽調息", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("深坑中殘留強烈輻射", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("施展某種玄奇法術", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("這塊晶片不知有何用處", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("儀器裡的電池快耗盡了", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("前方是一座廢棄工廠", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("據說那裡曾是兵工廠", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("跟人說了會兒話", 40), (false, "forbidden_term"));
    }

    #[test]
    fn quality_gate_round44_forbidden_terms() {
        assert_eq!(quality_gate_npc_line("用螺絲固定住木料", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("拿扳手把零件旋緊", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("工坊裡有蒸汽鍛打機", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("軸承上抹了些機油", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("試管中產生化學反應", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("據傳來自少林寺武僧", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("古塔下供奉著舍利子", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("那人被關進了瘋人院", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("桌案上放著金屬訂書機", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("信奉奇點神教的信徒", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("此處便是奇點世界", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("懂得施展玄奇魔法", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("手中握著鑲寶法杖", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("那位牧師正在祝禱", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("鎮上的教堂鐘聲響起", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("懷裡揣著幾張舊鈔票", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("這批貨物作價五十金", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("用不鏽鋼製成水壺", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("不銹鋼打造的刀刃", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("結實的尼龍繩索", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("塑膠水桶放在一旁", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("塑料盒子裝著零件", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("口袋裡的錢幣叮噹響", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("賞了幾枚金幣", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("數著手裡的鎂鈔", 40), (false, "forbidden_term"));
    }

    #[test]
    fn clean_npc_line_round54_replacements() {
        assert_eq!(
            clean_npc_line("剛在西口買了新絲弦，才在西口買了新絲弦，剛買了鹽包，才買了鹽包，剛賣了舊犁，才賣了舊犁。"),
            "早先在西口買了新絲弦，早先在西口買了新絲弦，新買了鹽包，新買了鹽包，早先賣了舊犁，早先賣了舊犁。"
        );
        assert_eq!(
            clean_npc_line("吃完面去煮面，吃面配熱湯面，端上一碗面。"),
            "吃完麵去煮麵，吃麵配熱湯麵，端上一碗麵。"
        );
        assert_eq!(
            clean_npc_line("剛才在西口買了新絲弦，方才買了鹽包。"),
            "早先在西口買了新絲弦，早先買了鹽包。"
        );
        assert_eq!(
            clean_npc_line("天才買主吧"),
            "天才買主吧"
        );
        assert_eq!(
            clean_npc_line("花了大力氣才打出來的純淨金屬"),
            "花了大力氣才打出來的純淨金屬"
        );
        assert_eq!(
            clean_npc_line("大戰了三百回合才打下來的"),
            "大戰了三百回合才打下來的"
        );
        assert_eq!(
            clean_npc_line("省吃儉用才買下來的"),
            "省吃儉用才買下來的"
        );
    }

    #[test]
    fn clean_npc_line_round56_cai_guo_replacements() {
        assert_eq!(
            replace_clause_initial_temporal_cai("才過瞭望所，樹影動得怪"),
            "早先過瞭望所，樹影動得怪"
        );
        assert_eq!(
            replace_clause_initial_temporal_cai("等風浪停了，才過得去"),
            "等風浪停了，才過得去"
        );
        assert_eq!(
            clean_npc_line("才過瞭望所，樹影動得怪"),
            "早先過瞭望所，樹影動得怪"
        );
        assert_eq!(
            clean_npc_line("等風浪停了，才過得去"),
            "等風浪停了，才過得去"
        );
    }

    #[test]
    fn test_round57_soak_dialogue_temporal_cai_and_colloquial_guards() {
        assert_eq!(
            clean_npc_line("嗯，我也早先挑了點菜，早先在水尾寮聽說"),
            "嗯，我也先挑了點菜，早先在水尾寮聽說"
        );
        assert_eq!(
            replace_clause_initial_temporal_cai("是他才吃完麵，又才歇下"),
            "是他早先吃完麵，又早先歇下"
        );
        assert_eq!(
            replace_clause_initial_temporal_cai("我這邊也才燉好湯，才舀了兩碗"),
            "我這邊也新燉好湯，早先舀了兩碗"
        );
        assert_eq!(
            replace_clause_initial_temporal_cai("他也才繫好纜繩，才順妥船帆"),
            "他也新繫好纜繩，早先順妥船帆"
        );
        assert_eq!(
            replace_clause_initial_temporal_cai("大家也都才檢查完工具，才踏進工棚"),
            "大家也都早先檢查完工具，早先踏進工棚"
        );
        assert_eq!(
            replace_clause_initial_temporal_cai("才點了兩盞燈，夜色深了"),
            "新點了兩盞燈，夜色深了"
        );
        assert_eq!(
            replace_clause_initial_temporal_cai("雨腳才落，海霧才散"),
            "雨腳初落，海霧初散"
        );
        assert_eq!(
            replace_clause_initial_temporal_cai("水力才轉，風勢才轉"),
            "水力初轉，風勢初轉"
        );
        assert_eq!(
            clean_npc_line("歇了會兒，曬了會兒，坐了會兒，等了會兒，聊了會兒"),
            "歇了片刻，曬了片刻，坐了片刻，等了片刻，聊了片刻"
        );
        assert_eq!(
            clean_npc_line("好一會兒沒見，等了一會兒，這會兒有空"),
            "好一陣子沒見，等了片刻，眼下有空"
        );
        assert_eq!(
            clean_npc_line("船工活兒也是手頭活兒，這活兒真不好幹，幹活兒得賣力，粗活兒細活兒都要做"),
            "船工活計也是手頭活計，這活計真不好幹，做工得賣力，粗活細活都要做"
        );
        assert_eq!(
            clean_npc_line("去攞貨還是幫你攞？刮得真勁，刮得勁，風勁得站不住，幹得得勁"),
            "去提貨還是幫你提？刮得真猛，刮得猛，風猛得站不住，幹得痛快"
        );
        assert_eq!(
            clean_npc_line("這批防水組件包含組件若乾，換上拖鞋拿著火柴起子和鋼筆"),
            "這批防浪油布包含構件若乾，換上便鞋拿著火籤旋鑿和硬筆"
        );
        assert_eq!(
            clean_npc_line("穿上風衣在黑板上記錄分紅，用放大鏡看著淋浴沐浴露牙刷濕巾平底鍋面膜"),
            "穿上防雨長衫在漆板上記錄分成，用助視鏡看著沖沐沐浴膏潔齒刷濕布巾平底煎鍋敷面膏"
        );
        assert_eq!(
            clean_npc_line("剛吃過飯，剛歇下，剛舀水，剛燉肉，剛檢修，剛繫繩，剛順手，剛踏出門"),
            "早先吃過飯，早先歇下，早先舀水，新燉肉，早先檢修，新繫繩，早先順手，早先踏出門"
        );
        assert_eq!(
            clean_npc_line("才吃完麵，才歇下，才舀水，才燉湯，才檢查完，才繫纜，才點燈，才押運進城"),
            "早先吃完麵，早先歇下，早先舀水，新燉湯，早先檢查完，新繫纜，新點燈，早先押運進城"
        );
        assert_eq!(
            clean_npc_line("晨霧才散，濃霧才散，山霧才散，雨勢才收"),
            "晨霧初散，濃霧初散，山霧初散，雨勢新收"
        );
        assert_eq!(
            clean_npc_line("我這邊也才收穫完，你那邊也才收拾完"),
            "我這邊也新收穫完，你那邊也新收拾完"
        );
        assert_eq!(
            clean_npc_line("他那邊也才整理完，咱這邊也才盤點完，這邊也才打掃完，那邊也才修補完"),
            "他那邊也早先整理完，我們這邊也早先盤點完，這邊也新打掃完，那邊也新修補完"
        );
        assert_eq!(
            clean_npc_line("人也才到，也都才吃，大家也都才散"),
            "人也初到，也都早先吃，大家也都初散"
        );
        assert_eq!(
            clean_npc_line("剛在西口買了新絲弦，才在西口買了新絲弦，剛在客舍前歇了會兒"),
            "早先在西口買了新絲弦，早先在西口買了新絲弦，早先在客舍前歇了片刻"
        );
        assert_eq!(
            clean_npc_line("才買好，才賣妥，才修齊，才打畢"),
            "新買好，早先賣妥，新修齊，新打畢"
        );
        assert_eq!(
            clean_npc_line("雨腳才落下來，海霧才散開，水力才轉動"),
            "雨腳初落下來，海霧初散開，水力初轉動"
        );
        assert_eq!(
            clean_npc_line("穿著風衣拿鋼筆，用平底鍋熱油"),
            "穿著防雨長衫拿硬筆，用平底煎鍋熱油"
        );
        assert_eq!(
            clean_npc_line("去攞防水組件，換了拖鞋洗了淋浴抹了面膜"),
            "去提防浪油布，換了便鞋洗了沖沐抹了敷面膏"
        );
    }

    #[test]
    fn test_round58_soak_dialogue_and_colloquial_guards() {
        assert_eq!(
            clean_npc_line("今早海霧湧得厲，衣衫都被濕透了"),
            "今早海霧湧得厲害，衣衫都濕透了"
        );
        assert_eq!(
            clean_npc_line("今早海霧湧得厲，衣衫被濕透"),
            "今早海霧湧得厲害，衣衫濕透了"
        );
        assert_eq!(
            clean_npc_line("才喘完氣，才割完草"),
            "早先喘完氣，早先割完草"
        );
        assert_eq!(
            clean_npc_line("我才拾了兩束，我才確認風路，我才量過"),
            "我早先拾了兩束，我早先確認風路，我早先量過"
        );
        assert_eq!(
            clean_npc_line("橋頭日晷才轉了，才泡過湯"),
            "橋頭日晷初轉了，早先泡過湯"
        );
        assert_eq!(
            clean_npc_line("我才用麻藤綁過邊材，我才剝的竹篾"),
            "我早先用麻藤綁過邊材，我新剝的竹篾"
        );
        assert_eq!(
            clean_npc_line("風向儀才轉了三度，這水澤邊的蘆草才割過"),
            "風向儀初轉了三度，這水澤邊的蘆草早先割過"
        );
        assert_eq!(
            clean_npc_line("我才瞧見，這邊的刀才磨好，我手上才補了貨"),
            "我早先瞧見，這邊的刀新磨好，我手上新補了貨"
        );
        assert_eq!(
            clean_npc_line("前頭獵團才押著石虎皮回來"),
            "前頭獵團早先押著石虎皮回來"
        );
        assert_eq!(
            clean_npc_line("點點頭，說著也往林道走，天色暗了"),
            "天色暗了"
        );
        assert_eq!(
            clean_npc_line("估摸著海風吹得碼頭發騷，門口松了，這會子怕不是要堵車，真拼"),
            "估量著海風吹得碼頭呼呼作響，門口鬆了，這陣子怕是要車隊受阻，真賣力"
        );
        assert_eq!(
            clean_npc_line("剛喘，剛割，剛拾，剛確，剛量，剛泡，剛用，剛剝，剛瞧，剛磨，剛補"),
            "早先喘，早先割，早先拾，早先確，早先量，早先泡，早先用，新剝，早先瞧，新磨，新補"
        );
        assert_eq!(
            clean_npc_line("早先才到，護身符，平安符，符紙，買一送一，買二送一，概不退換"),
            "早先到，辟瘴香囊，祈安木牌，草藥帖，買一贈一，買二贈一，離手不退"
        );
    }

    #[test]
    fn test_round60_soak_dialogue_gang_name_and_slang_guards() {
        assert_eq!(
            clean_npc_line("我聽說蕭景耀剛走過，說是貨倉見底"),
            "我聽說蕭景耀早先走過，說是貨倉見底"
        );
        assert_eq!(
            clean_npc_line("這邊水氣重，你那狗嗅到啥味沒？我才盯到一串腳印"),
            "這邊水氣重，你那狗嗅到什麼氣味沒？我早先盯到一串腳印"
        );
        assert_eq!(
            clean_npc_line("我才從農業商行回來，鋪子還缺啥？我也才去看了"),
            "我早先從農產行回來，鋪子還缺什麼？我也先去看了"
        );
        assert_eq!(
            clean_npc_line("才過山茶籽叢那邊，路上碰見誰有現成的，咱就換著用"),
            "早先過山茶籽叢那邊，路上碰見誰有現成的，我們就換著用"
        );
        assert_eq!(
            clean_npc_line("橋頭日晷初轉過卯時，傘骨廠的活計都快收尾了"),
            "橋頭日晷初轉過卯時，傘骨作坊的活計都快收尾了"
        );
        assert_eq!(
            clean_npc_line("這溪旁草灘新長的野草，嫩得能當柴燒，你去得開嗎？"),
            "這溪旁草灘新長的野草，長得肥嫩茂密，你走得開嗎？"
        );
        assert_eq!(quality_gate_npc_line("去傘骨廠拿零件", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("那是私家酒廠", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("剛建好的釀酒廠", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("咱今天去不去", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("吃啥好呢", 40), (false, "forbidden_term"));
    }

    #[test]
    fn test_round61_soak_dialogue_erhua_and_weather_guards() {
        assert_eq!(
            clean_npc_line("河灣那頭才傳說有隻雲豹進了密林，我才喝完水"),
            "河灣那頭早先傳說有隻雲豹進了密林，我早先喝完水"
        );
        assert_eq!(
            clean_npc_line("海風才轉涼，再等會兒就能封甕，你有看天氣預報嗎？"),
            "海風初轉涼，再稍後就能封甕，你有看風候天色嗎？"
        );
        assert_eq!(
            clean_npc_line("我才在修船塢閉眼喘了會兒，海味食堂才幫人熬了鍋海苔粥"),
            "我早先在修船塢閉眼喘了片刻，海味食堂早先幫人熬了鍋海苔粥"
        );
        assert_eq!(
            clean_npc_line("才從車廠出來，貨車還在廠棚裡待著，手腳麻利點"),
            "早先從大車坊出來，貨車還在工棚裡待著，手腳俐落點"
        );
        assert_eq!(
            clean_npc_line("這玩意兒我門兒清，待會兒大夥兒一塊兒去農業商行"),
            "這物件我門路清，待會大夥一同去農產行"
        );
        assert_eq!(quality_gate_npc_line("你有看天氣預報嗎", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("再待會兒就走", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("等一會兒再來", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("這會兒正忙著", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("那會兒風浪大", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("這玩意兒真稀罕", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("我對這裡門兒清", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("做事手腳麻利", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("他在大車廠做工", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("貨物堆在廠棚下", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("穿著白色襯衫", 40), (false, "forbidden_term"));
    }

    #[test]
    fn test_round62_clean_and_quality_gate_root_erhua_and_factory_terms() {
        assert_eq!(
            clean_npc_line("我正往貨運行去，修鞍子的活兒才完，也不覺得冷"),
            "我正往貨運行去，修鞍子的活計方歇，也不覺得冷"
        );
        assert_eq!(
            clean_npc_line("早先從水尾寮回來，我才盤貨也聽說這事"),
            "早先從水尾寮回來，我早先盤貨也聽說這事"
        );
        assert_eq!(
            clean_npc_line(
                "這風真帶勁，木雕跟活的似的，巡守隊正加強 ，大夥兒戴上老花鏡去鋸木廠與造船廠"
            ),
            "這風真猛烈，木雕如同活物一般，巡守隊正加強巡防，大夥戴上助視鏡去鋸木坊與造船塢"
        );
        assert_eq!(quality_gate_npc_line("修鞍子的活兒才完", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("大夥兒都在工棚", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("這點事兒不算啥", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("使點勁兒拉繩子", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("找不到門兒路", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("聞到一股味兒", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("昨兒才下過雨", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("今兒日頭正好", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("戴上老花鏡看帳", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("配一副眼鏡", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("鋸木廠送來木料", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("造船廠在整修碼頭", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("精煉廠出產鎂錠", 40), (false, "forbidden_term"));
    }

    #[test]
    fn test_round63_clean_and_quality_gate_modern_titles_and_colloquial_terms() {
        assert_eq!(
            clean_npc_line("他剛從秦氏會計事務所出來，正往商務會計教室去"),
            "他早先從秦氏算學居出來，正往百工商算學舍去"
        );
        assert_eq!(
            clean_npc_line("會計師與編輯助理、紀錄員、護理員、外務員都在會計事務所"),
            "商算大匠與校書生、抄錄書生、採藥人、押運夥計都在算學舍"
        );
        assert_eq!(
            clean_npc_line("民事調解員在陸老師宿舍旁，老師宿舍裡放著寒水玉保鮮櫃與保鮮櫃"),
            "民務通事在蒙學陸先生舍旁，先生舍裡放著寒水玉儲食石櫃與冰鑑櫃"
        );
        assert_eq!(
            clean_npc_line("別顯擺又憋屈，扯淡說八卦鏡能治發燒，還賣防感冒草藥"),
            "別誇耀又怨悶，閒磕牙說避邪銅鏡能治染寒熱，還賣辟寒草藥"
        );
        assert_eq!(
            clean_npc_line("系學生與巡防課研究員別得瑟嘚瑟，窩囊又磨嘰"),
            "生員與巡防處治學士別張揚張揚，怯弱又拖延"
        );
        assert_eq!(quality_gate_npc_line("會計事務所正在查帳", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("去商務會計教室上課", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("會計師正在算帳", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("編輯助理負責校稿", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("紀錄員正在謄寫", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("護理員正在換藥", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("外務員出發押車", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("民事調解員調解紛爭", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("陸老師宿舍在書院旁", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("老師宿舍整潔清幽", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("保鮮櫃裡有鮮肉", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("別在人前顯擺", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("心裡覺得憋屈", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("淨說些扯淡的話", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("門頭掛著八卦鏡", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("孩子夜裡發燒了", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("採了些防感冒草藥", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("他是中文系學生", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("巡防課派人查驗", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("研究員在整理文獻", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("別得瑟個沒完", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("嘚瑟什麼勁兒", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("樣子真夠窩囊", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("辦事別太磨嘰", 40), (false, "forbidden_term"));
    }

    #[test]
    fn test_round64_clean_and_quality_gate_vocab_terms() {
        assert_eq!(
            clean_npc_line("這望遠鏡架才從老船工那邊換的，我背架裡還留著WT-0893北嶺釘鞋"),
            "這望遠鏡架早先從老船工那邊換的，我背架裡還留著北嶺釘鞋"
        );
        assert_eq!(
            clean_npc_line("這望遠鏡架才從老船工那邊換的，我背架裡還留著-0893北嶺釘鞋"),
            "這望遠鏡架早先從老船工那邊換的，我背架裡還留著北嶺釘鞋"
        );
        assert_eq!(
            clean_npc_line("店長在鍋爐房拿了保溫桶與擔架，別在教室吹牛套近乎"),
            "掌事在炭火氣釜房拿了溫存桶與抬架，別在講舍誇口套交情"
        );
        assert_eq!(quality_gate_npc_line("教室裡正在講課", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("鍋爐房熱氣騰騰", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("發放遣散費", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("收費站正在查驗", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("信用額度用盡了", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("別破罐破摔", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("店長正在對帳", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("保溫桶裝滿熱湯", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("急救擔架抬了過來", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("重卡開過來了", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("門都沒有的事", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("不行就拉倒吧", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("想吃香喝辣的", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("他又去賭波了", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("澡堂子剛換水", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("休想跟我套近乎", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("全在胡說八道", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("淨在那裡瞎扯", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("別再瞎折騰了", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("整天就知道吹牛", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("好勒馬上就來", 40), (false, "forbidden_term"));
    }

    #[test]
    fn test_round65_clean_and_quality_gate_vocab_terms() {
        assert_eq!(
            clean_npc_line("海鹽堆邊的六分儀才被吹歪了，你有帶-1024嗎？"),
            "海鹽堆邊的六分儀早先被吹歪了，你有帶六分儀嗎？"
        );
        assert_eq!(
            clean_npc_line("六分儀的游標才調好"),
            "六分儀的游標早先調好"
        );
        assert_eq!(
            clean_npc_line("你有帶WT-1024嗎？"),
            "你有帶六分儀嗎？"
        );
        assert_eq!(
            clean_npc_line("這件WT-0893要收妥"),
            "這件硬木藤背架要收妥"
        );
        assert_eq!(
            clean_npc_line("海防警衛狼犬幼崽在值班室病床邊，陶磚上的雙翼翻板日曆發條座鐘響了"),
            "海防哨衛狼犬幼崽在當值房病榻邊，陶磚上的雙翼翻板星曆發條座鐘響了"
        );
        assert_eq!(
            clean_npc_line("車廂煞車壞了，明信片上印著輪胎井蓋幫浦藍圖"),
            "車輿制動壞了，風物圖卷上印著車輪箍渠石蓋唧筒工造圖樣"
        );
        assert_eq!(
            clean_npc_line("別想糊弄擦屁股背黑鍋，窮光蛋搞清楚清潔工科研流體力學，發條鐵青蛙剪板機下課給員工"),
            "別想矇混充作揩拭紙代人受過，赤貧窮漢弄個明白淨街夫考工研習格物流水，發條木雀裁板機歇功給夥計"
        );
        assert_eq!(quality_gate_npc_line("警衛在門口", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("值班室很冷", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("病床上躺著傷者", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("地上鋪了瓷磚", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("日曆翻了一頁", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("車廂裡坐滿了人", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("煞車失靈了", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("寄了一張明信片", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("輪胎磨損嚴重", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("井蓋鬆脫了", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("幫浦抽不出水", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("看了建築藍圖", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("別想糊弄過去", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("幫人擦屁股", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("替他背黑鍋", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("他是個窮光蛋", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("先把事情搞清楚", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("清潔工在掃地", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("專注科研工作", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("研究流體力學", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("桌上有發條鐵青蛙", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("操作剪板機", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("等下課再說", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("他是工坊員工", 40), (false, "forbidden_term"));
    }
}
