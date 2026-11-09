
pub fn find_scene_object_spec(id: &str, name: &str) -> Option<&'static SceneObjectTagSpec> {
    let clean_id = id.trim();
    let clean_name = name.trim();

    // 1. 比對已知 ID
    if !clean_id.is_empty() {
        for spec in SCENE_OBJECT_TAG_SPECS {
            if spec.id == clean_id {
                return Some(spec);
            }
        }
    }

    // 2. 比對完全相符的中文名稱別名
    if !clean_name.is_empty() {
        for spec in SCENE_OBJECT_TAG_SPECS {
            if spec.name_alias == clean_name {
                return Some(spec);
            }
        }
    }

    // 3. 特殊業務規則
    if clean_name.contains("當值櫃檯") || clean_name.contains("當值櫃台") || clean_id == "wt_venue_counter" {
        for spec in SCENE_OBJECT_TAG_SPECS {
            if spec.id == "wt_venue_counter" {
                return Some(spec);
            }
        }
    }

    if clean_name.ends_with("・圍獵遺骸") || clean_id.starts_with("hunt_kill_site_") {
        for spec in SCENE_OBJECT_TAG_SPECS {
            if spec.id == "hunt_kill_site_remains" {
                return Some(spec);
            }
        }
    }

    if clean_id.starts_with("detritus_fungi_") {
        for spec in SCENE_OBJECT_TAG_SPECS {
            if spec.id == "detritus_fungi_cluster" {
                return Some(spec);
            }
        }
    }

    // 4. 前後包含比對
    if !clean_name.is_empty() {
        for spec in SCENE_OBJECT_TAG_SPECS {
            if clean_name.contains(spec.name_alias) || spec.name_alias.contains(clean_name) {
                return Some(spec);
            }
        }
    }

    None
}

/// 取得物件的屬性標籤清單（attribute tags）。
pub fn object_attribute_tags(id: &str, name: &str) -> Vec<&'static str> {
    let clean_id = id.trim();
    let clean_name = name.trim();

    // 1. 若為千物總典項目（wt_cat_* 或 catalog 查詢命中），直接採用總典規範之屬性標籤
    if clean_id.starts_with("wt_cat_") {
        if let Some(entry) = crate::game::peninsula_catalog::find_catalog_entry_by_id_or_name(clean_id, clean_name) {
            return entry.attribute_tags().to_vec();
        }
        if let Some(entry) = crate::game::peninsula_catalog::find_catalog_entry(clean_id) {
            return entry.attribute_tags().to_vec();
        }
    }

    // 1b. 查竹林產出物品與地景物件規範
    if let Some(z) = crate::grid::zhulin::lookup_zhulin_item_def(clean_id) {
        return z.tags.attribute_tags.to_vec();
    }
    if let Some(gspec) = crate::grid::cell::find_grid_object_spec(clean_id, clean_name) {
        return gspec.attribute_tags.to_vec();
    }

    // 2. 查非目錄場景物件規範表（SCENE_OBJECT_TAG_SPECS）
    if let Some(spec) = find_scene_object_spec(clean_id, clean_name) {
        return spec.attribute_tags.to_vec();
    }
    // 3. 次選：一般名稱查目錄
    if let Some(entry) = crate::game::peninsula_catalog::find_catalog_entry_by_id_or_name(clean_id, clean_name) {
        return entry.attribute_tags().to_vec();
    }

    // 4. Fallback：依分類標籤查超分類屬性
    let cat = object_category_tag(clean_id, clean_name);
    let mut tags = crate::game::peninsula_catalog::super_category_tags_for_category(cat).to_vec();
    let arch = object_archetype_tag(clean_id, clean_name);
    if !arch.is_empty() && !tags.contains(&arch) {
        tags.push(arch);
    }
    tags
}


