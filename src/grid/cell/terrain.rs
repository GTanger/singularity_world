
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
    Beach,
    MinePass,
    HolyTower,
    EmptyFurrow,
    DryField,
    HerbGarden,
    Fence,
    House,
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
            Terrain::Beach => "beach",
            Terrain::MinePass => "mine_pass",
            Terrain::HolyTower => "holy_tower",
            Terrain::EmptyFurrow => "empty_furrow",
            Terrain::DryField => "dry_field",
            Terrain::HerbGarden => "herb_garden",
            Terrain::Fence => "fence",
            Terrain::House => "house",
        }
    }

    /// 從 snake_case 字串解析 Terrain。
    pub fn from_kind_str(s: &str) -> Option<Self> {
        match s {
            "plain" => Some(Terrain::Plain),
            "forest" => Some(Terrain::Forest),
            "forest_heavy" => Some(Terrain::ForestHeavy),
            "forest_light" => Some(Terrain::ForestLight),
            "mountain" => Some(Terrain::Mountain),
            "hills" => Some(Terrain::Hills),
            "water" => Some(Terrain::Water),
            "water_deep" => Some(Terrain::WaterDeep),
            "desert" => Some(Terrain::Desert),
            "swamp" => Some(Terrain::Swamp),
            "tundra" => Some(Terrain::Tundra),
            "grassland" => Some(Terrain::Grassland),
            "jungle" => Some(Terrain::Jungle),
            "urban" => Some(Terrain::Urban),
            "road" => Some(Terrain::Road),
            "bridge" => Some(Terrain::Bridge),
            "wall" => Some(Terrain::Wall),
            "farm_field" => Some(Terrain::FarmField),
            "farmhouse" => Some(Terrain::Farmhouse),
            "inn" => Some(Terrain::Inn),
            "tavern" => Some(Terrain::Tavern),
            "blacksmith" => Some(Terrain::Blacksmith),
            "general_store" => Some(Terrain::GeneralStore),
            "clinic" => Some(Terrain::Clinic),
            "workshop" => Some(Terrain::Workshop),
            "market" => Some(Terrain::Market),
            "guild_hall" => Some(Terrain::GuildHall),
            "temple" => Some(Terrain::Temple),
            "academy" => Some(Terrain::Academy),
            "library" => Some(Terrain::Library),
            "barracks" => Some(Terrain::Barracks),
            "guard_post" => Some(Terrain::GuardPost),
            "warehouse" => Some(Terrain::Warehouse),
            "granary" => Some(Terrain::Granary),
            "dock" => Some(Terrain::Dock),
            "bathhouse" => Some(Terrain::Bathhouse),
            "courthouse" => Some(Terrain::Courthouse),
            "jail" => Some(Terrain::Jail),
            "town_hall" => Some(Terrain::TownHall),
            "bank" => Some(Terrain::Bank),
            "mint" => Some(Terrain::Mint),
            "stables" => Some(Terrain::Stables),
            "caravanserai" => Some(Terrain::Caravanserai),
            "theater" => Some(Terrain::Theater),
            "arena" => Some(Terrain::Arena),
            "observatory" => Some(Terrain::Observatory),
            "alchemist" => Some(Terrain::Alchemist),
            "mage_tower" => Some(Terrain::MageTower),
            "embassy" => Some(Terrain::Embassy),
            "prison_yard" => Some(Terrain::PrisonYard),
            "beach" => Some(Terrain::Beach),
            "mine_pass" => Some(Terrain::MinePass),
            "holy_tower" => Some(Terrain::HolyTower),
            "empty_furrow" => Some(Terrain::EmptyFurrow),
            "dry_field" => Some(Terrain::DryField),
            "herb_garden" => Some(Terrain::HerbGarden),
            "fence" => Some(Terrain::Fence),
            "house" => Some(Terrain::House),
            _ => None,
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
            | Terrain::Jungle
            | Terrain::Beach
            | Terrain::MinePass
            | Terrain::EmptyFurrow
            | Terrain::FarmField
            | Terrain::DryField
            | Terrain::HerbGarden => "terrain",

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
            | Terrain::PrisonYard
            | Terrain::HolyTower
            | Terrain::House => "landmark",

            Terrain::Road | Terrain::Bridge | Terrain::Wall | Terrain::Fence => "infra",
        }
    }
    /// 此地形是否預設可步行通過
    pub fn walkable(self) -> bool {
        !matches!(
            self,
            Terrain::Mountain | Terrain::WaterDeep | Terrain::Wall | Terrain::Fence
        )
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
            | Terrain::PrisonYard
            | Terrain::HolyTower
            | Terrain::House
            | Terrain::EmptyFurrow
            | Terrain::HerbGarden => 1.0,
            Terrain::Forest | Terrain::ForestLight | Terrain::Hills => 1.5,
            Terrain::Desert | Terrain::Tundra | Terrain::FarmField | Terrain::DryField | Terrain::Beach | Terrain::MinePass => 1.5,
            Terrain::Water => 1.8,
            Terrain::ForestHeavy | Terrain::Jungle | Terrain::Swamp => 2.0,
            Terrain::WaterDeep | Terrain::Mountain | Terrain::Wall | Terrain::Fence => {
                f64::INFINITY
            }
        }
    }

    /// 取得地形對應之 23 個定案純圖樣符號
    pub fn display_char(self) -> &'static str {
        match self {
            Terrain::WaterDeep => "≋",
            Terrain::Beach => "⋯",
            Terrain::Water => "≈",
            Terrain::Bridge => "≍",
            Terrain::Swamp => "⋍",
            Terrain::Plain | Terrain::Grassland | Terrain::Desert | Terrain::Tundra => "𐡚",
            Terrain::Forest | Terrain::ForestHeavy | Terrain::ForestLight | Terrain::Jungle => "Φ",
            Terrain::Mountain => "⛰︎",
            Terrain::Hills | Terrain::MinePass => "△",
            Terrain::HolyTower | Terrain::MageTower => "∆",
            Terrain::EmptyFurrow => "≡",
            Terrain::FarmField => "▦",
            Terrain::DryField => "▤",
            Terrain::HerbGarden => "☷",
            Terrain::Wall => "■",
            Terrain::Fence => "≠",
            Terrain::Road => "::",
            Terrain::House
            | Terrain::Farmhouse
            | Terrain::Warehouse
            | Terrain::Granary
            | Terrain::Courthouse
            | Terrain::Jail
            | Terrain::PrisonYard
            | Terrain::Urban => "𖠿",
            Terrain::Market
            | Terrain::GeneralStore
            | Terrain::Bank
            | Terrain::Mint
            | Terrain::Dock
            | Terrain::Caravanserai => "$",
            Terrain::Blacksmith | Terrain::Workshop => "⚒︎",
            Terrain::Clinic => "+",
            Terrain::Inn | Terrain::Tavern | Terrain::Bathhouse => "⌂",
            Terrain::Academy
            | Terrain::Library
            | Terrain::Barracks
            | Terrain::GuardPost
            | Terrain::Temple
            | Terrain::GuildHall
            | Terrain::TownHall
            | Terrain::Theater
            | Terrain::Arena
            | Terrain::Observatory
            | Terrain::Alchemist
            | Terrain::Embassy
            | Terrain::Stables => "⚑",
        }
    }

    /// `display_char` 的別名方法，回傳 23 個定案純符號之一
    pub fn symbol(self) -> &'static str {
        self.display_char()
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
            Terrain::Water => "#70b0e0",
            Terrain::WaterDeep => "#204090",
            Terrain::Beach => "#e8d8b8",
            Terrain::Grassland => "#c2d6a4",
            Terrain::Plain | Terrain::Desert => "#e3d5ca",
            Terrain::Road | Terrain::Bridge => "#c4b8a8",
            Terrain::Tundra => "#e0f2f1",
            Terrain::FarmField => "#9ef01a",
            Terrain::DryField => "#d4c49a",
            Terrain::EmptyFurrow => "#c8b090",
            Terrain::HerbGarden => "#a8d5ba",
            Terrain::Wall => "#8c8c8c",
            Terrain::Fence => "#a08060",
            Terrain::MinePass => "#706860",
            Terrain::HolyTower => "#e0d8ff",
            Terrain::House => "#d8c8b8",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_object_tag_and_archetype() {
        let waymark = GridObject {
            id: "waymark_0_18".to_string(),
            kind: GridObjectKind::Other,
            name: "北麓指路石碑".to_string(),
            quantity: 1,
            max_quantity: 1,
            regrowth_per_tick: 0,
        };
        assert_eq!(waymark.tag(), "way_stele");
        assert_eq!(waymark.archetype_tag(), "stele");
        assert_eq!(waymark.category_tag(), "public_fixture");
        assert_eq!(waymark.kind.category_tag(), "wilderness_feature");
        assert!(!waymark.attribute_tags().is_empty());
        assert!(waymark.attribute_tags().contains(&"wayfinding"));
        assert!(waymark.has_tag("way_stele"));
        assert!(waymark.has_tag("stele"));
        assert!(waymark.has_tag("public_fixture"));
        assert!(!waymark.has_tag("furniture_bedding_building"));
        assert!(waymark.has_tag("wilderness_feature"));
        assert!(waymark.has_tag("wayfinding"));
        assert!(waymark.tags().contains(&"wayfinding"));
        assert!(waymark.tags().contains(&"way_stele"));

        let campfire = GridObject {
            id: "campfire_1_2".to_string(),
            kind: GridObjectKind::Other,
            name: "野地營火".to_string(),
            quantity: 5,
            max_quantity: 10,
            regrowth_per_tick: 1,
        };
        assert_eq!(campfire.tag(), "wilderness_campfire");
        assert_eq!(campfire.archetype_tag(), "campfire");
        assert_eq!(campfire.category_tag(), "maritime_transport_outdoor");
        assert_eq!(campfire.kind.category_tag(), "wilderness_feature");
        assert!(campfire.has_tag("campfire"));
        assert!(campfire.has_tag("wilderness_campfire"));
        assert!(campfire.has_tag("maritime_transport_outdoor"));
        assert!(campfire.has_tag("wilderness_feature"));
        assert!(!campfire.attribute_tags().is_empty());
        assert!(campfire.attribute_tags().contains(&"heating"));
        assert!(campfire.has_tag("heating"));
        assert!(campfire.tags().contains(&"heating"));
        assert!(campfire.tags().contains(&"wilderness_campfire"));

        let tracks = GridObject {
            id: "tracks_5_6".to_string(),
            kind: GridObjectKind::Other,
            name: "獸徑足印".to_string(),
            quantity: 1,
            max_quantity: 1,
            regrowth_per_tick: 0,
        };
        assert_eq!(tracks.tag(), "beast_tracks");
        assert_eq!(tracks.archetype_tag(), "beast_tracks");
        assert_eq!(tracks.category_tag(), "maritime_transport_outdoor");
        assert_eq!(tracks.kind.category_tag(), "wilderness_feature");
        assert!(tracks.has_tag("beast_tracks"));
        assert!(tracks.has_tag("maritime_transport_outdoor"));
        assert!(tracks.has_tag("wilderness_feature"));
        assert!(!tracks.attribute_tags().is_empty());
        assert!(tracks.has_tag("hunting"));
        assert_eq!(
            &tracks.tags()[..3],
            &["beast_tracks", "maritime_transport_outdoor", "wilderness_feature"]
        );

        let spring = GridObject {
            id: "spring_2_3".to_string(),
            kind: GridObjectKind::WaterSource,
            name: "山澗清泉眼".to_string(),
            quantity: 8,
            max_quantity: 12,
            regrowth_per_tick: 2,
        };
        assert_eq!(spring.tag(), "mountain_spring");
        assert_eq!(spring.archetype_tag(), "mountain_spring");
        assert_eq!(spring.category_tag(), "plumbing_bath_hygiene");
        assert_eq!(spring.kind.category_tag(), "water_source");
        assert!(spring.has_tag("mountain_spring"));
        assert!(spring.has_tag("plumbing_bath_hygiene"));
        assert!(spring.has_tag("water_source"));
        assert!(!spring.attribute_tags().is_empty());
        assert_eq!(
            &spring.tags()[..3],
            &["mountain_spring", "plumbing_bath_hygiene", "water_source"]
        );

        let niche = GridObject {
            id: "cairn_niche_7_8".to_string(),
            kind: GridObjectKind::Other,
            name: "避風岩龕".to_string(),
            quantity: 1,
            max_quantity: 1,
            regrowth_per_tick: 0,
        };
        assert_eq!(niche.tag(), "rock_niche");
        assert_eq!(niche.archetype_tag(), "rock_niche");
        assert_eq!(niche.category_tag(), "maritime_transport_outdoor");
        assert_eq!(niche.kind.category_tag(), "wilderness_feature");
        assert!(niche.has_tag("rock_niche"));
        assert!(niche.has_tag("maritime_transport_outdoor"));
        assert!(niche.has_tag("wilderness_feature"));
        assert!(!niche.attribute_tags().is_empty());
        assert_eq!(
            &niche.tags()[..3],
            &["rock_niche", "maritime_transport_outdoor", "wilderness_feature"]
        );

        let logs = GridObject {
            id: "stepping_logs_1_1".to_string(),
            kind: GridObjectKind::Other,
            name: "墊步木墩".to_string(),
            quantity: 1,
            max_quantity: 1,
            regrowth_per_tick: 0,
        };
        assert_eq!(logs.tag(), "stepping_logs");
        assert_eq!(logs.archetype_tag(), "stepping_logs");
        assert_eq!(logs.category_tag(), "maritime_transport_outdoor");
        assert_eq!(logs.kind.category_tag(), "wilderness_feature");
        assert!(logs.has_tag("stepping_logs"));
        assert!(logs.has_tag("maritime_transport_outdoor"));
        assert!(logs.has_tag("wilderness_feature"));
        assert!(!logs.attribute_tags().is_empty());
        assert_eq!(
            &logs.tags()[..3],
            &["stepping_logs", "maritime_transport_outdoor", "wilderness_feature"]
        );

        let post = GridObject {
            id: "hunter_post_3_3".to_string(),
            kind: GridObjectKind::Other,
            name: "巡山木樁".to_string(),
            quantity: 1,
            max_quantity: 1,
            regrowth_per_tick: 0,
        };
        assert_eq!(post.tag(), "hunter_post");
        assert_eq!(post.archetype_tag(), "hunter_post");
        assert_eq!(post.category_tag(), "maritime_transport_outdoor");
        assert_eq!(post.kind.category_tag(), "wilderness_feature");
        assert!(post.has_tag("hunter_post"));
        assert!(post.has_tag("maritime_transport_outdoor"));
        assert!(post.has_tag("wilderness_feature"));
        assert!(!post.attribute_tags().is_empty());
        assert_eq!(
            &post.tags()[..3],
            &["hunter_post", "maritime_transport_outdoor", "wilderness_feature"]
        );
        let herb = GridObject {
            id: "creek_herb_1_1".to_string(),
            kind: GridObjectKind::Flora,
            name: "溪邊水芹".to_string(),
            quantity: 3,
            max_quantity: 5,
            regrowth_per_tick: 1,
        };
        assert_eq!(herb.kind.category_tag(), "flora_resource");
        assert_eq!(herb.category_tag(), "herbs_flora_spices");
        assert!(herb.has_tag("flora_resource"));
        assert!(herb.has_tag("herbs_flora_spices"));

        let wood = GridObject {
            id: "fallen_wood_1_1".to_string(),
            kind: GridObjectKind::Flora,
            name: "林下落枯枝".to_string(),
            quantity: 3,
            max_quantity: 5,
            regrowth_per_tick: 1,
        };
        assert_eq!(wood.kind.category_tag(), "flora_resource");
        assert_eq!(wood.category_tag(), "timber_bamboo_fungi");
        assert!(wood.has_tag("flora_resource"));
        assert!(wood.has_tag("timber_bamboo_fungi"));

        let ore = GridObject {
            id: "jade_ore_1_1".to_string(),
            kind: GridObjectKind::Mineral,
            name: "溪畔水磨青卵石".to_string(),
            quantity: 5,
            max_quantity: 10,
            regrowth_per_tick: 1,
        };
        assert_eq!(ore.kind.category_tag(), "mineral_resource");
        assert_eq!(ore.category_tag(), "mineral_jade");
        assert!(ore.has_tag("mineral_resource"));
        assert!(ore.has_tag("mineral_jade"));
    }
}
