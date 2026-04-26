//! Grid 格上物件與地形型別。
//!
//! 原先定義於 `src/hex/cell.rs`，隨觀景窗 hex 概念下線，搬回方格模組根處。

use serde::{Deserialize, Serialize};

// ─── 格上物件型別 ────────────────────────────────────────────────────

/// 格子上可採集物件的種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GridObjectKind {
    /// 植物性資源（野果、草藥、木材等）
    Flora,
    /// 礦物性資源（礦石、寶石等）
    Mineral,
    /// 水源（淡水、溫泉等）
    WaterSource,
    /// 其他（遺跡、碎片等）
    Other,
}

/// 格子上的可採集物件。
///
/// 與 `model::RoomObject` 不同：此結構帶資源量與回復速率，
/// 用於 NPC 採集與資源生態模擬。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridObject {
    /// 物件唯一識別碼（通常為 `"{種類}_{座標}"`）
    pub id: String,
    pub kind: GridObjectKind,
    pub name: String,
    /// 目前剩餘量
    pub quantity: u32,
    /// 上限量（採滿後不再增長）
    pub max_quantity: u32,
    /// 每 tick 自然回復量（0 = 不回復）
    pub regrowth_per_tick: u32,
}

/// 地形類型
///
/// 決定預設通行性與移動成本。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Terrain {
    #[default]
    Plain,
    Forest,
    ForestHeavy,
    ForestLight,
    Mountain,
    Hills,
    Water,
    WaterDeep,
    Desert,
    Swamp,
    Tundra,
    Grassland,
    Jungle,
    Urban,
    Road,
    Bridge,
    Wall,
    FarmField,
    Farmhouse,
    Inn,
    Tavern,
    Blacksmith,
    GeneralStore,
    Clinic,
    Workshop,
    Market,
    GuildHall,
    Temple,
    Academy,
    Library,
    Barracks,
    GuardPost,
    Warehouse,
    Granary,
    Dock,
    Bathhouse,
    Courthouse,
    Jail,
    TownHall,
    Bank,
    Mint,
    Stables,
    Caravanserai,
    Theater,
    Arena,
    Observatory,
    Alchemist,
    MageTower,
    Embassy,
    PrisonYard,
}

