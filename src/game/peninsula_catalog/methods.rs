pub fn category_tag_for_num(category_num: &str) -> &'static str {
    match category_num.trim() {
        "01" | "1" => "mineral_jade",
        "02" | "2" => "timber_bamboo_fungi",
        "03" | "3" => "herbs_flora_spices",
        "04" | "4" => "livestock_hides_bones",
        "05" | "5" => "seafood_kelp",
        "06" | "6" => "grains_flour_cured",
        "07" | "7" => "condiments_pickles_sugar",
        "08" | "8" => "breakfast_street_staples",
        "09" | "9" => "banquet_hotpot_casserole",
        "10" => "tea_wine_pastries",
        "11" => "kitchenware_cookware",
        "12" => "plumbing_bath_hygiene",
        "13" => "furniture_bedding_building",
        "14" => "lighting_clocks_optics",
        "15" => "stationery_books_finance",
        "16" => "apparel_rainwear_footwear",
        "17" => "bags_umbrellas_hardware",
        "18" => "craft_machinery_tools",
        "19" => "maritime_transport_outdoor",
        "20" => "medical_hot_spring_wellness",
        _ => "general_goods",
    }
}

pub fn super_category_tags_for_category(category_tag: &str) -> &'static [&'static str] {
    match category_tag {
        "mineral_jade" => &["raw_material", "mineral", "ore_and_stone", "gatherable"],
        "timber_bamboo_fungi" => &["raw_material", "flora", "timber_and_fungi", "gatherable"],
        "herbs_flora_spices" => &["raw_material", "flora", "herb_and_spice", "gatherable"],
        "livestock_hides_bones" => &["raw_material", "fauna", "animal_product", "gatherable"],
        "seafood_kelp" => &["raw_material", "marine", "seafood", "gatherable"],
        "grains_flour_cured" => &["food_ingredient", "staple_and_cured", "food_and_drink"],
        "condiments_pickles_sugar" => &["food_ingredient", "seasoning", "food_and_drink"],
        "breakfast_street_staples" => &["cooked_food", "consumable", "food_and_drink", "market_dish"],
        "banquet_hotpot_casserole" => &["cooked_food", "consumable", "food_and_drink", "market_dish"],
        "tea_wine_pastries" => &["beverage_and_pastry", "consumable", "food_and_drink", "market_dish"],
        "kitchenware_cookware" => &["utensil", "kitchen_and_dining", "crafted_good", "market_ware"],
        "plumbing_bath_hygiene" => &["facility", "hygiene_and_water", "crafted_good", "market_ware"],
        "furniture_bedding_building" => &["furniture", "interior_fixture", "crafted_good", "market_ware"],
        "lighting_clocks_optics" => &["instrument", "lighting_and_optics", "crafted_good"],
        "stationery_books_finance" => &["stationery", "culture_and_finance", "crafted_good"],
        "apparel_rainwear_footwear" => &["apparel", "wearable", "crafted_good"],
        "bags_umbrellas_hardware" => &["carry_gear", "daily_sundries", "crafted_good"],
        "craft_machinery_tools" => &["tool", "workshop_machinery", "crafted_good"],
        "maritime_transport_outdoor" => &["transport_and_outdoor", "expedition_gear", "crafted_good"],
        "medical_hot_spring_wellness" => &["medical", "wellness_and_remedy", "crafted_good"],
        "public_fixture" => &["facility", "outdoor_fixture", "public_infrastructure"],
        _ => &["general_goods"],
    }
}
impl PeninsulaCatalogEntry {
    /// 依全稱確定性衍生自然洗練、3–5 字的人眼單一物件名（用作場景物件籌碼與快速互動名）。
    pub fn short_scene_name(&self) -> String {
        // 若包含《與》，提取書名號內標題
        if let Some(start) = self.name.find('《') {
            if let Some(end) = self.name.find('》') {
                if end > start {
                    let title = &self.name[start + '《'.len_utf8()..end];
                    if !title.trim().is_empty() {
                        return title.to_string();
                    }
                }
            }
        }

        // 若本名已 <= 5 字且不包含「與」，直接採用
        let char_count = self.name.chars().count();
        if char_count <= 5 && !self.name.contains('與') {
            return self.name.to_string();
        }

        // 若包含「與」或「配」，提取核心主體詞（避免「甘薯稀飯配醬瓜」被截尾成「飯配醬瓜」）
        let working = if let Some((before, after)) = self.name.split_once('與') {
            if after.chars().count() >= 3 {
                after
            } else {
                before
            }
        } else if !self.name.contains("配重")
            && !self.name.contains("配方")
            && !self.name.contains("配件")
            && self.name.contains('配')
        {
            if let Some((before, after)) = self.name.split_once('配') {
                if before.chars().count() >= 3 {
                    before
                } else {
                    after
                }
            } else {
                self.name
            }
        } else {
            self.name
        };

        // 去除前導聚落／地域／名家名
        const PREFIXES: &[&str] = &[
            "牛埔礫石灘", "三號坑口", "半嶺溪谷", "西海礁灣", "三水食堂", "泉香浴場", "東橋旅社",
            "三號坑", "半嶺鎮", "紅土崁", "紅土窯", "烏石鼻", "半嶺青脊", "分家巷", "風舶角",
            "風舶嶼", "風舶港", "白茫洲", "北嶺腳", "老堰頭", "風櫃尾", "大圳頭", "水尾寮",
            "文曜山", "霜林口", "水尾澤", "黑石灣", "大圳頭", "三水口", "印行館",
            "半嶺", "風舶", "北嶺", "風櫃", "大圳", "文曜", "霜林", "牛埔", "黑沼",
            "淺澤", "北山", "西海", "岑氏", "溫氏", "蘇家", "韓氏", "萬鈞",
            "柯氏", "周家", "林家", "陸家", "許氏", "張氏", "陳氏", "鄭氏", "吳氏", "馬氏", "何氏", "老鐵",
        ];

        let mut cur = working;
        loop {
            let mut stripped = false;
            for &pfx in PREFIXES {
                if let Some(rest) = cur.strip_prefix(pfx) {
                    cur = rest;
                    stripped = true;
                    break;
                }
            }
            if !stripped {
                break;
            }
        }

        let cur_owned = if cur.chars().count() > 5 && cur.contains("家用") {
            cur.replace("家用", "")
        } else {
            cur.to_string()
        };
        let cur = cur_owned.as_str();

        const SUFFIXES: &[&str] = &[
            "原石", "木板", "茶杯", "水栓", "水槽", "便池", "湯池", "大床", "暖被", "衣櫃",
            "茶盤", "台燈", "壁燈", "路燈", "海燈", "座鐘", "懷錶", "手札", "課本", "計數板",
            "潔齒刷", "助視鏡", "敷面膏", "沐浴膏", "圍裙", "便鞋", "背包", "背架", "背簍", "提籃", "摺疊傘", "保溫瓶", "火籤盒",
            "鐵爐", "鍛錘", "石磨", "大鋸", "切石機", "翻土犁", "榨油車", "大龍骨", "船纜", "重錨",
            "竹筏", "大貨車", "郵袋", "釣竿", "敷藥", "清心丸", "湯包", "溫灸盒", "香盤", "夾板",
            "銀針囊", "藥碾槽", "火鍋爐", "砂鍋", "湯盅", "石櫃", "碗盤組", "大釜", "牛肉湯", "火鍋",
            "工具箱", "走時鐘", "文書櫃", "陳列櫥", "展示櫃", "藥材櫃", "分格櫃", "收納箱", "工具袋",
        ];

        let cur_chars: Vec<char> = cur.chars().collect();
        let trimmed: String = if cur_chars.len() > 5 {
            let ends_with_suffix = SUFFIXES.iter().any(|&sfx| cur.ends_with(sfx));
            let take_len = if ends_with_suffix { 5 } else { 4 };
            if cur_chars.len() >= take_len {
                cur_chars[cur_chars.len() - take_len..].iter().collect()
            } else {
                cur.to_string()
            }
        } else {
            cur.to_string()
        };

        let mut trimmed_chars: Vec<char> = trimmed.chars().collect();
        if trimmed_chars.len() >= 4 && matches!(trimmed_chars[0], '製' | '式' | '膽') {
            trimmed_chars.remove(0);
        }
        let trimmed: String = trimmed_chars.iter().collect();
        if trimmed_chars.len() < 3 {
            let orig_chars: Vec<char> = self.name.chars().collect();
            if orig_chars.len() >= 3 {
                orig_chars[orig_chars.len() - 3..].iter().collect()
            } else {
                self.name.to_string()
            }
        } else if trimmed_chars.len() > 5 {
            trimmed_chars[trimmed_chars.len() - 5..].iter().collect()
        } else {
            trimmed
        }
    }

