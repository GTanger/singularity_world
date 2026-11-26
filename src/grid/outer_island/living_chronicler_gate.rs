//! 背景活史官（Living Chronicler）：`Q4-ColdFusion-27B` 事實卡輸出正體正規化與機械防幻覺閘門。
//!
//! 嚴守：零憑空捏造引號（含直彎引號）、零數字竄改或漏年份金額、零跨子句施受顛倒、零內陸濱海生態錯置、零玩家真名洩漏。

use std::collections::{BTreeMap, BTreeSet};

use super::causal_arcs::solar_year_zh;
use super::dialogue_llm::validate_clean_in_world_reply;
use super::living_chronicler::{ChronicleFactPacket, num_zh};

const AG_TOK: &str = "\u{e001}";
const PA_TOK: &str = "\u{e002}";

/// 將本機模型輸出之常見簡體字與大陸慣用語轉為標準台灣正體（全程使用 `\u{XXXX}` 轉義以嚴守源碼零簡體紅線；不含 `\u{91cc}` 防誤傷「鄰里」）
pub fn normalize_chronicler_output(raw: &str) -> String {
    let after_think = if let Some((_, after)) = raw.rsplit_once("</think>") {
        after.trim()
    } else if let Some((_, after)) = raw.rsplit_once("<|thought|>") {
        after.trim()
    } else {
        raw.trim()
    };
    let opencc_converted = try_opencc_s2twp(after_think);
    let base_text = opencc_converted.as_deref().unwrap_or(after_think);
    const PAIRS: &[(char, char)] = &[
        ('\u{4e0e}', '與'),
        ('\u{8be5}', '該'),
        ('\u{65f6}', '時'),
        ('\u{70b9}', '點'),
        ('\u{7edd}', '絕'),
        ('\u{5e76}', '並'),
        ('\u{94c1}', '鐵'),
        ('\u{4e3a}', '為'),
        ('\u{5355}', '單'),
        ('\u{73b0}', '現'),
        ('\u{8ba9}', '讓'),
        ('\u{7ecf}', '經'),
        ('\u{7eed}', '續'),
        ('\u{62a5}', '報'),
        ('\u{8bc6}', '識'),
        ('\u{8bb0}', '記'),
        ('\u{6743}', '權'),
        ('\u{9a8c}', '驗'),
        ('\u{8bc1}', '證'),
        ('\u{636e}', '據'),
        ('\u{8ba4}', '認'),
        ('\u{5f00}', '開'),
        ('\u{53d1}', '發'),
        ('\u{540e}', '後'),
        ('\u{5907}', '備'),
        ('\u{5e94}', '應'),
        ('\u{96be}', '難'),
        ('\u{8bae}', '議'),
        ('\u{73af}', '環'),
        ('\u{8fdb}', '進'),
        ('\u{6001}', '態'),
        ('\u{5e93}', '庫'),
        ('\u{7edf}', '統'),
        ('\u{95e8}', '門'),
        ('\u{6237}', '戶'),
        ('\u{94dc}', '銅'),
        ('\u{5e01}', '幣'),
        ('\u{9547}', '鎮'),
        ('\u{5c9b}', '島'),
        ('\u{9e25}', '鷗'),
        ('\u{98ce}', '風'),
        ('\u{7089}', '爐'),
        ('\u{9524}', '錘'),
        ('\u{953b}', '鍛'),
        ('\u{8239}', '船'),
        ('\u{5ce1}', '峽'),
        ('\u{5c81}', '歲'),
        ('\u{5386}', '曆'),
        ('\u{9633}', '陽'),
        ('\u{5b9e}', '實'),
        ('\u{8d26}', '帳'),
        ('\u{8fd8}', '還'),
        ('\u{8fd9}', '這'),
        ('\u{4e2a}', '個'),
        ('\u{4eec}', '們'),
        ('\u{6765}', '來'),
        ('\u{8fc7}', '過'),
        ('\u{5bf9}', '對'),
        ('\u{8bf4}', '說'),
        ('\u{8fde}', '連'),
        ('\u{89c1}', '見'),
        ('\u{4eb2}', '親'),
        ('\u{65e7}', '舊'),
        ('\u{4e24}', '兩'),
        ('\u{51e0}', '幾'),
        ('\u{6ca1}', '沒'),
        ('\u{8fb9}', '邊'),
        ('\u{5934}', '頭'),
        ('\u{58f0}', '聲'),
        ('\u{54cd}', '響'),
        ('\u{949f}', '鐘'),
        ('\u{5f7b}', '徹'),
        ('\u{53f7}', '號'),
        ('\u{4e07}', '萬'),
        ('\u{4e1c}', '東'),
        ('\u{4e61}', '鄉'),
        ('\u{4e66}', '書'),
        ('\u{4e70}', '買'),
        ('\u{4e71}', '亂'),
        ('\u{4e89}', '爭'),
        ('\u{4e91}', '雲'),
        ('\u{4ea7}', '產'),
        ('\u{4f17}', '眾'),
        ('\u{4f24}', '傷'),
        ('\u{4f20}', '傳'),
        ('\u{4f53}', '體'),
        ('\u{4f59}', '餘'),
        ('\u{503a}', '債'),
        ('\u{507f}', '償'),
        ('\u{5174}', '興'),
        ('\u{519c}', '農'),
        ('\u{51b2}', '衝'),
        ('\u{51b5}', '況'),
        ('\u{51c9}', '涼'),
        ('\u{51c6}', '準'),
        ('\u{521b}', '創'),
        ('\u{52b3}', '勞'),
        ('\u{52bf}', '勢'),
        ('\u{53cc}', '雙'),
        ('\u{53f9}', '嘆'),
        ('\u{542c}', '聽'),
        ('\u{56f4}', '圍'),
        ('\u{56fe}', '圖'),
        ('\u{575a}', '堅'),
        ('\u{5757}', '塊'),
        ('\u{574f}', '壞'),
        ('\u{5899}', '牆'),
        ('\u{58f3}', '殼'),
        ('\u{5904}', '處'),
        ('\u{591f}', '夠'),
        ('\u{5939}', '夾'),
        ('\u{5bfb}', '尋'),
        ('\u{5c18}', '塵'),
        ('\u{5c42}', '層'),
        ('\u{5cad}', '嶺'),
        ('\u{5e26}', '帶'),
        ('\u{5e2e}', '幫'),
        ('\u{5f02}', '異'),
        ('\u{5f20}', '張'),
        ('\u{5f3a}', '強'),
        ('\u{5f53}', '當'),
        ('\u{5f52}', '歸'),
        ('\u{6000}', '懷'),
        ('\u{60ac}', '懸'),
        ('\u{60ca}', '驚'),
        ('\u{62a4}', '護'),
        ('\u{62ac}', '抬'),
        ('\u{62c5}', '擔'),
        ('\u{6321}', '擋'),
        ('\u{635f}', '損'),
        ('\u{6362}', '換'),
        ('\u{6446}', '擺'),
        ('\u{6447}', '搖'),
        ('\u{6491}', '撐'),
        ('\u{654c}', '敵'),
        ('\u{65ad}', '斷'),
        ('\u{65e0}', '無'),
        ('\u{663e}', '顯'),
        ('\u{6652}', '曬'),
        ('\u{6653}', '曉'),
        ('\u{6740}', '殺'),
        ('\u{6742}', '雜'),
        ('\u{6761}', '條'),
        ('\u{67dc}', '櫃'),
        ('\u{6811}', '樹'),
        ('\u{6837}', '樣'),
        ('\u{6865}', '橋'),
        ('\u{6868}', '槳'),
        ('\u{697c}', '樓'),
        ('\u{6b22}', '歡'),
        ('\u{6b8b}', '殘'),
        ('\u{6c14}', '氣'),
        ('\u{6c47}', '匯'),
        ('\u{6c64}', '湯'),
        ('\u{6c9f}', '溝'),
        ('\u{6ca7}', '滄'),
        ('\u{6d45}', '淺'),
        ('\u{6d53}', '濃'),
        ('\u{6da6}', '潤'),
        ('\u{6da8}', '漲'),
        ('\u{6e10}', '漸'),
        ('\u{6e14}', '漁'),
        ('\u{6e29}', '溫'),
        ('\u{6e7e}', '灣'),
        ('\u{6e7f}', '濕'),
        ('\u{6ee1}', '滿'),
        ('\u{6ee9}', '灘'),
        ('\u{70df}', '煙'),
        ('\u{70e7}', '燒'),
        ('\u{70ed}', '熱'),
        ('\u{7275}', '牽'),
        ('\u{72ec}', '獨'),
        ('\u{730e}', '獵'),
        ('\u{76b1}', '皺'),
        ('\u{76d8}', '盤'),
        ('\u{76d6}', '蓋'),
        ('\u{77ff}', '礦'),
        ('\u{7816}', '磚'),
        ('\u{781a}', '硯'),
        ('\u{788e}', '碎'),
        ('\u{79cd}', '種'),
        ('\u{79ef}', '積'),
        ('\u{79f0}', '稱'),
        ('\u{7a33}', '穩'),
        ('\u{7a77}', '窮'),
        ('\u{7a91}', '窯'),
        ('\u{7b14}', '筆'),
        ('\u{7b3c}', '籠'),
        ('\u{7b51}', '築'),
        ('\u{7b79}', '籌'),
        ('\u{7b80}', '簡'),
        ('\u{7cae}', '糧'),
        ('\u{7d27}', '緊'),
        ('\u{7ea2}', '紅'),
        ('\u{7ea6}', '約'),
        ('\u{7eb8}', '紙'),
        ('\u{7ebf}', '線'),
        ('\u{7ec3}', '練'),
        ('\u{7ec4}', '組'),
        ('\u{7ec6}', '細'),
        ('\u{7ec8}', '終'),
        ('\u{7ed3}', '結'),
        ('\u{7ed5}', '繞'),
        ('\u{7ed9}', '給'),
        ('\u{7edc}', '絡'),
        ('\u{7ee7}', '繼'),
        ('\u{7ef3}', '繩'),
        ('\u{7ef4}', '維'),
        ('\u{7eff}', '綠'),
        ('\u{7f13}', '緩'),
        ('\u{7f16}', '編'),
        ('\u{7f1d}', '縫'),
        ('\u{7f34}', '繳'),
        ('\u{7f51}', '網'),
        ('\u{7f62}', '罷'),
        ('\u{804c}', '職'),
        ('\u{8054}', '聯'),
        ('\u{80a0}', '腸'),
        ('\u{80c6}', '膽'),
        ('\u{80dc}', '勝'),
        ('\u{8109}', '脈'),
        ('\u{810f}', '髒'),
        ('\u{8111}', '腦'),
        ('\u{811a}', '腳'),
        ('\u{8131}', '脫'),
        ('\u{8138}', '臉'),
        ('\u{814a}', '臘'),
        ('\u{8231}', '艙'),
        ('\u{8270}', '艱'),
        ('\u{8282}', '節'),
        ('\u{82cd}', '蒼'),
        ('\u{82cf}', '蘇'),
        ('\u{836f}', '藥'),
        ('\u{8425}', '營'),
        ('\u{84dd}', '藍'),
        ('\u{866b}', '蟲'),
        ('\u{867e}', '蝦'),
        ('\u{8680}', '蝕'),
        ('\u{8865}', '補'),
        ('\u{88c5}', '裝'),
        ('\u{88c2}', '裂'),
        ('\u{89c2}', '觀'),
        ('\u{89c4}', '規'),
        ('\u{89c6}', '視'),
        ('\u{89c9}', '覺'),
        ('\u{89e6}', '觸'),
        ('\u{8ba1}', '計'),
        ('\u{8ba2}', '訂'),
        ('\u{8ba8}', '討'),
        ('\u{8bad}', '訓'),
        ('\u{8baf}', '訊'),
        ('\u{8bb2}', '講'),
        ('\u{8bb8}', '許'),
        ('\u{8bba}', '論'),
        ('\u{8bbe}', '設'),
        ('\u{8bbf}', '訪'),
        ('\u{8bc9}', '訴'),
        ('\u{8bcd}', '詞'),
        ('\u{8bd1}', '譯'),
        ('\u{8bd5}', '試'),
        ('\u{8bd7}', '詩'),
        ('\u{8bda}', '誠'),
        ('\u{8bdd}', '話'),
        ('\u{8be2}', '詢'),
        ('\u{8be6}', '詳'),
        ('\u{8bed}', '語'),
        ('\u{8bef}', '誤'),
        ('\u{8bf7}', '請'),
        ('\u{8bf8}', '諸'),
        ('\u{8bfa}', '諾'),
        ('\u{8bfb}', '讀'),
        ('\u{8c01}', '誰'),
        ('\u{8c03}', '調'),
        ('\u{8c08}', '談'),
        ('\u{8c0b}', '謀'),
        ('\u{8c13}', '謂'),
        ('\u{8d1d}', '貝'),
        ('\u{8d1f}', '負'),
        ('\u{8d22}', '財'),
        ('\u{8d23}', '責'),
        ('\u{8d24}', '賢'),
        ('\u{8d25}', '敗'),
        ('\u{8d27}', '貨'),
        ('\u{8d28}', '質'),
        ('\u{8d29}', '販'),
        ('\u{8d2b}', '貧'),
        ('\u{8d2d}', '購'),
        ('\u{8d2f}', '貫'),
        ('\u{8d34}', '貼'),
        ('\u{8d35}', '貴'),
        ('\u{8d37}', '貸'),
        ('\u{8d38}', '貿'),
        ('\u{8d39}', '費'),
        ('\u{8d3a}', '賀'),
        ('\u{8d3c}', '賊'),
        ('\u{8d44}', '資'),
        ('\u{8d4a}', '賒'),
        ('\u{8d4e}', '贖'),
        ('\u{8d4f}', '賞'),
        ('\u{8d54}', '賠'),
        ('\u{8d56}', '賴'),
        ('\u{8d5a}', '賺'),
        ('\u{8d5e}', '贊'),
        ('\u{8d62}', '贏'),
        ('\u{8d76}', '趕'),
        ('\u{8d8b}', '趨'),
        ('\u{8dc3}', '躍'),
        ('\u{8e2a}', '蹤'),
        ('\u{8f66}', '車'),
        ('\u{8f68}', '軌'),
        ('\u{8f6c}', '轉'),
        ('\u{8f6e}', '輪'),
        ('\u{8f6f}', '軟'),
        ('\u{8f70}', '轟'),
        ('\u{8f74}', '軸'),
        ('\u{8f7b}', '輕'),
        ('\u{8f7d}', '載'),
        ('\u{8f83}', '較'),
        ('\u{8f86}', '輛'),
        ('\u{8f88}', '輩'),
        ('\u{8f93}', '輸'),
        ('\u{8f9e}', '辭'),
        ('\u{8fbe}', '達'),
        ('\u{8fc1}', '遷'),
        ('\u{8fc8}', '邁'),
        ('\u{8fd0}', '運'),
        ('\u{8fdc}', '遠'),
        ('\u{8fdd}', '違'),
        ('\u{8fdf}', '遲'),
        ('\u{8ff9}', '跡'),
        ('\u{9002}', '適'),
        ('\u{9009}', '選'),
        ('\u{9012}', '遞'),
        ('\u{903c}', '逼'),
        ('\u{904d}', '遍'),
        ('\u{9057}', '遺'),
        ('\u{9065}', '遙'),
        ('\u{90bb}', '鄰'),
        ('\u{917f}', '釀'),
        ('\u{91ca}', '釋'),
        ('\u{9274}', '鑑'),
        ('\u{9488}', '針'),
        ('\u{9489}', '釘'),
        ('\u{9493}', '釣'),
        ('\u{94a2}', '鋼'),
        ('\u{94a5}', '鑰'),
        ('\u{94a9}', '鉤'),
        ('\u{94b1}', '錢'),
        ('\u{94b3}', '鉗'),
        ('\u{94bb}', '鑽'),
        ('\u{94c5}', '鉛'),
        ('\u{94e0}', '鎧'),
        ('\u{94f2}', '鏟'),
        ('\u{94f6}', '銀'),
        ('\u{94f8}', '鑄'),
        ('\u{94fa}', '鋪'),
        ('\u{94fe}', '鏈'),
        ('\u{9500}', '銷'),
        ('\u{9501}', '鎖'),
        ('\u{9504}', '鋤'),
        ('\u{9505}', '鍋'),
        ('\u{9508}', '鏽'),
        ('\u{950b}', '鋒'),
        ('\u{9519}', '錯'),
        ('\u{9521}', '錫'),
        ('\u{9523}', '鑼'),
        ('\u{9526}', '錦'),
        ('\u{952d}', '錠'),
        ('\u{952e}', '鍵'),
        ('\u{952f}', '鋸'),
        ('\u{955c}', '鏡'),
        ('\u{957f}', '長'),
        ('\u{95ea}', '閃'),
        ('\u{95ed}', '閉'),
        ('\u{95ee}', '問'),
        ('\u{95ef}', '闖'),
        ('\u{95f2}', '閒'),
        ('\u{95f4}', '間'),
        ('\u{95f7}', '悶'),
        ('\u{95f9}', '鬧'),
        ('\u{95fb}', '聞'),
        ('\u{9605}', '閱'),
        ('\u{9614}', '闊'),
        ('\u{961f}', '隊'),
        ('\u{9634}', '陰'),
        ('\u{9635}', '陣'),
        ('\u{9636}', '階'),
        ('\u{9645}', '際'),
        ('\u{9646}', '陸'),
        ('\u{9648}', '陳'),
        ('\u{9669}', '險'),
        ('\u{9690}', '隱'),
        ('\u{96fe}', '霧'),
        ('\u{9759}', '靜'),
        ('\u{9798}', '鞘'),
        ('\u{97e7}', '韌'),
        ('\u{9875}', '頁'),
        ('\u{9876}', '頂'),
        ('\u{9879}', '項'),
        ('\u{987a}', '順'),
        ('\u{987b}', '須'),
        ('\u{987d}', '頑'),
        ('\u{987e}', '顧'),
        ('\u{987f}', '頓'),
        ('\u{9884}', '預'),
        ('\u{9886}', '領'),
        ('\u{9887}', '頗'),
        ('\u{9888}', '頸'),
        ('\u{9891}', '頻'),
        ('\u{9897}', '顆'),
        ('\u{9898}', '題'),
        ('\u{989d}', '額'),
        ('\u{989c}', '顏'),
        ('\u{98d8}', '飄'),
        ('\u{98de}', '飛'),
        ('\u{996d}', '飯'),
        ('\u{996e}', '飲'),
        ('\u{9971}', '飽'),
        ('\u{997c}', '餅'),
        ('\u{997f}', '餓'),
        ('\u{9986}', '館'),
        ('\u{9a6c}', '馬'),
        ('\u{9a6e}', '馱'),
        ('\u{9a71}', '驅'),
        ('\u{9a73}', '駁'),
        ('\u{9a76}', '駛'),
        ('\u{9a7b}', '駐'),
        ('\u{9a91}', '騎'),
        ('\u{9a97}', '騙'),
        ('\u{9c7c}', '魚'),
        ('\u{9c9c}', '鮮'),
        ('\u{9cb8}', '鯨'),
        ('\u{9cde}', '鱗'),
        ('\u{9e1f}', '鳥'),
        ('\u{9e21}', '雞'),
        ('\u{9e23}', '鳴'),
        ('\u{9e26}', '鴉'),
        ('\u{9e70}', '鷹'),
        ('\u{9ea6}', '麥'),
        ('\u{9ec4}', '黃'),
        ('\u{9f50}', '齊'),
        ('\u{9f7f}', '齒'),
        ('\u{9f99}', '龍'),
        ('\u{9a5b}', '驛'),
        ('\u{4ece}', '從'),
        ('\u{5173}', '關'),
        ('\u{4e3e}', '舉'),
        ('\u{4f1a}', '會'),
        ('\u{5251}', '劍'),
        ('\u{5e08}', '師'),
        ('\u{53c2}', '參'),
        ('\u{6784}', '構'),
        ('\u{6d4b}', '測'),
        ('\u{76d1}', '監'),
        ('\u{542f}', '啟'),
        ('\u{5199}', '寫'),
        ('\u{7ea7}', '級'),
        ('\u{7c7b}', '類'),
        ('\u{5417}', '嗎'),
        ('\u{5356}', '賣'),
        ('\u{513f}', '兒'),
        ('\u{5e10}', '帳'),
        ('\u{5fc6}', '憶'),
        ('\u{7237}', '爺'),
        ('\u{4e48}', '麼'),
    ];
    let out = if let Some(conv) = opencc_converted {
        conv
    } else {
        let map: BTreeMap<char, char> = PAIRS.iter().copied().collect();
        let mut fb = String::with_capacity(base_text.len());
        for ch in base_text.chars() {
            fb.push(map.get(&ch).copied().unwrap_or(ch));
        }
        fb
    };
    out.replace("鄰裡", "鄰里")
        .replace("平方裡", "平方里")
        .replace("萬裡", "萬里")
        .replace("百裡", "百里")
        .replace("十裡", "十里")
        .replace("數裡", "數里")
        .replace("幾裡", "幾里")
        .replace("曆經", "歷經")
        .replace("曆時", "歷時")
        .replace("經曆", "經歷")
        .replace("來曆", "來歷")
        .replace("閱曆", "閱歷")
        .replace('巖', "岩")
        .replace("村里", "村裡")
        .replace("手里", "手裡")
        .replace("心里", "心裡")
        .replace("屋里", "屋裡")
        .replace("巷里", "巷裡")
        .replace("鎮里", "鎮裡")
        .replace("店里", "店裡")
        .replace("鋪里", "鋪裡")
        .replace("坑里", "坑裡")
        .replace("林里", "林裡")
        .replace("海里", "海裡")
        .replace("風里", "風裡")
        .replace("雨里", "雨裡")
        .replace("夜里", "夜裡")
        .replace("這里", "這裡")
        .replace("那里", "那裡")
        .replace("哪里", "哪裡")
        .replace("咱們", "我們")
        .replace("喝西北風", "挨餓受凍")
        .replace("一碼歸一碼", "一筆歸一筆")
        .replace("乾淨利落", "乾淨俐落")
        .replace("幫襯", "照應")
        .replace("拮據", "緊絀")
        .replace("拮\u{636e}", "緊絀")
        .replace("抽冷子", "冷不防")
        .replace("扎進", "踏入")
        .replace("——", "，")
}

