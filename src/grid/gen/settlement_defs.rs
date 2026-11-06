// ─────────────────────────────────────────────────────────────────
// 具體聚落建築陣列配置
// ─────────────────────────────────────────────────────────────────
fn generate_city_wen_yao(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(55);

    // ── 西北象限：三水口商業與生活核心（共 14 棟）──
    b.push((Terrain::Bank, "三水總商行總部"));
    b.push((Terrain::GuardPost, "傳訊鐘樓與信使總站"));
    b.push((Terrain::Tavern, "三水大食堂"));
    b.push((Terrain::Bathhouse, "泉香溫水浴場"));
    b.push((Terrain::Market, "三水口乾貨海味行"));
    b.push((Terrain::House, "三水口・陸家宅邸"));
    b.push((Terrain::House, "三水口・石板弄堂天井"));
    b.push((Terrain::House, "三水口・陳氏庖廚居"));
    b.push((Terrain::House, "三水口・白氏湯屋居"));
    b.push((Terrain::House, "三水口・老水夫舍"));
    b.push((Terrain::House, "三水口・票號總管居"));
    b.push((Terrain::House, "三水口・信使快騎宿所"));
    b.push((Terrain::House, "三水口・晨汲青石宅"));
    b.push((Terrain::House, "三水口・河畔迎客小院"));

    // ── 東北象限：講堂街文教、出版與醫療街區（共 14 棟）──
    b.push((Terrain::Academy, "學院主校區"));
    b.push((Terrain::Observatory, "學院觀星台與評議廳"));
    b.push((Terrain::Library, "藏書閣與印行館"));
    b.push((Terrain::Academy, "百工商算學舍"));
    b.push((Terrain::Library, "講堂街書局與印行館"));
    b.push((Terrain::GeneralStore, "講堂街老字號剪剃鋪"));
    b.push((Terrain::Library, "講堂街書局"));
    b.push((Terrain::Academy, "文曜蒙學堂講舍"));
    b.push((Terrain::House, "講堂街・陸氏清雅書齋"));
    b.push((Terrain::House, "講堂街・溫家藥圃居"));
    b.push((Terrain::House, "講堂街・秦氏算學居"));
    b.push((Terrain::House, "講堂街・蒙學陸先生舍"));
    b.push((Terrain::House, "講堂街・抄經校勘齋"));
    b.push((Terrain::House, "講堂街・石硯墨香居"));
    // ── 東南象限：分家巷百工製造與旅社街區（共 14 棟）──
    b.push((Terrain::Blacksmith, "岑氏第一鍛造工坊"));
    b.push((Terrain::Inn, "東橋旅社與山風茶屋"));
    b.push((Terrain::Workshop, "巧木居所傢俱坊"));
    b.push((Terrain::GeneralStore, "雲錦織造與服飾坊"));
    b.push((Terrain::Clinic, "溫氏脈絡診所"));
    b.push((Terrain::Workshop, "溫氏百草烘藥房"));
    b.push((Terrain::Workshop, "分家巷習武堂試招院"));
    b.push((Terrain::GeneralStore, "分家巷口雜貨柑仔店"));
    b.push((Terrain::House, "分家巷・岑氏匠師宗宅"));
    b.push((Terrain::House, "分家巷・柯氏木作居"));
    b.push((Terrain::House, "分家巷・蘇氏繡坊居"));
    b.push((Terrain::House, "東橋畔・沈氏臨水居"));
    b.push((Terrain::House, "分家巷・岑氏鍛藝居"));
    b.push((Terrain::House, "分家巷・岑氏武勇舍"));
    // ── 西南象限：西門聯運站、物流與公議堂（共 13 棟）──
    b.push((Terrain::TownHall, "公議堂"));
    b.push((Terrain::Courthouse, "公議堂房地契書局"));
    b.push((Terrain::Caravanserai, "西門車馬聯運總站"));
    b.push((Terrain::Market, "韓氏聯運貨運行"));
    b.push((Terrain::Workshop, "西門皮靴與晴雨傘工坊"));
    b.push((Terrain::House, "西門・韓氏總站宅院"));
    b.push((Terrain::House, "西門・駝獸牧夫宿所"));
    b.push((Terrain::House, "西門・遠行行商棧舍"));
    b.push((Terrain::House, "西門・大車修理工寮"));
    b.push((Terrain::House, "西門・皮靴油布匠居"));
    b.push((Terrain::House, "西門・行道車馬行舍"));
    b.push((Terrain::House, "西門・聯運公秤舍"));
    b.push((Terrain::House, "西門・草料客商居"));

    build_city_55(map, 0, 0, "文曜", &b);
}