/// 3. 保證回傳確定性、非空且純英文的 snake_case 標籤。
pub fn canonical_object_tag(id: &str, name: &str) -> String {
    let clean_id = id.trim();
    let clean_name = name.trim();

    // 1. 若 ID 為白塔半島千物總典編號（wt_cat_*），優先直接回傳總典專屬 object_tag
    if clean_id.starts_with("wt_cat_") {
        if let Some(entry) = crate::game::peninsula_catalog::find_catalog_entry_by_id_or_name(clean_id, clean_name) {
            if !entry.object_tag.is_empty() {
                return entry.object_tag.to_string();
            }
        }
    }

    // 1b. 查竹林產出物品與地景物件規範
    if let Some(z) = crate::grid::zhulin::lookup_zhulin_item_def(clean_id) {
        return z.tags.exclusive_tag.to_string();
    }
    if let Some(gspec) = crate::grid::cell::find_grid_object_spec(clean_id, clean_name) {
        return gspec.object_tag.to_string();
    }

    // 2. 查場景物件規格表（SCENE_OBJECT_TAG_SPECS）
    if let Some(spec) = find_scene_object_spec(clean_id, clean_name) {
        return spec.object_tag.to_string();
    }
    // 3. 特殊格式比對
    if clean_id.starts_with("resource:") {
        let parts: Vec<&str> = clean_id.split(':').collect();
        if parts.len() >= 3 {
            let item_part = parts[2].trim().replace('-', "_").to_ascii_lowercase();
            return format!("resource_{}", item_part);
        }
        return "resource_node".to_string();
    }

    // 4. 以中文名稱或 ID 查白塔半島千物總典（1,109 件世界物件）
    if let Some(entry) = crate::game::peninsula_catalog::find_catalog_entry_by_id_or_name(clean_id, clean_name) {
        if !entry.object_tag.is_empty() {
            return entry.object_tag.to_string();
        }
    }

    // 5. 若 id 為合法的英數字 snake_case，直接轉為標籤
    let slug = slugify_identifier(clean_id);
    if !slug.is_empty() && slug != "scene_object" {
        return slug;
    }

    // 6. 依 archetype 與名稱特徵提供確定性英文標籤 fallback
    let arch = object_archetype_tag(clean_id, clean_name);
    format!("peninsula_{}", arch)
}


/// 取得物件的語意原型標籤（functional archetype tag）。
pub fn object_archetype_tag(id: &str, name: &str) -> &'static str {
    let clean_id = id.trim();
    let clean_name = name.trim();

    // 1. 優先查竹林產出物品、地景物件與場景物件規格表
    if crate::grid::zhulin::lookup_zhulin_item_def(clean_id).is_some() {
        return "resource_node";
    }
    if let Some(gspec) = crate::grid::cell::find_grid_object_spec(clean_id, clean_name) {
        return gspec.archetype_tag;
    }
    if let Some(spec) = find_scene_object_spec(clean_id, clean_name) {
        return spec.archetype_tag;
    }
    // 2. 查千物總典
    if let Some(entry) = crate::game::peninsula_catalog::find_catalog_entry_by_id_or_name(clean_id, clean_name) {
        // 從 entry.category_tag 或 entry.attribute_tags 判定原型
        for &attr in entry.attribute_tags() {
            match attr {
                "street_lamp" | "desk_lamp" | "sundial" | "telescope" | "astrolabe" | "bell"
                | "railing" | "bench" | "stele" | "bollard" | "bed" | "wardrobe" | "tea_table"
                | "chess_board" | "notice_board" | "bookshelf" | "ledger_cabinet" | "counter"
                | "zither" | "water_tap" | "bath_tub" | "laundry_tub" | "ice_cabinet"
                | "camp_stove" | "cauldron" | "mailbox" | "thermos_flask" | "fishing_rod"
                | "raft" | "cargo_ship" | "campfire" | "beast_tracks" | "rock_niche"
                | "hunter_post" | "stepping_logs" | "way_stele" | "watermill" | "stone_cutter"
                | "sawmill" | "mine_rail" | "anvil" | "firewood_pile" | "crop_plot"
                | "herb_mortar" | "mountain_spring" => return attr,
                _ => {}
            }
        }
        return "scene_fixture";
    }

    // 3. ID 特徵比對
    if clean_id.starts_with("resource:") || clean_id.starts_with("sq_") {
        return "resource_node";
    }
    "scene_fixture"
}


