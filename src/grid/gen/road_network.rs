// ─── 道路與主要聯絡線 ───────────────────────────────────────────────

type RoadMap = HashMap<SquareCoord, (Terrain, &'static str, &'static str)>;

fn connect_smooth_segment(
    map: &mut RoadMap,
    from: (i32, i32),
    to: (i32, i32),
    terrain: Terrain,
) {
    let mut x = from.0;
    let mut y = from.1;
    map.entry(SquareCoord::new(x, y))
        .or_insert((terrain, "", ""));

    let dx = (to.0 - x).abs();
    let dy = (to.1 - y).abs();
    let sx = if to.0 > x { 1 } else { -1 };
    let sy = if to.1 > y { 1 } else { -1 };
    let mut err = dx - dy;

    while x != to.0 || y != to.1 {
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
            map.entry(SquareCoord::new(x, y))
                .or_insert((terrain, "", ""));
        }
        if x == to.0 && y == to.1 {
            break;
        }
        if e2 < dx {
            err += dx;
            y += sy;
            map.entry(SquareCoord::new(x, y))
                .or_insert((terrain, "", ""));
        }
    }
}

/// 依序列航點平滑鋪設自然蜿蜒之道路網，消除生硬的百格折角
fn connect_winding_road(
    map: &mut RoadMap,
    waypoints: &[(i32, i32)],
    terrain: Terrain,
) {
    for window in waypoints.windows(2) {
        connect_smooth_segment(map, window[0], window[1], terrain);
    }
}

/// 生成連通 15 座聚落與 19 處獵場的自然蜿蜒道路與橋樑網。
/// 道路格的實際 zone 與「區名・道路」名稱於 `generate_white_tower_grid` 依 `classify_wilderness_cell` 統一賦予。
fn generate_road_network() -> HashMap<SquareCoord, (Terrain, &'static str, &'static str)> {
    let mut map = HashMap::new();

    // 1. 中軸北向：文曜 (0, 18) -> 老堰頭 (0, 125) -> 翠冠巨木林 (0, 240) -> 白塔雪巔 (0, 420)
    connect_winding_road(&mut map, &[(0, 0), (0, 18), (0, 24), (8, 52), (12, 84), (5, 108), (0, 116), (0, 125)], Terrain::Road);
    connect_winding_road(&mut map, &[(0, 125), (0, 135), (6, 164), (10, 198), (5, 224), (0, 240)], Terrain::Road);
    connect_winding_road(&mut map, &[(0, 240), (10, 282), (16, 332), (8, 378), (0, 406), (0, 420)], Terrain::Road);

    // 2. 中軸南向：文曜 (0, -18) -> 南哨 (0, -220) -> 淺澤泥口 (0, -340) -> 青籠關 (0, -450) -> 南下古隘道 (-4, -475) -> (0, -499)
    connect_winding_road(&mut map, &[(0, 0), (0, -18), (0, -24), (-10, -68), (-16, -118), (-8, -172), (0, -208), (0, -213), (0, -220)], Terrain::Road);
    connect_winding_road(&mut map, &[(0, -220), (0, -228), (0, -234), (-8, -268), (-12, -304), (0, -334), (0, -340)], Terrain::Road);
    connect_winding_road(&mut map, &[(0, -340), (6, -376), (10, -412), (0, -442), (0, -450)], Terrain::Road);
    connect_winding_road(&mut map, &[(0, -450), (-4, -475), (0, -499)], Terrain::Road);

    // 3. 西北西麓林道與風舶雙港：文曜 (-18, 0) -> 西麓林道 (-130, 80) -> 風舶角 (-260, 160) -> 風舶嶼 (-330, 160) -> 驚濤岩岸 (-360, 180) -> 風櫃尾 (-360, 95)
    connect_winding_road(&mut map, &[(0, 0), (-18, 0), (-63, 0), (-88, 16), (-112, 48), (-130, 80), (-158, 98), (-194, 130), (-226, 152), (-245, 160), (-250, 160), (-260, 160)], Terrain::Road);
    connect_winding_road(&mut map, &[(-260, 160), (-270, 160), (-322, 160), (-330, 160)], Terrain::Bridge);
    connect_winding_road(&mut map, &[(-330, 160), (-338, 160), (-344, 160), (-354, 170), (-360, 180)], Terrain::Road);
    connect_winding_road(&mut map, &[(-360, 180), (-354, 152), (-352, 124), (-360, 105), (-360, 95)], Terrain::Road);
    connect_winding_road(&mut map, &[(-260, 160), (-260, 171), (-260, 176), (-272, 206), (-290, 236), (-300, 260)], Terrain::Road);
    connect_winding_road(&mut map, &[(-260, 160), (-260, 149), (-260, 144), (-246, 112), (-248, 74), (-260, 45), (-260, 35)], Terrain::Road);
    connect_winding_road(&mut map, &[(-260, 35), (-260, 26), (-252, 4), (-260, -20)], Terrain::Road);

    // 4. 大圳頭（古河巨木林、紅土崁、大圳頭、長風草甸、牛埔、三號坑口、赤石坡、落日岬）
    connect_winding_road(&mut map, &[(-260, -20), (-242, -52), (-212, -82), (-188, -110), (-180, -128), (-180, -133), (-180, -140)], Terrain::Road);
    connect_winding_road(&mut map, &[(-35, 0), (-55, -28), (-80, -60), (-92, -96), (-85, -128), (-85, -134), (-85, -140)], Terrain::Road);
    connect_winding_road(&mut map, &[(-85, -140), (-85, -147), (-85, -152), (-78, -166), (-85, -180)], Terrain::Road);
    connect_winding_road(&mut map, &[(-85, -140), (-92, -140), (-96, -140), (-125, -132), (-152, -134), (-168, -139), (-173, -139)], Terrain::Road);
    connect_winding_road(&mut map, &[(-180, -140), (-180, -148), (-180, -152), (-170, -174), (-180, -195), (-190, -218), (-180, -235), (-180, -245)], Terrain::Road);
    connect_winding_road(&mut map, &[(-187, -139), (-194, -139), (-225, -146), (-256, -144), (-274, -140), (-279, -140), (-285, -140)], Terrain::Road);
    connect_winding_road(&mut map, &[(-285, -140), (-285, -147), (-285, -152), (-308, -156), (-326, -148), (-340, -140)], Terrain::Road);

    // 5. 南境水尾澤支線（牛埔 -> 白茫洲 -> 水尾寮 -> 南哨 -> 貼山深潭）
    connect_winding_road(&mut map, &[(-180, -245), (-180, -252), (-180, -256), (-162, -286), (-138, -308), (-120, -320)], Terrain::Road);
    connect_winding_road(&mut map, &[(-120, -320), (-100, -312), (-75, -294), (-75, -287), (-75, -280)], Terrain::Road);
    connect_winding_road(&mut map, &[(-75, -280), (-68, -280), (-64, -280), (-42, -258), (-22, -232), (-12, -220), (-7, -220), (0, -220)], Terrain::Road);
    connect_winding_road(&mut map, &[(0, -220), (7, -220), (14, -220), (48, -252), (86, -288), (120, -320)], Terrain::Road);

    // 6. 東北與青脊山支線（北嶺腳、清泉溪谷、半嶺鎮、霜林口、北坡雲杉林、雲腰雪林、西嶺礦谷、東海峭壁）
    connect_winding_road(&mut map, &[(0, 0), (18, 0), (24, 0), (56, 26), (86, 62), (110, 96), (110, 108), (110, 115)], Terrain::Road);
    connect_winding_road(&mut map, &[(0, 125), (7, 125), (12, 125), (36, 146), (60, 160)], Terrain::Road);
    connect_winding_road(&mut map, &[(60, 160), (78, 140), (94, 115), (103, 115), (110, 115)], Terrain::Road);
    connect_winding_road(&mut map, &[(110, 115), (110, 122), (110, 126), (124, 158), (146, 184), (160, 198), (160, 203), (160, 210)], Terrain::Road);
    connect_winding_road(&mut map, &[(160, 210), (160, 218), (160, 222), (172, 248), (190, 274), (190, 283), (190, 290)], Terrain::Road);
    connect_winding_road(&mut map, &[(190, 290), (190, 297), (190, 302), (180, 320)], Terrain::Road);
    connect_winding_road(&mut map, &[(180, 320), (132, 314), (80, 300)], Terrain::Road);
    connect_winding_road(&mut map, &[(80, 300), (42, 276), (0, 240)], Terrain::Road);
    connect_winding_road(&mut map, &[(110, 108), (118, 98), (148, 68), (180, 40)], Terrain::Road);
    connect_winding_road(&mut map, &[(180, 40), (225, 86), (275, 146), (320, 200)], Terrain::Road);
    connect_winding_road(&mut map, &[(160, 202), (168, 196), (218, 192), (272, 196), (320, 200)], Terrain::Road);
    map
}