fn generate_city_feng_bo_jiao(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(55);
    // 13 棟海事與生活建築：海事總商行/航海圖誌館/商盟總會/大船塢/帆具船材/診所/食堂/溫水浴場/旅社/車馬聯運/貨運行/碼頭/水產行
    b.push((Terrain::TownHall, "風舶海事商盟總會"));
    b.push((Terrain::Bank, "海事總商行與船貨契據所"));
    b.push((Terrain::Library, "風舶航海圖誌館"));
    b.push((Terrain::Workshop, "林氏大船塢"));
    b.push((Terrain::Workshop, "馭風帆具與船材工坊"));
    b.push((Terrain::Clinic, "港務脈絡診所"));
    b.push((Terrain::Tavern, "海風海鮮食堂"));
    b.push((Terrain::Bathhouse, "港灣溫水浴場"));
    b.push((Terrain::Inn, "海員旅社"));
    b.push((Terrain::Caravanserai, "港尾車馬聯運站"));
    b.push((Terrain::Market, "海陸聯運貨運行"));
    b.push((Terrain::Dock, "深水泊位與卸貨碼頭"));
    b.push((Terrain::Market, "港區魚獲水產交易行"));
    // 2 棟家族宅院
    b.push((Terrain::House, "築港頭・許家宅邸"));
    b.push((Terrain::House, "大船塢・林家宅院"));
    // 40 棟港埠與水手生活街屋民宅
    let feng_bo_jiao_houses = [
        "許氏海運門庭", "北冕號船員宿所", "領航副舵手舍", "大船塢老木工居", "帆纜編繩匠舍",
        "港尾車馬行屋", "海陸聯運帳房居", "泊位碼頭工舍", "海味水產乾貨家", "潮間帶補網小築",
        "林家船匠支系宅", "風舶航海錄校對齋", "海員避風客舍", "防浪厚石排屋一號", "防浪厚石排屋二號",
        "港灣晨汲水井居", "海風眺望石樓", "深水錨工家屋", "船具修配鐵匠居", "帆布油蠟作坊舍",
        "海陸過磅員民舍", "潮音茶憩小舍", "外海探險老舵手居", "築港石匠工寮居", "港尾雜貨管事居",
        "港口更夫直宿舍", "海員家眷安居樓", "遠洋物資存備宅", "避浪石巷居所", "大船塢學徒宿舍",
        "風舶海巡隊員居", "迎浪青石街屋", "碼頭起卸工小院", "港尾修靴防浸屋", "海風麵點鋪居",
        "港務文書書吏舍", "海波平寧安居宅", "防潮倉管家舍", "巨藻纜繩存收院", "風舶角落霞臨海居",
    ];
    for name in feng_bo_jiao_houses {
        b.push((Terrain::House, name));
    }
    build_city_55(map, -260, 160, "風舶角", &b);
}

fn generate_town_feng_bo_yu(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(40);
    // 10F 功能與生活建築
    b.push((Terrain::GuildHall, "外澳領航公會"));
    b.push((Terrain::Inn, "外澳海員旅社"));
    b.push((Terrain::Workshop, "島嶼修船塢"));
    b.push((Terrain::Market, "外澳水產集散行"));
    b.push((Terrain::Clinic, "外澳港務脈絡診所"));
    b.push((Terrain::GuardPost, "外澳海島信使中繼站"));
    b.push((Terrain::Dock, "外澳石砌碼頭"));
    b.push((Terrain::TownHall, "風舶嶼公所"));
    b.push((Terrain::Tavern, "外澳海味食堂"));
    b.push((Terrain::Bathhouse, "潮音溫水浴堂"));
    // 30R 民宅
    let feng_bo_yu_houses = [
        "顧氏領航世家老宅", "外澳夜光羅盤工匠舍", "外澳老舵工石屋", "海員修整歇憩所", "修船塢刮蠣匠工居",
        "水產集散帳房舍", "港務巡查員居室", "潮音石階臨海宅", "外澳漁火守望屋", "外澳石碼頭排屋",
        "外澳海風曬網院", "防浪粗石砌舍一號", "防浪粗石砌舍二號", "採貝漁夫溫馨宅", "海員補給小院",
        "避風港船伕家屋", "深水礁岩釣客居", "外澳信使備鞍舍", "潮聲茶水歇腳舍", "外澳老更夫值班居",
        "島嶼石匠切石宅", "風波平伏安樂舍", "海灣晨曦石屋", "外澳船帆修補居", "深海巨藻編織院",
        "外澳海產乾味舍", "外澳雜貨小鋪居", "海港渡頭巡視居", "外澳修網匠家", "外澳迎潮望海樓",
    ];
    for name in feng_bo_yu_houses {
        b.push((Terrain::House, name));
    }
    build_town_40(map, -330, 160, "風舶嶼", &b);
}