/// 優先透過系統 `libopencc.so.1.1`（`s2tw.json` 正體字形，不誤改「消息→訊息」）執行完整台灣正體轉換（純安全行程管線，`unsafe = 0`）
fn try_opencc_s2twp(input: &str) -> Option<String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    if input.is_empty()
        || !std::path::Path::new("/usr/lib/x86_64-linux-gnu/libopencc.so.1.1").exists()
        || !std::path::Path::new("/usr/share/opencc/s2tw.json").exists()
    {
        return None;
    }
    const PY_OPENCC: &str = "import ctypes,sys;lib=ctypes.CDLL('libopencc.so.1.1');lib.opencc_open.argtypes=[ctypes.c_char_p];lib.opencc_open.restype=ctypes.c_void_p;lib.opencc_convert_utf8.argtypes=[ctypes.c_void_p,ctypes.c_char_p,ctypes.c_size_t];lib.opencc_convert_utf8.restype=ctypes.c_void_p;lib.opencc_convert_utf8_free.argtypes=[ctypes.c_void_p];lib.opencc_close.argtypes=[ctypes.c_void_p];cc=lib.opencc_open(b'/usr/share/opencc/s2tw.json');b=sys.stdin.buffer.read();p=lib.opencc_convert_utf8(cc,b,len(b));sys.stdout.buffer.write(ctypes.string_at(p));lib.opencc_convert_utf8_free(p);lib.opencc_close(cc)";
    let mut child = Command::new("python3")
        .args(["-c", PY_OPENCC])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input.as_bytes());
    }
    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn extract_quoted_spans(s: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    if s.contains('"') || s.contains('\'') {
        out.insert("__ASCII_QUOTE__".to_string());
    }
    for (open, close) in [
        ('「', '」'),
        ('『', '』'),
        ('\u{201c}', '\u{201d}'),
        ('\u{2018}', '\u{2019}'),
    ] {
        let mut rest = s;
        while let Some(i) = rest.find(open) {
            let after = &rest[i + open.len_utf8()..];
            if let Some(j) = after.find(close) {
                out.insert(after[..j].to_string());
                rest = &after[j + close.len_utf8()..];
            } else {
                out.insert("__UNCLOSED_QUOTE__".to_string());
                break;
            }
        }
    }
    out
}