/// 根據座標將野外格子歸入正典 19 獵場區域（委派予 `classify_wilderness_cell`）。
#[allow(dead_code)]
fn classify_wilderness_zone(coord: SquareCoord) -> (&'static str, &'static str) {
    let (zone, sub_geo, _) = classify_wilderness_cell(coord.x, coord.y);
    (zone, sub_geo)
}

// ─── 15 座聚落完整鋪設（共 405 棟建築原尺寸攤平）──────────────────────

type SettlementCellMap = HashMap<SquareCoord, GridCell>;

fn generate_all_settlements() -> SettlementCellMap {
    let mut map = SettlementCellMap::new();

    // ── 2 城（各 55 棟建築）─────────────────────────────────────────
    generate_city_wen_yao(&mut map);
    generate_city_feng_bo_jiao(&mut map);

    // ── 4 鎮（各 40 棟建築：10F + 30R）─────────────────────────────
    generate_town_feng_bo_yu(&mut map);
    generate_town_da_zhun_tou(&mut map);
    generate_town_ban_ling(&mut map);
    generate_town_nan_shao(&mut map);

    // ── 9 村（各 15 棟建築：5F + 10R）──────────────────────────────
    generate_village_lao_yan_tou(&mut map);
    generate_village_bei_ling_jiao(&mut map);
    generate_village_shuang_lin_kou(&mut map);
    generate_village_feng_gui_wei(&mut map);
    generate_village_wu_shi_bi(&mut map);
    generate_village_niu_pu(&mut map);
    generate_village_hong_tu_kan(&mut map);
    generate_village_san_hao_keng_kou(&mut map);
    generate_village_shui_wei_liao(&mut map);

    map
}

// ─────────────────────────────────────────────────────────────────
// 城鎮村具體佈局產生器
// ─────────────────────────────────────────────────────────────────