fn generate_town_da_zhun_tou(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(40);
    // 10F 功能與生活建築
    b.push((Terrain::TownHall, "大圳農牧總社"));
    b.push((Terrain::Caravanserai, "大圳頭車馬聯運站"));
    b.push((Terrain::Workshop, "大圳農具與百工坊"));
    b.push((Terrain::Clinic, "大圳頭脈絡診所"));
    b.push((Terrain::Bank, "大圳頭農產行"));
    b.push((Terrain::Inn, "大圳頭商旅社"));
    b.push((Terrain::Market, "大圳頭農產與糧油市集"));
    b.push((Terrain::Tavern, "田園風味大食堂"));
    b.push((Terrain::Bathhouse, "圳水溫和浴堂"));
    b.push((Terrain::House, "大圳頭・周家莊園"));
    // 30R 民宅
    let da_zhun_tou_houses = [
        "田氏農具匠人老宅", "省力翻土犁工坊舍", "水力榨油匠居", "大圳頭農具百工舍", "大圳頭糧油調度員宅",
        "農牧總社管事居", "圳道引水水工舍", "草獸牽引大車伕居", "大圳頭棉麻織造家", "原野收穫儲糧宅",
        "大圳頭田園蔬食居", "拓墾老農舍", "大圳頭甘蔗榨汁小院", "圳水磨坊工匠宅", "農產市集主事居",
        "大圳頭聯運駝獸伕舍", "大圳頭行商會客小舍", "田園風光青石屋", "大圳頭修耙補犁鋪居", "草料集運宅",
        "大圳頭稻香安居舍", "大圳頭育苗育種園居", "大圳頭農耕民家一號", "大圳頭農耕民家二號", "大圳頭農耕民家三號",
        "大圳頭農耕民家四號", "大圳頭農耕民家五號", "大圳頭農耕民家六號", "大圳頭清泉飲水舍", "大圳頭望野樓",
    ];
    for name in da_zhun_tou_houses {
        b.push((Terrain::House, name));
    }
    build_town_40(map, -180, -140, "大圳頭", &b);
}

fn generate_town_ban_ling(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(40);
    // 10F 功能與生活建築
    b.push((Terrain::Arena, "半嶺試招場"));
    b.push((Terrain::Inn, "半嶺山泉旅社"));
    b.push((Terrain::Workshop, "寒水玉切石工坊"));
    b.push((Terrain::Clinic, "半嶺脈絡診所"));
    b.push((Terrain::Market, "半嶺山貨特產行"));
    b.push((Terrain::TownHall, "半嶺鎮公所"));
    b.push((Terrain::Observatory, "半嶺氣象觀測所"));
    b.push((Terrain::Bathhouse, "青脊石湯溫水浴場"));
    b.push((Terrain::Tavern, "半嶺山野茶屋食堂"));
    b.push((Terrain::House, "半嶺鎮・岑家二房別院"));
    // 30R 民宅
    let ban_ling_houses = [
        "石家寒水玉切石匠宅", "袁家半嶺山泉客舍", "高家青銅測風儀觀測居", "半嶺試招武師居", "半嶺山岳急救草藥舍",
        "半嶺山貨收購商舍", "半嶺氣象風雪預警值房", "寒水玉握石拋光齋", "半嶺石湯溫熱居", "雪嶺暖身茶湯鋪居",
        "半嶺登山鐵齒釘鞋作坊舍", "青竹坪山徑嚮導居", "半嶺坡地石砌排屋一號", "半嶺坡地石砌排屋二號", "半嶺坡地石砌排屋三號",
        "半嶺松木柴薪庫居", "半嶺高台聽泉樓", "半嶺山泉汲水石台居", "半嶺山澗釣翁居", "半嶺避雪溫暖石舍",
        "半嶺切石水力水車工舍", "半嶺山野果子乾味舍", "半嶺石壁行商歇腳房", "半嶺林緣巡山衛舍", "半嶺古道老鋪民宅",
        "半嶺青脊礦石辨識齋", "半嶺雲海觀景雅舍", "半嶺山風草曬製院", "半嶺石階清掃工居", "半嶺半山迎客別院",
    ];
    for name in ban_ling_houses {
        b.push((Terrain::House, name));
    }
    build_town_40(map, 160, 210, "半嶺鎮", &b);
}