    pub fn to_room_object(&self) -> RoomObject {
        let mut responses = HashMap::new();
        responses.insert("Look".to_string(), self.look_text.to_string());
        responses.insert(self.socket.to_string(), self.action_text.to_string());
        let mut sockets = vec!["Look".to_string(), self.socket.to_string()];

        let is_wood = matches!(self.category_num, "02" | "2")
            || ((self.name.contains("原木") || self.name.contains("柴") || self.name.contains("木炭"))
                && !matches!(self.category_num, "09" | "11" | "12" | "13" | "14" | "15" | "16" | "17" | "18" | "19" | "20"));

        let is_forge_or_woodwork = (self.category_num == "11" || self.category_num == "18")
            && (self.name.contains("爐")
                || self.name.contains("鋸")
                || self.name.contains("刨")
                || self.name.contains("鍛")
                || self.name.contains("窯")
                || self.name.contains("鑽")
                || self.name.contains("切石")
                || self.name.contains("車床"));

        let is_mineral = matches!(self.category_num, "01" | "1");

        let is_herb = matches!(self.category_num, "03" | "3");

        let is_fiber = matches!(self.category_num, "02" | "2")
            && (self.name.contains("麻")
                || self.name.contains("蘆")
                || self.name.contains("繩")
                || self.name.contains("紙漿"));

        let is_hide_fish = matches!(self.category_num, "04" | "4" | "05" | "5")
            && (self.name.contains("皮")
                || self.name.contains("骨")
                || self.name.contains("貝")
                || self.name.contains("鱗")
                || self.name.contains("膠"));
        if is_wood {
            let wood_sockets = [
                "Look",
                "Acquire",
                "MakeFire",
                "SplitWood",
                "RefinePlank",
                "BurnCharcoal",
                "CraftTorch",
                "RepairStructure",
            ];
            for s in wood_sockets {
                if !sockets.iter().any(|existing| existing == s) {
                    sockets.push(s.to_string());
                }
            }
            responses.entry("Acquire".to_string()).or_insert_with(|| {
                format!("你取下「{}」，妥善收妥入行囊。", self.name)
            });
            responses.entry("MakeFire".to_string()).or_insert_with(|| {
                format!("你取用「{}」作為薪柴引燃火星，在空地中央架起通風柴堆，迅速生起一堆溫暖明亮的野地營火。", self.name)
            });
            responses.entry("SplitWood".to_string()).or_insert_with(|| {
                format!("你運力順著木紋利索剖劈「{}」，將其細細剖分劈成易於引火的乾燥柴枝束。", self.name)
            });
            responses.entry("RefinePlank".to_string()).or_insert_with(|| {
                format!("你推動長鋸順紋切削「{}」，將其裁鋸打磨為平整規整的樟松木板料。", self.name)
            });
            responses.entry("BurnCharcoal".to_string()).or_insert_with(|| {
                format!("你將「{}」截切後置入炭窯泥封慢火悶燒，凝煉出無煙透亮的硬木白炭。", self.name)
            });
            responses.entry("CraftTorch".to_string()).or_insert_with(|| {
                format!("你挑選「{}」之堅韌部分塗抹脂膏緊密紮裹，製成一柄照明驅寒的浸脂松明火把。", self.name)
            });
            responses.entry("RepairStructure".to_string()).or_insert_with(|| {
                format!("你取用「{}」加固修整破損的建築構件與設施踏板，使其重歸穩固。", self.name)
            });
        } else if is_forge_or_woodwork {
            let forge_sockets = [
                "Look",
                "Operate",
                "AddFuel",
                "RefinePlank",
                "BurnCharcoal",
                "CraftHandle",
                "RepairStructure",
            ];
            for s in forge_sockets {
                if !sockets.iter().any(|existing| existing == s) {
                    sockets.push(s.to_string());
                }
            }
            responses.entry("Operate".to_string()).or_insert_with(|| {
                format!("你熟練地操作校準「{}」，各處機件配合嚴密流暢。", self.name)
            });
            responses.entry("AddFuel".to_string()).or_insert_with(|| {
                format!("你向「{}」添入乾燥柴枝與硬木白炭，只見烈火騰湧、爐溫劇升，散發出熾白耀眼的熱能。", self.name)
            });
            responses.entry("RefinePlank".to_string()).or_insert_with(|| {
                format!("你藉助「{}」的平整導軌順紋推拉裁切，將原木加工為光滑端正的樟松木板料。", self.name)
            });
            responses.entry("BurnCharcoal".to_string()).or_insert_with(|| {
                format!("你利用「{}」的高溫控氧環境密閉悶燒硬木，凝煉出熱值極高的精純白炭。", self.name)
            });
            responses.entry("CraftHandle".to_string()).or_insert_with(|| {
                format!("你利用「{}」車削打磨硬木，製成一根堅實順手、契合各類工具的硬木工具長柄。", self.name)
            });
            responses.entry("RepairStructure".to_string()).or_insert_with(|| {
                format!("你取用「{}」加工的結構料件，細心修補加固周遭損耗的架座與設施。", self.name)
            });
        } else if is_mineral {
            let mineral_sockets = [
                "Look",
                "Acquire",
                "WashOre",
                "SmeltBloom",
                "ForgeRepairStrip",
                "SharpenBlade",
                "BuildFieldAnvil",
            ];
            for s in mineral_sockets {
                if !sockets.iter().any(|existing| existing == s) {
                    sockets.push(s.to_string());
                }
            }
            responses.entry("Acquire".to_string()).or_insert_with(|| {
                format!("你取下「{}」，妥善收妥入行囊。", self.name)
            });
            responses.entry("WashOre".to_string()).or_insert_with(|| {
                format!("你藉由急流淘洗「{}」，洗淨附著泥砂雜質，篩出純淨的洗選礦砂。", self.name)
            });
            responses.entry("SmeltBloom".to_string()).or_insert_with(|| {
                format!("你將「{}」投入高溫土爐鼓風熔煉，凝煉出質地純良堅實的回收熟鐵料塊。", self.name)
            });
            responses.entry("ForgeRepairStrip".to_string()).or_insert_with(|| {
                format!("你持鍛錘反覆捶打延展「{}」，裁切精工鍛打成平整耐磨的修補熟鐵薄片。", self.name)
            });
            responses.entry("SharpenBlade".to_string()).or_insert_with(|| {
                format!("你在磨石上細心推磨開刃「{}」，淬火鍛造成一把刃口如霜的精鍛防身獵刀。", self.name)
            });
            responses.entry("BuildFieldAnvil".to_string()).or_insert_with(|| {
                format!("你挑選「{}」之厚重平整處架設墊砧牢牢楔固，在原地立起一座堅實的野地修具石砧。", self.name)
            });
        } else if is_herb {
            let herb_sockets = [
                "Look",
                "Acquire",
                "WashHerb",
                "DryHerb",
                "CrushHerb",
                "BoilDecoction",
                "SetupHerbStove",
            ];
            for s in herb_sockets {
                if !sockets.iter().any(|existing| existing == s) {
                    sockets.push(s.to_string());
                }
            }
            responses.entry("Acquire".to_string()).or_insert_with(|| {
                format!("你取下「{}」，妥善收妥入行囊。", self.name)
            });
            responses.entry("WashHerb".to_string()).or_insert_with(|| {
                format!("你捧取清泉淘洗「{}」，洗去泥沙浮塵，草木清香撲鼻，神清氣爽。", self.name)
            });
            responses.entry("DryHerb".to_string()).or_insert_with(|| {
                format!("你將「{}」攤在竹匾上向陽翻曬散去水氣，收攏出成色上好的山野鮮藥草。", self.name)
            });
            responses.entry("CrushHerb".to_string()).or_insert_with(|| {
                format!("你取藥臼細細研磨搗爛「{}」，加入植物油脂濃縮收膏，製成溫脈舒筋活血膏。", self.name)
            });
            responses.entry("BoilDecoction".to_string()).or_insert_with(|| {
                format!("你將「{}」入銚慢火煨煮，藥香瀰漫四溢，煎出清心安神百草茶包。", self.name)
            });
            responses.entry("SetupHerbStove".to_string()).or_insert_with(|| {
                format!("你取「{}」配合周遭溪石搭起文火小灶，在原地架設起一座野地煎藥陶銚。", self.name)
            });
        } else if is_fiber {
            let fiber_sockets = [
                "Look",
                "Acquire",
                "TwistRope",
                "WeaveSackcloth",
                "OilCanvas",
                "PressPaper",
            ];
            for s in fiber_sockets {
                if !sockets.iter().any(|existing| existing == s) {
                    sockets.push(s.to_string());
                }
            }
            responses.entry("Acquire".to_string()).or_insert_with(|| {
                format!("你取下「{}」，妥善收妥入行囊。", self.name)
            });
            responses.entry("TwistRope".to_string()).or_insert_with(|| {
                format!("你將「{}」浸油分股、雙手交錯搓撚絞緊，製成強韌結實的浸油粗麻繩卷。", self.name)
            });
            responses.entry("WeaveSackcloth".to_string()).or_insert_with(|| {
                format!("你穿梭引線將「{}」經緯交織密密編排，織出耐磨厚實的粗織麻袋布匹。", self.name)
            });
            responses.entry("OilCanvas".to_string()).or_insert_with(|| {
                format!("你將「{}」均勻浸刷熟桐油膠收邊縫扣，製成防風耐用的防風浸油帆布行旅斗篷。", self.name)
            });
            responses.entry("PressPaper".to_string()).or_insert_with(|| {
                format!("你將「{}」蒸煮舂爛入槽撈紙壓乾，揭下平整細密的原漿宣紙刀。", self.name)
            });
        } else if is_hide_fish {
            let hide_fish_sockets = [
                "Look",
                "Acquire",
                "ScrapeHide",
                "BoilFishGlue",
                "CalcifyShell",
                "CobbleBoots",
            ];
            for s in hide_fish_sockets {
                if !sockets.iter().any(|existing| existing == s) {
                    sockets.push(s.to_string());
                }
            }
            responses.entry("Acquire".to_string()).or_insert_with(|| {
                format!("你取下「{}」，妥善收妥入行囊。", self.name)
            });
            responses.entry("ScrapeHide".to_string()).or_insert_with(|| {
                format!("你握刀刮理「{}」，刮除殘肉軟化皮質，整理出強韌厚實的熟鞣皮帶料。", self.name)
            });
            responses.entry("BoilFishGlue".to_string()).or_insert_with(|| {
                format!("你將「{}」入釜溫火慢熬收汁濃縮，凝固成防水密封的封縫船膠料。", self.name)
            });
            responses.entry("CalcifyShell".to_string()).or_insert_with(|| {
                format!("你將「{}」堆入火中高溫煅燒出窯化粉，裝成耐水防潮的耐水貝灰石粉袋。", self.name)
            });
            responses.entry("CobbleBoots".to_string()).or_insert_with(|| {
                format!("你將「{}」細心裁剪錐孔，與厚木齒底緊密縫合，精工縫製出一雙厚底防滑行旅皮靴。", self.name)
            });
        }
        if is_portable_catalog_entry(self) && self.socket != "Gather" {
            if !sockets.iter().any(|s| s == "Acquire") {
                sockets.push("Acquire".to_string());
            }
            responses.insert(
                "Acquire".to_string(),
                format!("你取下「{}」，妥善收妥入行囊。", self.name),
            );
        }
        RoomObject {
            id: self.id.to_string(),
            name: self.short_scene_name(),
            owner: "world".to_string(),
            sockets,
            responses,
            move_to_room_id: String::new(),
        }
    }