/// 組合聚落名稱與場所/建築名稱，徹底消除任何「聚落・聚落...」雙重前綴
fn format_settlement_cell_name(settlement_name: &str, raw_name: &str) -> String {
    let mut s = raw_name;
    while let Some(rest) = s.strip_prefix(settlement_name) {
        s = rest.strip_prefix('・').unwrap_or(rest);
    }
    format!("{}・{}", settlement_name, s)
}

/// 依據建築名稱與類型產生自然、具生活煙火氣之場所描述
fn generate_settlement_building_desc(settlement_name: &str, bldg_name: &str, terrain: Terrain) -> String {
    if bldg_name.contains("三水總商行") {
        format!("{settlement_name}全島通商貿易與鎂幣匯兌調度的總商行總部，高敞的匯兌大廳內計數板與鋼印聲清脆，櫃檯陳列著防偽鋼印、通行鎂幣存摺與跨鎮通兌匯票，秩序井然。")
    } else if bldg_name.contains("傳訊鐘樓") {
        format!("{settlement_name}高聳厚重的傳訊鐘樓與信使總部，頂層懸掛報時青銅巨鐘、日晷與圭表，正午子夜鳴鐘校時；樓前疾行草獸快騎隨時備鞍出發，將緊急手信與公文發往全島。")
    } else if bldg_name.contains("學院主校區") || bldg_name.contains("開筆大講堂") || (bldg_name.contains("文曜學院") && !bldg_name.contains("觀星台")) {
        format!("{settlement_name}學院的核心開筆講堂殿堂，高大的杉木書架整齊排列，講堂前方的烏木教案上擱著黃銅硬筆與講義，及齡學生在此行開筆之禮、受贈鋼尖蘸水硬筆與記事手札。")
    } else if bldg_name.contains("曆算講堂") {
        format!("{settlement_name}講授天文曆算與星度推步之講堂，室內立著黃銅渾天星軌推步儀，前方烏木教案上擺著黃銅硬筆與推步講義，學生在此撥動黑檀計數板核對時序。")
    } else if bldg_name.contains("四時星曆") {
        format!("{settlement_name}協助觀星台推算二十四節氣與編訂歲時星曆的曆算學者精舍，庭中設青石日晷與銅壺滴漏，案頭整齊疊放著歷年節氣交時實測表冊。")
    } else if bldg_name.contains("觀星台") || terrain == Terrain::Observatory {
        format!("{settlement_name}開闊高敞的學院觀星台，青石度盤上立著精工鍛造的三百六十星軌銅儀，金光隱現，用於測算及齡學生的本命氣息與星度交角，內設評議議事廳。")
    } else if bldg_name.contains("藏書別院") {
        format!("{settlement_name}講堂街收藏各城鎮方志、水利輿圖與前朝善本之藏書別院，樟木書櫥間懸掛花椒防蛀香囊，守書人每日細心巡查通風。")
    } else if bldg_name.contains("書局與印行館") {
        format!("{settlement_name}講堂街展售新印書籍與學子文具之印行書局，博古木架陳列著新刊算學講本、白話故事小冊與竹綿紙手札，櫃前備有鋼尖蘸水硬筆與松煙墨錠供師生選購。")
    } else if bldg_name.contains("藏書閣") || bldg_name.contains("印行館") {
        format!("{settlement_name}典藏古卷與編纂地方誌的印行書閣，高聳書架間陳列著《白塔風物誌》與歷期《半島旬報》，長案上整齊擺放著活字木框、拓碑微雕筆與校勘底本。")
    } else if bldg_name.contains("商算") || bldg_name.contains("算學舍") || bldg_name.contains("商務會計") {
        format!("{settlement_name}傳習籌算與帳冊的講舍，四壁架上整齊陳列著各地行商貨價公道帳與會計圖表，案几上擺放著烏木便攜計數板與核算圖表。")
    } else if (bldg_name.contains("診所") || terrain == Terrain::Clinic) && !bldg_name.contains("分家巷") && !bldg_name.contains("醫館") {
        format!("{settlement_name}幽靜清涼的脈絡診所，診案上整齊陳列著青鹽薄荷潔牙粉、黑沼水苔止血藥與百草溫灸盒，紫銅火爐溫著熱水，滿室草木清香。")
    } else if bldg_name.contains("剪剃") || bldg_name.contains("理髮") {
        format!("{settlement_name}講堂街的老字號理容鋪，設有紫銅溫水管木躺椅，木架上陳列著精鋼摺疊剃刀、磨刀布與乾淨的蜂格棉毛巾，街坊常在此閒話家常。")
    } else if bldg_name.contains("第一鍛造") || (bldg_name.contains("岑氏") && bldg_name.contains("鍛造")) {
        format!("{settlement_name}爐火通紅的岑氏第一鍛造工坊，千度火爐烈焰灼灼，巨大的鑄鐵砧與風箱井然排列，架上懸掛著新出爐的耐磨獵具、農具與各色鍛錘。")
    } else if bldg_name.contains("東橋旅社") {
        format!("{settlement_name}臨水三層的東橋旅社與茶屋，一樓茶座散發著烘青茶與糕點香氣，窗邊擺放著烏木白貝圍棋盤，樓上客房配備夜光貝母調光台燈，安詳恬適。")
    } else if bldg_name.contains("巧木居所") || (bldg_name.contains("巧木") && bldg_name.contains("傢俱")) {
        format!("{settlement_name}散發著老雲杉天然樹脂芬芳的木工坊，工作台上陳列著導能刨刀、角尺與各式雕花鑿，四處整齊堆放著防蟲松香木大床與寒水玉保鮮石櫃的半成品。")
    } else if bldg_name.contains("雲錦") {
        format!("{settlement_name}布匹織造與成衣名坊，高大的織布機與裁剪案几整齊陳列，架上展示著塗布防水樹膠的防雨長衫與五色絲線刺繡的精美霞帔。")
    } else if bldg_name.contains("習武堂") || bldg_name.contains("試招") {
        format!("{settlement_name}分家巷剛勁樸實的練武院落，四角立著包革木人樁與沙袋，兵器架上橫陳著試招木刀、護腕與防身革甲。")
    } else if bldg_name.contains("診所") || bldg_name.contains("醫館") {
        format!("{settlement_name}分家巷石砌診所，藥櫃陳列著祛寒薑黃艾草油膏與山風草安神茶包，紫銅火爐溫著熱水，滿室草木清香。")
    } else if bldg_name.contains("烘藥房") || bldg_name.contains("百草") {
        format!("{settlement_name}分家巷溫氏專司焙製草藥之工坊，紅土炭火石灶烘烤著自北嶺採集的新鮮七星草與祛寒艾絨，藥香醇厚四溢。")
    } else if bldg_name.contains("契書局") || bldg_name.contains("房地契") {
        format!("{settlement_name}公議堂側翼專司房地產權、鋪面租賃與田畝地界核驗之契書局，鐵木檔案櫃中整齊封存著歷年紅契正本、街坊地籍輿圖與官印火漆。")
    } else if bldg_name.contains("公議堂") && !bldg_name.contains("前街") {
        format!("{settlement_name}巍峨莊嚴的公議堂石殿，正階開闊，堂內掌管全島戶籍登譜、里坊調解與民事公議，是全島規矩與誠信之基石。")
    } else if bldg_name.contains("西門車馬聯運總站") || (bldg_name.contains("西門") && bldg_name.contains("車馬")) {
        format!("{settlement_name}西門車馬與駝獸聯運總部，開闊的車馬埕上停駐著整裝待發的雙輪駝獸大車，調度案前整齊收納著通往全島各鎮的路引票單。")
    } else if bldg_name.contains("韓氏聯運") || bldg_name.contains("貨運行") {
        format!("{settlement_name}聯運貨物轉運分流行，庫房堆滿自牛埔、大圳頭運抵的重貨箱與乾草捆，店員們過磅核單，井然有序。")
    } else if bldg_name.contains("修傘") || bldg_name.contains("補靴") {
        format!("{settlement_name}西門街角的老手藝攤鋪，老匠人手腳俐落地削竹更換油布晴雨傘骨，以粗蠟線細密縫補防浸皮靴，手藝紮實耐用。")
    } else if bldg_name.contains("弄堂") || bldg_name.contains("天井") {
        format!("{settlement_name}三水口青石石板鋪就的弄堂天井與公井，高搭韌竹晾衣架翻曬被褥，清泉從石槽湧出，街坊鄰里清晨常在此汲水寒暄。")
    } else if bldg_name.contains("海味") || bldg_name.contains("乾貨") {
        format!("{settlement_name}三水口臨河的乾貨鋪子，吊掛著成串的鹽烤海苔片、酸甜山果乾、壺底黑豆醬油與紅土崁黑糖，海味與醬香撲鼻。")
    } else if bldg_name.contains("蒙學") || bldg_name.contains("講舍") {
        format!("{settlement_name}學院啟蒙蒙學學童的講舍，几案低矮平整，壁上懸掛著晨課雲板與千字文掛軸，案頭整齊擺放著沙盤與習字木筆。")
    } else if bldg_name.contains("陸家宅邸") {
        format!("{settlement_name}三水口最具威望的陸家老宅，古樸青石大門刻著通商信條，庭院中老藤攀架，陸氏長輩常在藤椅上品茗聽泉。")
    } else if bldg_name.contains("陸氏清雅書齋") {
        format!("{settlement_name}屋主清雅的私人書齋，案上整齊陳列著四方採風卷帙、拓印石碑與微型雕刻筆，墨香與竹綿紙香滿室飄散。")
    } else if bldg_name.contains("溫家藥圃居") || bldg_name.contains("溫家宅院") {
        format!("{settlement_name}講堂街溫氏宗族石屋，後院整齊分畦栽種著清心薄荷、水苔與祛寒草木，屋簷下掛滿晾曬風乾的藥草束。")
    } else if bldg_name.contains("秦氏算學居") {
        format!("{settlement_name}商算名家素雅的私人居所，書齋四壁皆是各地行商貨價公道帳與會計圖表，案上計數板溫潤潔淨。")
    } else if bldg_name.contains("岑氏匠師宗宅") || bldg_name.contains("岑家大房") {
        format!("{settlement_name}分家巷岑家大房老宅，條石深厚，門前放著鍛打石砧與整齊柴火，屋內陳列著歷代匠人改良的導能農具與火磚樣品。")
    } else if bldg_name.contains("柯氏木作居") {
        format!("{settlement_name}木作匠人的雅致居所，玄關擺放著齒輪平直畫線儀與各式夾鋼榫卯刨刀，桌椅門窗無不榫卯嚴整、天然生潤。")
    } else if bldg_name.contains("蘇氏繡坊居") {
        format!("{settlement_name}繡坊女主人的生活雅居，窗明几淨，繃架上繃著正待完工的精緻錦繡霞帔，針線盒內五色絲線流光溢彩。")
    } else if bldg_name.contains("韓氏總站宅院") || bldg_name.contains("韓家宅院") {
        format!("{settlement_name}聯運站管事的樸實宅邸，院角整齊碼放著修理大車的厚木輪轂與革套，整潔寬敞。")
    } else if bldg_name.contains("陳氏庖廚居") {
        format!("{settlement_name}大食堂掌勺師傅的家宅，灶上常年溫著砂鍋高湯，案頭陳列著數十種香料瓷罐，充滿濃郁敦實的生活煙火氣。")
    } else if bldg_name.contains("沈氏臨水居") {
        format!("{settlement_name}東橋畔的雅致臨水小樓，推窗即見三水碧波，案上置黑沼烏木與風舶白貝圍棋盤，茶香伴隨水聲寧靜致遠。")
    } else if bldg_name.contains("白氏湯屋居") {
        format!("{settlement_name}浴場主管的溫暖住家，引溫泉餘熱鋪地，滿室乾燥溫潤，簷下常曬著洗淨烘乾的厚長絨蜂格棉面巾。")
    } else if bldg_name.contains("岑氏鍛藝居") {
        format!("{settlement_name}分家巷青年匠師的研藝居所，室內擺放著拆解的水栓閥芯與新式器物圖樣，充滿探索鑽研的朝氣。")
    } else if bldg_name.contains("岑氏武勇舍") {
        format!("{settlement_name}習武堂武師的質樸居室，牆上橫掛著兩柄包革試招木刀與護腕，窗前立著石鎖，剛健整潔。")
    } else if bldg_name.contains("老水夫舍") {
        format!("{settlement_name}三水口河畔老船工的住屋，屋簷下掛著乾燥魚網與巨藻船纜，屋內擺著竹編火籠與防浪竹笠。")
    } else if bldg_name.contains("票號總管居") {
        format!("{settlement_name}三水總商行帳房主管的住宅，書案上放著公道銅尺與鋼印泥盒，門戶端整嚴密。")
    } else if bldg_name.contains("信使快騎宿所") {
        format!("{settlement_name}傳訊鐘樓急件信使休宿院，備有烘乾皮靴架與厚帆布防風長衫，常有信使在此換裝整裝。")
    } else if bldg_name.contains("晨汲青石宅") {
        format!("{settlement_name}緊鄰三水口公井的街坊石屋，門口青石被歲月磨得溫潤發亮，晨昏汲水洗衣聲不絕於耳。")
    } else if bldg_name.contains("河畔迎客小院") {
        format!("{settlement_name}三水口迎候四方商客的臨水小院，窗臨河道，花木扶疏，常飄出清淡的迎客茶香。")
    } else if bldg_name.contains("蒙學陸先生舍") {
        format!("{settlement_name}學院蒙學先生的雅室，案上整齊碼放著《白塔基礎讀本》，壁上懸掛晨課雲板。")
    } else if bldg_name.contains("抄經校勘齋") {
        format!("{settlement_name}印行館校勘工匠居處，桌前擺著弧形吸墨器、金箔紙與細毛刷，滿室皆是墨香。")
    } else if bldg_name.contains("石硯墨香居") {
        format!("{settlement_name}講堂街專事研墨製硯工匠之家，庭中蓄水洗硯，案上陳列著各色黑松脂墨與細石硯台。")
    } else if bldg_name.contains("精工火磚匠廬") {
        format!("{settlement_name}專造導能耐火磚的匠人居所，後院堆著各色黏土與磚坯樣板，磚香撲鼻。")
    } else if bldg_name.contains("金石榫卯木舍") {
        format!("{settlement_name}分家巷木工與五金學徒的合宿之所，工具整齊懸於木架，少年匠人在此切磋手藝。")
    } else if bldg_name.contains("駝獸牧夫宿所") {
        format!("{settlement_name}西門聯運駝獸牧伕的住舍，屋外懸掛著大鬃毛刷與草料韁繩，溫暖厚實。")
    } else if bldg_name.contains("遠行行商棧舍") {
        format!("{settlement_name}隨車押貨行商的歇腳居舍，行囊整齊疊放在通鋪旁，旅客在此互通各地物產市況。")
    } else if bldg_name.contains("大車修理工寮") {
        format!("{settlement_name}維修雙輪重型貨車的工匠宿所，屋前散落著堅韌雲杉軸心與生鐵輪箍，鐵器鏗鏘。")
    } else if bldg_name.contains("皮靴油布匠居") {
        format!("{settlement_name}修傘補靴老匠人的居所，廊下掛著浸透防雨桐油的竹骨雨傘，案頭擺滿蠟線與裁皮刀。")
    } else if bldg_name.contains("行道車馬行舍") {
        format!("{settlement_name}西門車隊管事居所，門口停著調度輕便板車，東家／主事正對著行路簿盤點日程。")
    } else if bldg_name.contains("聯運公秤舍") {
        format!("{settlement_name}西門過磅公秤所，立著大生鐵地秤與砝碼，為進出城大宗貨物校準分量。")
    } else if bldg_name.contains("草料客商居") {
        format!("{settlement_name}販售甜苜蓿草料行商的屋舍，堆疊如山的青綠乾草捆散發著濃郁田野清香。")
    } else if bldg_name.contains("大食堂") || bldg_name.contains("食堂") {
        format!("{settlement_name}熱鬧的食堂，嵌著蓄熱重礦底座的紅鐵大釜正冒著熱氣，早上有現磨溫米漿、淋黑糖漿的熱豆花、蔥香米烙餅與牛埔鮮肉包，午晚則有慢燉原牛肉湯、紫銅鴛鴦火鍋與切仔湯麵配滷蛋，蒸籠裡常備著熱氣騰騰的桂花新米糕與黑芝麻湯圓；桌邊擺著壺底黑豆醬油與白釉瓷碗，牆角寒水玉保鮮石櫃裡冰鎮著豆腐、鮮乳與寒水玉冰鎮仙草凍。")
    } else if bldg_name.contains("浴場") || bldg_name.contains("浴堂") || bldg_name.contains("石湯") || terrain == Terrain::Bathhouse {
        format!("{settlement_name}引流山泉活水的溫水浴場，池底鋪著溫潤的寒水玉、池壁嵌有恆溫導能火磚，水裡飄著祛寒草木湯包的香氣；盥洗台亮銀明水鏡前擺滿了青鹽薄荷潔牙粉、軟毛竹柄潔齒刷、山茶松脂香胰、草木灰柑橘洗潔皂與柔軟的蜂格棉面巾，牆邊立著烘乾衣物的松炭溫風烘衣櫥。")
    } else if bldg_name.contains("服飾") || bldg_name.contains("織造") {
        format!("{settlement_name}陳列各色衣著與隨身小物的服飾坊，架上掛著細棉麻透氣短衫、北山雪狼絨禦寒冬衣與塗了黑沼樹膠的防雨長衫，櫃台還擺著雙扣帆布斜背包、軟牛皮零錢夾、隨手五色針線盒、軟底居家布便鞋與竹骨晴雨摺疊傘。")
    } else if bldg_name.contains("傢俱") || bldg_name.contains("巧木") {
        format!("{settlement_name}飄著淡淡松木清香的傢俱工坊，展示著繃好彈力床網的松香木大床、衣櫃、鑲貝書案與內嵌北山寒水玉薄板的保鮮石櫃，架上還擺著旋蓋夜光貝母調光台燈、沉水烏木茶盤、烏木筷與松香木梳。")
    } else if bldg_name.contains("藏書閣") || bldg_name.contains("圖誌館") || bldg_name.contains("書局") || terrain == Terrain::Library {
        format!("{settlement_name}收納古卷、地方風物誌與山海圖冊的書局藏書處，架上擺著竹綿紙印製的《白塔風物誌》、每旬發行的《半島旬報》、白話故事小冊，桌邊還備有竹綿紙記事手札與鋼尖蘸水硬筆。")
    } else if bldg_name.contains("鍛造") || bldg_name.contains("鐵器") || terrain == Terrain::Blacksmith {
        format!("{settlement_name}爐火通紅的鍛造工坊，除了打製獵具與農具，架上也擺滿了家家戶戶必備的黃銅水栓、精鋼摺疊剃刀、防潮紅磷火籤、輕便隨身保溫瓶內膽與小巧的黃銅發條座鐘、懷錶。")
    } else if bldg_name.contains("商行") || bldg_name.contains("商盟") || terrain == Terrain::Bank {
        format!("{settlement_name}負責大宗物資買賣、海事保單與帳契結算的商行，廳堂寬敞明亮，掌事人員正忙著核對貨單、兌付軟牛皮錢夾裡的商行票據與調度各地來貨。")
    } else if bldg_name.contains("車馬聯運") || bldg_name.contains("聯運") || terrain == Terrain::Caravanserai {
        format!("{settlement_name}往來各城鎮的車馬與駝獸聯運站，棚下停著載貨大車與健壯的原牛駝獸，旅客背著雙扣帆布斜背包、提著竹骨晴雨摺疊傘與隨身保溫瓶上下車。")
    } else if bldg_name.contains("旅社") || terrain == Terrain::Inn {
        format!("{settlement_name}乾淨舒適的旅社，一樓茶屋飄著鍋煮鮮奶茶、冰鎮甘蔗檸檬汁、奶油酥餅、黑糖米香與桂花新米糕的甜香，茶客正對弈烏木白貝圍棋、翻看《半島旬報》；樓上獨立銅鎖單人客房鋪著蓬鬆羽絨暖被與山風草安神枕，桌頭立著旋蓋夜光貝母台燈。")
    // ── 區域聚落專屬特色建築描述（針對 14 個非文曜聚落） ──
    } else if bldg_name.contains("風櫃尾燈塔") {
        format!("{settlement_name}高聳巍峨的防風海岬燈塔，塔頂安裝著精工鍛造的長明海燈與聚光銅鏡，強烈光束穿透夜霧，為往來風舶海域的舟楫指引平安航道。")
    } else if bldg_name.contains("冷燻") {
        format!("{settlement_name}海風強勁的水產冷燻工坊，室內整齊掛滿深海大魚與石斑，匠人以乾燥松屑燃起微煙緩慢冷燻，魚香撲鼻，乾爽耐久。")
    } else if bldg_name.contains("鑿石") || (bldg_name.contains("玄武岩") && bldg_name.contains("工坊")) {
        format!("{settlement_name}沉重剛毅的玄武岩採鑿石坊，匠人們揮動大鋼鎬開鑿烏石重錨與防波堤石料，火星迸濺，石屑紛飛。")
    } else if bldg_name.contains("灰窯") || bldg_name.contains("海貝灰") {
        format!("{settlement_name}以巨大海貝殼高溫燒製防水灰漿的灰窯，熱氣騰騰，出產的灰漿堅密防滲，為港埠防波堤與城牆不可或缺之建材。")
    } else if bldg_name.contains("石磨工坊") || bldg_name.contains("水輪石磨") {
        format!("{settlement_name}引流老堰頭急湍活水驅動的巨大石磨工坊，水輪轟鳴旋轉，成排石磨精細碾磨新收稻穀、大豆與乾燥藥材，米香與豆香洋溢。")
    } else if bldg_name.contains("鋸木") || bldg_name.contains("水車鋸木") {
        format!("{settlement_name}高山急流帶動巨大排鋸的原木鋸木坊，巨大的雲杉巨木在水力牽引下被平整剖開，清新的松木樹脂芬芳飄散在整座山谷。")
    } else if bldg_name.contains("切石") || bldg_name.contains("寒水玉切石") {
        format!("{settlement_name}利用山泉落差驅動銀砂水力切石機的工坊，高速運轉的水輪將堅韌的寒水玉原石平整剖切，打磨出溫潤潔淨的玉石磚板與隨身握石。")
    } else if bldg_name.contains("氣象觀測") || bldg_name.contains("測風儀") {
        format!("{settlement_name}高踞岩台的氣象觀測所，青銅測風儀在山風中飛速旋轉，觀測師細緻記錄風速、雲層與氣壓，隨時發送高山風雪預警。")
    } else if bldg_name.contains("農牧總社") {
        format!("{settlement_name}掌管大圳頭廣袤農牧拓墾之調度中心，開闊大廳掛著各區田畝水利圖冊與草場輪牧日程，農人在此協調農具與肥料分配。")
    } else if bldg_name.contains("乳酪") || bldg_name.contains("羊毛與乳酪") {
        format!("{settlement_name}向陽草甸旁的乳品精製坊，巨大的蓄熱紅鐵鍋慢火濃縮原牛乳汁，木架上整齊陳列著金黃硬實的熟成乳酪塊與壓製厚毛氈。")
    } else if bldg_name.contains("選礦") || bldg_name.contains("選礦場") {
        format!("{settlement_name}階地旁的重型選礦場，坡道上流水沖洗各色礦石，工人們以鐵槌敲選純淨赤紋鐵礦與伴生銅砂，分級裝入大車運往工坊。")
    } else if bldg_name.contains("坑口") || bldg_name.contains("礦務所") || bldg_name.contains("台車維修") {
        format!("{settlement_name}深山礦谷咽喉的礦務作業所與台車棚，重型鋼軌與手搖絞盤整齊架設，通風設備呼呼作響，坑工們在此檢點礦燈與安全支柱。")
    } else if bldg_name.contains("高腳木屋") || bldg_name.contains("竹筏埠") {
        format!("{settlement_name}以粗壯烏木打樁避開積水與泥甲鱷獸的架高聚落，平底竹筏繫於木樁旁，通風涼爽，水草與魚鮮香氣撲面。")
    } else if bldg_name.contains("公所") || bldg_name.contains("公事房") || bldg_name.contains("公議所") || bldg_name.contains("公議廳") || terrain == Terrain::TownHall {
        format!("{settlement_name}凝聚全鄉鎮民心與事務決策之公所，條石樸素堅實，堂內陳列地方墾拓沿革圖與互助規約，居民在此商議公共事務。")
    } else if bldg_name.contains("宅邸") || bldg_name.contains("宅院") || bldg_name.contains("莊園") || bldg_name.contains("別院") || bldg_name.contains("老宅") {
        match settlement_name {
            "風舶角" | "風舶嶼" => format!("{settlement_name}歷代經營海事與造船的世家宅院「{bldg_name}」，以厚重防浪青石與百年雲杉樑柱築成，廳內懸掛著遠洋海圖與夜光羅盤，庭院乾爽開闊。"),
            "大圳頭" | "老堰頭" => format!("{settlement_name}世代主持水利與農耕的宗族大宅「{bldg_name}」，院前活水圳渠清澈流淌，廊下陳列著精良水車構件與翻土重犁，充滿豐饒踏實的田園家風。"),
            "半嶺鎮" | "北嶺腳" | "霜林口" => format!("{settlement_name}依山勢修築的匠師世家老宅「{bldg_name}」，石牆厚實防風，屋內引山泉活水並砌有暖炕，案頭擺放著寒水玉器與山林鍛件。"),
            "南哨" | "水尾寮" => format!("{settlement_name}以千年不腐的沼生烏木與條石高架築成的名門宅院「{bldg_name}」，簷下懸掛著避瘴香草束與精鋼獵具，通風防潮、守望相助。"),
            "牛埔" => format!("{settlement_name}向陽草甸上的牧業世家大院「{bldg_name}」，石砌圍牆內設有恆溫乳酪熟成窖與寬敞駝獸棚，微風送來陣陣牧草與乳香。"),
            "紅土崁" | "三號坑口" => format!("{settlement_name}以千度耐火紅磚與堅岩砌成的礦業宗族宅院「{bldg_name}」，庭中陳列著各色赤紋礦石標本與精良選礦工具，門風剛健敦厚。"),
            _ => format!("{settlement_name}深厚質樸的在地家族宅院「{bldg_name}」，建材質料取自周邊山海山石，庭院開闊，生活陳設紮實耐用，洋溢著經年累月的家風溫情。"),
        }
    } else if bldg_name.contains("民宅") || bldg_name.contains("住宅") || bldg_name.contains("公寓") || bldg_name.contains("街屋") || bldg_name.contains("居所") || bldg_name.contains("家屋") || bldg_name.contains("農舍") || bldg_name.contains("宿舍") || terrain == Terrain::House {
        match settlement_name {
            "文曜" => format!("{settlement_name}街區整潔雅致的居民屋舍「{bldg_name}」，青石牆基配著明亮窗櫺，廚房引流山泉活水，桌上點著旋蓋夜光貝母台燈，生活井然安適。"),
            "風舶角" | "風舶嶼" => format!("{settlement_name}臨港防風的石砌海事屋舍「{bldg_name}」，以海貝灰漿封縫抵禦海潮鹽霧，屋簷下掛著防風提燈與巨藻纜繩，屋內乾爽溫暖。"),
            "風櫃尾" | "烏石鼻" => format!("{settlement_name}依海岬岩岸用黑曜玄武岩壘砌的防浪漁家「{bldg_name}」，厚重石牆能擋狂風巨浪，院裡晾曬著漁網與冷燻魚乾，屋內火塘暖意融融。"),
            "大圳頭" | "老堰頭" => format!("{settlement_name}傍著清澈水圳而建的農家屋舍「{bldg_name}」，門前石階可汲引活水洗菜淘米，院角整齊碼放著脫穀木桶與改良農具，飄散著新米與炊煙香氣。"),
            "半嶺鎮" | "北嶺腳" => format!("{settlement_name}順應陡峭山坡修築的石木山居「{bldg_name}」，屋頂壓著防風青石板，廊下備有登山鐵齒釘鞋與背架，室內松脂暖爐烘得滿屋生春。"),
            "霜林口" => format!("{settlement_name}以粗壯北坡雲杉原木榫接而成的林工木屋「{bldg_name}」，空氣中瀰漫著濃郁松脂木香，屋內鋪著厚實防寒毛氈，壁爐柴火霹啪作響。"),
            "南哨" => format!("{settlement_name}高棧坪上以硬木與山石架高的防禦屋舍「{bldg_name}」，門窗裝有防蚊防瘴細紗與堅固木栓，牆上掛著入林開山刀、皮靴與紅磷信號筒。"),
            "水尾寮" => format!("{settlement_name}以耐水烏木高椿架於淺澤之上的高腳木屋「{bldg_name}」，屋下停靠著輕便平底竹筏，窗前掛著蘆葦編簾與止血水苔乾束，涼爽通風。"),
            "牛埔" => format!("{settlement_name}長風草甸向陽坡上的石基厚氈暖屋「{bldg_name}」，屋頂覆著防風壓石，屋內擺著發酵乳酪木桶與厚實羊毛毯，推窗即見無邊青翠草浪。"),
            "紅土崁" | "三號坑口" => format!("{settlement_name}工礦階地上的耐火紅磚與條石屋舍「{bldg_name}」，門前設有沖洗礦泥的石槽與防風礦燈架，屋內火炕溫熱，陳設結實耐用。"),
            _ => format!("{settlement_name}尋常人家的起居居所「{bldg_name}」，依地勢就地取材建造，門口備有防雨竹編信箱與工具架，屋內生著暖爐，陳設質樸安寧。"),
        }
    } else if bldg_name.contains("大船塢") || bldg_name.contains("船材") || bldg_name.contains("造船") || bldg_name.contains("修船") {
        format!("{settlement_name}臨水而建的造船塢，架上停著正在拼合百年雲杉龍骨、絞掛深海巨藻船纜與烏石重錨的大船，空氣裡滿是杉木與桐油的氣味。")
    } else if bldg_name.contains("鐘樓") || bldg_name.contains("信使") || bldg_name.contains("瞭望") || terrain == Terrain::GuardPost {
        format!("{settlement_name}負責報時、守望與收發各地書信的鐘樓信使站，疾行草獸快騎背著防雨郵袋準時發班，居民也常來此校對隨身黃銅懷錶的時刻。")
    } else if bldg_name.contains("學院") || terrain == Terrain::Academy {
        format!("{settlement_name}講授曆算、字韻與百工學問的學院，學生們背著雙扣帆布斜背包、手捧竹綿紙記事手札與鋼尖蘸水硬筆，在綠蔭庭院間切磋討論。")
    } else if bldg_name.contains("公會") || terrain == Terrain::GuildHall {
        format!("{settlement_name}同行聚會、交換山海情報與張貼差事的公會大廳，桌邊坐滿了經驗老到的前輩與喝著隨身保溫瓶熱茶的年輕人。")
    } else if bldg_name.contains("碼頭") || bldg_name.contains("泊位") || bldg_name.contains("竹筏埠") || terrain == Terrain::Dock {
        format!("{settlement_name}船隻與竹筏停靠的碼頭，水手與漁家熟練地拋出巨藻船纜繫船，將新捕獲的鮮魚、鹽烤海苔與外地貨物搬上岸。")
    } else if bldg_name.contains("鋪") || bldg_name.contains("行") || bldg_name.contains("店") || terrain == Terrain::Market {
        format!("{settlement_name}熱鬧的市集鋪子，攤上擺滿了壺底黑豆醬油、紅土崁黑糖、牛埔硬乳酪、黑糖米香、鹽烤海苔片、酸甜山果乾與各色家用小物，買賣聲此起彼落。")
    } else {
        format!("座落於{settlement_name}的{bldg_name}，屋舍整潔，人來人往。")
    }
}