fn generate_town_nan_shao(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(40);
    // 10F 功能與生活建築
    b.push((Terrain::TownHall, "南哨防務公議廳"));
    b.push((Terrain::GuardPost, "南哨傳訊信使站"));
    b.push((Terrain::Workshop, "重型獵具工坊"));
    b.push((Terrain::Clinic, "南哨脈絡解毒診所"));
    b.push((Terrain::GuildHall, "入林嚮導公會"));
    b.push((Terrain::Inn, "高棧坪獵人旅社"));
    b.push((Terrain::Market, "黑沼素材收購行"));
    b.push((Terrain::Tavern, "南哨原野食堂"));
    b.push((Terrain::Bathhouse, "高棧坪溫水浴場"));
    b.push((Terrain::House, "高棧坪・羅家宅院"));
    // 30R 民宅
    let nan_shao_houses = [
        "雷家重型獵具大匠宅", "葉家白茫避瘴清心齋", "羅家黑沼防務精舍", "南哨鐵絲巨藻獵網工棚舍", "南哨飛爪索精造舍",
        "黑沼紅磷信號筒工匠居", "高棧坪首席嚮導歇宿舍", "南哨防瘴草藥焙製院", "南哨重獵具維護工寮", "南哨獸材分級過磅居",
        "高棧坪高架木石屋一號", "高棧坪高架木石屋二號", "高棧坪高架木石屋三號", "高棧坪高架木石屋四號", "高棧坪高架木石屋五號",
        "南哨防泥浸厚皮靴舍", "南哨隘口值守衛士居", "南哨夜間警備更房", "南哨獵犬與角鹿馴養舍", "高棧坪篝火圍坐大院",
        "黑沼探險老獵夫舍", "南哨驅蚊艾草曬場居", "南哨硬木弓弩修造居", "高棧坪山野乾糧小棧", "南哨解毒藥草培植圃居",
        "高棧坪晨霧瞭望石居", "南哨防濕木料棧舍", "南哨涉水長皮靴手藝居", "高棧坪獵友聚義小院", "南哨黑沼門戶安防石宅",
    ];
    for name in nan_shao_houses {
        b.push((Terrain::House, name));
    }
    build_town_40(map, 0, -220, "南哨", &b);
}

fn generate_village_lao_yan_tou(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(15);
    // 5F 功能建築
    b.push((Terrain::TownHall, "老堰頭農產公議所"));
    b.push((Terrain::Clinic, "老堰頭脈絡草木診所"));
    b.push((Terrain::Workshop, "水輪石磨工坊"));
    b.push((Terrain::Tavern, "堰渠水畔食堂"));
    b.push((Terrain::Inn, "水門茶憩旅社"));
    // 10R 民宅
    let lao_yan_tou_houses = [
        "秦氏水輪石磨傳承老宅", "老堰頭水門守閘水工舍", "堰渠石匠維修工舍", "陸家舊堰農契典藏舍", "水畔碾米磨藥工匠居",
        "老堰頭清流甘泉水豆腐坊舍", "老堰頭水圳巡渠員居", "堰上微風迎客小院", "老堰頭水車木匠家屋", "老堰頭古堰安瀾石屋",
    ];
    for name in lao_yan_tou_houses {
        b.push((Terrain::House, name));
    }
    build_village_15(map, 0, 125, "老堰頭", &b);
}