    pub fn catalog_entry_sockets(&self) -> Vec<String> {
        self.to_room_object().sockets
    }

    /// 判定此物典條目是否為工作制服物件（如工坊圍裙、長衫、短褐、主事背心等）。
    pub fn is_work_uniform_object(&self) -> bool {
        self.attribute_tags.contains(&"uniform")
    }

    /// 物件日常用途與脈絡職能之語意別名
    pub fn sense_and_function(&self) -> &'static str {
        self.daily_function
    }

    pub fn attribute_tags(&self) -> &'static [&'static str] {
        self.attribute_tags
    }

    pub fn has_attribute_tag(&self, wanted: &str) -> bool {
        let clean = wanted.trim();
        self.attribute_tags.contains(&clean)
    }

    pub fn tags(&self) -> Vec<&'static str> {
        let mut tags = Vec::with_capacity(8 + self.attribute_tags.len());
        if !self.object_tag.is_empty() {
            tags.push(self.object_tag);
        }
        if !self.category_tag.is_empty() && !tags.contains(&self.category_tag) {
            tags.push(self.category_tag);
        }
        for &sup in super_category_tags_for_category(self.category_tag) {
            if !sup.is_empty() && !tags.contains(&sup) {
                tags.push(sup);
            }
        }
        for &attr in self.attribute_tags {
            if !attr.is_empty() && !tags.contains(&attr) {
                tags.push(attr);
            }
        }
        if !self.room_tag.is_empty() && !tags.contains(&self.room_tag) {
            tags.push(self.room_tag);
        }
        let sock = match self.socket {
            "Look" => "look",
            "Use" => "use",
            "Operate" => "operate",
            "Taste" => "taste",
            "Smell" => "smell",
            "Listen" => "listen",
            "Read" => "read",
            "Sit" => "sit",
            "Rest" => "rest",
            "Take" => "take",
            "Gather" => "gather",
            "DryOff" => "dry_off",
            "WarmHands" => "warm_hands",
            "Open" => "open",
            _ => "",
        };
        if !sock.is_empty() && !tags.contains(&sock) {
            tags.push(sock);
        }
        tags
    }

    pub fn semantic_tags(&self) -> Vec<&'static str> {
        self.tags()
    }

    /// 檢查是否包含特定標籤（不區分大小寫）
    pub fn has_tag(&self, wanted: &str) -> bool {
        let clean = wanted.trim();
        if clean.is_empty() {
            return false;
        }
        self.tags().iter().any(|&t| t.eq_ignore_ascii_case(clean))
    }
}
pub fn catalog_entry_sockets(entry: &PeninsulaCatalogEntry) -> Vec<String> {
    entry.catalog_entry_sockets()
}