/// 取得物件的分類標籤（category tag）。
pub fn object_category_tag(id: &str, name: &str) -> &'static str {
    let clean_id = id.trim();
    let clean_name = name.trim();

    // 1. 若 ID 為白塔半島千物總典編號（wt_cat_*）或目錄查詢命中，優先取總典 category_tag
    if clean_id.starts_with("wt_cat_") {
        if let Some(entry) = crate::game::peninsula_catalog::find_catalog_entry_by_id_or_name(clean_id, clean_name) {
            return crate::game::peninsula_catalog::category_tag_for_num(entry.category_num);
        }
        if let Some(entry) = crate::game::peninsula_catalog::find_catalog_entry(clean_id) {
            return crate::game::peninsula_catalog::category_tag_for_num(entry.category_num);
        }
    }

    // 1b. 查竹林產出物品與地景物件規範
    if let Some(z) = crate::grid::zhulin::lookup_zhulin_item_def(clean_id) {
        return z.tags.category_tags.first().copied().unwrap_or("timber_bamboo_fungi");
    }
    if let Some(gspec) = crate::grid::cell::find_grid_object_spec(clean_id, clean_name) {
        return gspec.category_tag;
    }

    // 2. 查場景物件規格表（SCENE_OBJECT_TAG_SPECS）
    if let Some(spec) = find_scene_object_spec(clean_id, clean_name) {
        return spec.category_tag;
    }
    // 3. 目錄一般查詢
    if let Some(entry) = crate::game::peninsula_catalog::find_catalog_entry_by_id_or_name(clean_id, clean_name) {
        return crate::game::peninsula_catalog::category_tag_for_num(entry.category_num);
    }

    // 4. 由原型或 ID 映射
    let arch = object_archetype_tag(clean_id, clean_name);
    match arch {
        "street_lamp" | "desk_lamp" | "sundial" | "telescope" | "astrolabe" | "bell" => {
            "lighting_clocks_optics"
        }
        "railing" | "bench" | "stele" | "bollard" | "notice_board" | "way_stele"
        | "water_trough" | "gate_door" | "shelter_awning" => "public_fixture",
        "bed" | "wardrobe" | "tea_table" | "chess_board" => "furniture_bedding_building",
        "bookshelf" | "ledger_cabinet" | "counter" | "zither" => {
            "stationery_books_finance"
        }
        "water_tap" | "bath_tub" | "laundry_tub" | "razor_stand" => "plumbing_bath_hygiene",
        "ice_cabinet" | "camp_stove" | "cauldron" => "kitchenware_cookware",
        "mailbox" | "thermos_flask" | "carrying_pole" | "sack_pile" => "bags_umbrellas_hardware",
        "fishing_rod" | "raft" | "cargo_ship" | "campfire" | "beast_tracks" | "rock_niche"
        | "hunter_post" | "stepping_logs" | "crossbow" | "mooring_rope" => "maritime_transport_outdoor",
        "watermill" | "stone_cutter" | "sawmill" | "mine_rail" | "anvil" | "firewood_pile"
        | "crop_plot" | "rail_cart" => "craft_machinery_tools",
        "herb_mortar" | "mountain_spring" | "hot_spring_pool" => "medical_hot_spring_wellness",
        "resource_node" => {
            if clean_id.contains("ore")
                || clean_id.contains("stone")
                || clean_id.contains("jade")
                || clean_id.contains("sand")
            {
                "mineral_jade"
            } else if clean_id.contains("wood")
                || clean_id.contains("timber")
                || clean_id.contains("bamboo")
                || clean_id.contains("fungi")
            {
                "timber_bamboo_fungi"
            } else if clean_id.contains("fish")
                || clean_id.contains("kelp")
                || clean_id.contains("shell")
            {
                "seafood_kelp"
            } else if clean_id.contains("hide")
                || clean_id.contains("meat")
                || clean_id.contains("milk")
            {
                "livestock_hides_bones"
            } else {
                "herbs_flora_spices"
            }
        }
        _ => "furniture_bedding_building",
    }
}