fn generate_village_bei_ling_jiao(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(15);
    // 5F 功能建築
    b.push((Terrain::Inn, "北嶺腳山林旅社"));
    b.push((Terrain::Clinic, "北嶺腳脈絡急救診所"));
    b.push((Terrain::GeneralStore, "北嶺山徑補給行"));
    b.push((Terrain::Tavern, "嶺腳暖火食堂"));
    b.push((Terrain::TownHall, "北嶺腳公事房"));
    // 10R 民宅
    let bei_ling_jiao_houses = [
        "馬家鐵齒登山釘鞋工坊宅", "硬木藤背架老匠居", "松脂暖手爐銅匠舍", "北嶺山風草採藥人居", "北嶺腳急救醫助居",
        "北嶺腳山道背夫歇腳院", "北嶺腳登山行囊補配鋪舍", "嶺腳坡地石砌家屋一號", "嶺腳坡地石砌家屋二號", "北嶺腳山泉甘洌安居宅",
    ];
    for name in bei_ling_jiao_houses {
        b.push((Terrain::House, name));
    }
    build_village_15(map, 110, 115, "北嶺腳", &b);
}

fn generate_village_shuang_lin_kou(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(15);
    // 5F 功能建築
    b.push((Terrain::GuardPost, "霜林護林瞭望所"));
    b.push((Terrain::Clinic, "霜林脈絡診所"));
    b.push((Terrain::Workshop, "原木水車鋸木坊"));
    b.push((Terrain::Market, "高山木料調配站"));
    b.push((Terrain::Inn, "林緣溫暖旅社"));
    // 10R 民宅
    let shuang_lin_kou_houses = [
        "梁氏水車鋸木大匠宅", "雲杉木滑道維護工舍", "伐一補三護林員居", "高山融雪水力排鋸工居", "重原木裝車大木匠居",
        "霜林口雪松松脂採集舍", "霜林口耐寒雪狼絨皮匠居", "霜林口林緣伐木工排屋一號", "霜林口林緣伐木工排屋二號", "霜林口林木繁育種苗居",
    ];
    for name in shuang_lin_kou_houses {
        b.push((Terrain::House, name));
    }
    build_village_15(map, 190, 290, "霜林口", &b);
}

fn generate_village_feng_gui_wei(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(15);
    // 5F 功能建築
    b.push((Terrain::GuardPost, "風櫃尾燈塔"));
    b.push((Terrain::Clinic, "風櫃尾脈絡草藥診所"));
    b.push((Terrain::Workshop, "水產冷燻工坊"));
    b.push((Terrain::Tavern, "風櫃尾海風食堂"));
    b.push((Terrain::Workshop, "風櫃尾巨藻晾曬場"));
    // 10R 民宅
    let feng_gui_wei_houses = [
        "洪家守塔與冷燻匠師宅", "風櫃尾燈塔長明海燈維護舍", "風櫃海風松屑冷燻魚乾坊居", "海岬風洞聽潮石屋", "風櫃尾採藻編繩漁家",
        "耐浪防風厚玄武石屋一號", "耐浪防風厚玄武石屋二號", "風櫃尾海浪警報守望舍", "風櫃尾海燕鳥羽採集居", "風櫃尾海角避濤安泰宅",
    ];
    for name in feng_gui_wei_houses {
        b.push((Terrain::House, name));
    }
    build_village_15(map, -360, 95, "風櫃尾", &b);
}

fn generate_village_wu_shi_bi(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(15);
    // 5F 功能建築
    b.push((Terrain::Dock, "烏石鼻漁船泊位"));
    b.push((Terrain::Clinic, "烏石鼻脈絡草木診所"));
    b.push((Terrain::Blacksmith, "玄武岩鑿石工坊"));
    b.push((Terrain::Market, "烏石水產集散行"));
    b.push((Terrain::Workshop, "烏石鼻海貝灰窯"));
    // 10R 民宅
    let wu_shi_bi_houses = [
        "鄭家烏石重錨鑿石老宅", "烏石鼻海貝灰窯燒漿匠居", "深水黑曜玄武岩採鑿工舍", "烏石鼻十二眼八卦網老漁夫居", "烏石鼻深海石斑釣客舍",
        "烏石鼻玄武岩堆砌漁舍一號", "烏石鼻玄武岩堆砌漁舍二號", "烏石鼻海鹽結晶提煉作坊居", "烏石鼻防滲灰漿調配舍", "烏石鼻黑石灣觀濤石宅",
    ];
    for name in wu_shi_bi_houses {
        b.push((Terrain::House, name));
    }
    build_village_15(map, -260, 35, "烏石鼻", &b);
}

