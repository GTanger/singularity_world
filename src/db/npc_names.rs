// NPC 隨機姓名，對齊既有 db/npc_names。

use std::sync::LazyLock;

use rand::Rng;

static SINGLE_SURNAMES: LazyLock<Vec<char>> = LazyLock::new(|| {
    "王李張劉陳楊黃趙周吳徐孫馬朱胡郭何高林羅鄭梁謝宋唐許韓馮鄧曹彭曾蕭田董潘袁蔡蔣余杜葉程蘇魏呂丁任沈姚盧姜崔鍾譚陸汪范金石廖賈夏韋傅方白鄒孟熊秦邱江尹薛閻段雷龍黎史陶賀顧毛郝龔邵萬錢嚴覃武戴莫孔向湯"
        .chars()
        .collect()
});

pub static MALE_GIVEN_NAMES: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    vec![
        "冠廷", "宇翔", "志豪", "俊傑", "柏翰", "冠宇", "志明", "家豪", "宗翰", "承恩",
        "宇軒", "冠霖", "哲維", "振武", "建興", "致遠", "子傑", "明哲", "清和", "懷安",
        "文軒", "正明", "長青", "明德", "家瑋", "文華", "豐年", "志強", "忠義", "宏偉",
        "崇安", "毅宏", "銘凱", "駿宏", "皓天", "景耀", "啟銘", "承宏", "秉諺", "耀庭",
        "翔宇", "彥良", "凱鈞", "瀚睿", "睿佑", "辰穎", "建宏", "冠良", "子豪", "政憲",
        "廷宇", "博文", "俊宏", "宇辰", "宗憲", "廷翰", "柏廷", "弘毅", "安邦", "正邦",
    ]
});

pub static FEMALE_GIVEN_NAMES: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    vec![
        "雅婷", "雅筑", "婷萱", "妤涵", "晴雯", "琳琪", "柔馨", "潔儀", "欣真", "宜佩",
        "慈惠", "秀娟", "雅竹", "婉清", "素心", "芷蘭", "佩玉", "碧雲", "映雪", "淑敏",
        "秋月", "秀英", "婉君", "佳穎", "子晴", "宥彤", "語晴", "品妤", "心怡", "筱涵",
        "芷瑜", "佩珊", "庭瑄", "宇婷", "靜雯", "碧華", "映辰", "采妮", "青樺", "淑芬",
        "美玲", "淑惠", "詠晴", "若彤", "依晨", "采薇", "涵玥", "芷嫣", "曼琳", "詩涵",
        "依婷", "佳欣", "佩芬", "欣儀", "雅雲", "佩容", "宣慈", "淳淳", "真真", "芳華",
    ]
});

/// 產生「姓＋名」2～4 字；依傳入性別（M/F、男/女）選取男性或女性名字池。
pub fn generate_npc_name(gender: &str) -> String {
    let mut rng = rand::rng();
    let singles = &*SINGLE_SURNAMES;
    let surname = singles[rng.random_range(0..singles.len())].to_string();

    let is_female = match gender.trim().to_uppercase().as_str() {
        "F" | "女" | "FEMALE" => true,
        "M" | "男" | "MALE" => false,
        _ => rng.random_range(0..2) == 0,
    };

    let givens = if is_female {
        &*FEMALE_GIVEN_NAMES
    } else {
        &*MALE_GIVEN_NAMES
    };
    let given = givens[rng.random_range(0..givens.len())];
    format!("{surname}{given}")
}

/// 字串第一個字；空或無效則「人」（對齊既有 `FirstRune`）。
#[must_use]
pub fn first_rune(s: &str) -> String {
    s.chars()
        .next()
        .map_or_else(|| "人".to_string(), |c| c.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_length_2_to_4_chars() {
        for _ in 0..50 {
            let n = generate_npc_name("");
            let count = n.chars().count();
            assert!((2..=4).contains(&count), "{n} len {count}");
            let nf = generate_npc_name("F");
            assert!((2..=4).contains(&nf.chars().count()));
            let nm = generate_npc_name("M");
            assert!((2..=4).contains(&nm.chars().count()));
        }
    }

    #[test]
    fn female_names_do_not_use_male_pool() {
        for _ in 0..50 {
            let nf = generate_npc_name("F");
            let given: String = nf.chars().skip(1).collect();
            assert!(FEMALE_GIVEN_NAMES.contains(&given.as_str()), "{nf} not in female pool");
        }
    }

    #[test]
    fn first_rune_fallback() {
        assert_eq!(first_rune(""), "人");
        assert_eq!(first_rune("張三"), "張");
    }
}