fn room_tag_for_peninsula_room(room_name: &str) -> &'static str {
    if room_name.ends_with('街')
        || room_name.ends_with('道')
        || room_name.ends_with('巷')
        || room_name.ends_with('口')
        || room_name.ends_with('橋')
        || room_name.contains("前街")
        || room_name.contains("東街")
        || room_name.contains("西街")
        || room_name.contains("南街")
        || room_name.contains("北街")
    {
        "street"
    } else if room_name.contains("食堂") {
        "canteen"
    }
    else if room_name.contains("旅社") || room_name.contains("茶") { "inn" }
    else if room_name.contains("浴場") || room_name.contains("浴堂") || room_name.contains("石湯") { "bathhouse" }
    else if room_name.contains("傢俱") || room_name.contains("巧木") { "carpentry" }
    else if room_name.contains("服飾") || room_name.contains("織造") || room_name.contains("補給行") { "textile" }
    else if room_name.contains("鍛造") || room_name.contains("鐵器") || room_name.contains("鑿石") { "forge" }
    else if room_name.contains("書局") || room_name.contains("藏書閣") || room_name.contains("圖誌館") || room_name.contains("學院") { "academy" }
    else if room_name.contains("診所") { "clinic" }
    else if room_name.contains("石磨") { "grindstone" }
    else if room_name.contains("鋸木") { "lumber" }
    else if room_name.contains("切石") { "masonry" }
    else if room_name.contains("觀測") { "observatory" }
    else if room_name.contains("試招") { "defense" }
    else if room_name.contains("農具") { "farm_tool" }
    else if room_name.contains("乳酪") || room_name.contains("羊毛") { "ranch" }
    else if room_name.contains("選礦") { "ore" }
    else if room_name.contains("台車") { "mine_cart" }
    else if room_name.contains("燈塔") { "lighthouse" }
    else if room_name.contains("冷燻") { "smoked_fish" }
    else if room_name.contains("巨藻") { "kelp" }
    else if room_name.contains("灰窯") { "kiln" }
    else if room_name.contains("獵具") { "hunting" }
    else if room_name.contains("竹編") || room_name.contains("蘆葦") { "bamboo_craft" }
    else if room_name.contains("船塢") || room_name.contains("帆具") || room_name.contains("船材") { "shipyard" }
    else if room_name.contains("商行") || room_name.contains("契據") || room_name.contains("公議") || room_name.contains("商盟") || room_name.contains("公會") { "shop" }
    else if room_name.contains("聯運") || room_name.contains("貨運") { "courier" }
    else if room_name.contains("鐘樓") || room_name.contains("信使") || room_name.contains("瞭望") { "bell_tower" }
    else if room_name.contains("碼頭") || room_name.contains("泊位") || room_name.contains("竹筏埠") { "dock" }
    else if room_name.contains("市集") || room_name.contains("特產行") || room_name.contains("水產") || room_name.contains("物資行") { "market" }
    else if room_name.contains("住宅")
        || room_name.contains("民宅")
        || room_name.contains("居所")
        || room_name.contains("家屋")
        || room_name.contains("公寓")
        || room_name.contains("街屋")
        || room_name.contains("農舍")
        || room_name.contains("石屋")
        || room_name.contains("漁舍")
        || room_name.contains("暖屋")
        || room_name.contains("宿舍")
        || room_name.contains("高腳木屋")
        || room_name.contains("宅邸")
        || room_name.contains("宅院")
        || room_name.contains("莊園")
        || room_name.contains("別院")
        || room_name.contains("民舍")
        || room_name.ends_with('居')
        || room_name.ends_with('宅')
        || room_name.ends_with('齋')
        || room_name.ends_with('廬')
        || room_name.ends_with('院')
        || room_name.ends_with('舍')
        || room_name.ends_with('家')
        || room_name.ends_with('築')
        || room_name.ends_with('寮')
        || room_name.ends_with('房')
        || room_name.ends_with('莊')
        || room_name.ends_with('屋')
        || room_name.ends_with('樓')
    {
        "residence"
    }
    else { "street" }
}

