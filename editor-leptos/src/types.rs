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
    /// 預設：僅平移／縮放閱覽，左鍵拖曳不會上色（避免誤觸）
    View,
    Paint,
    Erase,
    Select,
    Move,
}

impl ToolMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ToolMode::View => "view",
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
            // 自然地形
            Terrain::Plain       => "#c8c4a0",
            Terrain::Grassland   => "#a4b870",
            Terrain::Forest      => "#5a7a3a",
            Terrain::ForestHeavy => "#3d5c28",
            Terrain::ForestLight => "#7a9a52",
            Terrain::Jungle      => "#3a6e3a",
            Terrain::Hills       => "#9a8e60",
            Terrain::Mountain    => "#7a7a7a",
            Terrain::Water       => "#4a7a9a",
            Terrain::WaterDeep   => "#2a4a6a",
            Terrain::Desert      => "#c8a858",
            Terrain::Swamp       => "#5a6848",
            Terrain::Tundra      => "#a0aab0",

            // 基礎設施
            Terrain::Road        => "#8a8068",
            Terrain::Bridge      => "#7a6a4a",
            Terrain::Wall        => "#4a4e56",

            // 農業
            Terrain::FarmField   => "#8aaa48",
            Terrain::Farmhouse   => "#9a8a5a",
            Terrain::Granary     => "#a09050",

            // 住宅
            Terrain::Urban       => "#7a7068",

            // 商業
            Terrain::Market      => "#b8884a",
            Terrain::GeneralStore=> "#9a7a52",
            Terrain::Bank        => "#8a7a5a",
            Terrain::Mint        => "#8a8a5a",
            Terrain::Warehouse   => "#6a6858",
            Terrain::Dock        => "#5a7888",

            // 服務
            Terrain::Inn         => "#9a7040",
            Terrain::Tavern      => "#8a5a3a",
            Terrain::Clinic      => "#6a8a6a",
            Terrain::Bathhouse   => "#5a8a8a",
            Terrain::Stables     => "#7a6a4a",
            Terrain::Caravanserai=> "#8a7848",

            // 工藝
            Terrain::Blacksmith  => "#5a5a6a",
            Terrain::Workshop    => "#7a6a50",
            Terrain::Alchemist   => "#6a5a7a",

            // 宗教文教
            Terrain::Temple      => "#7a6a80",
            Terrain::Academy     => "#5a6a80",
            Terrain::Library     => "#6a7a7a",
            Terrain::Observatory => "#5a6878",

            // 軍政
            Terrain::Barracks    => "#6a5a4a",
            Terrain::GuardPost   => "#6a6050",
            Terrain::Courthouse  => "#6a6a6a",
            Terrain::Jail        => "#4a4a50",
            Terrain::TownHall    => "#7a7068",
            Terrain::PrisonYard  => "#4a4848",

            // 特殊
            Terrain::GuildHall   => "#7a6860",
            Terrain::Theater     => "#8a5a6a",
            Terrain::Arena       => "#8a6a4a",
            Terrain::MageTower   => "#5a5a8a",
            Terrain::Embassy     => "#6a6a78",
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

/// 與後端 `crate::hex::contract_pins` 對齊：遊戲釘死之彩格（如出生點）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractPin {
    pub q: i32,
    pub r: i32,
    pub role: String,
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
    /// 遊戲釘死契約座標（非格內資料，由伺服器附加）
    #[serde(default)]
    pub contract_pins: Vec<ContractPin>,
}
