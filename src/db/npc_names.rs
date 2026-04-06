// NPC 隨機姓名，對齊既有 db/npc_names。

use std::sync::LazyLock;

use rand::Rng;

static SINGLE_SURNAMES: LazyLock<Vec<char>> = LazyLock::new(|| {
    "王李張劉陳楊黃趙周吳徐孫馬朱胡郭何高林羅鄭梁謝宋唐許韓馮鄧曹彭曾蕭田董潘袁蔡蔣余杜葉程蘇魏呂丁任沈姚盧姜崔鍾譚陸汪范金石廖賈夏韋傅方白鄒孟熊秦邱江尹薛閻段雷龍黎史陶賀顧毛郝龔邵萬錢嚴覃武戴莫孔向湯"
        .chars()
        .collect()
});

static GIVEN_NAME_CHARS: LazyLock<Vec<char>> = LazyLock::new(|| {
    "明華強建國志偉芳娜敏靜麗磊軍洋勇艷杰濤超秀英慧萍玲紅梅飛燕霞蘭平康安寧俊傑慧敏雅琪俊偉志明淑芬建國美玲志偉淑惠"
        .chars()
        .collect()
});

const COMPOUND_SURNAMES: &[&str] = &[
    "歐陽", "司馬", "諸葛", "上官", "司徒", "司空", "皇甫", "慕容", "宇文", "長孫", "聞人", "東方", "獨孤", "南宮", "萬俟", "尉遲",
];

/// 產生「姓＋名」2～4 字；`gender` 預留（對齊既有 `GenerateNPCName`）。
pub fn generate_npc_name(_gender: &str) -> String {
    let mut rng = rand::rng();
    let singles = &*SINGLE_SURNAMES;
    let givens = &*GIVEN_NAME_CHARS;
    debug_assert!(!singles.is_empty() && !givens.is_empty());

    let use_compound = rng.random_range(0..5) == 0;
    let surname = if use_compound {
        COMPOUND_SURNAMES[rng.random_range(0..COMPOUND_SURNAMES.len())].to_string()
    } else {
        singles[rng.random_range(0..singles.len())].to_string()
    };
    let given_len = 1 + rng.random_range(0..2);
    let mut given = String::new();
    for _ in 0..given_len {
        given.push(givens[rng.random_range(0..givens.len())]);
    }
    surname + &given
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
        }
    }

    #[test]
    fn first_rune_fallback() {
        assert_eq!(first_rune(""), "人");
        assert_eq!(first_rune("張三"), "張");
    }
}