fn stable_room_hash(s: &str) -> usize {
    let mut h: usize = 5381;
    for b in s.as_bytes() {
        h = h.wrapping_mul(33).wrapping_add(*b as usize);
    }
    h
}

/// 從 1,120 件《白塔半島千物總典》中，依房間類型與名稱確定性抽選 2 件專屬常態物件。
pub fn catalog_objects_for_room(room_name: &str) -> Vec<RoomObject> {
    let tag = room_tag_for_peninsula_room(room_name);
    if tag == "street" || room_name.contains("樞紐石橋") || room_name.contains("樞紐橋") || room_name.contains("城牆") {
        return Vec::new();
    }
    let exclude_raw = tag != "hunting";

    const SETTLEMENTS: &[&str] = &[
        "文曜", "風舶角", "風舶嶼", "風櫃尾", "烏石鼻", "半嶺",
        "三號坑", "大圳", "牛埔", "老堰頭", "北嶺腳", "南哨", "霜林", "水尾寮",
    ];
    let is_hub = room_name.contains("總商行")
        || room_name.contains("聯運")
        || room_name.contains("貨運")
        || room_name.contains("市集")
        || room_name.contains("特產");
    let current_settlement = SETTLEMENTS.iter().find(|&&s| room_name.contains(s)).copied();

    let candidates: Vec<&PeninsulaCatalogEntry> = PENINSULA_WORLD_OBJECTS
        .iter()
        .filter(|e| {
            if !(e.room_tag == tag || crate::grid::canonical_room_tag(e.room_tag) == tag || e.room_tag.contains(tag) || tag.contains(e.room_tag)) {
                return false;
            }
            if exclude_raw
                && (matches!(
                    e.category_num,
                    "01" | "02" | "03" | "04" | "05" | "06" | "07"
                ) || e.socket == "Gather")
            {
                return false;
            }
            if tag == "residence"
                && (matches!(e.category_num, "08" | "09" | "10") || e.socket == "Taste")
            {
                return false;
            }
            if let Some(settlement) = current_settlement {
                if !is_hub {
                    let has_other_settlement = SETTLEMENTS
                        .iter()
                        .any(|&other| other != settlement && e.name.contains(other));
                    if has_other_settlement {
                        return false;
                    }
                }
            }
            if room_name.contains("總商行") && !matches!(e.category_num, "09" | "10" | "12") {
                return false;
            }
            if (tag == "academy" || tag == "書局")
                && (e.category_num == "08"
                    || e.category_num == "16"
                    || e.category_name.contains("服飾")
                    || e.category_name.contains("常服"))
            {
                return false;
            }
            true
        })
        .collect();
    if candidates.is_empty() {
        return Vec::new();
    }
    let h = stable_room_hash(room_name);
    let first = candidates[h % candidates.len()];
    let mut out = vec![first.to_room_object()];
    if candidates.len() > 1 {
        let second_idx = (h / 7 + 13) % candidates.len();
        let second = if second_idx == h % candidates.len() {
            candidates[(second_idx + 1) % candidates.len()]
        } else {
            candidates[second_idx]
        };
        out.push(second.to_room_object());
    }
    out
}

