pub fn lookup_ecology_item_def(item_id: &str) -> Option<crate::db::ItemDefBrief> {
    if let Some((source, quality)) = crate::combat::wordplate::parse_crystal_item_id(item_id) {
        return Some(crate::db::ItemDefBrief {
            name: format!("{}・{}晶石", quality.label_zh(), source.label_zh()),
            item_type: "word_crystal".to_string(),
            slot: String::new(),
            description: format!("自白塔半島{}地脈凝結而成的天然晶石（{}），質地溫潤。", source.label_zh(), quality.label_zh()),
            weight: 0.1,
            vit_bonus: 0,
            dex_bonus: 0,
            atk_bonus: 0,
            origin: format!("白塔半島・{}", source.label_zh()),
        });
    }
    if item_id == "blank_manual" || item_id == "wt_blank_manual" {
        return Some(crate::db::ItemDefBrief {
            name: "空白手札".to_string(),
            item_type: "manual".to_string(),
            slot: String::new(),
            description: "裝幀素雅的棉紙手札，可將自身融會貫通之武學套路或百工訣竅著譜立說，流傳於世。".to_string(),
            weight: 0.2,
            vit_bonus: 0,
            dex_bonus: 0,
            atk_bonus: 0,
            origin: "白塔半島・文曜印行書閣".to_string(),
        });
    }
    if item_id == "wt_beast_bone" {
        return Some(crate::db::ItemDefBrief {
            name: "堅實獸骨".to_string(),
            item_type: "material".to_string(),
            slot: String::new(),
            description: "自山海異獸殘軀分理出的堅實獸骨，質地細密堅韌，可作為百工骨器、骨針或熬湯之材。".to_string(),
            weight: 0.5,
            vit_bonus: 0,
            dex_bonus: 0,
            atk_bonus: 0,
            origin: "白塔半島・山海獵場".to_string(),
        });
    }
    if item_id == "wt_down_feather" {
        return Some(crate::db::ItemDefBrief {
            name: "水鳥蓬鬆絨羽".to_string(),
            item_type: "material".to_string(),
            slot: String::new(),
            description: "採集自澤岸水鳥棲息處的細密絨羽，輕盈保暖，可用於縫製防寒襯衣或精細百工箭羽。".to_string(),
            weight: 0.1,
            vit_bonus: 0,
            dex_bonus: 0,
            atk_bonus: 0,
            origin: "白塔半島・水尾澤岸".to_string(),
        });
    }
    if item_id.starts_with("manual_") {
        if let Some(rec) = crate::pg::living_world::get_transcribed_manual(item_id) {
            return Some(crate::db::ItemDefBrief {
                name: format!("《{}》手札", rec.formal_name),
                item_type: "manual".to_string(),
                slot: String::new(),
                description: format!(
                    "由【{}】親手著譜之傳世手札（路數：{}・要訣：{}）。{}",
                    rec.author_name,
                    rec.slot_label,
                    rec.word_chars.join("・"),
                    rec.origin_basis
                ),
                weight: 0.3,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: rec.author_name,
            });
        }
        return Some(crate::db::ItemDefBrief {
            name: "傳世手札".to_string(),
            item_type: "manual".to_string(),
            slot: String::new(),
            description: "記載武學行氣路線的手札，字跡古雅。".to_string(),
            weight: 0.3,
            vit_bonus: 0,
            dex_bonus: 0,
            atk_bonus: 0,
            origin: "白塔半島".to_string(),
        });
    }
    if item_id.starts_with("corpse_") {
        if let Some(rec) = crate::pg::living_world::get_beast_corpse(item_id) {
            return Some(crate::db::ItemDefBrief {
                name: format!("{}殘軀", rec.creature_title),
                item_type: "corpse".to_string(),
                slot: String::new(),
                description: format!(
                    "尚留餘溫的山海異獸殘軀（{}）。可執行【肢體分解】取下「{}」等部位材料，或攜往百工坊與庖廚入饌成器。",
                    rec.shanhai_note,
                    rec.primary_loot_name
                ),
                weight: 15.0,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: rec.creature_title,
            });
        }
        return Some(crate::db::ItemDefBrief {
            name: "異獸殘軀".to_string(),
            item_type: "corpse".to_string(),
            slot: String::new(),
            description: "尚留餘溫的山海異獸殘軀。可執行【肢體分解】取下「獸皮」等部位材料，或攜往百工坊與庖廚入饌成器。".to_string(),
            weight: 15.0,
            vit_bonus: 0,
            dex_bonus: 0,
            atk_bonus: 0,
            origin: "大荒山海".to_string(),
        });
    }
    if item_id == "craft_wooden_handle" || item_id == "craft_wooden_tool_handle" {
        return Some(crate::db::ItemDefBrief {
            name: "堅韌硬木工具長柄".to_string(),
            item_type: "equipment".to_string(),
            slot: "hold_r".to_string(),
            description: "經導能木作刨削打磨的堅韌硬木長柄，既可組裝開山鎬與獵刀，亦可作為修補渡船踏板與漁網架的關鍵構件。".to_string(),
            weight: 1.0,
            vit_bonus: 4,
            dex_bonus: 3,
            atk_bonus: 3,
            origin: "霜林口".to_string(),
        });
    }
    if item_id == "craft_pine_torch" {
        return Some(crate::db::ItemDefBrief {
            name: "浸脂松明火把".to_string(),
            item_type: "equipment".to_string(),
            slot: "hold_l".to_string(),
            description: "以乾燥柴木浸塗天然松脂紮製而成的導能照明火把，可於暗夜荒野照明驅獸、引燃營火或驅散濕寒。".to_string(),
            weight: 0.8,
            vit_bonus: 3,
            dex_bonus: 4,
            atk_bonus: 4,
            origin: "霜林口".to_string(),
        });
    }
    match item_id {
        "mat_iron_scrap" => {
            return Some(crate::db::ItemDefBrief {
                name: "回收熟鐵料塊".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "自高爐鍛坊或鹽鐵倉清整所得的成色純良熟鐵料塊，可於鍛台或百工面板煉製精鋼器具。".to_string(),
                weight: 1.5,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "文曜萬爐鐵坊棚".to_string(),
            });
        }
        "mat_timber_log" => {
            return Some(crate::db::ItemDefBrief {
                name: "伐採原木".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "紋理細密堅實的乾燥良木，可於木作台或百工面板車製行旅長杖與木器。".to_string(),
                weight: 2.0,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "霜林口林場木棧".to_string(),
            });
        }
        "mat_reed_bundle" => {
            return Some(crate::db::ItemDefBrief {
                name: "水澤老蘆草綑".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "自河岸水澤收割晾乾的柔韌蘆草與青竹篾，可編織提籃或防風雨披。".to_string(),
                weight: 0.8,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "水尾寮河口與老蘆叢".to_string(),
            });
        }
        "mat_foraged_herb" => {
            return Some(crate::db::ItemDefBrief {
                name: "山野鮮藥草".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "帶著晨露清香的鮮嫩草藥，可焙製清心安神藥茶包或熬煉生肌膏。".to_string(),
                weight: 0.3,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "半嶺清泉溪谷".to_string(),
            });
        }
        "mat_coarse_paper_ream" => {
            return Some(crate::db::ItemDefBrief {
                name: "原漿宣紙刀".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "青竹抄紙槽濾出的純淨宣紙原漿紙刀，可裝訂採風手札或拓印長卷。".to_string(),
                weight: 0.5,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "文曜東市商行街".to_string(),
            });
        }
        "mat_hemp_rope_roll" => {
            return Some(crate::db::ItemDefBrief {
                name: "浸油粗麻繩卷".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "經桐油浸泡絞合的強韌苧麻繩卷，可編製攀岩繩或加固遠行麻布囊。".to_string(),
                weight: 1.0,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "水尾寮渡口與漚麻坊".to_string(),
            });
        }
        "mat_kindling_bundle" => {
            return Some(crate::db::ItemDefBrief {
                name: "乾燥柴枝束".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "自山林拾撿劈鋸而成的乾燥細柴枝束，引火極佳，為野地生火、工爐添薪與營火保溫之良材。".to_string(),
                weight: 0.5,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "霜林口農舍前坪".to_string(),
            });
        }
        "mat_plank_board" => {
            return Some(crate::db::ItemDefBrief {
                name: "樟松木板料".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "經木作台鋸切刨平的樟松木板料，質地堅韌不易開裂，適用於修補渡船踏板、棧橋與各類建築構件。".to_string(),
                weight: 1.2,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "霜林口梁氏鋸木坊".to_string(),
            });
        }
        "mat_charcoal_sack" => {
            return Some(crate::db::ItemDefBrief {
                name: "硬木白炭袋".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "以硬木密閉悶燒凝煉的高溫無煙白炭，熱值極高，為鍛爐深層淬火與精煉工造的必備燃料。".to_string(),
                weight: 1.2,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "霜林口相思木悶炭窯".to_string(),
            });
        }
        "mat_repair_strip" => {
            return Some(crate::db::ItemDefBrief {
                name: "修補熟鐵薄片".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "輕便平整的修補用熟鐵薄片，可用於修整加固各式器具設施。".to_string(),
                weight: 0.6,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "分家巷機巧修車棚".to_string(),
            });
        }
        "mat_sackcloth_bolt" => {
            return Some(crate::db::ItemDefBrief {
                name: "粗織麻袋布匹".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "粗織結實的麻布匹料，可用於縫製耐磨行囊與防風遮棚。".to_string(),
                weight: 1.2,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "水尾寮渡頭市集與倉棚".to_string(),
            });
        }
        "mat_fresh_shrimp" => {
            return Some(crate::db::ItemDefBrief {
                name: "水尾寮鮮沼蝦".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "自水尾寮清澈河口與溪流捕撈起的鮮活沼蝦，殼薄肉彈、滋味清甜，可炭炙成鮮蝦串或入鍋熬煮鮮湯。".to_string(),
                weight: 0.3,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "水尾寮河口與老蘆叢".to_string(),
            });
        }
        "shanhai_fresh_fish" => {
            return Some(crate::db::ItemDefBrief {
                name: "深海鮮魚".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "自外海與港灣釣獲的肥美鮮魚，魚鱗銀亮、肉質緊實，可相思木冷燻成魚乾或下鍋煨煮魚湯。".to_string(),
                weight: 0.8,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "風舶角避風港".to_string(),
            });
        }
        "shanhai_beast_meat" => {
            return Some(crate::db::ItemDefBrief {
                name: "山海鮮獸肉".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "自山海獵場取得的精實鮮獸肉，肌理分明、富含氣血，可揉入馬告炭炙成獸肉脯或慢火燉湯。".to_string(),
                weight: 1.0,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "牛埔獵場".to_string(),
            });
        }
        "mat_salted_fish" => {
            return Some(crate::db::ItemDefBrief {
                name: "海鹽醃魚乾".to_string(),
                item_type: "consumable".to_string(),
                slot: String::new(),
                description: "以日曬粗鹽醃漬風乾的海魚乾，鹹鮮耐儲，行旅途中食用可迅速補充體力。".to_string(),
                weight: 0.4,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "烏石鼻".to_string(),
            });
        }
        "mat_grain_sack" => {
            return Some(crate::db::ItemDefBrief {
                name: "農家精選穀糧袋".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "大圳頭良田收成並精篩飽滿的稻穀糧袋，可磨粉烙製行旅耐儲米糧餅或入釜熬粥。".to_string(),
                weight: 1.4,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "大圳頭".to_string(),
            });
        }
        "mat_pasture_seed_sack" => {
            return Some(crate::db::ItemDefBrief {
                name: "優質牧草種籽袋".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "牛埔草甸篩選收存的飽滿牧草種籽袋，可播種育草或焙炒成雜糧乾糧。".to_string(),
                weight: 0.55,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "牛埔".to_string(),
            });
        }
        "harvest_fish" => {
            return Some(crate::db::ItemDefBrief {
                name: "鮮活河魚".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "自清澈溪澗捕獲的鮮活河魚，肉質細嫩無腥，適宜炭烤或慢火煨湯。".to_string(),
                weight: 0.6,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "水尾寮".to_string(),
            });
        }
        "harvest_herb" => {
            return Some(crate::db::ItemDefBrief {
                name: "野地草藥".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "採自山野林間的天然草藥，帶有清冽草木氣息，可熬製活血膏或焙製百草茶包。".to_string(),
                weight: 0.3,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "半嶺鎮".to_string(),
            });
        }
        "harvest_wood" => {
            return Some(crate::db::ItemDefBrief {
                name: "北坡雲杉原木".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "採自北坡雲杉林的堅實原木，木質細密挺直，適合車製木柄或百工器具。".to_string(),
                weight: 2.0,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "霜林口".to_string(),
            });
        }
        "harvest_ore" => {
            return Some(crate::db::ItemDefBrief {
                name: "赤紋鐵礦".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "採自赤石坡與礦坑的含鐵原礦，夾雜赤紅礦紋，可淘洗冶煉鍛打成器。".to_string(),
                weight: 1.8,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "下營".to_string(),
            });
        }
        "harvest_berries" => {
            return Some(crate::db::ItemDefBrief {
                name: "原野漿果".to_string(),
                item_type: "consumable".to_string(),
                slot: String::new(),
                description: "採自林緣灌叢的飽滿野漿果，酸甜多汁，生食可解渴生津，亦可入饌熬醬。".to_string(),
                weight: 0.2,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "霜林口".to_string(),
            });
        }
        "harvest_vine" => {
            return Some(crate::db::ItemDefBrief {
                name: "深海巨藻藤".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "採自巨藻淺灣的強韌海藻藤，富含膠質且纖維堅韌，可編繩或入藥。".to_string(),
                weight: 0.5,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "烏石鼻".to_string(),
            });
        }
        "harvest_hardstone" => {
            return Some(crate::db::ItemDefBrief {
                name: "緻密硬石".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "質地堅硬細密的天然石料，可用於打造石砧、修補工事或鍛造開山重具。".to_string(),
                weight: 1.6,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "半嶺鎮".to_string(),
            });
        }
        "harvest_rare_wood" => {
            return Some(crate::db::ItemDefBrief {
                name: "溫木心材".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "取自百年老樟與溫木的核心良材，散發天然幽香，防潮防蛀，最適合雕琢隨身木匣。".to_string(),
                weight: 1.5,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "霜林口".to_string(),
            });
        }
        "harvest_deadwood" => {
            return Some(crate::db::ItemDefBrief {
                name: "乾燥枯枝".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "山林間自然風乾的枯樹枝，極易引火，可紮製松明火把或添入營火。".to_string(),
                weight: 0.5,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "霜林口".to_string(),
            });
        }
        "harvest_crystal" => {
            return Some(crate::db::ItemDefBrief {
                name: "寒水玉原石".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "蘊含清冽寒氣的天然玉石原礦，質地澄澈溫潤。".to_string(),
                weight: 0.8,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "半嶺鎮".to_string(),
            });
        }
        "mat_herb_poultice" => {
            return Some(crate::db::ItemDefBrief {
                name: "百草外敷藥膏".to_string(),
                item_type: "consumable".to_string(),
                slot: String::new(),
                description: "採集百草與藥泥精煉熬製的溫經外敷藥膏，清涼透肌，善消紅腫暗傷。".to_string(),
                weight: 0.4,
                vit_bonus: 2,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "半嶺鎮".to_string(),
            });
        }
        "mat_beast_hide" => {
            return Some(crate::db::ItemDefBrief {
                name: "粗製獸皮".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "經初步鞣製刮脂的粗韌獸皮，質厚耐磨，為裁縫防風斗篷與工靴的紮實皮料。".to_string(),
                weight: 1.2,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "霜林口".to_string(),
            });
        }
        "beast_scale" => {
            return Some(crate::db::ItemDefBrief {
                name: "山海獸鱗".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "山林巨獸或水裔大妖身上蛻落的堅厚鱗片，質地堅若精鋼，可用於編綴護甲或入藥。".to_string(),
                weight: 0.6,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "下營".to_string(),
            });
        }
        "key_brass_home_lock" => {
            return Some(crate::db::ItemDefBrief {
                name: "黃銅家宅門鎖鑰匙".to_string(),
                item_type: "tool".to_string(),
                slot: String::new(),
                description: "公議堂配發的黃銅家宅門鎖鑰匙，匙齒精準咬合，為安居私宅的憑信之物。".to_string(),
                weight: 0.1,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "文曜".to_string(),
            });
        }
        "item_coastal_sea_chart" => {
            return Some(crate::db::ItemDefBrief {
                name: "黑潮順時針海圖摺頁".to_string(),
                item_type: "misc".to_string(),
                slot: String::new(),
                description: "以油布皮紙精細繪製的沿岸潮汐與暗礁海圖，標註黑潮迴流與淺灣錨地。".to_string(),
                weight: 0.3,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "風舶角".to_string(),
            });
        }
        "item_personal_astrolabe_scroll" => {
            return Some(crate::db::ItemDefBrief {
                name: "周天命格星盤拓卷".to_string(),
                item_type: "misc".to_string(),
                slot: String::new(),
                description: "觀星台依星軌儀推步校準的個人先天星盤拓本，墨色深邃，周天星斗分野明晰。".to_string(),
                weight: 0.3,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "半嶺鎮".to_string(),
            });
        }
        "item_oiled_bamboo_umbrella" => {
            return Some(crate::db::ItemDefBrief {
                name: "雙層桐油防雨竹傘".to_string(),
                item_type: "equipment".to_string(),
                slot: "hold_l".to_string(),
                description: "三年生重竹骨架，傘面覆以七層熟桐油浸染皮紙，風雨不侵，收展清脆沉著。".to_string(),
                weight: 0.9,
                vit_bonus: 3,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "水尾寮".to_string(),
            });
        }
        "item_spruce_votive_plaque" => {
            return Some(crate::db::ItemDefBrief {
                name: "北嶺雲杉祈福木牌".to_string(),
                item_type: "misc".to_string(),
                slot: String::new(),
                description: "採北嶺古雲杉心材雕琢的祈福木牌，質地細密且帶松木清香，寄託行者安寧心願。".to_string(),
                weight: 0.2,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "霜林口".to_string(),
            });
        }
        "item_white_tower_gazetteer_scroll" => {
            return Some(crate::db::ItemDefBrief {
                name: "白塔風物孤本鈔卷".to_string(),
                item_type: "misc".to_string(),
                slot: String::new(),
                description: "墨香猶存的孤本手鈔手卷，詳載白塔半島地理水文、礦脈動植與九大聚落風俗。".to_string(),
                weight: 0.4,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "半嶺鎮".to_string(),
            });
        }
        "item_ebony_mortise_tea_tray" => {
            return Some(crate::db::ItemDefBrief {
                name: "沉水烏木榫卯茶盤".to_string(),
                item_type: "misc".to_string(),
                slot: String::new(),
                description: "珍貴沉水烏木全榫卯咬合製成的茶盤，質地沉凝溫潤，水潑不漬，古雅厚重。".to_string(),
                weight: 1.1,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "半嶺鎮".to_string(),
            });
        }
        "item_quenched_steel_whetstone" => {
            return Some(crate::db::ItemDefBrief {
                name: "雙面精鋼油石".to_string(),
                item_type: "tool".to_string(),
                slot: String::new(),
                description: "粗細雙面金剛淬刃磨石，專為推平捲刃崩口、砥礪百煉刀劍鋒芒而造。".to_string(),
                weight: 0.8,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "文曜".to_string(),
            });
        }
        "item_indigo_rain_cloak" => {
            return Some(crate::db::ItemDefBrief {
                name: "山藍防雨斗篷".to_string(),
                item_type: "equipment".to_string(),
                slot: "body".to_string(),
                description: "天然木藍浸染、過油密織帆布縫製的寬大斗篷，遮風禦寒，能抵禦嶺上暴雨狂風。".to_string(),
                weight: 1.4,
                vit_bonus: 3,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "風舶角".to_string(),
            });
        }
        "item_wenyao_cadastral_rubbing" => {
            return Some(crate::db::ItemDefBrief {
                name: "文曜四坊地籍圖鈔頁".to_string(),
                item_type: "misc".to_string(),
                slot: String::new(),
                description: "文曜城四坊街巷與公私宅舍地籍拓本，字跡嚴整分明，為勘定產業之官方憑證。".to_string(),
                weight: 0.2,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "文曜".to_string(),
            });
        }
        "item_braised_beef_broth_jar" => {
            return Some(crate::db::ItemDefBrief {
                name: "慢燉原牛肉湯陶盅".to_string(),
                item_type: "consumable".to_string(),
                slot: String::new(),
                description: "三水大食堂紅鐵大釜慢火細燉的大角原牛肉與甜白蘿蔔濃湯，封於保溫粗陶盅內，香氣四溢。".to_string(),
                weight: 0.6,
                vit_bonus: 5,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "半嶺鎮".to_string(),
            });
        }
        "item_roasted_frost_tea_tin" => {
            return Some(crate::db::ItemDefBrief {
                name: "炭焙霜林烘青茶罐".to_string(),
                item_type: "consumable".to_string(),
                slot: String::new(),
                description: "東橋山風茶屋以相思木炭文火慢焙的霜林高地烘青茶葉，錫罐密封，茶湯清冽回甘。".to_string(),
                weight: 0.3,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "霜林口".to_string(),
            });
        }
        "item_seven_star_moxa_pack" => {
            return Some(crate::db::ItemDefBrief {
                name: "九蒸七星艾絨藥包".to_string(),
                item_type: "consumable".to_string(),
                slot: String::new(),
                description: "溫氏百草烘藥房以紅泥焙爐九蒸九曝精製的陳年七星艾絨與當歸藥包，溫經通絡、芳香避穢。".to_string(),
                weight: 0.3,
                vit_bonus: 4,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "半嶺鎮".to_string(),
            });
        }
        "item_collated_stele_rubbing" => {
            return Some(crate::db::ItemDefBrief {
                name: "精校金石碑帖拓本".to_string(),
                item_type: "misc".to_string(),
                slot: String::new(),
                description: "講堂街校勘齋與藏書閣東巷逐字比勘摹拓的白塔古碑金石拓本，墨色烏金、字口鋒稜畢現。".to_string(),
                weight: 0.2,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "半嶺鎮".to_string(),
            });
        }
        "item_polished_quartz_lens" => {
            return Some(crate::db::ItemDefBrief {
                name: "精磨紅土水晶鏡片".to_string(),
                item_type: "misc".to_string(),
                slot: String::new(),
                description: "觀星台以紅土崁天然白水晶與細金剛砂精磨校準的雙凸光學鏡片，澄澈無瑕、聚光精準。".to_string(),
                weight: 0.2,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "紅土崁".to_string(),
            });
        }
        "item_smoked_mullet_roe_pack" => {
            return Some(crate::db::ItemDefBrief {
                name: "風櫃尾冷燻烏魚子禮包".to_string(),
                item_type: "consumable".to_string(),
                slot: String::new(),
                description: "三水口乾貨海味行嚴選的風櫃尾海鹽壓製、相思木冷燻烏魚子，色澤金珀、鹹鮮綿密。".to_string(),
                weight: 0.5,
                vit_bonus: 4,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "南哨".to_string(),
            });
        }
        _ => {}
    }
    for g in STANDARD_FINISHED_GOODS {
        if g.id == item_id {
            return Some(crate::db::ItemDefBrief {
                name: g.name.to_string(),
                item_type: g.item_type.to_string(),
                slot: g.slot.to_string(),
                description: g.description.to_string(),
                weight: g.weight,
                vit_bonus: g.vit_bonus,
                dex_bonus: g.dex_bonus,
                atk_bonus: g.atk_bonus,
                origin: g.origin.to_string(),
            });
        }
    }
    if let Some(rest) = item_id.strip_prefix("mut_") {
        for arch in MUTATION_ARCHETYPES {
            let suffix = format!("_{}", arch.key);
            if let Some(base_part) = rest.strip_suffix(&suffix) {
                let base_def = if let Some(base_good) = STANDARD_FINISHED_GOODS
                    .iter()
                    .find(|g| g.id == base_part || g.id.starts_with(base_part))
                {
                    Some(crate::db::ItemDefBrief {
                        name: base_good.name.to_string(),
                        item_type: base_good.item_type.to_string(),
                        slot: base_good.slot.to_string(),
                        description: base_good.description.to_string(),
                        weight: base_good.weight,
                        vit_bonus: base_good.vit_bonus,
                        dex_bonus: base_good.dex_bonus,
                        atk_bonus: base_good.atk_bonus,
                        origin: base_good.origin.to_string(),
                    })
                } else if base_part != item_id && !base_part.starts_with("mut_") {
                    lookup_ecology_item_def(base_part)
                } else {
                    None
                };

                let base_def = base_def.unwrap_or_else(|| {
                    let base_good = &STANDARD_FINISHED_GOODS[0];
                    crate::db::ItemDefBrief {
                        name: base_good.name.to_string(),
                        item_type: base_good.item_type.to_string(),
                        slot: base_good.slot.to_string(),
                        description: base_good.description.to_string(),
                        weight: base_good.weight,
                        vit_bonus: base_good.vit_bonus,
                        dex_bonus: base_good.dex_bonus,
                        atk_bonus: base_good.atk_bonus,
                        origin: base_good.origin.to_string(),
                    }
                });

                return Some(crate::db::ItemDefBrief {
                    name: format!("{}・{}", arch.name_prefix, base_def.name),
                    item_type: base_def.item_type,
                    slot: base_def.slot,
                    description: format!(
                        "{}（{}，器物成色與質理俱臻上乘。）",
                        base_def.description,
                        arch.desc_note
                    ),
                    weight: base_def.weight,
                    vit_bonus: base_def.vit_bonus + arch.vit_bonus,
                    dex_bonus: base_def.dex_bonus + arch.dex_bonus,
                    atk_bonus: base_def.atk_bonus + arch.atk_bonus,
                    origin: base_def.origin,
                });
            }
        }
    }
    if let Some(cat) = crate::game::peninsula_catalog::find_catalog_entry(item_id) {
        if let Some((item_type, slot, vit_bonus, dex_bonus, atk_bonus)) =
            classify_catalog_wearable_slot(cat.name, cat.category_num, cat.daily_function)
        {
            return Some(crate::db::ItemDefBrief {
                name: cat.name.to_string(),
                item_type: item_type.to_string(),
                slot: slot.to_string(),
                description: cat.look_text.to_string(),
                weight: crate::game::peninsula_catalog::catalog_entry_weight_kg(cat),
                vit_bonus,
                dex_bonus,
                atk_bonus,
                origin: format!("白塔半島・{}", cat.workshop),
            });
        }
        let (item_type, slot) = if matches!(cat.category_num, "08" | "09" | "10" | "8" | "9")
            || cat.socket == "Taste"
        {
            ("consumable", "")
        } else {
            ("material", "")
        };
        return Some(crate::db::ItemDefBrief {
            name: cat.name.to_string(),
            item_type: item_type.to_string(),
            slot: slot.to_string(),
            description: if item_type == "consumable" {
                cat.look_text.replace("，主要用於", "；")
            } else {
                cat.look_text.to_string()
            },
            weight: crate::game::peninsula_catalog::catalog_entry_weight_kg(cat),
            vit_bonus: 0,
            dex_bonus: 0,
            atk_bonus: 0,
            origin: format!("白塔半島・{}", cat.workshop),
        });
    }
    None
}

