use serde::{Deserialize, Serialize};

use crate::model::RoomObject;

use super::coord::HexCoord;

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

// ─────────────────────────────────────────────────────────────────

/// 六角格單元
///
/// 取代舊 `model::Room`。每個格子有座標、地形、名稱、描述、
/// 可互動物件等。鄰接由幾何自動決定，不需手動建 exit。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexCell {
    pub coord: HexCoord,
    pub terrain: Terrain,
    pub name: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub zone: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    /// 格子內的可互動物件（沿用既有 RoomObject 結構）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<RoomObject>,
}

impl HexCell {
    pub fn new(coord: HexCoord, terrain: Terrain, name: impl Into<String>) -> Self {
        Self {
            coord,
            terrain,
            name: name.into(),
            zone: String::new(),
            tags: Vec::new(),
            description: String::new(),
            objects: Vec::new(),
        }
    }

    pub fn with_zone(mut self, zone: impl Into<String>) -> Self {
        self.zone = zone.into();
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }
}

// ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_walkable() {
        assert!(Terrain::Plain.walkable());
        assert!(Terrain::Forest.walkable());
        assert!(Terrain::Road.walkable());
        assert!(Terrain::Bridge.walkable());
        assert!(!Terrain::Water.walkable());
        assert!(!Terrain::WaterDeep.walkable());
        assert!(!Terrain::Mountain.walkable());
        assert!(!Terrain::Wall.walkable());
    }

    #[test]
    fn terrain_move_cost_ordering() {
        assert!(Terrain::Road.move_cost() < Terrain::Plain.move_cost());
        assert!(Terrain::Plain.move_cost() < Terrain::ForestHeavy.move_cost());
        assert!(Terrain::Water.move_cost().is_infinite());
    }

    #[test]
    fn cell_builder() {
        let c = HexCell::new(HexCoord::ORIGIN, Terrain::Forest, "密林")
            .with_zone("wild")
            .with_tags(vec!["watabou".into()])
            .with_description("古木參天");
        assert_eq!(c.zone, "wild");
        assert_eq!(c.tags, vec!["watabou"]);
        assert_eq!(c.description, "古木參天");
    }

    #[test]
    fn serde_roundtrip() {
        let c = HexCell::new(HexCoord::new(3, -1), Terrain::Desert, "沙丘");
        let json = serde_json::to_string(&c).unwrap();
        let back: HexCell = serde_json::from_str(&json).unwrap();
        assert_eq!(back.coord, c.coord);
        assert_eq!(back.terrain, Terrain::Desert);
        assert_eq!(back.name, "沙丘");
    }
}
