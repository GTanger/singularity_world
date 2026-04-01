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
    Wall,
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
        Terrain::Urban,
        Terrain::Road,
        Terrain::Wall,
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
            Terrain::Wall => "牆體",
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
            Terrain::Wall => "#3b3f46",
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
            Terrain::Wall => "wall",
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridResponse {
    pub cells: Vec<HexCell>,
    #[serde(default)]
    pub barriers: Vec<BarrierEntry>,
    #[serde(default)]
    pub portals: Vec<Portal>,
}