fn is_cjk_numeral(c: char) -> bool {
    matches!(
        c,
        '〇' | '零'
            | '一'
            | '二'
            | '兩'
            | '三'
            | '四'
            | '五'
            | '六'
            | '七'
            | '八'
            | '九'
            | '十'
            | '百'
            | '千'
            | '萬'
    )
}

fn scrub_idiomatic_numerals(s: &str) -> String {
    let mut work = s.to_string();
    const IDIOMS: &[&str] = &[
        "千萬",
        "一同",
        "一舉",
        "一時",
        "一樁",
        "一道",
        "一雙",
        "一片",
        "一筆",
        "一口",
        "一回",
        "一番",
        "一旁",
        "一體",
        "一格",
        "一分",
        "一層",
        "一碗",
        "一身",
        "一生",
        "一度",
        "一些",
        "一樣",
        "一直",
        "一塊",
        "一處",
        "一間",
        "一棟",
        "一把",
        "一件",
        "一頭",
        "一隻",
        "一盞",
        "兩家",
        "兩人",
        "兩個人",
        "兩側",
        "兩岸",
        "百年",
        "多年",
        "連日",
        "當日",
        "往日",
        "昔日",
        "今日",
        "雙雙",
        "十天半月",
    ];
    for idm in IDIOMS {
        work = work.replace(idm, "  ");
    }
    work
}

