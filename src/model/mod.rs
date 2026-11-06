// model 模組 — 與 db、store 共用的資料型別，避免循環依賴。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 房間內可互動物件（Look/Read/Smell 等），name 須與房間描述中〔〕一致。
/// move_to_room_id 為選填：當 sockets 含 Move 時，執行 Move 會將玩家切換到該房間。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomObject {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub sockets: Vec<String>,
    #[serde(default)]
    pub responses: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub move_to_room_id: String,
}

impl RoomObject {
    /// 房間物件的專屬英文規範標籤（dedicated English snake_case tag）。
    pub fn tag(&self) -> String {
        canonical_object_tag(&self.id, &self.name)
    }

    /// 房間物件的分類標籤（category tag）。
    pub fn category_tag(&self) -> &'static str {
        object_category_tag(&self.id, &self.name)
    }

    /// 房間物件的功能原型標籤（functional archetype tag）。
    pub fn archetype_tag(&self) -> &'static str {
        object_archetype_tag(&self.id, &self.name)
    }

    /// 房間物件的屬性標籤清單（attribute tags）。
    pub fn attribute_tags(&self) -> Vec<String> {
        object_attribute_tags(&self.id, &self.name)
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    /// 取得物件的所有語意標籤清單（包含專屬 tag、category tag、archetype tag、超分類標籤、目錄房間 tag、sockets 標籤，去重）。
    pub fn tags(&self) -> Vec<String> {
        object_semantic_tags(&self.id, &self.name, &self.sockets)
    }

    /// 檢查物件是否包含指定標籤（比對專屬 tag、category tag、archetype tag 或語意標籤清單）。
    pub fn has_tag(&self, wanted: &str) -> bool {
        if wanted.is_empty() {
            return false;
        }
        let clean_wanted = wanted.trim().to_ascii_lowercase();
        let dedicated = self.tag();
        if dedicated == clean_wanted {
            return true;
        }
        let cat = self.category_tag();
        if cat == clean_wanted {
            return true;
        }
        let arch = self.archetype_tag();
        if arch == clean_wanted {
            return true;
        }
        let all_tags = self.tags();
        all_tags.iter().any(|t| t == &clean_wanted)
    }
}

/// 將字串轉換為合法的純英文數字 snake_case 標籤 fallback。
fn slugify_identifier(id_str: &str) -> String {
    let mut slug = String::with_capacity(id_str.len() + 4);
    let mut prev_is_sep = false;
    for ch in id_str.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            prev_is_sep = false;
        } else if ch == '_' || ch == '-' || ch == ':' || ch == '.' || ch.is_whitespace() {
            if !prev_is_sep && !slug.is_empty() {
                slug.push('_');
                prev_is_sep = true;
            }
        }
    }
    let trimmed = slug.trim_matches('_');
    if trimmed.is_empty() {
        return "scene_object".to_string();
    }
    if trimmed.starts_with(|c: char| c.is_ascii_digit()) {
        format!("obj_{}", trimmed)
    } else {
        trimmed.to_string()
    }
}

/// 取得物件的專屬英文規範標籤（dedicated canonical English tag）。
///
/// 1. 優先查白塔半島千物總典（若命中則回傳其專屬 `object_tag`）。
/// 2. 比對已知常見動態/場景物件與名稱。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneObjectTagSpec {
    pub id: &'static str,
    pub name_alias: &'static str,
    pub object_tag: &'static str,
    pub category_tag: &'static str,
    pub archetype_tag: &'static str,
    pub attribute_tags: &'static [&'static str],
}


include!("scene_object_specs.rs");
include!("model_tests.rs");
