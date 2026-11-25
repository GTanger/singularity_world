//! 銀鷗島五鎮地景生態自洽之雙邊 `CanonicalArcFact` 因果本末事件表（供 `causal_arcs` 調用）。
//!
//! 嚴格分離五鎮地景（灰松林驛純內陸山林驛道／西峽渡口海峽棧橋／鷗尾漁港黑潮礁灣／海風北塢斷崖燈塔／白鷗港防波石堤），
//! 杜絕內陸山鎮出現「封凍海灣／出海捕魚」等穿幫描述，且每樁本末皆具備「起因危機 → 當場動作 → 白話原話 → 長久印記」。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalArcFact {
    pub arc_id: String,
    pub solar_year: i16,
    pub season_zh: &'static str,
    pub town_slug: String,
    pub place_zh: String,
    pub benefactor_id: String,
    pub benefactor_name: String,
    pub recipient_id: String,
    pub recipient_name: String,
    pub crisis_setup_zh: String,
    pub action_climax_zh: String,
    pub spoken_line_zh: String,
    pub lasting_mark_zh: String,
    pub ep_benefactor_zh: String,
    pub ep_recipient_zh: String,
    pub note_benefactor_zh: String,
    pub note_recipient_zh: String,
    pub warmth_delta: i8,
    pub vessel_provenance_for_recipient: Option<String>,
}

pub fn solar_year_zh(yr: i16) -> String {
    let digits = ['〇', '一', '二', '三', '四', '五', '六', '七', '八', '九'];
    yr.abs()
        .to_string()
        .chars()
        .map(|c| {
            c.to_digit(10)
                .and_then(|d| digits.get(d as usize).copied())
                .unwrap_or(c)
        })
        .collect()
}

pub fn juvenile_witness_age(race: &str) -> i16 {
    match race {
        "精靈" => 32,
        "矮人" => 22,
        _ => 10,
    }
}

