/// 清理分區字串中的純聚落／地名，避免聚落名稱讓全鎮所有道路與無關建築全部沾染單一資源標籤
fn strip_settlement_zone_names(zone: &str) -> String {
    let mut s = zone.to_string();
    for settlement in [
        "文曜", "風舶角", "風舶嶼", "大圳頭", "半嶺鎮", "半嶺", "南哨", "老堰頭",
        "北嶺腳", "北嶺", "霜林口", "風櫃尾", "烏石鼻", "牛埔", "紅土崁", "三號坑口",
        "三號坑", "水尾寮", "山坳鎮", "築港頭", "南灣",
    ] {
        s = s.replace(settlement, "");
    }
    s
}

/// 將房間語意標籤（中文或英文）標準化為英文常規標籤
pub fn canonical_room_tag(tag: &str) -> &'static str {
    match tag {
        "residence" | "住宅" | "民居" | "民宅" => "residence",
        "street" | "街道" => "street",
        "road" | "道路" => "road",
        "bridge" | "石橋" | "橋樑" => "bridge",
        "plaza" | "廣場" => "plaza",
        "dock" | "碼頭" => "dock",
        "waterfront" | "水岸" => "waterfront",
        "beach" | "海灘" => "beach",
        "water" | "水脈" => "water",
        "fishing_port" | "漁港" => "fishing_port",
        "salt_pan" | "鹽場" => "salt_pan",
        "granary" | "糧倉" => "granary",
        "farm" | "農田" => "farm",
        "mine" | "礦場" => "mine",
        "lumber" | "林場" | "鋸木" => "lumber",
        "herb" | "藥圃" => "herb",
        "clinic" | "醫館" | "診所" => "clinic",
        "ranch" | "牧場" | "乳酪" => "ranch",
        "blacksmith" | "鐵匠鋪" => "blacksmith",
        "forge" | "鍛造" => "forge",
        "workshop" | "工坊" => "workshop",
        "carpentry" | "木工" | "傢俱" => "carpentry",
        "masonry" | "石工" | "切石" | "石材" => "masonry",
        "kiln" | "窯場" | "灰窯" => "kiln",
        "textile" | "織坊" | "服飾" => "textile",
        "canteen" | "食堂" => "canteen",
        "inn" | "旅社" | "歇宿" => "inn",
        "teahouse" | "茶屋" | "茶坊" => "teahouse",
        "bathhouse" | "浴場" => "bathhouse",
        "well" | "水井" => "well",
        "courier" | "聯運" => "courier",
        "market" | "市集" => "market",
        "shop" | "商鋪" | "商行" => "shop",
        "bank" | "票號匯兌" => "bank",
        "civic" | "公議堂" | "政務" => "civic",
        "academy" | "書院" | "書局" => "academy",
        "defense" | "關防" | "哨所" | "試招" => "defense",
        "warehouse" | "倉儲" => "warehouse",
        "settlement" | "聚落" | "聚落場域" => "settlement",
        "counter" | "櫃檯" => "counter",
        "restricted_staff_only" | "非本店人員禁止進入區" => "restricted_staff_only",
        "civic_facility" | "民生設施" => "civic_facility",
        "supply_hub" | "補給據點" => "supply_hub",
        "social_hub" | "社交據點" => "social_hub",
        "garden" | "園林" | "草坪" => "garden",
        "grain" | "穀物" => "grain",
        "ore" | "礦石" | "選礦" => "ore",
        "timber" | "木材" => "timber",
        "medicine" | "藥材" => "medicine",
        "hunting" | "獵場" | "獵具" => "hunting",
        "leather" | "皮革" => "leather",
        "sea_salt" | "海鹽" => "sea_salt",
        "fabric" | "布料" => "fabric",
        "grindstone" | "磨石" | "石磨" => "grindstone",
        "hot_food" | "熱食" => "hot_food",
        "fresh_drink" | "現煮飲品" => "fresh_drink",
        "sluice" | "分洪閘" => "sluice",
        "observatory" | "觀測" => "observatory",
        "lighthouse" | "燈塔" => "lighthouse",
        "bell_tower" | "鐘樓" => "bell_tower",
        "shipyard" | "船塢" => "shipyard",
        "farm_tool" | "農具" => "farm_tool",
        "mine_cart" | "台車" => "mine_cart",
        "smoked_fish" | "冷燻" => "smoked_fish",
        "kelp" | "巨藻" => "kelp",
        "bamboo_craft" | "竹編" => "bamboo_craft",
        "bamboo" | "刺竹" | "竹林" | "巨竹" | "竹篁" => "bamboo",
        "bamboo_shoot" | "竹筍" | "巨筍" => "bamboo_shoot",
        "bamboo_rhizome" | "竹鞭" => "bamboo_rhizome",
        "spring" | "冷泉" | "湧泉" | "伏流" => "spring",
        "pebble_flat" | "卵石灘" | "石英卵石" => "pebble_flat",
        "red_clay" | "紅土" | "赭紅礫石" => "red_clay",
        "camphor" | "樟木" | "樟樹" | "老樟" => "camphor",
        "melia" | "苦楝" => "melia",
        "humus" | "腐殖土" | "腐葉" => "humus",
        _ => "",
    }
}

