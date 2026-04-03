use std::collections::HashMap;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolMode {
    Paint,
    Erase,
    Select,
    Move,
}

impl ToolMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ToolMode::Paint => "paint",
            ToolMode::Erase => "erase",
            ToolMode::Select => "select",
            ToolMode::Move => "move",
        }
    }
}

impl Terrain {
    pub const ALL: &[Terrain] = &[
        Terrain::Forest,
        Terrain::Mountain,
        Terrain::Hills,
        Terrain::Water,
        Terrain::Desert,
        Terrain::Swamp,
        Terrain::Tundra,
        Terrain::Grassland,
        Terrain::Jungle,
        Terrain::Road,
        Terrain::Bridge,
        Terrain::Wall,
        Terrain::FarmField,
        Terrain::Farmhouse,
        Terrain::Inn,
        Terrain::Tavern,
        Terrain::Blacksmith,
        Terrain::GeneralStore,
        Terrain::Clinic,
        Terrain::Workshop,
        Terrain::Market,
        Terrain::GuildHall,
        Terrain::Temple,
        Terrain::Academy,
        Terrain::Library,
        Terrain::Barracks,
        Terrain::GuardPost,
        Terrain::Warehouse,
        Terrain::Granary,
        Terrain::Dock,
        Terrain::Bathhouse,
        Terrain::Courthouse,
        Terrain::Jail,
        Terrain::TownHall,
        Terrain::Bank,
        Terrain::Mint,
        Terrain::Stables,
        Terrain::Caravanserai,
        Terrain::Theater,
        Terrain::Arena,
        Terrain::Observatory,
        Terrain::Alchemist,
        Terrain::MageTower,
        Terrain::Embassy,
        Terrain::PrisonYard,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Terrain::Plain => "平原",
            Terrain::Forest => "森林",
            Terrain::ForestHeavy => "密林",
            Terrain::ForestLight => "疏林",
            Terrain::Mountain => "山地",
            Terrain::Hills => "丘陵",
            Terrain::Water => "水域",
            Terrain::WaterDeep => "深水",
            Terrain::Desert => "沙漠",
            Terrain::Swamp => "沼澤",
            Terrain::Tundra => "凍原",
            Terrain::Grassland => "草原",
            Terrain::Jungle => "叢林",
            Terrain::Urban => "城區",
            Terrain::Road => "道路",
            Terrain::Bridge => "橋樑",
            Terrain::Wall => "牆體",
            Terrain::FarmField => "農田",
            Terrain::Farmhouse => "農舍",
            Terrain::Inn => "旅店",
            Terrain::Tavern => "酒館",
            Terrain::Blacksmith => "鐵匠鋪",
            Terrain::GeneralStore => "雜貨店",
            Terrain::Clinic => "醫館",
            Terrain::Workshop => "工坊",
            Terrain::Market => "市集",
            Terrain::GuildHall => "公會大廳",
            Terrain::Temple => "神殿",
            Terrain::Academy => "學院",
            Terrain::Library => "圖書館",
            Terrain::Barracks => "兵營",
            Terrain::GuardPost => "衛所",
            Terrain::Warehouse => "倉庫",
            Terrain::Granary => "糧倉",
            Terrain::Dock => "碼頭",
            Terrain::Bathhouse => "浴場",
            Terrain::Courthouse => "法院",
            Terrain::Jail => "監所",
            Terrain::TownHall => "市政廳",
            Terrain::Bank => "銀行",
            Terrain::Mint => "鑄幣所",
            Terrain::Stables => "馬廄",
            Terrain::Caravanserai => "商旅驛站",
            Terrain::Theater => "劇院",
            Terrain::Arena => "競技場",
            Terrain::Observatory => "觀測台",
            Terrain::Alchemist => "鍊金工房",
            Terrain::MageTower => "法師塔",
            Terrain::Embassy => "使館",
            Terrain::PrisonYard => "囚院",
        }
    }

    pub fn color(self) -> &'static str {
        match self {
            Terrain::Plain => "#c8d68e",
            Terrain::Forest => "#5a8c42",
            Terrain::ForestHeavy => "#2d5a1e",
            Terrain::ForestLight => "#8bb86a",
            Terrain::Mountain => "#8a8a8a",
            Terrain::Hills => "#b5a068",
            Terrain::Water => "#4a90d9",
            Terrain::WaterDeep => "#2a5a9a",
            Terrain::Desert => "#e8d48a",
            Terrain::Swamp => "#6b7a44",
            Terrain::Tundra => "#d0d8e0",
            Terrain::Grassland => "#a8cc6a",
            Terrain::Jungle => "#3a6e2a",
            Terrain::Urban => "#aaa090",
            Terrain::Road => "#c4b48a",
            Terrain::Bridge => "#9c7b5b",
            Terrain::Wall => "#3b3f46",
            Terrain::FarmField => "#86b34d",
            Terrain::Farmhouse => "#8b7355",
            Terrain::Inn => "#b88a5a",
            Terrain::Tavern => "#8f5c3f",
            Terrain::Blacksmith => "#5e6269",
            Terrain::GeneralStore => "#c29a6b",
            Terrain::Clinic => "#8fc8b0",
            Terrain::Workshop => "#9f8f7f",
            Terrain::Market => "#d1a85a",
            Terrain::GuildHall => "#6e78a8",
            Terrain::Temple => "#b8a0c9",
            Terrain::Academy => "#7fa8c9",
            Terrain::Library => "#9c7f5e",
            Terrain::Barracks => "#7b868e",
            Terrain::GuardPost => "#5a6570",
            Terrain::Warehouse => "#8f826d",
            Terrain::Granary => "#c0a35b",
            Terrain::Dock => "#5b87a8",
            Terrain::Bathhouse => "#7bb8b1",
            Terrain::Courthouse => "#8f8f9f",
            Terrain::Jail => "#4e545e",
            Terrain::TownHall => "#7e6fa8",
            Terrain::Bank => "#b8a45f",
            Terrain::Mint => "#9fa8b2",
            Terrain::Stables => "#8a6f4f",
            Terrain::Caravanserai => "#b78961",
            Terrain::Theater => "#9b6fa8",
            Terrain::Arena => "#b97862",
            Terrain::Observatory => "#5f7fa8",
            Terrain::Alchemist => "#6f9b8a",
            Terrain::MageTower => "#6a63a8",
            Terrain::Embassy => "#7f97a8",
            Terrain::PrisonYard => "#5a5f69",
        }
    }

    pub fn serde_name(self) -> &'static str {
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
}

