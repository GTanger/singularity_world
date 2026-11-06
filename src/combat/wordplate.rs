// wordplate 模組 — 500 詞 ＋ 360 星盤 × 隨身【詞匣】 × 田忌賽馬四戰術序位核心系統。
// 嚴守台灣正體中文，徹底拔除 Codex 偽科幻詞彙；後台資料嚴謹，前台體感去數值化。

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::LazyLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

include!("wordplate/types.rs");

const RAW_WORDS_JSON: &str = include_str!("../../data/config/word_elements.json");
const RAW_GAMEDIMS_JSON: &str = include_str!("../../data/config/word_elements_gamedims.json");

include!("wordplate/casket.rs");
include!("wordplate/synthesized.rs");
include!("wordplate/tactical.rs");
include!("wordplate/clash.rs");

#[cfg(test)]
mod tests {
    use super::*;
    include!("wordplate/tests_vector.rs");
    include!("wordplate/tests_clash.rs");
}