/// 依據地形、房間名稱與分區推導房間語意標籤
pub fn derive_room_tags(terrain: Terrain, name: &str, zone: &str) -> Vec<String> {
    if terrain == Terrain::House || is_private_residence_room_name(name) {
        return vec!["residence".to_string()];
    }
    if is_street_or_gate_room_name(name) {
        return vec!["street".to_string(), "road".to_string()];
    }
    let mut tags = Vec::new();
    let mut add_tag = |t: &str| {
        let canon = canonical_room_tag(t);
        let key = if !canon.is_empty() { canon } else { t };
        if !tags.iter().any(|existing: &String| existing == key) {
            tags.push(key.to_string());
        }
    };

    // 地基優先：依 Terrain 注入基礎語意標籤
    match terrain {
        Terrain::House => {
            add_tag("residence");
        }
        Terrain::Dock => {
            add_tag("dock");
            add_tag("courier");
            add_tag("fishing_port");
            add_tag("waterfront");
        }
        Terrain::Beach => {
            add_tag("beach");
            add_tag("waterfront");
        }
        Terrain::Water | Terrain::WaterDeep => {
            add_tag("water");
            add_tag("waterfront");
        }
        Terrain::Bridge => {
            add_tag("bridge");
            add_tag("plaza");
        }
        Terrain::Road => {
            add_tag("street");
            add_tag("road");
        }
        _ => {}
    }

    let n = name;
    let zone_without_person = crate::npc::causal_core::strip_trailing_person_name_from_role(zone);
    let z_clean = strip_settlement_zone_names(zone_without_person);
    let z = z_clean.as_str();
    // 糧倉、穀物、糧食、米、大圳、農田
    if matches!(
        terrain,
        Terrain::Granary
            | Terrain::FarmField
            | Terrain::Farmhouse
            | Terrain::DryField
            | Terrain::EmptyFurrow
    ) || n.contains('糧')
        || n.contains('穀')
        || n.contains('倉')
        || n.contains('米')
        || n.contains("大圳")
        || n.contains('農')
        || n.contains('田')
        || z.contains('糧')
        || z.contains('穀')
        || z.contains('米')
        || z.contains("大圳")
        || z.contains('農')
    {
        add_tag("grain");
        if matches!(terrain, Terrain::Granary) || n.contains('倉') || n.contains('糧') {
            add_tag("granary");
            add_tag("counter");
            add_tag("restricted_staff_only");
        }
        if matches!(
            terrain,
            Terrain::FarmField | Terrain::Farmhouse | Terrain::DryField | Terrain::EmptyFurrow
        ) || n.contains('田')
            || n.contains('耕')
        {
            add_tag("farm");
        }
    }

    // 礦場、礦石、西嶺礦谷
    if matches!(terrain, Terrain::MinePass)
        || n.contains('礦')
        || n.contains("西嶺")
        || z.contains('礦')
        || z.contains("西嶺")
    {
        add_tag("mine");
        add_tag("ore");
        add_tag("counter");
        add_tag("restricted_staff_only");
    }

    // 林場、木材、霜林口
    if matches!(
        terrain,
        Terrain::Forest | Terrain::ForestHeavy | Terrain::ForestLight
    ) || n.contains('林')
        || n.contains('木')
        || n.contains('杉')
        || n.contains("霜林")
        || z.contains('林')
        || z.contains('木')
        || z.contains("霜林")
    {
        add_tag("lumber");
        add_tag("timber");
    }

    // 藥圃、藥材、醫館、百草、北山藥圃
    if matches!(
        terrain,
        Terrain::Clinic | Terrain::HerbGarden | Terrain::Alchemist
    ) || n.contains('藥')
        || n.contains('醫')
        || n.contains('診')
        || n.contains("百草")
        || n.contains("草木")
        || z.contains('藥')
        || z.contains('醫')
        || z.contains("百草")
    {
        add_tag("herb");
        add_tag("medicine");
        add_tag("counter");
        add_tag("restricted_staff_only");
    }

    // 獵場、皮革、鹿皮、長風草甸
    if matches!(terrain, Terrain::Jungle)
        || n.contains('獵')
        || n.contains('皮')
        || n.contains('鹿')
        || n.contains("草甸")
        || z.contains('獵')
        || z.contains('皮')
        || z.contains("草甸")
    {
        add_tag("hunting");
        add_tag("leather");
    }
    // 漁港、海鹽、鮮魚、南灣
    if matches!(
        terrain,
        Terrain::Dock | Terrain::Beach | Terrain::Water | Terrain::WaterDeep
    ) || n.contains('漁')
        || n.contains('港')
        || n.contains('鹽')
        || n.contains('海')
        || n.contains("南灣")
        || z.contains('漁')
        || z.contains('港')
        || z.contains('鹽')
        || z.contains('海')
        || z.contains("南灣")
    {
        add_tag("fishing_port");
        add_tag("sea_salt");
        add_tag("counter");
        add_tag("restricted_staff_only");
    }

    // 織坊、布料、裁縫、文曜織坊
    if n.contains('織')
        || n.contains('布')
        || n.contains('染')
        || n.contains('縫')
        || z.contains('織')
        || z.contains('布')
        || z.contains('染')
    {
        add_tag("textile");
        add_tag("fabric");
        add_tag("workshop");
        add_tag("counter");
        add_tag("restricted_staff_only");
    }

    // 鐵匠鋪、鍛造、工坊、磨石、老堰頭水輪石磨
    if matches!(terrain, Terrain::Blacksmith | Terrain::Workshop)
        || n.contains('鐵')
        || n.contains('鍛')
        || n.contains("工坊")
        || n.contains("石磨")
        || n.contains("水輪")
        || n.contains("老堰頭")
        || z.contains('鐵')
        || z.contains('鍛')
        || z.contains("工坊")
        || z.contains("老堰頭")
    {
        if matches!(terrain, Terrain::Blacksmith) || n.contains('鐵') || n.contains('鍛') {
            add_tag("blacksmith");
        }
        add_tag("workshop");
        add_tag("grindstone");
        add_tag("counter");
        add_tag("restricted_staff_only");
    }

    // 食堂、熱食、現煮飲品、客棧、茶居、三水大食堂
    if matches!(terrain, Terrain::Tavern | Terrain::Inn)
        || n.contains("食堂")
        || n.contains('餐')
        || n.contains('飯')
        || n.contains('茶')
        || n.contains('酒')
        || n.contains("客棧")
        || n.contains('飲')
        || z.contains("食堂")
        || z.contains('餐')
        || z.contains('茶')
        || z.contains('酒')
    {
        add_tag("canteen");
        add_tag("hot_food");
        add_tag("fresh_drink");
        add_tag("counter");
        add_tag("restricted_staff_only");
    }

    // 商鋪、市集
    if matches!(
        terrain,
        Terrain::GeneralStore | Terrain::Market | Terrain::Caravanserai | Terrain::Bank
    ) || n.contains('商')
        || n.contains('鋪')
        || n.contains("市集")
        || n.contains('店')
        || n.contains('行')
        || z.contains('商')
        || z.contains("市集")
    {
        add_tag("shop");
        add_tag("market");
        add_tag("counter");
        add_tag("restricted_staff_only");
        if matches!(terrain, Terrain::Bank)
            || n.contains("總商行")
            || n.contains("信匯")
            || n.contains("票號")
            || n.contains("銀號")
            || n.contains("匯兌")
        {
            add_tag("bank");
        }
    }

    // 水脈、分洪閘、清泉、堰渠
    if n.contains("水脈")
        || n.contains("清泉")
        || n.contains('閘')
        || n.contains('渠')
        || n.contains('堰')
        || z.contains("水脈")
        || z.contains("清泉")
        || z.contains('閘')
        || z.contains('渠')
        || z.contains('堰')
    {
        add_tag("water");
        add_tag("sluice");
    }

    // 浴場、湯屋、泉香
    if n.contains('浴') || n.contains('湯') || n.contains("泉香") || z.contains('浴') {
        add_tag("bathhouse");
    }

    // 茶屋
    if matches!(terrain, Terrain::Tavern) || n.contains('茶') || z.contains('茶') {
        add_tag("teahouse");
    }

    // 旅社、驛站、歇舍
    if matches!(terrain, Terrain::Inn)
        || n.contains("旅社")
        || n.contains("驛")
        || n.contains("通鋪")
        || n.contains("歇舍")
    {
        add_tag("inn");
    }

    // 水井、山泉、清泉
    if n.contains('井') || n.contains("山泉") || n.contains("清泉") {
        add_tag("well");
    }

    // 聯運、碼頭、渡口
    if matches!(terrain, Terrain::Dock | Terrain::Caravanserai)
        || n.contains("聯運")
        || n.contains("貨運")
        || n.contains("碼頭")
        || n.contains("渡口")
        || n.contains("船塢")
        || n.contains("車馬")
    {
        add_tag("courier");
        add_tag("dock");
    }

    // 石橋、廣場、樞紐
    if matches!(terrain, Terrain::Bridge)
        || n.contains("石橋")
        || n.contains("樞紐")
        || n.contains("石坪")
        || n.contains("廣場")
    {
        add_tag("bridge");
        add_tag("plaza");
    }

    tags
}