impl std::fmt::Display for Terrain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HexCoord {
    pub q: i32,
    pub r: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HexCell {
    pub coord: HexCoord,
    pub terrain: Terrain,
    pub name: String,
    #[serde(default)]
    pub zone: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub objects: Vec<RoomObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarrierEntry {
    pub coord: HexCoord,
    pub dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portal {
    pub name: String,
    pub from: HexCoord,
    pub to: HexCoord,
    pub bidirectional: bool,
    #[serde(default)]
    pub counts_as_official_link: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportMode {
    Road,
    Water,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportLinkClass {
    #[default]
    Official,
    Shortcut,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TransportEndpoint {
    Settlement { settlement_id: String },
    Cell(HexCoord),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransportEdge {
    #[serde(default)]
    pub id: Option<String>,
    pub endpoint_a: TransportEndpoint,
    pub endpoint_b: TransportEndpoint,
    pub mode: TransportMode,
    #[serde(default = "default_true")]
    pub operational: bool,
    #[serde(default)]
    pub link_class: TransportLinkClass,
    #[serde(default)]
    pub weight: Option<f64>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridResponse {
    #[serde(default)]
    pub world_seed: u64,
    pub cells: Vec<HexCell>,
    #[serde(default)]
    pub barriers: Vec<BarrierEntry>,
    #[serde(default)]
    pub portals: Vec<Portal>,
    #[serde(default)]
    pub transport_edges: Vec<TransportEdge>,
}