#[cfg(test)]
mod item_definitions_tests {
    use super::*;

    #[test]
    fn test_lookup_ecology_item_def_mutated_catalog_item() {
        // wt_cat_1021 is "岩羊角手柄折疊獵刀", category 19, hold_r
        let base = lookup_ecology_item_def("wt_cat_1021").expect("base catalog item should exist");
        assert_eq!(base.name, "岩羊角手柄折疊獵刀");
        assert_eq!(base.slot, "hold_r");
        assert_eq!(base.item_type, "equipment");

        // Test mutation with arch "star_resonance" (MUTATION_ARCHETYPES[0])
        let arch = &MUTATION_ARCHETYPES[0];
        let mut_id = format!("mut_wt_cat_1021_{}", arch.key);
        let mutated = lookup_ecology_item_def(&mut_id).expect("mutated catalog item should resolve");

        // Name must be prefixed with mutation prefix and retain true catalog name
        assert_eq!(mutated.name, format!("{}・{}", arch.name_prefix, base.name));
        assert_eq!(mutated.slot, "hold_r");
        assert_eq!(mutated.item_type, "equipment");
        assert_eq!(mutated.atk_bonus, base.atk_bonus + arch.atk_bonus);
        assert_eq!(mutated.vit_bonus, base.vit_bonus + arch.vit_bonus);
        assert_eq!(mutated.dex_bonus, base.dex_bonus + arch.dex_bonus);
        assert_eq!(mutated.origin, base.origin);
    }
}