/// 剝除前導聚落或地域名，供查找千物總典時向下相容查詢。
pub fn strip_leading_place_prefix(s: &str) -> &str {
    const PREFIXES: &[&str] = &[
        "牛埔礫石灘", "三號坑口", "半嶺溪谷", "西海礁灣", "風舶外海", "老堰頭渠", "三水食堂", "泉香浴場",
        "東橋旅社", "印行館", "夏季北山", "冬季北山", "三號坑", "半嶺鎮", "紅土崁", "紅土窯", "烏石鼻",
        "半嶺青脊", "分家巷", "風舶角", "風舶嶼", "風舶港", "白茫洲", "北嶺腳", "老堰頭", "風櫃尾",
        "大圳頭", "水尾寮", "文曜山", "霜林口", "水尾澤", "黑石灣", "大圳頭", "三水口", "半嶺",
        "風舶", "北嶺", "風櫃", "大圳", "文曜", "霜林", "牛埔", "黑沼", "淺澤", "北山", "西海", "南哨",
    ];
    for &pfx in PREFIXES {
        if let Some(rest) = s.strip_prefix(pfx) {
            if rest.chars().count() >= 3 {
                return rest;
            }
        }
    }
    s
}

/// 依 ID 或名稱在 1,109 件千物總典中查找物品。
pub fn find_catalog_entry_by_id_or_name(id: &str, name: &str) -> Option<&'static WorldObjectEntry> {
    let clean_id = id.trim();
    let clean_name = name.trim();
    if !clean_id.is_empty() {
        if let Some(entry) = PENINSULA_WORLD_OBJECTS.iter().find(|e| e.id == clean_id) {
            return Some(entry);
        }
    }
    if !clean_name.is_empty() {
        let stripped = strip_leading_place_prefix(clean_name);
        if let Some(entry) = PENINSULA_WORLD_OBJECTS.iter().find(|e| e.name == clean_name || e.name == stripped) {
            return Some(entry);
        }
        if let Some(entry) = PENINSULA_WORLD_OBJECTS.iter().find(|e| e.short_scene_name() == clean_name || e.short_scene_name() == stripped) {
            return Some(entry);
        }
    }
    None
}

