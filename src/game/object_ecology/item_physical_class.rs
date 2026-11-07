#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemPhysicalClass {
    LiquidBeverage,
    GrainOrPowder,
    BundledFiber,
    RawMineralOrTimber,
    SheetOrFabric,
    AquaticOrMeat,
    CookedFoodOrPastry,
    MedicineOrSalve,
    BladedWeaponOrTool,
    BluntCraftTool,
    WovenContainerOrPack,
    UmbrellaOrFan,
    OuterCloakOrRobe,
    GarmentOrArmor,
    FootwearOrHeadgear,
    AccessoryOrTrinket,
    BookOrScroll,
    HeavyFixture,
    GeneralGood,
}

impl ItemPhysicalClass {
    pub fn strip_mutation_affixes(item_name: &str) -> String {
        let mut clean_name = item_name.trim().to_string();
        while let Some(start) = clean_name.find("【異變") {
            if let Some(end_rel) = clean_name[start..].find('】') {
                let after = start + end_rel + '】'.len_utf8();
                let rest = clean_name[after..].trim_start_matches('・').trim();
                clean_name = format!("{}{}", &clean_name[..start], rest);
            } else {
                break;
            }
        }
        clean_name.trim_matches(|c| c == '【' || c == '】').trim().to_string()
    }

    pub fn classify(item_id: &str, item_name: &str) -> Self {
        let mut base_id = item_id.trim();
        while let Some(rest) = base_id.strip_prefix("mut_") {
            base_id = rest;
        }
        for arch in &MUTATION_ARCHETYPES {
            let sfx = format!("_{}", arch.key);
            if let Some(stripped) = base_id.strip_suffix(&sfx) {
                base_id = stripped;
                break;
            }
        }

        let mut clean_name = item_name.trim().to_string();
        if clean_name.is_empty()
            && let Some(cat) = crate::game::peninsula_catalog::find_catalog_entry(base_id)
        {
            clean_name = cat.name.to_string();
        }
        let clean_name = Self::strip_mutation_affixes(&clean_name);
        let clean_name = clean_name.as_str();

        if base_id.ends_with("_bundle")
            || clean_name.contains("草綑")
            || clean_name.contains("柴綑")
            || clean_name.contains("麻束")
            || clean_name.contains("藤束")
            || clean_name.contains("蘆草")
            || clean_name.contains("柴薪")
            || clean_name.contains("邊角料")
        {
            return Self::BundledFiber;
        }

        if clean_name.contains('傘')
            || clean_name.contains('扇')
            || base_id.contains("umbrella")
        {
            return Self::UmbrellaOrFan;
        }
        if ["籃", "簍", "袋", "桶", "網", "背囊", "背包"]
            .iter()
            .any(|k| clean_name.contains(k))
        {
            return Self::WovenContainerOrPack;
        }


        if let Some(cat) = crate::game::peninsula_catalog::find_catalog_entry(base_id) {
            let is_fixture = ["箱", "櫃", "架", "車", "台", "檯", "爐", "砧", "磨", "機", "缸"]
                .iter()
                .any(|k| cat.name.contains(k))
                && crate::game::peninsula_catalog::catalog_entry_weight_kg(cat) > 15.0;
            if is_fixture {
                return Self::HeavyFixture;
            }
            if cat.has_tag("sharp") {
                return Self::BladedWeaponOrTool;
            }
            if cat.has_tag("tool") {
                return Self::BluntCraftTool;
            }
        }

        if clean_name.contains("斗篷")
            || clean_name.contains("披風")
            || clean_name.contains("蓑衣")
            || base_id.contains("cloak")
        {
            return Self::OuterCloakOrRobe;
        }

        if clean_name.contains("手札")
            || clean_name.contains("圖譜")
            || clean_name.contains("清冊")
            || clean_name.contains("古書")
            || clean_name.contains('曆')
            || clean_name.contains('籍')
        {
            return Self::BookOrScroll;
        }

        if (clean_name.contains("信箋")
            || clean_name.contains("宣紙")
            || clean_name.contains("竹紙")
            || clean_name.contains("棉布")
            || clean_name.contains("麻布")
            || clean_name.contains("帆布")
            || clean_name.contains("牛皮")
            || clean_name.contains("獸皮"))
            && !clean_name.contains("斗篷")
            && !clean_name.contains("披風")
            && !clean_name.contains("蓑衣")
            && !clean_name.contains('靴')
            && !clean_name.contains('鞋')
            && !clean_name.contains("帶")
            && !clean_name.contains('袍')
            && !clean_name.contains('甲')
        {
            return Self::SheetOrFabric;
        }

        if clean_name.contains('膏')
            || clean_name.contains('散')
            || clean_name.contains('丸')
            || clean_name.contains('丹')
            || clean_name.contains('藥')
            || clean_name.contains('灸')
        {
            return Self::MedicineOrSalve;
        }

        if clean_name.contains("長柄")
            || clean_name.contains("木柄")
            || clean_name.contains("毛刷")
            || clean_name.contains("竹刷")
            || clean_name.contains("篩架")
            || clean_name.contains("算盤")
            || clean_name.contains("木尺")
            || clean_name.contains("曲尺")
            || clean_name.contains("扁擔")
            || clean_name.contains("釣竿")
            || clean_name.contains("木槌")
            || clean_name.contains("鐵錘")
            || clean_name.contains("風箱")
            || clean_name.contains("墨斗")
            || base_id == "craft_wooden_tool_handle"
        {
            return Self::BluntCraftTool;
        }

        if ["刀", "劍", "匕", "斧", "鎬", "槍", "刃", "鑿", "鏨", "鐮", "剪", "鋸", "鏟", "鍘"]
            .iter()
            .any(|k| clean_name.contains(k))
        {
            return Self::BladedWeaponOrTool;
        }

        if ["米漿", "豆漿", "湯", "茶湯", "熱茶", "涼茶", "清泉", "泉水", "溪水", "井水", "桐油", "麻油", "魚油", "飲", "酒"]
            .iter()
            .any(|k| clean_name.contains(k))
        {
            return Self::LiquidBeverage;
        }

        if ["粉", "米", "麥", "穀", "豆", "鹽", "糖", "石灰", "砂", "礬"]
            .iter()
            .any(|k| clean_name.contains(k))
        {
            return Self::GrainOrPowder;
        }

        if ["礦", "石", "岩", "晶", "鐵塊", "銅塊", "煤", "原木", "木料", "竹材"]
            .iter()
            .any(|k| clean_name.contains(k))
            || base_id.contains("ore")
        {
            return Self::RawMineralOrTimber;
        }

        if ["魚", "蝦", "蟹", "貝", "蜆", "蛤", "肉", "鹿", "豬", "禽"]
            .iter()
            .any(|k| clean_name.contains(k))
        {
            return Self::AquaticOrMeat;
        }

        if ["餅", "包", "饅", "麵", "飯", "粥", "糕", "果", "筍"]
            .iter()
            .any(|k| clean_name.contains(k))
        {
            return Self::CookedFoodOrPastry;
        }

        if ["鞋", "靴", "履", "屐", "帽", "冠", "笠", "頭巾", "護腕", "手套", "腰帶", "束帶", "圍裙"]
            .iter()
            .any(|k| clean_name.contains(k))
        {
            return Self::FootwearOrHeadgear;
        }

        if ["衫", "衣", "袍", "襖", "褂", "裙", "褲", "甲", "冑"]
            .iter()
            .any(|k| clean_name.contains(k))
        {
            return Self::GarmentOrArmor;
        }

        if ["佩", "羅盤", "印", "囊", "墜", "牌"]
            .iter()
            .any(|k| clean_name.contains(k))
        {
            return Self::AccessoryOrTrinket;
        }

        Self::GeneralGood
    }