fn extract_quantity_tokens(s: &str) -> BTreeSet<String> {
    let scrubbed = scrub_idiomatic_numerals(s);
    let mut out = BTreeSet::new();
    let chars: Vec<char> = scrubbed.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            out.insert(chars[start..i].iter().collect());
            continue;
        }
        if is_cjk_numeral(chars[i]) {
            let start = i;
            while i < chars.len() && is_cjk_numeral(chars[i]) {
                i += 1;
            }
            let tok: String = chars[start..i].iter().collect();
            let has_unit = matches!(
                chars.get(i).copied(),
                Some('年' | '日' | '枚' | '文' | '成' | '棟' | '處' | '號' | '戶' | '響' | '鐘')
            );
            if tok.chars().count() >= 2 || has_unit {
                out.insert(tok.replace('兩', "二"));
            }
            continue;
        }
        i += 1;
    }
    out
}

fn subject_pos_in_clause(clause: &str, tok: &str) -> Option<usize> {
    let idx = clause.find(tok)?;
    let prev_ch = clause[..idx].chars().next_back();
    if matches!(
        prev_ch,
        Some(
            '見' | '助'
                | '救'
                | '下'
                | '替'
                | '向'
                | '對'
                | '與'
                | '同'
                | '跟'
                | '遭'
                | '被'
                | '將'
                | '把'
                | '給'
                | '予'
        )
    ) {
        None
    } else {
        Some(idx)
    }
}