#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "five distinct town ecologies × multi-beat causal arc catalog kept in one pure table"
)]
pub(super) fn build_town_ecological_arc(
    arc_id: String,
    town_slug: &str,
    town_zh: &str,
    ev_year: i16,
    slot_variant: u64,
    a_id: &str,
    a_name: &str,
    a_role: &str,
    b_id: &str,
    b_name: &str,
    b_role: &str,
    b_weapon_name: &str,
) -> CanonicalArcFact {
    let yr_zh = solar_year_zh(ev_year);
    let pick = (slot_variant % 5) as usize;
    let a_craft_metal = a_role.contains("鍛") || a_role.contains("鐵") || a_role.contains("石");
    let a_craft_herb = a_role.contains("藥") || a_role.contains("醫");

    match (town_slug, pick) {
        // ── 灰松林驛（純內陸山林驛道、雪封嶺道、灰耳斥候、山澗斷橋、採藥護車）──
        ("greypine_post", 0) => CanonicalArcFact {
            arc_id,
            solar_year: ev_year,
            season_zh: "寒冬",
            town_slug: town_slug.to_string(),
            place_zh: format!("{town_zh}東西碎石驛道"),
            benefactor_id: a_id.to_string(),
            benefactor_name: a_name.to_string(),
            recipient_id: b_id.to_string(),
            recipient_name: b_name.to_string(),
            crisis_setup_zh: format!(
                "太陽曆{yr_zh}年寒冬大雪封死灰松嶺碎石驛道十餘日，往返白鷗港與西峽渡口的雙馬篷車全數斷絕，巷裡存糧見底"
            ),
            action_climax_zh: format!(
                "{a_name}趁夜踩著沒膝積雪敲開{b_name}家木窗，從自家過冬存貨裡分出半袋粗麥乾糧與兩捆乾松柴塞進爐邊"
            ),
            spoken_line_zh: "雪還沒停，先把爐火生起來，人熬過去開春再算帳".to_string(),
            lasting_mark_zh: format!(
                "那袋雪夜救命糧讓{b_name}一家撐到驛道鏟通，兩戶人家自此在林驛裡成了過命街坊"
            ),
            ep_benefactor_zh: format!(
                "太陽曆 {ev_year} 年冬大雪封死灰松嶺驛道十餘日，篷車斷絕，我夜裡勻出半袋粗麥與兩捆乾松柴送去幫{b_name}家度過斷炊死劫。"
            ),
            ep_recipient_zh: format!(
                "太陽曆 {ev_year} 年冬暴雪封嶺斷了過路篷車，家裡米缸見底，全靠{a_name}趁雪夜敲窗送來半袋粗麥與乾松柴才熬過那場寒凍。"
            ),
            note_benefactor_zh: "大雪封嶺斷糧那年曾親手分糧接濟的老街坊".to_string(),
            note_recipient_zh: "雪夜封驛斷炊時送來救命乾糧柴火的恩人街坊".to_string(),
            warmth_delta: 44,
            vessel_provenance_for_recipient: None,
        },
        ("greypine_post", 1) => CanonicalArcFact {
            arc_id,
            solar_year: ev_year,
            season_zh: "金秋",
            town_slug: town_slug.to_string(),
            place_zh: format!("{town_zh}北側荒氣山谷"),
            benefactor_id: a_id.to_string(),
            benefactor_name: a_name.to_string(),
            recipient_id: b_id.to_string(),
            recipient_name: b_name.to_string(),
            crisis_setup_zh: format!(
                "太陽曆{yr_zh}年秋雨連綿，{b_name}深入灰松北谷辦{b_role}活計時踏滑青苔墜入陡坡扭傷腳踝，谷口又有灰耳哥布林斥候吹骨哨遊蕩，同行者皆不敢折返"
            ),
            action_climax_zh: format!(
                "{a_name}聞訊二話不說拎起麻繩與隨身傢伙冒雨下谷，藉濃霧掩護硬把動彈不得的{b_name}背回林驛敷上苦艾膏"
            ),
            spoken_line_zh: "抓緊我肩膀別出聲，林驛的人沒有丟在半山腰餵斥候的道理".to_string(),
            lasting_mark_zh: format!(
                "{b_name}養傷半月方能下地，此後每逢{a_name}家屋頂修繕或搬運重料總是第一個趕來搭手"
            ),
            ep_benefactor_zh: format!(
                "太陽曆 {ev_year} 年秋雨裡{b_name}在北谷墜坡傷了腳踝，外頭灰耳斥候遊蕩，我帶麻繩冒雨下谷將人背回林驛救治。"
            ),
            ep_recipient_zh: format!(
                "太陽曆 {ev_year} 年秋雨在灰松北谷失足傷踝受困於斥候哨線旁，多虧{a_name}獨自冒雨下谷將我背回林驛撿回一命。"
            ),
            note_benefactor_zh: "當年冒雨入灰松北谷親手背回來的同鎮街坊".to_string(),
            note_recipient_zh: "灰松北谷遇險受困時冒雨將自己背回來的救命恩人".to_string(),
            warmth_delta: 52,
            vessel_provenance_for_recipient: None,
        },
        ("greypine_post", 2) => CanonicalArcFact {
            arc_id,
            solar_year: ev_year,
            season_zh: "仲夏",
            town_slug: town_slug.to_string(),
            place_zh: format!("{town_zh}山澗木橋"),
            benefactor_id: a_id.to_string(),
            benefactor_name: a_name.to_string(),
            recipient_id: b_id.to_string(),
            recipient_name: b_name.to_string(),
            crisis_setup_zh: format!(
                "太陽曆{yr_zh}年夏末山洪暴漲沖歪林驛西側山澗木橋，狂風更在夜半掀裂了{b_name}家堆放乾貨藥材的後屋木樑"
            ),
            action_climax_zh: format!(
                "{a_name}扛著備用杉木椽子與鐵釘頂著暴雨趕來，踩上濕滑屋脊與{b_name}合力壓住油布、釘死橫樑，雙手被木刺刮得鮮血淋漓"
            ),
            spoken_line_zh: "先把油布四角釘死，這屋裡的吃飯傢伙淋爛了明年全得挨餓受凍".to_string(),
            lasting_mark_zh: format!(
                "屋內存貨保住了大半，但{b_name}心裡清楚欠下{a_name}一樁難還的風雨人情"
            ),
            ep_benefactor_zh: format!(
                "太陽曆 {ev_year} 年夏末山洪暴雨夜裡{b_name}家後屋險些塌樑，我扛著杉木料冒雨上頂幫忙釘牢才保住屋裡存貨。"
            ),
            ep_recipient_zh: format!(
                "太陽曆 {ev_year} 年夏末狂風暴雨夜裡家裡屋樑開裂，全靠{a_name}扛杉木冒雨爬上屋頂合力釘死才保住吃飯家當。"
            ),
            note_benefactor_zh: "山洪暴雨夜曾扛木料替對方搶修屋樑的鄰居".to_string(),
            note_recipient_zh: "暴雨掀樑那夜冒險上屋頂遞木搶修的厚道街坊".to_string(),
            warmth_delta: 38,
            vessel_provenance_for_recipient: None,
        },
        ("greypine_post", 3) => CanonicalArcFact {
            arc_id,
            solar_year: ev_year,
            season_zh: "初春",
            town_slug: town_slug.to_string(),
            place_zh: format!("{town_zh}驛站車棚"),
            benefactor_id: a_id.to_string(),
            benefactor_name: a_name.to_string(),
            recipient_id: b_id.to_string(),
            recipient_name: b_name.to_string(),
            crisis_setup_zh: format!(
                "太陽曆{yr_zh}年開春，{a_name}與{b_name}合夥承接了一批替過嶺雙馬篷車整補護具與乾糧藥包的急單"
            ),
            action_climax_zh: format!(
                "兩人連熬三個通宵趕在車隊發車前交齊貨品，結算銅幣時卻因誰墊的松脂藥材與皮繩耗損更多而當場紅了臉爭執起來"
            ),
            spoken_line_zh: "帳目一筆歸一筆，今天少算兩枚銅板，往後這條巷子裡沒法再合夥"
                .to_string(),
            lasting_mark_zh: format!(
                "{a_name}最後雖主動讓了一成工錢把帳結清，但{b_name}總覺得對方話裡帶刺，此後兩家見面總隔著一層客氣"
            ),
            ep_benefactor_zh: format!(
                "太陽曆 {ev_year} 年春與{b_name}合包驛車急單，我墊料出力最多，結帳時讓了一成銅板卻還落了埋怨，往後合夥得留神。"
            ),
            ep_recipient_zh: format!(
                "太陽曆 {ev_year} 年春與{a_name}合辦驛車補給急單，為工料折算爭過幾句，帳雖結清，心裡始終覺得對方算得太精。"
            ),
            note_benefactor_zh: "早年合包驛車急單分帳鬧過彆扭的同鎮鄰居".to_string(),
            note_recipient_zh: "早年合辦驛站活計時因工料分帳留有芥蒂的街坊".to_string(),
            warmth_delta: -20,
            vessel_provenance_for_recipient: None,
        },
        ("greypine_post", _) => {
            let prov = if a_craft_herb {
                format!("太陽曆{yr_zh}年經{a_name}以山谷苦艾蠟油反覆浸養防裂之隨身舊器")
            } else {
                format!("太陽曆{yr_zh}年經{a_name}親手削換老灰松木握柄並纏緊牛筋之隨身舊器")
            };
            CanonicalArcFact {
                arc_id,
                solar_year: ev_year,
                season_zh: "金秋",
                town_slug: town_slug.to_string(),
                place_zh: format!("{town_zh}石井巷口"),
                benefactor_id: a_id.to_string(),
                benefactor_name: a_name.to_string(),
                recipient_id: b_id.to_string(),
                recipient_name: b_name.to_string(),
                crisis_setup_zh: format!(
                    "太陽曆{yr_zh}年秋，{b_name}賴以謀生的【{b_weapon_name}】在山道撬石防身時震裂了握柄，偏逢手頭銅幣緊絀換不起新器"
                ),
                action_climax_zh: format!(
                    "{a_name}取來自家陰乾三年的老灰松芯木與浸油牛筋，在燈下替{b_name}重削握柄、打緊銅箍，只收了半價材料本錢"
                ),
                spoken_line_zh: "在灰松林裡討生活，手裡的傢伙鬆一分就是拿命開玩笑".to_string(),
                lasting_mark_zh: format!(
                    "那件重箍握柄的【{b_weapon_name}】{b_name}一直順手用到今天，只是{a_name}偶爾在茶餘提起這樁半價人情，讓好強的{b_name}心裡有些五味雜陳"
                ),
                ep_benefactor_zh: format!(
                    "太陽曆 {ev_year} 年秋替手頭緊的{b_name}用老灰松芯木重箍了【{b_weapon_name}】握柄，只收半價工本。"
                ),
                ep_recipient_zh: format!(
                    "太陽曆 {ev_year} 年秋手裡的【{b_weapon_name}】震裂握柄，靠{a_name}半價換上老灰松木柄才沒耽誤生計。"
                ),
                note_benefactor_zh: "曾親手替對方整修隨身吃飯傢伙的老街坊".to_string(),
                note_recipient_zh: "手頭隨身器物握柄當年全靠對方半價整修加固".to_string(),
                warmth_delta: 26,
                vessel_provenance_for_recipient: Some(prov),
            }
        }

        // ── 西峽渡口（海峽雙桅渡輪、關稅稽查、湍急水道、跨海纜繩）──
        ("west_strait_ferry", 0 | 3) => CanonicalArcFact {
            arc_id,
            solar_year: ev_year,
            season_zh: "金秋",
            town_slug: town_slug.to_string(),
            place_zh: format!("{town_zh}雙桅渡輪棧橋"),
            benefactor_id: a_id.to_string(),
            benefactor_name: a_name.to_string(),
            recipient_id: b_id.to_string(),
            recipient_name: b_name.to_string(),
            crisis_setup_zh: format!(
                "太陽曆{yr_zh}年秋潮暴漲，西峽水道暗流突變，滿載重貨靠岸的雙桅渡輪崩斷副纜，棧橋上的{b_name}被橫掃的斷纜擦傷險些墜入急流"
            ),
            action_climax_zh: format!(
                "在場的{a_name}眼疾手快飛身探出木樁，一把死死拽住{b_name}後腰皮帶，再用備用麻纜繞樁三圈硬剎住偏擺的船頭"
            ),
            spoken_line_zh: "站穩腳跟別往下看，西峽急流捲進去連船板都撈不回來".to_string(),
            lasting_mark_zh: format!(
                "{b_name}手臂留下一道麻纜灼出的淺疤，從此把{a_name}視作渡頭上最信得過的過命交情"
            ),
            ep_benefactor_zh: format!(
                "太陽曆 {ev_year} 年秋西峽渡輪斷纜橫掃棧橋，我一把拽住險些落海的{b_name}並繞樁剎住船頭。"
            ),
            ep_recipient_zh: format!(
                "太陽曆 {ev_year} 年秋潮渡輪崩斷副纜險將我掃下海峽急流，全靠{a_name}飛身拽住皮帶救回這條命。"
            ),
            note_benefactor_zh: "渡輪斷纜那回在棧橋邊親手拉上來的同鎮熟人".to_string(),
            note_recipient_zh: "西峽棧橋斷纜時拽住自己撿回一命的救命恩人".to_string(),
            warmth_delta: 50,
            vessel_provenance_for_recipient: None,
        },
        ("west_strait_ferry", 1 | 2) => CanonicalArcFact {
            arc_id,
            solar_year: ev_year,
            season_zh: "寒冬",
            town_slug: town_slug.to_string(),
            place_zh: format!("{town_zh}木桶酒館"),
            benefactor_id: a_id.to_string(),
            benefactor_name: a_name.to_string(),
            recipient_id: b_id.to_string(),
            recipient_name: b_name.to_string(),
            crisis_setup_zh: format!(
                "太陽曆{yr_zh}年冬，海峽風浪連封半月加上關稅嚴查，{b_name}押在渡口的貨款遲遲結不下來，連過冬黑煤與口糧都周轉不開"
            ),
            action_climax_zh: format!(
                "{a_name}在木桶酒館角落聽聞難處，沒立字據便從貼身錢袋數出幾枚應急銅幣與一袋黑麥乾糧推過桌面，替{b_name}解了燃眉之急"
            ),
            spoken_line_zh: "渡口討生活誰沒遇過逆風壓船，等開春渡輪通航再慢慢還".to_string(),
            lasting_mark_zh: format!(
                "次年開春{b_name}雖連本帶利補上乾糧與幫工，但心頭始終記著{a_name}在逆風年景裡不落井下石的義氣"
            ),
            ep_benefactor_zh: format!(
                "太陽曆 {ev_year} 年冬海峽逆風停航，{b_name}貨款卡死，我在木桶酒館私下周轉了銅幣與黑麥助其過冬。"
            ),
            ep_recipient_zh: format!(
                "太陽曆 {ev_year} 年冬海峽停航壓死生計，多虧{a_name}沒要字據便借出應急銅幣與黑麥乾糧才撐到開春。"
            ),
            note_benefactor_zh: "海峽停航過冬時曾私下周轉接濟過的街坊".to_string(),
            note_recipient_zh: "逆風停航斷炊時不立字據仗義周轉的恩人".to_string(),
            warmth_delta: 40,
            vessel_provenance_for_recipient: None,
        },

        // ── 鷗尾漁港（外海黑潮、夜霧暗礁、鹽漬魚寮）──
        ("gull_tail_port", 0 | 3) => CanonicalArcFact {
            arc_id,
            solar_year: ev_year,
            season_zh: "金秋",
            town_slug: town_slug.to_string(),
            place_zh: format!("{town_zh}外灣鎮海礁"),
            benefactor_id: a_id.to_string(),
            benefactor_name: a_name.to_string(),
            recipient_id: b_id.to_string(),
            recipient_name: b_name.to_string(),
            crisis_setup_zh: format!(
                "太陽曆{yr_zh}年秋汛濃霧突鎖鷗尾海道，{b_name}隨小舟收網返航時偏離航標擦撞暗礁，船底滲水受困於黑潮邊緣"
            ),
            action_climax_zh: format!(
                "岸上聽見三聲求救海螺的{a_name}抓起浸油長纜與魚叉燈籠駕舢舨衝入濃霧，拋纜鉤住受損小舟硬是頂著退潮拖回內灣"
            ),
            spoken_line_zh: "把濕網全砍斷棄海裡，保住人跟船骨頭比幾簍魚強".to_string(),
            lasting_mark_zh: format!(
                "那夜同船共纜撿回性命後，每逢魚汛開桶醃鹽，{b_name}總會挑最肥厚的兩尾鹹魚送到{a_name}門前"
            ),
            ep_benefactor_zh: format!(
                "太陽曆 {ev_year} 年秋汛夜霧鎖灣，{b_name}的小舟擦撞暗礁受困，我提燈駕舢舨拋纜將人船拖回鷗尾內灣。"
            ),
            ep_recipient_zh: format!(
                "太陽曆 {ev_year} 年秋汛夜霧迷航觸礁，聽見海螺趕來的{a_name}拋纜將我拖回岸邊，這條命是對方從黑潮裡搶回來的。"
            ),
            note_benefactor_zh: "秋汛濃霧觸礁那夜駕舟拖救回港的同港人家".to_string(),
            note_recipient_zh: "夜霧觸礁險葬黑潮時提燈拋纜救命的恩人".to_string(),
            warmth_delta: 52,
            vessel_provenance_for_recipient: None,
        },
        ("gull_tail_port", 1 | 2) => CanonicalArcFact {
            arc_id,
            solar_year: ev_year,
            season_zh: "寒冬",
            town_slug: town_slug.to_string(),
            place_zh: format!("{town_zh}鹽漬魚寮"),
            benefactor_id: a_id.to_string(),
            benefactor_name: a_name.to_string(),
            recipient_id: b_id.to_string(),
            recipient_name: b_name.to_string(),
            crisis_setup_zh: format!(
                "太陽曆{yr_zh}年冬汛結算，{a_name}與{b_name}合用一口鹽漬大槽趕製外銷白鷗港的鹹魚桶，卻因一批粗鹽受潮導致底層兩桶魚乾發酸折價"
            ),
            action_climax_zh: format!(
                "兩人在魚寮油燈下翻開木桶對質，各執一詞怪對方夜裡沒蓋嚴油布，最後{a_name}獨自貼補了折價虧空才沒砸掉交貨船期"
            ),
            spoken_line_zh: "貨先按期送上船，壞掉這兩桶算我認栽，明年我們各開各的鹽槽".to_string(),
            lasting_mark_zh: format!(
                "此後兩家在碼頭碰面雖不撕破臉，合夥醃魚的交情卻自那口受潮鹽槽起徹底散了"
            ),
            ep_benefactor_zh: format!(
                "太陽曆 {ev_year} 年冬與{b_name}合槽醃魚因粗鹽受潮折了兩桶本錢，我獨自認賠保住船期，自此不再合夥。"
            ),
            ep_recipient_zh: format!(
                "太陽曆 {ev_year} 年冬與{a_name}合用鹽槽壞了兩桶魚乾，對方雖墊了虧空卻把責任全推過來，此後兩家漸趨疏淡。"
            ),
            note_benefactor_zh: "早年合槽醃魚受潮折本後分道揚鑣的鄰戶".to_string(),
            note_recipient_zh: "因冬汛鹽槽受潮舊帳結下心結的同港街坊".to_string(),
            warmth_delta: -22,
            vessel_provenance_for_recipient: None,
        },

        // ── 海風北塢（北岬峭壁、燈塔油脂火、寒潮孤塢）──
        ("north_cape_beacon", 0 | 3) => CanonicalArcFact {
            arc_id,
            solar_year: ev_year,
            season_zh: "寒冬",
            town_slug: town_slug.to_string(),
            place_zh: format!("{town_zh}北岬燈塔坡道"),
            benefactor_id: a_id.to_string(),
            benefactor_name: a_name.to_string(),
            recipient_id: b_id.to_string(),
            recipient_name: b_name.to_string(),
            crisis_setup_zh: format!(
                "太陽曆{yr_zh}年寒冬狂風裹著冰霰席捲北岬斷崖，挑運燈用油脂上坡的{b_name}在結冰石階失足滑向崖緣"
            ),
            action_climax_zh: format!(
                "迎面巡坡的{a_name}棄掉手裡木桶，整個人撲倒扣住石縫，死死拉住{b_name}的麻布斗篷直到旁人趕來拖上避風石窩"
            ),
            spoken_line_zh: "油撒了還能再熬，人掉下北岬玄武岩可聽不見半聲響".to_string(),
            lasting_mark_zh: format!(
                "{a_name}左膝在冰石上磕出舊傷，每逢陰雨微痠，{b_name}一家則把這份斷崖救命恩記在骨子裡"
            ),
            ep_benefactor_zh: format!(
                "太陽曆 {ev_year} 年寒冬北岬石階結冰，我在斷崖邊撲身拉住滑倒的{b_name}，膝頭磕傷卻保住一條人命。"
            ),
            ep_recipient_zh: format!(
                "太陽曆 {ev_year} 年寒冬挑油上北岬燈塔時踏滑冰階險墜懸崖，全靠{a_name}棄桶撲救才活到今天。"
            ),
            note_benefactor_zh: "北岬冰階險墜崖那回親手拉回來的同塢鄰人".to_string(),
            note_recipient_zh: "北岬斷崖結冰石階上捨身拉住自己的救命恩人".to_string(),
            warmth_delta: 54,
            vessel_provenance_for_recipient: None,
        },

        // ── 白鷗港與其餘海港通用深度本末（防波石堤、修器鍛紋、秋暴護屋、商單分帳）──
        (_, 0) => CanonicalArcFact {
            arc_id,
            solar_year: ev_year,
            season_zh: "金秋",
            town_slug: town_slug.to_string(),
            place_zh: format!("{town_zh}東翼防波石堤與支巷"),
            benefactor_id: a_id.to_string(),
            benefactor_name: a_name.to_string(),
            recipient_id: b_id.to_string(),
            recipient_name: b_name.to_string(),
            crisis_setup_zh: format!(
                "太陽曆{yr_zh}年秋暴黑潮夜襲海灣，鹹浪越過石堤倒灌巷口，狂風更將{b_name}家屋頂瓦片與橫木掀開大洞"
            ),
            action_climax_zh: format!(
                "{a_name}頂著鹹濕狂風扛來兩根備用船肋木與麻繩，爬上搖晃屋簷替{b_name}家綁死樑架、壓緊防雨帆布，忙到破曉初鐘才渾身濕透下梯"
            ),
            spoken_line_zh: "先把迎風面壓住，等天亮潮退了我們再重鋪石板".to_string(),
            lasting_mark_zh: format!(
                "那夜搶修保住了{b_name}一屋生計器具，這份風雨裡遞木搭手的恩情讓兩家往來格外厚實"
            ),
            ep_benefactor_zh: format!(
                "太陽曆 {ev_year} 年秋暴狂風夜裡扛著船肋木幫{b_name}家綁死掀裂的屋簷，熬到破曉才保住屋裡器具。"
            ),
            ep_recipient_zh: format!(
                "太陽曆 {ev_year} 年秋暴夜裡自家屋頂險被狂風掀翻，全靠{a_name}冒雨扛來船肋木合力綁牢才免於塌屋。"
            ),
            note_benefactor_zh: "秋暴夜曾扛船肋木替對方搶修屋頂的街坊".to_string(),
            note_recipient_zh: "秋暴掀頂那夜冒著鹹雨扛木搭手的恩情街坊".to_string(),
            warmth_delta: 40,
            vessel_provenance_for_recipient: None,
        },
        (_, 1) => {
            let prov = if a_craft_metal {
                format!("太陽曆{yr_zh}年經{a_name}重新開爐淬火校刃並打緊鐵箍之隨身舊器")
            } else {
                format!("太陽曆{yr_zh}年經{a_name}以桐油浸麻重包握柄加固之隨身舊器")
            };
            CanonicalArcFact {
                arc_id,
                solar_year: ev_year,
                season_zh: "初春",
                town_slug: town_slug.to_string(),
                place_zh: format!("{town_zh}街坊工案"),
                benefactor_id: a_id.to_string(),
                benefactor_name: a_name.to_string(),
                recipient_id: b_id.to_string(),
                recipient_name: b_name.to_string(),
                crisis_setup_zh: format!(
                    "太陽曆{yr_zh}年開春趕工之際，{b_name}隨身操持的【{b_weapon_name}】崩裂鬆動，偏逢家中剛交完房租手頭緊絀"
                ),
                action_climax_zh: format!(
                    "{a_name}瞧見後把那件【{b_weapon_name}】留到夜裡收工，用自家壓箱好料親手整修加固，隔天一早只收了半價炭料本錢便交還{b_name}"
                ),
                spoken_line_zh: "手藝人靠傢伙吃飯，刃口鈍了怎麼養活一屋子人".to_string(),
                lasting_mark_zh: format!(
                    "{b_name}握著那把重修過的【{b_weapon_name}】順利接下開春活計，心裡一直記著那半價工本背後的情分"
                ),
                ep_benefactor_zh: format!(
                    "太陽曆 {ev_year} 年春夜裡用壓箱好料替手頭緊的{b_name}整修加固【{b_weapon_name}】，只收了半價材料本錢。"
                ),
                ep_recipient_zh: format!(
                    "太陽曆 {ev_year} 年春隨身【{b_weapon_name}】損壞又缺錢，多虧{a_name}連夜用好料半價整修才沒斷了營生。"
                ),
                note_benefactor_zh: "曾連夜用好料替對方半價整修隨身器物的熟人".to_string(),
                note_recipient_zh: format!("手頭【{b_weapon_name}】當年全靠對方半價整修保住生計"),
                warmth_delta: 36,
                vessel_provenance_for_recipient: Some(prov),
            }
        }
        (_, 2) => CanonicalArcFact {
            arc_id,
            solar_year: ev_year,
            season_zh: "寒冬",
            town_slug: town_slug.to_string(),
            place_zh: format!("{town_zh}市集巷口"),
            benefactor_id: a_id.to_string(),
            benefactor_name: a_name.to_string(),
            recipient_id: b_id.to_string(),
            recipient_name: b_name.to_string(),
            crisis_setup_zh: format!(
                "太陽曆{yr_zh}年寒冬連旬風雪令外海商船停靠不得，市集糧價騰貴，{b_name}家裡一時青黃不接"
            ),
            action_climax_zh: format!(
                "{a_name}從自家儲糧箱裡勻出半袋鹽漬乾糧與幾枚周轉銅幣塞給{b_name}應急，助其熬過開春船班恢復前的難關"
            ),
            spoken_line_zh: "同住一條巷子，總不能看著街坊灶台斷煙".to_string(),
            lasting_mark_zh: format!(
                "後來{b_name}雖多次幫工送物抵還，但兩邊對這筆舊人情帳的輕重各自留著計較"
            ),
            ep_benefactor_zh: format!(
                "太陽曆 {ev_year} 年冬船班斷絕糧貴時，勻出半袋鹽漬乾糧與周轉銅幣幫{b_name}家熬過青黃不接。"
            ),
            ep_recipient_zh: format!(
                "太陽曆 {ev_year} 年冬寒潮斷航手頭緊絀，靠{a_name}勻出乾糧與銅幣應急，這份舊人情至今記在心上。"
            ),
            note_benefactor_zh: "寒冬斷航時曾勻糧周轉接濟過的鄰戶".to_string(),
            note_recipient_zh: "寒冬斷航青黃不接時曾伸手接濟的街坊".to_string(),
            warmth_delta: 32,
            vessel_provenance_for_recipient: None,
        },
        _ => CanonicalArcFact {
            arc_id,
            solar_year: ev_year,
            season_zh: "仲夏",
            town_slug: town_slug.to_string(),
            place_zh: format!("{town_zh}港務棧橋"),
            benefactor_id: a_id.to_string(),
            benefactor_name: a_name.to_string(),
            recipient_id: b_id.to_string(),
            recipient_name: b_name.to_string(),
            crisis_setup_zh: format!(
                "太陽曆{yr_zh}年夏，靠岸大船急徵人手趕辦一批限時交割的港務大單，做{a_role}的{a_name}與做{b_role}的{b_name}合夥應下差事"
            ),
            action_climax_zh: format!(
                "兩人頂著烈日趕在退潮前交清貨件，分算銅幣時卻因誰擔的風險重、誰出的耗材多而在棧橋邊爭得面紅耳赤"
            ),
            spoken_line_zh: "銅板可以少拿兩枚，但誰出了死力氣得在明面上講清楚".to_string(),
            lasting_mark_zh: format!(
                "帳面雖勉強分平，{a_name}與{b_name}心裡卻各自憋了一口氣，往後街頭碰面只剩客套點頭"
            ),
            ep_benefactor_zh: format!(
                "太陽曆 {ev_year} 年夏與{b_name}合夥趕辦港務急單，分帳時為工料輕重爭執一場，此後不再合夥。"
            ),
            ep_recipient_zh: format!(
                "太陽曆 {ev_year} 年夏與{a_name}合包港務活計，結算時話不投機留了心結，平素往來轉趨冷淡。"
            ),
            note_benefactor_zh: "合夥趕辦港務急單時因分帳結下心結的同行街坊".to_string(),
            note_recipient_zh: "早年合辦港務大單分帳不歡而散的舊識".to_string(),
            warmth_delta: -18,
            vessel_provenance_for_recipient: None,
        },
    }
}