    pub fn measure_word(&self, item_name: &str) -> &'static str {
        match self {
            Self::LiquidBeverage => {
                if item_name.contains('碗') || item_name.contains('湯') {
                    "碗"
                } else if item_name.contains('壺') || item_name.contains('酒') || item_name.contains('茶') {
                    "壺"
                } else {
                    "桶"
                }
            }
            Self::GrainOrPowder => "袋",
            Self::BundledFiber => "綑",
            Self::RawMineralOrTimber => {
                if item_name.contains('木') || item_name.contains('竹') {
                    "根"
                } else {
                    "塊"
                }
            }
            Self::SheetOrFabric => {
                if item_name.contains('布') || item_name.contains('綢') {
                    "匹"
                } else if item_name.contains('皮') {
                    "張"
                } else {
                    "疊"
                }
            }
            Self::AquaticOrMeat => {
                if item_name.contains('魚') && !item_name.contains('乾') {
                    "尾"
                } else if item_name.contains("魚乾") {
                    "條"
                } else {
                    "份"
                }
            }
            Self::CookedFoodOrPastry => "份",
            Self::MedicineOrSalve => {
                if item_name.contains('膏') || item_name.contains('丸') || item_name.contains('丹') {
                    "盒"
                } else {
                    "帖"
                }
            }
            Self::BladedWeaponOrTool => "把",
            Self::BluntCraftTool => {
                if item_name.contains('柄')
                    || item_name.contains('竿')
                    || item_name.contains('尺')
                    || item_name.contains("扁擔")
                {
                    "根"
                } else if item_name.contains('架')
                    || item_name.contains("算盤")
                    || item_name.contains("墨斗")
                {
                    "具"
                } else {
                    "把"
                }
            }
            Self::WovenContainerOrPack => {
                if item_name.contains('袋') || item_name.contains('囊') || item_name.contains('包') {
                    "口"
                } else if item_name.contains('網') {
                    "張"
                } else {
                    "只"
                }
            }
            Self::UmbrellaOrFan => "把",
            Self::OuterCloakOrRobe | Self::GarmentOrArmor | Self::GeneralGood => "件",
            Self::FootwearOrHeadgear => {
                if item_name.contains('鞋')
                    || item_name.contains('靴')
                    || item_name.contains('履')
                    || item_name.contains('屐')
                    || item_name.contains("護腕")
                    || item_name.contains("手套")
                {
                    "雙"
                } else if item_name.contains('帶') {
                    "條"
                } else {
                    "頂"
                }
            }
            Self::AccessoryOrTrinket => "枚",
            Self::BookOrScroll => "冊",
            Self::HeavyFixture => "座",
        }
    }

    pub fn format_qty(&self, item_name: &str, qty: i32) -> String {
        format!("{item_name} {qty} {}", self.measure_word(item_name))
    }

    pub fn is_bladed(&self) -> bool {
        matches!(self, Self::BladedWeaponOrTool)
    }

    pub fn is_vocational_or_combat_vessel(&self) -> bool {
        matches!(
            self,
            Self::BladedWeaponOrTool | Self::BluntCraftTool | Self::WovenContainerOrPack
        )
    }

    pub fn condition_label(&self, cond: crate::pg::living_world::VesselCondition) -> &'static str {
        use crate::pg::living_world::VesselCondition;
        match self {
            Self::BladedWeaponOrTool => match cond {
                VesselCondition::Sharp => "鋒利",
                VesselCondition::SlightlyDull => "刃口微鈍",
                VesselCondition::RolledEdge => "捲刃待修",
            },
            Self::UmbrellaOrFan => match cond {
                VesselCondition::Sharp => "傘面完好",
                VesselCondition::SlightlyDull => "傘骨微鬆",
                VesselCondition::RolledEdge => "傘骨待修",
            },
            _ => match cond {
                VesselCondition::Sharp => "完好趁手",
                VesselCondition::SlightlyDull => "略顯磨耗",
                VesselCondition::RolledEdge => "器身待修",
            },
        }
    }

    pub fn wear_or_carry_prefix(&self, slot: &str) -> &'static str {
        match (slot, self) {
            ("body", Self::OuterCloakOrRobe) => "外罩",
            ("body", _) => "身著",
            ("head", _) => "頭戴",
            ("feet", _) => "足踏",
            ("hold_r" | "hold_l" | "hand" | "weapon" | "tool" | "hold", _) => "手持",
            _ => "隨身佩著",
        }
    }

    pub fn maintenance_action_phrase(self) -> &'static str {
        match self {
            Self::WovenContainerOrPack => "洗淨擦拭、重新紮緊",
            Self::BladedWeaponOrTool => "細細砥礪刃口、擦拭上油",
            Self::BluntCraftTool => "細細修整校準",
            Self::OuterCloakOrRobe | Self::GarmentOrArmor | Self::FootwearOrHeadgear => {
                "拍淨塵土、細細縫補整燙"
            }
            Self::LiquidBeverage
            | Self::GrainOrPowder
            | Self::BundledFiber
            | Self::RawMineralOrTimber
            | Self::SheetOrFabric
            | Self::AquaticOrMeat
            | Self::CookedFoodOrPastry
            | Self::MedicineOrSalve => "妥善分裝、防潮封存",
            Self::UmbrellaOrFan | Self::AccessoryOrTrinket | Self::HeavyFixture | Self::GeneralGood => {
                "細細擦拭養護、妥善收整"
            }
            Self::BookOrScroll => "細細拂拭紙頁、壓平整帙",
        }
    }

    pub fn canonical_vocation_supply_item(role_or_work: &str) -> (&'static str, &'static str) {
        if role_or_work.contains('鍛')
            || role_or_work.contains('鐵')
            || role_or_work.contains('爐')
            || role_or_work.contains('鑄')
            || role_or_work.contains('銅')
        {
            ("mat_coal_lump", "硬木白炭")
        } else if role_or_work.contains('木')
            || role_or_work.contains('鋸')
            || role_or_work.contains('船')
            || role_or_work.contains('筏')
            || role_or_work.contains('林')
            || role_or_work.contains('樵')
        {
            ("mat_timber_log", "伐採原木")
        } else if role_or_work.contains('織')
            || role_or_work.contains('布')
            || role_or_work.contains('染')
            || role_or_work.contains('繡')
            || role_or_work.contains('衣')
            || role_or_work.contains('裁')
            || role_or_work.contains('網')
            || role_or_work.contains('繩')
        {
            ("mat_hemp_fiber", "韌麻纖維")
        } else if role_or_work.contains('藥')
            || role_or_work.contains('醫')
            || role_or_work.contains('診')
            || role_or_work.contains('草')
            || role_or_work.contains('丹')
            || role_or_work.contains('香')
        {
            ("mat_foraged_herb", "山野鮮藥草")
        } else if role_or_work.contains('獵')
            || role_or_work.contains('皮')
            || role_or_work.contains('靴')
            || role_or_work.contains('毛')
            || role_or_work.contains('牧')
        {
            ("mat_beast_hide", "山獸生皮")
        } else if role_or_work.contains('石')
            || role_or_work.contains('玉')
            || role_or_work.contains('礦')
            || role_or_work.contains('窯')
            || role_or_work.contains('陶')
            || role_or_work.contains('磚')
            || role_or_work.contains('灰')
        {
            ("mat_rough_ore", "粗採礦石")
        } else if role_or_work.contains('海')
            || role_or_work.contains('漁')
            || role_or_work.contains('鹽')
            || role_or_work.contains('港')
            || role_or_work.contains('舶')
            || role_or_work.contains('鮮')
            || role_or_work.contains('魚')
        {
            ("mat_sea_salt", "日曬粗海鹽")
        } else if role_or_work.contains('書')
            || role_or_work.contains('學')
            || role_or_work.contains('院')
            || role_or_work.contains('講')
            || role_or_work.contains("校勘")
            || role_or_work.contains('算')
            || role_or_work.contains('帳')
            || role_or_work.contains('秤')
            || role_or_work.contains('庫')
            || role_or_work.contains('署')
            || role_or_work.contains('哨')
            || role_or_work.contains('衛')
            || role_or_work.contains('巡')
        {
            ("wt_cat_0784", "防潮竹紙帳冊")
        } else {
            ("mat_wheat_grain", "飽滿小麥穗")
        }
    }

    pub fn is_item_coherent_with_vocation(item_id: &str, role_or_work: &str) -> bool {
        if item_id.starts_with("wt-good-") {
            return true;
        }
        if item_id == Self::canonical_vocation_supply_item(role_or_work).0 {
            return true;
        }
        let vessel = crate::npc::causal_core::resolve_vocational_vessel(role_or_work);
        if item_id == vessel.item_id || item_id == vessel.catalog_id {
            return true;
        }
        false
    }
}