/// 取得物件的所有語意標籤清單（包含專屬 tag、category tag、archetype tag、屬性標籤、超分類標籤、目錄房間 tag、sockets 標籤，去重）。
pub fn object_semantic_tags(id: &str, name: &str, sockets: &[String]) -> Vec<String> {
    let mut tags = Vec::new();
    let dedicated = canonical_object_tag(id, name);
    if !dedicated.is_empty() {
        tags.push(dedicated);
    }
    let cat_tag = object_category_tag(id, name);
    if !cat_tag.is_empty() && !tags.iter().any(|t| t == cat_tag) {
        tags.push(cat_tag.to_string());
    }
    let archetype = object_archetype_tag(id, name);
    if !archetype.is_empty() && !tags.iter().any(|t| t == archetype) {
        tags.push(archetype.to_string());
    }
    for attr in object_attribute_tags(id, name) {
        if !attr.is_empty() && !tags.iter().any(|t| t == attr) {
            tags.push(attr.to_string());
        }
    }
    for super_tag in crate::game::peninsula_catalog::super_category_tags_for_category(cat_tag) {
        if !super_tag.is_empty() && !tags.iter().any(|t| t == super_tag) {
            tags.push(super_tag.to_string());
        }
    }
    if let Some(entry) = crate::game::peninsula_catalog::find_catalog_entry_by_id_or_name(id.trim(), name.trim()) {
        if !entry.room_tag.is_empty() && !tags.iter().any(|t| t == entry.room_tag) {
            tags.push(entry.room_tag.to_string());
        }
    }
    for sock in sockets {
        let sock_clean = sock.trim().to_ascii_lowercase();
        if !sock_clean.is_empty() && !tags.iter().any(|t| t == &sock_clean) {
            tags.push(sock_clean);
        }
    }
    tags
}


/// 單一房間節點。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub zone: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<RoomObject>,
}

/// 單一出口：方向 → 目標房間。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exit {
    pub direction: String,
    pub to_room_id: String,
    #[serde(default)]
    pub to_room_name: String,
    /// 正式世界路線 id；舊房間出口留空。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub route_id: String,
    /// walk / road / ferry / passage。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mode: String,
    /// 人物共用的通行耗時；舊房間出口為 0。
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub travel_cost: i32,
}