/// 依 ID 或名稱在 1,120 件千物總典中查找物品。
pub fn find_catalog_entry(id_or_name: &str) -> Option<&'static PeninsulaCatalogEntry> {
    let stripped = strip_leading_place_prefix(id_or_name);
    PENINSULA_WORLD_OBJECTS.iter().find(|e| {
        e.id == id_or_name
            || e.name == id_or_name
            || e.short_scene_name() == id_or_name
            || e.name == stripped
            || e.short_scene_name() == stripped
    })
}

/// 依物件類型與名稱計算基準售價或契稅（鎂）。
pub fn catalog_entry_price_mg(entry: &PeninsulaCatalogEntry) -> i32 {
    if entry.id == "wt_cat_0793" || entry.name.contains("紅印屋契") {
        return 50;
    }
    match entry.category_num {
        "11" => {
            if entry.name.contains("火鍋")
                || entry.name.contains("大釜")
                || entry.name.contains("牛肉")
                || entry.name.contains("酒")
            {
                3
            } else {
                2
            }
        }
        "01" | "02" | "03" | "04" => 3,
        "05" | "08" | "12" | "13" | "16" => 4,
        "06" | "07" | "09" | "10" | "14" | "15" | "17" | "18" | "19" | "20" => 6,
        _ => 3,
    }
}