fn generate_village_niu_pu(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(15);
    // 5F 功能建築
    b.push((Terrain::TownHall, "牛埔牧業公議所"));
    b.push((Terrain::Clinic, "草甸脈絡調理診所"));
    b.push((Terrain::Workshop, "羊毛與乳酪精製坊"));
    b.push((Terrain::Market, "牛埔牧畜交易市集"));
    b.push((Terrain::Inn, "牧野歇腳旅社"));
    // 10R 民宅
    let niu_pu_houses = [
        "牧家牛埔硬乳酪發酵老宅", "向陽草甸原牛厚毛氈壓製居", "牛埔重型駝獸高鞍皮匠舍", "牛埔原生甜苜蓿草種子庫居", "牛埔疾行草獸馴育舍",
        "牛埔重角原牛擠乳工舍", "草甸石基暖屋一號", "草甸石基暖屋二號", "牛埔牧道巡草員舍", "牛埔長風草浪安居宅",
    ];
    for name in niu_pu_houses {
        b.push((Terrain::House, name));
    }
    build_village_15(map, -180, -245, "牛埔", &b);
}

fn generate_village_hong_tu_kan(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(15);
    // 5F 功能建築
    b.push((Terrain::Workshop, "紅土崁選礦場"));
    b.push((Terrain::Clinic, "紅土崁脈絡復原診所"));
    b.push((Terrain::Blacksmith, "紅土百工鐵器坊"));
    b.push((Terrain::Market, "鐵礦原料調撥處"));
    b.push((Terrain::House, "紅土崁・方家宅院"));
    // 10R 民宅
    let hong_tu_kan_houses = [
        "方氏分支選礦與耐火磚老宅", "紅土崁千度耐火磚瓦窯居", "赤石階地選礦選石匠居", "紅土崁重型鍛錘鋼模修造舍", "紅土崁赤紋鐵礦調撥員居",
        "紅磚紅瓦工人家屋一號", "紅磚紅瓦工人家屋二號", "紅土崁防潮紅泥倉儲居", "紅土崁耐火泥漿攪拌工舍", "紅土階地向陽安居屋",
    ];
    for name in hong_tu_kan_houses {
        b.push((Terrain::House, name));
    }
    build_village_15(map, -85, -140, "紅土崁", &b);
}

fn generate_village_san_hao_keng_kou(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(15);
    // 5F 功能建築
    b.push((Terrain::TownHall, "三號坑口礦務所"));
    b.push((Terrain::Clinic, "三號坑口脈絡急救診所"));
    b.push((Terrain::Workshop, "坑口台車維修棚"));
    b.push((Terrain::Market, "礦山生活物資行"));
    b.push((Terrain::Inn, "坑口礦工旅社"));
    // 10R 民宅
    let san_hao_keng_kou_houses = [
        "方石礦務辦事與技術研討舍", "三號坑口重型捲揚軌道機工居", "採礦防風礦燈檢修舍", "雙刃碎石手鎬淬火工匠舍", "礦道通風支柱老木匠居",
        "依嶺石砌礦工宿舍一號", "依嶺石砌礦工宿舍二號", "三號坑口礦石樣本檢測齋", "三號坑口洗礦沉澱水工居", "三號坑口西嶺堅石安平宅",
    ];
    for name in san_hao_keng_kou_houses {
        b.push((Terrain::House, name));
    }
    build_village_15(map, -285, -140, "三號坑口", &b);
}

fn generate_village_shui_wei_liao(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(15);
    // 5F 功能建築
    b.push((Terrain::Dock, "水尾寮平底竹筏埠"));
    b.push((Terrain::Clinic, "濕地脈絡防瘴診所"));
    b.push((Terrain::Workshop, "蘆葦與竹編工棚"));
    b.push((Terrain::Market, "黑沼生鮮水產行"));
    b.push((Terrain::Inn, "水尾浮木旅社"));
    // 10R 民宅
    let shui_wei_liao_houses = [
        "阮家高腳木屋與平底竹筏匠老宅", "水尾寮沼生烏木打樁水工舍", "水尾寮柔韌蘆葦編簾工匠居", "白茫洲止血水苔採收人舍", "淺澤泥甲鱷獸防護哨舍",
        "水尾寮架高避水木屋一號", "水尾寮架高避水木屋二號", "水尾寮平底竹筏修配棚舍", "黑沼生鮮水產暫存冷浸舍", "水尾寮浮水蘆荻安樂居",
    ];
    for name in shui_wei_liao_houses {
        b.push((Terrain::House, name));
    }
    build_village_15(map, -75, -280, "水尾寮", &b);
}