impl Terrain {
    /// 序列化標記（snake_case），送前端用。
    ///
    /// 對齊 `#[serde(rename_all = "snake_case")]` 的 JSON 表示。
    pub fn kind_str(self) -> &'static str {
        match self {
            Terrain::Plain => "plain",
            Terrain::Forest => "forest",
            Terrain::ForestHeavy => "forest_heavy",
            Terrain::ForestLight => "forest_light",
            Terrain::Mountain => "mountain",
            Terrain::Hills => "hills",
            Terrain::Water => "water",
            Terrain::WaterDeep => "water_deep",
            Terrain::Desert => "desert",
            Terrain::Swamp => "swamp",
            Terrain::Tundra => "tundra",
            Terrain::Grassland => "grassland",
            Terrain::Jungle => "jungle",
            Terrain::Urban => "urban",
            Terrain::Road => "road",
            Terrain::Bridge => "bridge",
            Terrain::Wall => "wall",
            Terrain::FarmField => "farm_field",
            Terrain::Farmhouse => "farmhouse",
            Terrain::Inn => "inn",
            Terrain::Tavern => "tavern",
            Terrain::Blacksmith => "blacksmith",
            Terrain::GeneralStore => "general_store",
            Terrain::Clinic => "clinic",
            Terrain::Workshop => "workshop",
            Terrain::Market => "market",
            Terrain::GuildHall => "guild_hall",
            Terrain::Temple => "temple",
            Terrain::Academy => "academy",
            Terrain::Library => "library",
            Terrain::Barracks => "barracks",
            Terrain::GuardPost => "guard_post",
            Terrain::Warehouse => "warehouse",
            Terrain::Granary => "granary",
            Terrain::Dock => "dock",
            Terrain::Bathhouse => "bathhouse",
            Terrain::Courthouse => "courthouse",
            Terrain::Jail => "jail",
            Terrain::TownHall => "town_hall",
            Terrain::Bank => "bank",
            Terrain::Mint => "mint",
            Terrain::Stables => "stables",
            Terrain::Caravanserai => "caravanserai",
            Terrain::Theater => "theater",
            Terrain::Arena => "arena",
            Terrain::Observatory => "observatory",
            Terrain::Alchemist => "alchemist",
            Terrain::MageTower => "mage_tower",
            Terrain::Embassy => "embassy",
            Terrain::PrisonYard => "prison_yard",
        }
    }

    /// 分類：terrain（自然）／landmark（人造地標）／infra（通行建設）。
    pub fn category(self) -> &'static str {
        match self {
            Terrain::Plain
            | Terrain::Forest
            | Terrain::ForestLight
            | Terrain::ForestHeavy
            | Terrain::Mountain
            | Terrain::Hills
            | Terrain::Water
            | Terrain::WaterDeep
            | Terrain::Desert
            | Terrain::Swamp
            | Terrain::Tundra
            | Terrain::Grassland
            | Terrain::Jungle => "terrain",

            Terrain::Urban
            | Terrain::Inn
            | Terrain::Tavern
            | Terrain::Blacksmith
            | Terrain::GeneralStore
            | Terrain::Clinic
            | Terrain::Workshop
            | Terrain::Market
            | Terrain::GuildHall
            | Terrain::Temple
            | Terrain::Farmhouse
            | Terrain::FarmField
            | Terrain::Academy
            | Terrain::Library
            | Terrain::Barracks
            | Terrain::GuardPost
            | Terrain::Warehouse
            | Terrain::Granary
            | Terrain::Dock
            | Terrain::Bathhouse
            | Terrain::Courthouse
            | Terrain::Jail
            | Terrain::TownHall
            | Terrain::Bank
            | Terrain::Mint
            | Terrain::Stables
            | Terrain::Caravanserai
            | Terrain::Theater
            | Terrain::Arena
            | Terrain::Observatory
            | Terrain::Alchemist
            | Terrain::MageTower
            | Terrain::Embassy
            | Terrain::PrisonYard => "landmark",

            Terrain::Road | Terrain::Bridge | Terrain::Wall => "infra",
        }
    }

    /// 此地形是否預設可步行通過
    pub fn walkable(self) -> bool {
        !matches!(self, Terrain::Water | Terrain::WaterDeep | Terrain::Mountain | Terrain::Wall)
    }

    /// 移動成本倍率（1.0 = 正常，越高越慢）
    pub fn move_cost(self) -> f64 {
        match self {
            Terrain::Road | Terrain::Bridge => 0.5,
            Terrain::Plain
            | Terrain::Grassland
            | Terrain::Urban
            | Terrain::Farmhouse
            | Terrain::Inn
            | Terrain::Tavern
            | Terrain::Blacksmith
            | Terrain::GeneralStore
            | Terrain::Clinic
            | Terrain::Workshop
            | Terrain::Market
            | Terrain::GuildHall
            | Terrain::Temple
            | Terrain::Academy
            | Terrain::Library
            | Terrain::Barracks
            | Terrain::GuardPost
            | Terrain::Warehouse
            | Terrain::Granary
            | Terrain::Dock
            | Terrain::Bathhouse
            | Terrain::Courthouse
            | Terrain::Jail
            | Terrain::TownHall
            | Terrain::Bank
            | Terrain::Mint
            | Terrain::Stables
            | Terrain::Caravanserai
            | Terrain::Theater
            | Terrain::Arena
            | Terrain::Observatory
            | Terrain::Alchemist
            | Terrain::MageTower
            | Terrain::Embassy
            | Terrain::PrisonYard => 1.0,
            Terrain::Forest | Terrain::ForestLight | Terrain::Hills => 1.5,
            Terrain::Desert | Terrain::Tundra | Terrain::FarmField => 1.5,
            Terrain::ForestHeavy | Terrain::Jungle | Terrain::Swamp => 2.0,
            Terrain::Water | Terrain::WaterDeep | Terrain::Mountain | Terrain::Wall => f64::INFINITY,
        }
    }

    /// 取得地形對應的顯示字（簡化版，取第一個候選字）
    pub fn display_char(self) -> &'static str {
        match self {
            Terrain::Forest | Terrain::ForestHeavy | Terrain::ForestLight | Terrain::Jungle => "木",
            Terrain::Mountain => "山",
            Terrain::Hills => "石",
            Terrain::Swamp => "沼",
            Terrain::Water | Terrain::WaterDeep => "水",
            Terrain::Grassland => "屮屮",
            Terrain::Plain | Terrain::Desert => "荒",
            Terrain::Road | Terrain::Bridge => "道",
            Terrain::Tundra => "冰",
            Terrain::FarmField => "田",
            Terrain::Wall => "牆",
            Terrain::Urban
            | Terrain::Farmhouse
            | Terrain::Inn
            | Terrain::Tavern
            | Terrain::Blacksmith
            | Terrain::GeneralStore
            | Terrain::Clinic
            | Terrain::Workshop
            | Terrain::Market
            | Terrain::GuildHall
            | Terrain::Temple
            | Terrain::Academy
            | Terrain::Library
            | Terrain::Barracks
            | Terrain::GuardPost
            | Terrain::Warehouse
            | Terrain::Granary
            | Terrain::Dock
            | Terrain::Bathhouse
            | Terrain::Courthouse
            | Terrain::Jail
            | Terrain::TownHall
            | Terrain::Bank
            | Terrain::Mint
            | Terrain::Stables
            | Terrain::Caravanserai
            | Terrain::Theater
            | Terrain::Arena
            | Terrain::Observatory
            | Terrain::Alchemist
            | Terrain::MageTower
            | Terrain::Embassy
            | Terrain::PrisonYard => "地",
        }
    }

    /// 取得地形對應的輔助顏色（簡化版，取第一個候選色）
    pub fn color(self) -> &'static str {
        match self {
            Terrain::Forest | Terrain::ForestHeavy | Terrain::ForestLight | Terrain::Jungle => {
                "#a0d080"
            }
            Terrain::Mountain => "#98af9d",
            Terrain::Hills => "#d3d3d3",
            Terrain::Swamp => "#507050",
            Terrain::Water | Terrain::WaterDeep => "#b0d8f0",
            Terrain::Grassland => "#c2d6a4",
            Terrain::Plain | Terrain::Desert => "#e3d5ca",
            Terrain::Road | Terrain::Bridge => "#c4b8a8",
            Terrain::Tundra => "#e0f2f1",
            Terrain::FarmField => "#9ef01a",
            Terrain::Wall => "#8c8c8c",
            _ => "#c4b8a8", // 其他建築或都市地形預設色
        }
    }

    /// 地形字顏色（None 表示依底色亮度自動選黑/白）
    pub fn text_color(self) -> Option<&'static str> {
        match self {
            Terrain::Grassland => Some("#3a6b35"),
            _ => None,
        }
    }

    /// 地形字字型（None 表示使用預設字型）
    pub fn text_font(self) -> Option<&'static str> {
        match self {
            Terrain::Grassland => Some("FZJiaGuWen"),
            _ => None,
        }
    }
}