fn tokenize_pair_roles(text: &str, ag: &str, pa: &str) -> String {
    let clean_base = text.replace(['\u{e001}', '\u{e002}'], "");
    if ag.is_empty() && pa.is_empty() {
        return clean_base;
    }
    if ag.is_empty() {
        return clean_base.replace(pa, PA_TOK);
    }
    if pa.is_empty() || ag == pa {
        return clean_base.replace(ag, AG_TOK);
    }
    if ag.len() >= pa.len() {
        clean_base.replace(ag, AG_TOK).replace(pa, PA_TOK)
    } else {
        clean_base.replace(pa, PA_TOK).replace(ag, AG_TOK)
    }
}

/// 機械防幻覺閘門：同時支援事實卡直寫與草稿驗證
pub fn validate_polished_deed_prose(
    fact: &ChronicleFactPacket,
    scope: &str,
    draft: &str,
    candidate_raw: &str,
) -> Result<String, &'static str> {
    let mut normalized = normalize_chronicler_output(candidate_raw);
    for exact_label in [
        fact.agent_public_label.as_str(),
        fact.patient_public_label.as_str(),
        fact.town_zh.as_str(),
        fact.place_zh.as_str(),
        fact.vessel_zh.as_str(),
    ] {
        if !exact_label.is_empty() && validate_clean_in_world_reply(exact_label).is_some() {
            let mangled = normalize_chronicler_output(exact_label);
            if mangled != exact_label && normalized.contains(&mangled) {
                normalized = normalized.replace(&mangled, exact_label);
            }
        }
    }
    let Some(clean) = validate_clean_in_world_reply(&normalized) else {
        return Err("failed_clean_in_world_reply");
    };
    let char_cnt = clean.chars().count();
    let (min_len, max_len) = if scope == "annals" {
        (110usize, 1100usize)
    } else {
        (48usize, 480usize)
    };
    if char_cnt < min_len || char_cnt > max_len {
        return Err("length_out_of_bounds");
    }
    for para in clean.split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
        if !para.ends_with(['。', '！', '？', '」', '』']) {
            return Err("truncated_incomplete_sentence");
        }
    }
    if clean.contains("pl:") || clean.contains("——") || clean.contains('`') || clean.contains('#')
    {
        return Err("forbidden_syntax_or_token");
    }

    const BANNED_STYLE: &[&str] = &[
        "之乎",
        "者也",
        "乃一",
        "未嘗",
        "遂於",
        "遂將",
        "誠可謂",
        "太史公曰",
        "嗚呼",
        "莫不稱快",
        "愈顯活絡",
        "風裡的光",
        "命運的齒輪",
        "歲月靜好",
        "無聲的訴說",
        "人際網絡",
        "溫暖的連結",
        "見證了歲月",
        "詮釋了",
        "值得一提的是",
        "總而言之",
        "不僅如此",
    ];
    if BANNED_STYLE.iter().any(|bad| clean.contains(bad)) {
        return Err("banned_wenyan_or_cliche");
    }
    if fact.town_slug == "greypine_post"
        || (fact.tide_zh.is_empty()
            && !fact.town_slug.is_empty()
            && fact.town_slug != "white_gull_harbor"
            && fact.town_slug != "gull_tail_port"
            && fact.town_slug != "west_strait_ferry"
            && fact.town_slug != "north_cape_beacon")
    {
        const COASTAL_WORDS: &[&str] = &[
            "潮汐",
            "海灣",
            "海港",
            "出海",
            "漁舟",
            "商船",
            "船隻",
            "防波堤",
            "漲潮",
            "退潮",
            "海風",
            "海浪",
            "海水",
            "碼頭",
            "棧橋",
            "鹹濕",
            "漂木",
            "漂流木",
        ];
        if COASTAL_WORDS.iter().any(|w| clean.contains(w)) {
            return Err("coastal_word_in_inland_greypine");
        }
    }
    for raw_pl in &fact.forbidden_raw_names {
        if !raw_pl.is_empty() && clean.contains(raw_pl) {
            return Err("player_real_name_leaked");
        }
    }
    if draft.contains("外來旅人") && !clean.contains("外來旅人") {
        return Err("missing_masked_traveler_literal");
    }

    // 1. 引號防捏造（僅允許 `draft` 或 `fact.phase_zh` 內既有之招式／名號引號）
    let mut allowed_quotes = extract_quoted_spans(draft);
    if !fact.phase_zh.is_empty() {
        allowed_quotes.insert(fact.phase_zh.clone());
    }
    let cand_quotes = extract_quoted_spans(&clean);
    if cand_quotes.contains("__UNCLOSED_QUOTE__")
        || cand_quotes.contains("__ASCII_QUOTE__")
        || !cand_quotes.is_subset(&allowed_quotes)
    {
        return Err("fabricated_quote_detected");
    }

    // 2. 數字集合防竄改 ＋ 關鍵年份與金額防刪減
    let mut allowed_nums = extract_quantity_tokens(draft);
    for extra_n in &fact.allowed_numbers_zh {
        allowed_nums.insert(extra_n.replace('兩', "二"));
    }
    for bf in &fact.bullet_facts_zh {
        allowed_nums.extend(extract_quantity_tokens(bf));
    }
    let cand_nums = extract_quantity_tokens(&clean);
    if !cand_nums.is_subset(&allowed_nums) {
        return Err("altered_number_detected");
    }
    if fact.solar_year > 0 {
        let yr_zh = solar_year_zh(fact.solar_year);
        if draft.contains(&yr_zh) && !clean.contains(&yr_zh) {
            return Err("missing_solar_year");
        }
    }
    if fact.amount_copper > 0 {
        let amt_zh = num_zh(fact.amount_copper);
        if draft.contains(&amt_zh) && !clean.contains(&amt_zh) {
            return Err("missing_copper_amount");
        }
    }

    // 3. 實體錨點與跨子句施受方向鎖死
    let ag = &fact.agent_public_label;
    let pa = &fact.patient_public_label;
    let draft_tok = tokenize_pair_roles(draft, ag, pa);
    let clean_tok = tokenize_pair_roles(&clean, ag, pa);

    let has_agent_in_draft = !ag.is_empty() && draft_tok.contains(AG_TOK);
    let has_patient_in_draft = !pa.is_empty() && draft_tok.contains(PA_TOK);
    if has_agent_in_draft && !clean_tok.contains(AG_TOK) {
        return Err("missing_agent_label");
    }
    if has_patient_in_draft && !clean_tok.contains(PA_TOK) {
        return Err("missing_patient_label");
    }
    if !fact.town_zh.is_empty()
        && (draft.contains(&fact.town_zh) || draft.contains(&fact.place_zh))
        && !clean.contains(&fact.town_zh)
        && !clean.contains(&fact.place_zh)
    {
        return Err("missing_place_anchor");
    }

    if has_agent_in_draft && has_patient_in_draft && ag != pa {
        let d_ag = draft_tok.find(AG_TOK).unwrap_or(0);
        let d_pa = draft_tok.find(PA_TOK).unwrap_or(0);
        let c_ag = clean_tok.find(AG_TOK).unwrap_or(0);
        let c_pa = clean_tok.find(PA_TOK).unwrap_or(0);

        if fact.kind.starts_with("crisis_rescue") {
            for bad in [
                &format!("蒙{PA_TOK}"),
                &format!("幸得{PA_TOK}"),
                &format!("多虧{PA_TOK}"),
                &format!("見{AG_TOK}遭"),
                &format!("撞見{AG_TOK}"),
                &format!("{AG_TOK}正遭"),
                &format!("{AG_TOK}被"),
                &format!("{AG_TOK}遇險"),
                &format!("救下{AG_TOK}"),
                &format!("搭救{AG_TOK}"),
                &format!("助{AG_TOK}"),
            ] {
                if clean_tok.contains(bad) {
                    return Err("inverted_rescue_direction");
                }
            }
            let mut active_subj: Option<&str> = None;
            let mut ag_performs_rescue = false;
            for clause in clean_tok.split(['，', '。', '；', '！']) {
                let ag_s = subject_pos_in_clause(clause, AG_TOK);
                let pa_s = subject_pos_in_clause(clause, PA_TOK);
                let clause_lead = match (ag_s, pa_s) {
                    (Some(a), Some(p)) => {
                        if a < p {
                            Some(AG_TOK)
                        } else {
                            Some(PA_TOK)
                        }
                    }
                    (Some(_), None) => Some(AG_TOK),
                    (None, Some(_)) => Some(PA_TOK),
                    (None, None) => active_subj,
                };
                active_subj = clause_lead;
                for verb in [
                    "拔刀",
                    "揮動",
                    "揮斧",
                    "揮起",
                    "握緊",
                    "切入戰",
                    "從側翼切入",
                    "救下",
                    "出手解圍",
                    "震偏敵勢",
                ] {
                    if let Some(vi) = clause.find(verb) {
                        let ag_before_verb = ag_s.is_some_and(|ai| ai < vi);
                        if ag_before_verb {
                            ag_performs_rescue = true;
                        } else if clause_lead == Some(PA_TOK) {
                            return Err("inverted_rescue_direction");
                        }
                    }
                }
            }
            if (d_ag < d_pa) != (c_ag < c_pa)
                && !ag_performs_rescue
                && !clean_tok.contains(&format!("幸得{AG_TOK}"))
                && !clean_tok.contains(&format!("蒙{AG_TOK}"))
                && !clean_tok.contains(&format!("多虧{AG_TOK}"))
            {
                return Err("inverted_subject_object_order");
            }
        } else if fact.kind.starts_with("pact_default") {
            for bad in [
                &format!("遭{PA_TOK}拖欠"),
                &format!("向{AG_TOK}清償"),
                &format!("欠{AG_TOK}"),
            ] {
                if clean_tok.contains(bad) {
                    return Err("inverted_default_direction");
                }
            }
            if (d_ag < d_pa) != (c_ag < c_pa) && !clean_tok.contains(&format!("遭{AG_TOK}")) {
                return Err("inverted_subject_object_order");
            }
            let mut active_subj: Option<&str> = None;
            for clause in clean_tok.split(['，', '。', '；', '！']) {
                let ag_s = subject_pos_in_clause(clause, AG_TOK);
                let pa_s = subject_pos_in_clause(clause, PA_TOK);
                let clause_lead = match (ag_s, pa_s) {
                    (Some(a), Some(p)) => {
                        if a < p {
                            Some(AG_TOK)
                        } else {
                            Some(PA_TOK)
                        }
                    }
                    (Some(_), None) => Some(AG_TOK),
                    (None, Some(_)) => Some(PA_TOK),
                    (None, None) => active_subj,
                };
                active_subj = clause_lead;
                for pred in [
                    "周轉不開",
                    "緊絀",
                    "未能湊足",
                    "未能清償",
                    "未能履約",
                    "拖欠",
                    "違約",
                    "失信",
                ] {
                    if let Some(pi) = clause.find(pred) {
                        let ag_before_pred = ag_s.is_some_and(|ai| ai < pi)
                            && !clause.contains(&format!("遭{AG_TOK}拖欠"));
                        if !ag_before_pred
                            && clause_lead == Some(PA_TOK)
                            && !clause.contains(&format!("遭{AG_TOK}"))
                        {
                            return Err("inverted_default_direction");
                        }
                    }
                }
            }
        } else if (d_ag < d_pa) != (c_ag < c_pa) {
            return Err("inverted_subject_object_order");
        }
    }

    Ok(clean)
}