fn is_zero_i32(value: &i32) -> bool {
    *value == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_object_tag_and_archetype() {
        // 1. Catalog lookup
        let cat_entry_tag = canonical_object_tag("wt_cat_0001", "赤鐵重礦");
        assert!(!cat_entry_tag.is_empty());
        assert!(cat_entry_tag.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'));

        // 2. Specific scene object names from specification
        assert_eq!(canonical_object_tag("wt_obj_1", "垂柳石欄"), "willow_stone_railing");
        assert_eq!(object_archetype_tag("wt_obj_1", "垂柳石欄"), "railing");

        assert_eq!(canonical_object_tag("wt_obj_2", "防汛水尺碑"), "flood_gauge_stele");
        assert_eq!(object_archetype_tag("wt_obj_2", "防汛水尺碑"), "stele");

        assert_eq!(canonical_object_tag("wt_obj_3", "貝母街燈"), "mother_of_pearl_street_lamp");
        assert_eq!(object_archetype_tag("wt_obj_3", "街燈"), "street_lamp");

        assert_eq!(canonical_object_tag("wt_obj_4", "繫船石柱"), "wharf_mooring_bollard");
        assert_eq!(object_archetype_tag("wt_obj_4", "繫船石柱"), "bollard");

        assert_eq!(canonical_object_tag("wt_obj_5", "青石歇腳凳"), "bluestone_rest_bench");
        assert_eq!(canonical_object_tag("wt_obj_6", "青石長凳"), "bluestone_rest_bench");
        assert_eq!(object_archetype_tag("wt_obj_5", "歇腳凳"), "bench");

        assert_eq!(canonical_object_tag("wt_obj_7", "防雨告示欄"), "rainproof_notice_board");
        assert_eq!(object_archetype_tag("wt_obj_7", "告示欄"), "notice_board");

        assert_eq!(canonical_object_tag("wt_obj_8", "黃銅水栓"), "brass_water_tap");
        assert_eq!(object_archetype_tag("wt_obj_8", "水栓"), "water_tap");

        assert_eq!(canonical_object_tag("wt_reg_wenyao_home", "床榻"), "woven_rush_bed");
        assert_eq!(object_archetype_tag("wt_reg_wenyao_home", "床榻"), "bed");

        assert_eq!(canonical_object_tag("wt_inn_bath_Preview", "沐浴桶"), "warm_stone_bath_tub");
        assert_eq!(object_archetype_tag("wt_inn_bath_Preview", "沐浴桶"), "bath_tub");

        assert_eq!(canonical_object_tag("wt_home_shell_lamp", "檯燈"), "mother_of_pearl_desk_lamp");
        assert_eq!(object_archetype_tag("wt_home_shell_lamp", "檯燈"), "desk_lamp");

        assert_eq!(canonical_object_tag("wt_home_jade_cabinet", "冰鑑櫃"), "cold_jade_ice_cabinet");
        assert_eq!(object_archetype_tag("wt_home_jade_cabinet", "冰鑑櫃"), "ice_cabinet");

        assert_eq!(canonical_object_tag("wt_home_bamboo_mailbox", "信箱"), "oiled_bamboo_mailbox");
        assert_eq!(object_archetype_tag("wt_home_bamboo_mailbox", "信箱"), "mailbox");

        assert_eq!(canonical_object_tag("wt_home_charcoal_wardrobe", "衣櫥"), "camphor_charcoal_wardrobe");
        assert_eq!(object_archetype_tag("wt_home_charcoal_wardrobe", "衣櫥"), "wardrobe");

        assert_eq!(canonical_object_tag("wt_coastal_angling_spot", "釣竿"), "bamboo_reel_fishing_rod");
        assert_eq!(object_archetype_tag("wt_coastal_angling_spot", "釣竿"), "fishing_rod");

        assert_eq!(canonical_object_tag("wt_lookout_telescope", "望遠鏡"), "red_earth_brass_telescope");
        assert_eq!(object_archetype_tag("wt_lookout_telescope", "望遠鏡"), "telescope");

        assert_eq!(canonical_object_tag("wt_field_camp_stove", "卡式導能便攜爐"), "cassette_energy_camp_stove");
        assert_eq!(object_archetype_tag("wt_field_camp_stove", "便攜爐"), "camp_stove");

        assert_eq!(canonical_object_tag("wt_venue_counter", "鐵鋪・當值櫃檯"), "venue_duty_counter");
        assert_eq!(object_archetype_tag("wt_venue_counter", "當值櫃檯"), "counter");

        assert_eq!(canonical_object_tag("wt_dining_iron_cauldron", "紅鐵燉肉釜"), "red_iron_stew_cauldron");
        assert_eq!(object_archetype_tag("wt_dining_iron_cauldron", "紅鐵燉肉釜"), "cauldron");

        assert_eq!(canonical_object_tag("wt_dining_rice_milk_stall", "溫米漿桶"), "warm_rice_milk_barrel");
        assert_eq!(canonical_object_tag("wt_item_flask", "隨身保溫瓶"), "ebony_thermos_flask");
        assert_eq!(canonical_object_tag("wt_dock_riverbank_raft", "平底竹筏"), "flat_bottom_bamboo_raft");
        assert_eq!(canonical_object_tag("wt_harbor_clockwise_ship", "順時針環島海運商船"), "clockwise_coastal_merchant_ship");
        assert_eq!(canonical_object_tag("wt_harbor_insurance_desk", "築港頭海事承保櫃檯"), "maritime_insurance_counter");
        assert_eq!(canonical_object_tag("wt_caravan_ticket_counter", "班次木牌案"), "caravan_schedule_desk");
        assert_eq!(canonical_object_tag("wt_courier_post_desk", "疾行草獸信使櫃檯"), "courier_post_counter");
        assert_eq!(canonical_object_tag("wt_bank_ledger_drawer", "分格存摺櫃"), "compartment_passbook_cabinet");
        assert_eq!(canonical_object_tag("wt_field_crop_plot", "當季田壟苗畦"), "seasonal_crop_plot");
        assert_eq!(canonical_object_tag("wt_multi_firewood_pile", "風乾柴木垛"), "seasoned_firewood_stack");
        assert_eq!(canonical_object_tag("dyn_campfire", "野地營火堆"), "wilderness_campfire");
        assert_eq!(canonical_object_tag("dyn_field_anvil", "野地修具石砧"), "field_repair_stone_anvil");
        assert_eq!(canonical_object_tag("dyn_herb_stove", "野地煎藥陶銚"), "field_herb_decoction_pot");
        assert_eq!(canonical_object_tag("shanhai_apex_trace", "新鮮霸主獸跡"), "apex_beast_tracks");
        assert_eq!(canonical_object_tag("detritus_fungi_3_4", "腐木牛樟靈菇叢"), "camphor_spirit_fungi_cluster");

        // 3. RoomObject methods
        let room_obj = RoomObject {
            id: "wt_home_bamboo_mailbox".to_string(),
            name: "信箱".to_string(),
            owner: "world".to_string(),
            sockets: vec!["Look".to_string(), "DispatchCourier".to_string()],
            responses: HashMap::new(),
            move_to_room_id: String::new(),
        };
        assert_eq!(room_obj.tag(), "oiled_bamboo_mailbox");
        assert_eq!(room_obj.archetype_tag(), "mailbox");
        assert_eq!(room_obj.category_tag(), "bags_umbrellas_hardware");
        assert!(!room_obj.attribute_tags().is_empty());
        assert!(room_obj.attribute_tags().contains(&"bamboo".to_string()));
        assert!(room_obj.attribute_tags().contains(&"storage".to_string()));
        assert!(room_obj.attribute_tags().contains(&"waterproof".to_string()));
        assert!(room_obj.has_tag("oiled_bamboo_mailbox"));
        assert!(room_obj.has_tag("mailbox"));
        assert!(room_obj.has_tag("bags_umbrellas_hardware"));
        assert!(room_obj.has_tag("carry_gear"));
        assert!(room_obj.has_tag("crafted_good"));
        assert!(room_obj.has_tag("dispatchcourier"));
        let tags = room_obj.tags();
        assert!(tags.contains(&"oiled_bamboo_mailbox".to_string()));
        assert!(tags.contains(&"mailbox".to_string()));
        assert!(tags.contains(&"bags_umbrellas_hardware".to_string()));
        assert!(tags.contains(&"carry_gear".to_string()));
        assert!(tags.contains(&"crafted_good".to_string()));
        assert!(tags.contains(&"look".to_string()));

        // 4. Multi-level category tag and super-category assertions across categories
        let ore_obj = RoomObject {
            id: "wt_cat_0001".to_string(),
            name: "赤鐵重礦".to_string(),
            owner: "world".to_string(),
            sockets: vec!["Take".to_string()],
            responses: HashMap::new(),
            move_to_room_id: String::new(),
        };
        assert_eq!(ore_obj.category_tag(), "mineral_jade");
        assert!(ore_obj.has_tag("mineral_jade"));
        assert!(ore_obj.has_tag("raw_material"));
        assert!(ore_obj.has_tag("mineral"));
        assert!(ore_obj.tags().contains(&"mineral_jade".to_string()));
        assert!(ore_obj.tags().contains(&"raw_material".to_string()));

        let stove_obj = RoomObject {
            id: "wt_field_camp_stove".to_string(),
            name: "便攜爐".to_string(),
            owner: "world".to_string(),
            sockets: vec!["Cook".to_string()],
            responses: HashMap::new(),
            move_to_room_id: String::new(),
        };
        assert_eq!(stove_obj.category_tag(), "kitchenware_cookware");
        assert!(stove_obj.has_tag("kitchenware_cookware"));
        assert!(stove_obj.has_tag("crafted_good"));
        assert!(stove_obj.tags().contains(&"kitchenware_cookware".to_string()));

        let railing_obj = RoomObject {
            id: "wt_obj_1".to_string(),
            name: "垂柳石欄".to_string(),
            owner: "world".to_string(),
            sockets: vec!["Look".to_string()],
            responses: HashMap::new(),
            move_to_room_id: String::new(),
        };
        assert_eq!(railing_obj.category_tag(), "public_fixture");
        assert!(railing_obj.has_tag("public_fixture"));
        assert!(railing_obj.has_tag("outdoor_fixture"));
        assert!(!railing_obj.has_tag("furniture"));
        assert!(!railing_obj.has_tag("interior_fixture"));
        assert!(railing_obj.tags().contains(&"public_fixture".to_string()));
        let lamp_obj = RoomObject {
            id: "wt_obj_3".to_string(),
            name: "貝母街燈".to_string(),
            owner: "world".to_string(),
            sockets: vec!["Look".to_string()],
            responses: HashMap::new(),
            move_to_room_id: String::new(),
        };
        assert_eq!(lamp_obj.category_tag(), "lighting_clocks_optics");
        assert!(lamp_obj.has_tag("lighting_clocks_optics"));
        assert!(lamp_obj.has_tag("instrument"));
        assert!(lamp_obj.tags().contains(&"lighting_clocks_optics".to_string()));
    }

    #[test]
    fn test_orthogonal_three_tag_invariants_no_contradictions() {
        // 1. 驗證千物總典 1,109 筆：attribute_tags 不得混入錯誤跨類標籤或室內外矛盾
        for entry in crate::game::peninsula_catalog::PENINSULA_WORLD_OBJECTS {
            let attrs = entry.attribute_tags();
            assert!(!attrs.contains(&entry.object_tag), "{} attribute_tags repeats object_tag", entry.id);
            assert!(!attrs.contains(&entry.category_tag), "{} attribute_tags repeats category_tag", entry.id);
            assert!(
                !(attrs.contains(&"fixed") && attrs.contains(&"portable")),
                "{} has both fixed and portable",
                entry.id
            );
            assert!(
                !(attrs.contains(&"interior_fixture") && attrs.contains(&"outdoor_fixture")),
                "{} has both interior_fixture and outdoor_fixture",
                entry.id
            );
            if !matches!(entry.category_num, "04" | "05" | "06" | "07" | "08" | "09" | "10") {
                for &food_tag in &["shellfish", "hot_food", "fresh_drink", "grain", "edible", "drinkable", "food_ingredient"] {
                    assert!(
                        !attrs.contains(&food_tag),
                        "{} ({}) in non-food category {} has food tag {}",
                        entry.id,
                        entry.name,
                        entry.category_num,
                        food_tag
                    );
                }
            }
        }
        // 2. 驗證場景設施表 118 筆：無重複標籤、無室內外矛盾、無誤標 shellfish、戶外橋欄與水尺碑必為 public_fixture
        for spec in SCENE_OBJECT_TAG_SPECS {
            let attrs = spec.attribute_tags;
            assert!(!attrs.contains(&spec.object_tag), "Scene spec {} repeats object_tag", spec.id);
            assert!(!attrs.contains(&spec.category_tag), "Scene spec {} repeats category_tag", spec.id);
            assert!(!attrs.contains(&spec.archetype_tag), "Scene spec {} repeats archetype_tag", spec.id);
            assert!(
                !(attrs.contains(&"interior_fixture") && attrs.contains(&"outdoor_fixture")),
                "Scene spec {} has both interior_fixture and outdoor_fixture",
                spec.id
            );
            if spec.category_tag != "seafood_kelp" {
                assert!(
                    !attrs.contains(&"shellfish"),
                    "Scene spec {} ({}) has false-positive shellfish tag",
                    spec.id,
                    spec.name_alias
                );
            }
        }
        // 3. 驗證野外地景物件表 44 筆：無重複標籤、無室內外矛盾、戶外石碑／繫樁／護欄必為 public_fixture
        for spec in crate::grid::cell::GRID_OBJECT_TAG_SPECS {
            let attrs = spec.attribute_tags;
            assert!(!attrs.contains(&spec.object_tag), "Grid spec {} repeats object_tag", spec.pattern);
            assert!(!attrs.contains(&spec.category_tag), "Grid spec {} repeats category_tag", spec.pattern);
            assert!(!attrs.contains(&spec.archetype_tag), "Grid spec {} repeats archetype_tag", spec.pattern);
            assert!(
                !(attrs.contains(&"interior_fixture") && attrs.contains(&"outdoor_fixture")),
                "Grid spec {} has both interior_fixture and outdoor_fixture",
                spec.pattern
            );
            if matches!(spec.archetype_tag, "stele" | "bollard" | "bench" | "railing") {
                assert_eq!(
                    spec.category_tag, "public_fixture",
                    "Grid spec {} ({}) outdoor archetype {} must have category_tag public_fixture",
                    spec.pattern, spec.name_alias, spec.archetype_tag
                );
            }
        }
    }
}