#[cfg(test)]
mod item_physical_class_tests {
    use super::*;

    #[test]
    fn test_item_physical_class_acceptance() {
        use crate::pg::living_world::VesselCondition;

        let cls_bundle = ItemPhysicalClass::classify("mat_reed_bundle", "水澤老蘆草綑");
        assert_eq!(cls_bundle, ItemPhysicalClass::BundledFiber);
        assert_eq!(
            cls_bundle.format_qty("水澤老蘆草綑", 12),
            "水澤老蘆草綑 12 綑"
        );

        assert!(
            !crate::pg::living_world::is_weapon_or_tool_item(
                "craft_bamboo_oil_umbrella",
                "韌竹骨桐油晴雨傘"
            )
        );

        assert_eq!(
            VesselCondition::Sharp.label_for_item("wt_cat_0973", "平底石磨分粉篩架"),
            "完好趁手"
        );

        assert_eq!(
            VesselCondition::Sharp.label_for_item("wt_cat_1021", "岩羊角手柄折疊獵刀"),
            "鋒利"
        );
        assert_eq!(
            VesselCondition::Sharp.label_for_item("craft_bamboo_oil_umbrella", "韌竹骨桐油晴雨傘"),
            "傘面完好"
        );
        assert_eq!(
            ItemPhysicalClass::classify("mat_rough_ore", "粗礦").format_qty("粗礦", 12),
            "粗礦 12 塊"
        );
        assert_eq!(
            ItemPhysicalClass::classify("craft_wooden_tool_handle", "堅韌硬木工具長柄").format_qty("堅韌硬木工具長柄", 1),
            "堅韌硬木工具長柄 1 根"
        );
        assert_eq!(
            ItemPhysicalClass::classify("craft_warm_rice_syrup", "現磨溫米漿").format_qty("現磨溫米漿", 5),
            "現磨溫米漿 5 桶"
        );
        assert_eq!(
            ItemPhysicalClass::classify("mat_flour", "特級在來米粉").format_qty("特級在來米粉", 7),
            "特級在來米粉 7 袋"
        );
        assert_eq!(
            ItemPhysicalClass::classify("mat_rice", "待磨精碾白米").format_qty("待磨精碾白米", 9),
            "待磨精碾白米 9 袋"
        );
    }
}