/// 依物件類型估算基準重量（公斤）。
pub fn catalog_entry_weight_kg(entry: &PeninsulaCatalogEntry) -> f64 {
    match entry.category_num {
        "01" | "02" | "06" | "09" | "10" => 1.5,
        "11" | "05" | "14" | "15" | "16" => 0.3,
        _ => 0.8,
    }
}

/// 是否為可隨身攜帶收納之器物（固定重型設施不可隨身收納）。
pub fn is_portable_catalog_entry(entry: &PeninsulaCatalogEntry) -> bool {
    const HEAVY_KEYWORDS: &[&str] = &[
        "大釜", "大床", "石櫃", "烘衣櫥", "便池", "湯池", "躺椅",
        "水車", "切石機", "石磨", "龍骨", "重錨", "鍛造爐", "水栓", "熱水槽",
    ];
    !HEAVY_KEYWORDS.iter().any(|&k| entry.name.contains(k))
}

/// 依名稱在千物總典中查找物品。
pub fn find_catalog_entry_by_name(name: &str) -> Option<&'static PeninsulaCatalogEntry> {
    let clean = name.trim();
    if clean.is_empty() {
        return None;
    }
    let stripped = strip_leading_place_prefix(clean);
    PENINSULA_WORLD_OBJECTS.iter().find(|e| {
        e.name == clean
            || e.short_scene_name() == clean
            || e.name == stripped
            || e.short_scene_name() == stripped
            || e.name.contains(clean)
    })
}

/// 依器物代碼或名稱判定是否為可見之工作制服物件。
pub fn is_work_uniform_item_id_or_name(val: &str) -> bool {
    let clean = val.trim();
    if clean.is_empty() {
        return false;
    }
    if let Some(entry) = find_catalog_entry(clean).or_else(|| find_catalog_entry_by_name(clean)) {
        if entry.is_work_uniform_object() {
            return true;
        }
    }
    const UNIFORM_KEYWORDS: &[&str] = &[
        "制服", "圍裙", "工裝", "短褐", "號衣", "主事衫", "匠人服", "罩衫", "皮圍裙", "當值",
        "0840", "0846", "0848", "0849", "0851", "0854",
    ];
    UNIFORM_KEYWORDS.iter().any(|&k| clean.contains(k))
}

