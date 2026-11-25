//! 《奇點世界・山海人物與風土戰史全誌》卷一〈方輿志〉與三卷跨篇索引核心。
//!
//! 統一承載：
//! 1. 創作者專屬 `/chronicle` 三卷純文學書卷（〈人物列傳〉、〈方輿志〉、〈紀事本末〉）與入樂純文複製；
//! 2. 遊戲世界內 `/` 主畫面各鎮廣場「端詳鎮志石碑」與酒館「翻閱海峽與蒼淵兵燹抄本」之公開史料（零活人私密劇透）。

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::gazetteer_wars::all_historical_annals;
use super::types::{IslandWorldDelta, WorldStaticDoc};
use super::world_weave_bio::settlement_slug_of;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChronicleLinkRef {
    pub surface: String,
    pub kind: &'static str, // "actor" | "region" | "annals"
    pub target_id: String,
}

#[derive(Debug, Clone)]
pub struct GazetteerArticle {
    pub id: &'static str,
    pub volume: &'static str, // "region" | "annals"
    pub title_zh: &'static str,
    pub subtitle_zh: &'static str,
    pub era_zh: &'static str,
    pub town_slug: Option<&'static str>,
    pub carrier_zh: &'static str,
    pub paragraphs: &'static [&'static str],
    pub related_ids: &'static [(&'static str, &'static str, &'static str)], // (surface, kind, id)
    pub witness_event_prefix: Option<&'static str>,
}

pub static REGIONAL_GAZETTEERS: &[GazetteerArticle] = &[
    GazetteerArticle {
        id: "reg:silver_gull_realm",
        volume: "region",
        title_zh: "銀鷗海領總志",
        subtitle_zh: "寧靜大洋東南外海大島・五鎮山海格局與四族生計總覽",
        era_zh: "太陽曆一二三年開埠至三二九年",
        town_slug: None,
        carrier_zh: "白鷗港市政廳石壁《銀鷗海領山海輿圖總記》",
        paragraphs: &[
            "銀鷗島孤懸在蒼淵主陸東南外海，四面受寧靜大洋黑潮與季風沖刷，全島幅員超過一萬平方里，山海地勢呈東西橫脊、南北抱灣的格局。東岸亂石深灣築有首府白鷗港，東南岬角伸入黑潮漁場建起鷗尾漁港，中央群峰疊翠、灰松蔽日的山谷設有灰松林驛，西端扼守白鷗水道咽喉的是西峽渡口，北端斷崖迎風高懸長明油脂火的則是海風北塢。島上四族混居通商，人類船戶與農商約佔一半，精靈多住在林灣熬製草藥與掌舵領航，矮人自北林封坑後聚在石牆內的鍛鐵工坊，獸人則憑著過人筋骨撐起碼頭扛包與遠洋船槳。",
            "島上買賣不興官府印發的空頭紙券，百工萬貨全憑早年開埠鑄定的實心銅幣當面點清、有借有還。白晝晨鐘響起，漁舟從礁灣卸下鹽漬海魚，灰松林驛的雙馬篷車載著山林藥草與粗木順著碎石驛道東下，巴爾鍛鐵鋪與各鎮石爐的錘聲此起彼落；入夜暮鐘落鎖，海岬潮氣與北林荒氣漸重，守門衛兵巡視八角石牆，往來商旅則投宿酒館二樓客房，把貴重礦料鎖進床頭厚木箱。兩百多年來，島民在海嘯、荒獸、兩岸關稅與南洋疫船的夾縫裡討生活，規矩不在官紙條文上，而在每一筆親手交割的銅板與風雨夜裡遞過來的木料之中。",
            "全島雖以五座石木城驛為骨幹，山谷河灣間還分布著青苔溪集、艾森河集、格伯橋集、黑煤谷村與琥珀橋村等二十九處活絡的鄉野集場、農礦村寨與海塢工寮，另有南岸風語橋檢疫石哨外釘封廢棄的古加津等五處船疫舊址，以及北嶺塌陷荒廢的黑煤鎮舊址。自從太陽曆二七四年東西碎石驛道貫通，白鷗港到西峽渡口車馬一天就能往返；而北城門外灰松斷橋下的封死石閘，至今仍壓著太陽曆二三八年矮人先輩打入的重鐵楔，提醒過路旅人這座海島繁華煙火底下並未完全散盡的地底荒氣。",
        ],
        related_ids: &[
            ("白鷗港", "region", "reg:white_gull_harbor"),
            ("鷗尾漁港", "region", "reg:gull_tail_port"),
            ("西峽渡口", "region", "reg:west_strait_ferry"),
            ("灰松林驛", "region", "reg:greypine_post"),
            ("海風北塢", "region", "reg:north_cape_beacon"),
            ("蒼淵主陸", "region", "reg:cangyuan_mainland"),
            ("北林封坑", "annals", "ann:epoch_238_mine_seal"),
            ("東西碎石驛道", "annals", "ann:epoch_274_greypine_road"),
        ],
        witness_event_prefix: Some("hist:wgh:epoch_123"),
    },
    GazetteerArticle {
        id: "reg:white_gull_harbor",
        volume: "region",
        title_zh: "白鷗港風土志",
        subtitle_zh: "銀鷗海領首府・萬石彎月長堤與十巷百工煙火錄",
        era_zh: "太陽曆一二三年立寨至今",
        town_slug: Some("white_gull_harbor"),
        carrier_zh: "白鷗港・市集廣場《開港築堤與十巷風土碑》",
        paragraphs: &[
            "白鷗港坐落在銀鷗島東灣，背靠北林山麓，面朝白鷗水道。太陽曆一二三年先遣船隊在東灣亂石灘拋錨立寨時，還只是拆解舊船肋木圍成的簡陋木埠；直到一七九年外海黑潮接連拍毀三座木棧橋，全港船戶與石工花了整整五年鑿山拋石，築起環抱海灣的東翼彎月防波石堤，這才擋住外洋狂浪。二三八年北林老礦坑驚出地底巨蛛，矮人匠族背著精鐵與祖傳重錘退入城內，替白鷗港合龍了北段八角外城石牆，也在北牆根下闢出終日爐火通紅的鐵匠街與鐵砧巷。",
            "今日港內主街四家老鋪各司其職：巴爾鍛鐵鋪錘聲沉猛，負責鍛打兵刃與鑲刻符文；鏽錨酒館一樓麥湯翻滾，二樓客房供遠客鎖箱安歇；疾風驛馬行的雙馬篷車每日往返東西兩岸；苦艾草藥堂則用北谷苦艾熬膏治傷。主街四鋪收徒講究老規矩，入室學徒都出自城內支巷人家，白天在鋪裡拉風箱、掌木杓，逢年過節仍回支巷老屋探望父母，把主街商號與鐵砧、船帆、銅幣、鹹魚、桶匠、繩索、石燈、海風、老井、南牆根十條支巷緊緊織成一張熟人網。",
            "不過海港風光背後也留著兩道近年的傷痕：先是二九六年海峽二成關稅風波切斷了舊九邦航線，船帆巷與桶匠巷兩戶老匠商結清帳目後黯然離島；接著三二六年夏末南岸古加津病船熱病漫進支巷，苦艾草藥堂徹夜熬湯送藥，終究沒能留住鐵砧巷、銅幣巷、鹹魚巷與繩索巷四戶人家。如今那六棟刷過防疫白石灰的舊屋仍掛在市政廳名下待售，門栓上的舊麻繩與冷壁爐裡的苦艾藥渣，都是白鷗港街坊心裡還沒淡去的往事。",
        ],
        related_ids: &[
            ("東灣平荒", "annals", "ann:epoch_123_founding"),
            ("彎月防波石堤", "annals", "ann:epoch_179_seawall"),
            ("北林老礦坑", "annals", "ann:epoch_238_mine_seal"),
            ("海峽二成關稅", "annals", "ann:epoch_296_strait_tariff"),
            ("病船熱病", "annals", "ann:epoch_326_plague_ship"),
            ("灰松林驛", "region", "reg:greypine_post"),
            ("西峽渡口", "region", "reg:west_strait_ferry"),
        ],
        witness_event_prefix: Some("hist:wgh:"),
    },
    GazetteerArticle {
        id: "reg:gull_tail_port",
        volume: "region",
        title_zh: "鷗尾漁港風土志",
        subtitle_zh: "東南外海岬角・黑潮漁場、鹽漬魚寮與礁灣生死規",
        era_zh: "太陽曆一六〇年代成港至今",
        town_slug: Some("gull_tail_port"),
        carrier_zh: "鷗尾漁港・市集廣場《鎮海礁石刻與漁寮公約碑》",
        paragraphs: &[
            "鷗尾漁港位於銀鷗島東南尾端，地形像一道彎曲鷗翼斜插進寧靜大洋。這裡不走大宗跨海商船，卻正對暖寒洋流交會的黑潮漁場，每逢秋冬魚汛，成群銀背海魚順流湧入鷗尾海道，岸邊鹽漬魚寮的粗鹽木桶從碼頭一路堆到巷尾。港裡人家多半靠捕魚、織網、剖魚醃漬與修補船肋維生，空氣裡常年浸著濃重的海鹽與桐油氣味，連孩童認字前都先學會看浪頭白沫辨認暗礁。",
            "外海討生活不講虛禮，鷗尾人最重「同船共纜」四個字。早年黑潮夜霧常讓單桅漁舟迷航觸礁，港裡因此立下死規矩：只要夜裡聽見海螺連吹三聲，岸上各戶不論平日有沒有嫌隙，都必須提著油燈趕去防波棧橋接應拉纜；誰要是敢在風浪裡見死不救，往後魚寮分鹽、船塢借木再也沒人肯搭半句話。鎮上酒館與鐵匠鋪雖不如白鷗港寬敞，打出的倒刺魚叉與浸油麻纜卻是全島最耐海水侵蝕的硬貨。",
            "近年海峽關稅改向後，主陸羅泉等免稅邦國對銀鷗鹽漬魚乾需求大增，鷗尾漁港的醃魚木桶經白鷗港轉運出海，讓勤快的漁戶攢下幾枚壓箱銅幣，也有幾家攢夠本錢的老漁戶遷往主港行商，在曬網巷、鯨骨巷與石階巷尾留下數間落鎖待售的石木舊屋。三二六年南邊古加津傳來的熱病風聲曾讓全港戒備好幾個月，老漁戶至今仍習慣在船頭掛一束乾苦艾，盼著每次出灣都能在暮鐘敲響前平安歸港。",
        ],
        related_ids: &[
            ("白鷗港", "region", "reg:white_gull_harbor"),
            ("外海黑潮", "annals", "ann:epoch_179_seawall"),
            ("海峽關稅", "annals", "ann:epoch_296_strait_tariff"),
            ("古加津", "annals", "ann:epoch_326_plague_ship"),
        ],
        witness_event_prefix: None,
    },
    GazetteerArticle {
        id: "reg:west_strait_ferry",
        volume: "region",
        title_zh: "西峽渡口風土志",
        subtitle_zh: "白鷗水道咽喉・雙桅渡輪、關稅木牌與暗礁夜潮錄",
        era_zh: "太陽曆二一〇年代設渡至今",
        town_slug: Some("west_strait_ferry"),
        carrier_zh: "西峽渡口・市集廣場《海峽渡津與通商盟約碑》",
        paragraphs: &[
            "西峽渡口扼守銀鷗島最西端的狹窄海岬，隔著湍急深暗的白鷗水道遙望蒼淵主陸群山。這裡是全島離主陸最近的跳板，雙桅渡輪每日趁白晝平潮解纜西渡，載去島上的鍛爐黑煤、鹽漬魚乾與山林苦艾，運回主陸的織布、麥粉與流離過客。渡頭木屋街與纜棚巷順著斜坡一路鋪到棧橋邊，街面上永遠混雜著人類船商的算盤聲、獸人苦力的號子與異邦旅人的陌生口音。",
            "太陽曆二七四年中央灰松驛道鑿通前，西峽渡口與東岸白鷗港往來全靠繞島帆船，遇上逆風往往得困上十天半月；驛道貫通後，雙馬篷車直抵西峽車馬站，渡口一躍成為全島貨流咽喉。不過二九六年對岸九座城邦聯手哄抬船鈔、銀鷗海領反課二成防禦關稅，西峽渡口首當其衝：往日掛著九邦旗號的布匹大船一夜絕跡，改由羅泉王國等七邦免稅商船接手，不少跑慣舊線的老船夫與算手被迫改行拉纜，渡頭巷、纜棚巷與望星坡尾也因此留下幾間退租空置的石木屋舍。",
            "白天的西峽渡口規矩森嚴，港務官逐箱核驗關稅紅印；可一旦入夜沉鐘響過、海霧漫上望星坡巷，南北礁灣偶爾會閃過幾盞不掛船號的暗燈。自從大陸諸邦戰火四起，總有避稅的檀香私貨或逃避戰禍的邊境難民試圖摸黑橫渡海峽，木桶酒館長桌邊的低語，往往比白鷗港市政廳早三天知道對岸哪座堅城又換了旗號。",
        ],
        related_ids: &[
            ("白鷗港", "region", "reg:white_gull_harbor"),
            ("灰松林驛", "region", "reg:greypine_post"),
            ("蒼淵主陸", "region", "reg:cangyuan_mainland"),
            ("灰松驛道", "annals", "ann:epoch_274_greypine_road"),
            ("二成防禦關稅", "annals", "ann:epoch_296_strait_tariff"),
            ("檀香私貨", "annals", "ann:smuggle_sandalwood_trail"),
        ],
        witness_event_prefix: None,
    },
    GazetteerArticle {
        id: "reg:greypine_post",
        volume: "region",
        title_zh: "灰松林驛風土志",
        subtitle_zh: "島心群峰嶺道・碎石車轍、深谷苦艾與灰耳冷箭之界",
        era_zh: "太陽曆二七四年設驛至今",
        town_slug: Some("greypine_post"),
        carrier_zh: "灰松林驛・市集廣場《劈嶺開路與護驛記功碑》",
        paragraphs: &[
            "灰松林驛不臨寸海，孤立在銀鷗島中央脊嶺的蒼翠山谷間，四周盡是高達數丈的耐寒灰松與終年不散的濕冷山嵐。太陽曆一二五年東灣平荒戰事落幕時，不肯入城受港規管束的原住散部退入北面深谷，經兩百多年山林荒氣浸染，演化成今日熟悉獸徑、善使骨鏃冷箭的灰耳哥布林部族。太陽曆二三八年，曾有矮人探礦隊順著北嶺裂谷試鑿老礦坑，卻因驚動坑底盤踞的吞石巨蛛而震斷木橋、立楔封坑，整片山脊就此再度沉寂為橫亙東西兩岸的險境。",
            "轉機始於太陽曆二七四年。白鷗港市政廳徵調全島石工、樵夫與巡路衛兵劈開荊棘，在泥濘山脊鋪出貫穿東西的碎石驛道，並在正中泉眼處立柵建起灰松林驛。自此雙馬篷車每日往返白鷗港與西峽渡口，車夫與押貨行商必在林驛換馬歇腳、喝一碗熱湯，苔井巷、松脂巷與鋸木場巷裡也漸漸聚起採藥師、修輪木匠、獵戶與巡林人家；只是早年開路石工完工下山後，巷尾仍留著幾間落鎖空置的舊石屋。",
            "住在灰松林驛的人不看潮汐，只看山頭雲色與雪線高低。每年入冬大雪封嶺或秋暴山洪沖毀木橋，東西篷車一斷便是十天半月，巷內人家全靠平日囤下的粗麥乾糧、松柴與鄰里互通有無熬過寒冬；而進北谷採摘止血苦艾或拾取風倒枯木與裸露礦石的人，更得時刻提防林霧裡的灰耳斥候陷阱。這裡的人說話簡短實在，誰在雪夜裡遞過半袋口糧、誰冒雨下谷背回傷患，整條巷子能記上半輩子。",
        ],
        related_ids: &[
            ("白鷗港", "region", "reg:white_gull_harbor"),
            ("西峽渡口", "region", "reg:west_strait_ferry"),
            ("灰松散部分流", "annals", "ann:epoch_123_founding"),
            ("劈荊開嶺設驛", "annals", "ann:epoch_274_greypine_road"),
        ],
        witness_event_prefix: None,
    },
    GazetteerArticle {
        id: "reg:north_cape_beacon",
        volume: "region",
        title_zh: "海風北塢風土志",
        subtitle_zh: "北端斷崖孤塢・油脂長明燈塔與寒洋守望者之鄉",
        era_zh: "太陽曆一八四年立塔至今",
        town_slug: Some("north_cape_beacon"),
        carrier_zh: "海風北塢・市集廣場《北岬長明火守望石刻》",
        paragraphs: &[
            "海風北塢盤踞在銀鷗島最北端的玄武岩斷崖之下，北面直迎寧靜大洋毫無遮攔的凜冽寒風，海浪拍擊崖底礁石的轟鳴聲晝夜不停。太陽曆一八四年白鷗港修築東翼防波堤時，航海領航員為防北方商船在夜霧裡誤撞北岬暗礁，在斷崖頂端壘石築起望海高塔，日夜燃起鯨脂與松脂混煉的長明火；看守燈火的匠戶、採集崖草的精靈與避風修船的北路漁人便在崖下凹灣聚成了北塢。",
            "北塢地處偏遠，人口雖是五鎮最少，屋舍石牆卻砌得最厚實低矮，屋頂都壓著防風巨石，塔腳巷與流木巷尾還留著兩間早年守塔匠人輪調南遷後空出的舊石屋。每逢寒冬北風封凍淺灣，外鎮船隻難以靠岸，塢內居民便圍著石爐編織耐寒麻纜、熬煮海獸燈油、硝製厚革皮靴，等春暖開航再與白鷗港及西峽渡口換取精鐵與穀物。",
            "正因為地處孤絕，海風北塢成了全島望向北方汪洋與蒼淵主陸東北航線的第一雙眼睛。無論是暴風前夕異常南遷的海鳥群、迷航漂來的異邦殘破船板，還是北方大汗國戰火波及航道的傳聞，往往最先被北塢燈塔上的守望者收入眼底，再順著山徑風信傳遍全島。",
        ],
        related_ids: &[
            ("銀鷗海領", "region", "reg:silver_gull_realm"),
            ("白鷗港", "region", "reg:white_gull_harbor"),
            ("萬石築堤與立塔", "annals", "ann:epoch_179_seawall"),
            ("蒼淵主陸", "region", "reg:cangyuan_mainland"),
        ],
        witness_event_prefix: None,
    },
    GazetteerArticle {
        id: "reg:cangyuan_mainland",
        volume: "region",
        title_zh: "蒼淵主陸百邦輿略",
        subtitle_zh: "隔海六十二萬平方里大地・百國並立、十四語系與連天烽火",
        era_zh: "太陽曆三二九年當下大勢",
        town_slug: None,
        carrier_zh: "五鎮酒館長桌流傳《海峽彼岸百邦風物與兵燹略記》",
        paragraphs: &[
            "從西峽渡口搭雙桅帆船橫渡白鷗水道，對岸便是幅員超過六十二萬平方里的蒼淵主陸。這片浩瀚大陸上百邦林立，八百四十多座城池星羅棋布，順著山川水文自然演化出十四支地域文化：北方廣袤草原與高寒荒磧馳騁著獸人與矮人混編的迪克大汗國、沙格祭司邦與阿魯游牧兀魯思；中央九百里闊葉古林與澄鏡湖畔，精靈古裔與人類王國如羅泉、奧納、迪拉、莫利世代並立；南方雨林大澤與珊礁海岸則盤踞著維洛聖座領、伊頓王國與塞蘭親王國。",
            "主陸百工各有所長：織梭都與克萊澳的匠師掌握金銀首飾秘傳，青灣岬熬製松煙墨汁，杜恩塞與泰維堅城開爐鑄幣，鹽風灣提煉燈用鯨脂油，礫灘灣則以製造攻城重型扭力投石機讓諸國敬畏。七十六門民間信仰與古祀遍布城鄉，三百二十六條陸海商道過橋穿關，原本維繫著四族萬貨的繁榮流轉。",
            "然而太陽曆三二〇年代以來，蒼淵主陸已陷入數百年少見的連環戰禍與天災之中。十二場跨國大戰同時引爆，舊日協防盟約在強兵壓境下像廢紙般接連撕毀；卡麥宗派與澤恩離教派遭討伐軍焚祠搜剿；南方米蘭岬猩紅熱、哈蘭港血咳疫與克登火山噴發接連襲來，塞谷強震與貝納雪崩更截斷多條內陸幹道。無數破產行商、失地匠人與邊境流民正順著維泉至潮音港的暗道或東南航線湧向海岸，讓孤懸海外的銀鷗島成了亂世風暴邊緣難得的喘息之地。",
        ],
        related_ids: &[
            ("西峽渡口", "region", "reg:west_strait_ferry"),
            ("伊頓破佩德之戰", "annals", "ann:war_koaiakia_pede"),
            (
                "黑磧二十邦合圍大戰",
                "annals",
                "ann:war_shagr_twenty_banners",
            ),
            ("迪拉征莫利七國棄盟", "annals", "ann:war_hemudea_sariromia"),
            ("歐橋霍原反目之戰", "annals", "ann:war_bocada_betrayal"),
            (
                "異端征剿與三大疫災",
                "annals",
                "ann:calamity_crusades_and_plagues",
            ),
            ("萬里檀香暗線", "annals", "ann:smuggle_sandalwood_trail"),
        ],
        witness_event_prefix: None,
    },
];

pub fn find_gazetteer_article(id: &str) -> Option<&'static GazetteerArticle> {
    REGIONAL_GAZETTEERS
        .iter()
        .chain(all_historical_annals().iter())
        .find(|a| a.id == id)
}

pub fn town_region_article_id(town_slug: &str) -> &'static str {
    match town_slug {
        "white_gull_harbor" => "reg:white_gull_harbor",
        "gull_tail_port" => "reg:gull_tail_port",
        "west_strait_ferry" => "reg:west_strait_ferry",
        "greypine_post" => "reg:greypine_post",
        "north_cape_beacon" => "reg:north_cape_beacon",
        _ => "reg:silver_gull_realm",
    }
}

/// 建立單篇〈方輿志〉或〈紀事本末〉之純文學 JSON（供 `/chronicle` 與遊戲內公開碑刻／抄本共用；
/// `include_actor_links` 於遊戲內 `/` 設為 `false` 以嚴守零全知劇透鐵律！）
pub fn build_gazetteer_article_json(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    article_id: &str,
    include_actor_links: bool,
) -> Option<Value> {
    if let Some(dyn_ann) = delta
        .living_chronicle
        .committed
        .dynamic_annals
        .get(article_id)
    {
        let masked_prose = super::slow_think::mask_player_names_in_text(delta, &dyn_ann.prose_zh);
        let paragraphs: Vec<String> = masked_prose
            .split("\n\n")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let mut linked_refs: Vec<ChronicleLinkRef> = if article_id.starts_with("annals_apex_") {
            vec![
                ChronicleLinkRef {
                    surface: "銀鷗海領總志".to_string(),
                    kind: "region",
                    target_id: "reg:silver_gull_realm".to_string(),
                },
                ChronicleLinkRef {
                    surface: "白鷗港".to_string(),
                    kind: "region",
                    target_id: "reg:white_gull_harbor".to_string(),
                },
            ]
        } else if article_id == "ann:live:wgh_vacant_rekindled" {
            vec![
                ChronicleLinkRef {
                    surface: "白鷗港".to_string(),
                    kind: "region",
                    target_id: "reg:white_gull_harbor".to_string(),
                },
                ChronicleLinkRef {
                    surface: "海峽二成關稅與九邦商路洗牌紀".to_string(),
                    kind: "annals",
                    target_id: "ann:epoch_296_strait_tariff".to_string(),
                },
                ChronicleLinkRef {
                    surface: "古加津病船熱病與支巷六屋封門紀".to_string(),
                    kind: "annals",
                    target_id: "ann:epoch_326_plague_ship".to_string(),
                },
            ]
        } else {
            vec![
                ChronicleLinkRef {
                    surface: "白鷗港".to_string(),
                    kind: "region",
                    target_id: "reg:white_gull_harbor".to_string(),
                },
                ChronicleLinkRef {
                    surface: "灰松林驛".to_string(),
                    kind: "region",
                    target_id: "reg:greypine_post".to_string(),
                },
                ChronicleLinkRef {
                    surface: "北林深坑破隙與矮人封閘入城紀".to_string(),
                    kind: "annals",
                    target_id: "ann:epoch_238_mine_seal".to_string(),
                },
            ]
        };
        let mut related_actors: Vec<Value> = Vec::new();
        if include_actor_links
            && let Ok(fp) = serde_json::from_str::<super::living_chronicler::ChronicleFactPacket>(
                &dyn_ann.fact_json,
            )
        {
            let mut seen_ids = std::collections::BTreeSet::new();
            for aid in [fp.agent_id.as_str(), fp.patient_id.as_str()] {
                if aid.is_empty() || aid.starts_with("pl:") || !seen_ids.insert(aid) {
                    continue;
                }
                if let Some(a) = delta.actors.get(aid)
                    && a.controller_id.is_none()
                    && a.alive
                    && !a.household_id.is_empty()
                    && masked_prose.contains(&a.display_name)
                {
                    let home_name = doc
                        .nodes
                        .get(&a.home_node)
                        .map_or_else(|| a.home_node.clone(), |n| n.name.replace('・', "．"));
                    linked_refs.push(ChronicleLinkRef {
                        surface: a.display_name.clone(),
                        kind: "actor",
                        target_id: a.actor_id.clone(),
                    });
                    related_actors.push(json!({
                        "actor_id": a.actor_id,
                        "name": a.display_name,
                        "role": a.role,
                        "race": a.race,
                        "door_zh": home_name,
                    }));
                }
            }
        }
        linked_refs.sort_by_key(|r| std::cmp::Reverse(r.surface.chars().count()));
        let plain_copy_text = format!(
            "{}\n{}\n\n{}",
            dyn_ann.title_zh,
            dyn_ann.subtitle_zh,
            paragraphs.join("\n\n")
        );
        return Some(json!({
            "ok": true,
            "id": dyn_ann.target_id,
            "volume": "annals",
            "title_zh": dyn_ann.title_zh,
            "subtitle_zh": dyn_ann.subtitle_zh,
            "era_zh": dyn_ann.era_zh,
            "carrier_zh": dyn_ann.carrier_zh,
            "paragraphs": paragraphs,
            "linked_refs": linked_refs,
            "related_actors": related_actors,
            "plain_copy_text": plain_copy_text,
        }));
    }

    let art = find_gazetteer_article(article_id)?;
    let mut paragraphs: Vec<String> = art.paragraphs.iter().map(|s| (*s).to_string()).collect();
    if art.volume == "region" {
        paragraphs.extend(
            delta
                .living_chronicle
                .region_paragraphs(art.id)
                .into_iter()
                .map(|p| super::slow_think::mask_player_names_in_text(delta, &p)),
        );
    }
    let mut linked_refs: Vec<ChronicleLinkRef> = art
        .related_ids
        .iter()
        .map(|(surf, kind, tid)| ChronicleLinkRef {
            surface: (*surf).to_string(),
            kind,
            target_id: (*tid).to_string(),
        })
        .collect();

    let mut related_actors: Vec<Value> = Vec::new();
    if include_actor_links {
        let mut exact_dyn_ids = std::collections::BTreeSet::new();
        if let Some(tslug) = art.town_slug {
            if tslug == "greypine_post" {
                for (aid, rows) in &delta.living_chronicle.committed.actor_chapters {
                    if rows
                        .iter()
                        .any(|r| r.deed_id == "deed:island:first_spider_slain")
                    {
                        exact_dyn_ids.insert(aid.as_str());
                    }
                }
            }
            for (nid, aid) in &delta.sold_houses {
                if super::living_chronicler::town_slug_of_node(nid) == tslug {
                    exact_dyn_ids.insert(aid.as_str());
                }
            }
            for bh in delta.built_houses.values() {
                if super::living_chronicler::town_slug_of_node(&bh.anchor_node_id) == tslug {
                    exact_dyn_ids.insert(bh.owner_id.as_str());
                }
            }
        }
        for a in delta.actors.values() {
            if a.controller_id.is_some()
                || a.actor_id.starts_with("pl:")
                || !a.alive
                || a.household_id.is_empty()
            {
                continue;
            }
            let mentioned_in_dyn = exact_dyn_ids.contains(a.actor_id.as_str());
            let matched = if let Some(pref) = art.witness_event_prefix {
                a.mind
                    .ltm
                    .episodic
                    .iter()
                    .any(|e| e.event_id.starts_with(pref))
            } else if let Some(tslug) = art.town_slug {
                mentioned_in_dyn
                    || (settlement_slug_of(&a.household_id) == tslug
                        && (a.household_id.contains(":shop_") || a.household_bond.contains("當家")))
            } else {
                false
            };
            if matched && (mentioned_in_dyn || related_actors.len() < 12) {
                let home_name = doc
                    .nodes
                    .get(&a.home_node)
                    .map_or_else(|| a.home_node.clone(), |n| n.name.replace('・', "．"));
                linked_refs.push(ChronicleLinkRef {
                    surface: a.display_name.clone(),
                    kind: "actor",
                    target_id: a.actor_id.clone(),
                });
                if related_actors.len() < 14 {
                    related_actors.push(json!({
                        "actor_id": a.actor_id,
                        "name": a.display_name,
                        "role": a.role,
                        "race": a.race,
                        "door_zh": home_name,
                    }));
                }
            }
        }
    }
    linked_refs.sort_by_key(|r| std::cmp::Reverse(r.surface.chars().count()));

    let plain_copy_text = format!(
        "{}\n{}\n\n{}",
        art.title_zh,
        art.subtitle_zh,
        paragraphs.join("\n\n")
    );

    Some(json!({
        "ok": true,
        "id": art.id,
        "volume": art.volume,
        "title_zh": art.title_zh,
        "subtitle_zh": art.subtitle_zh,
        "era_zh": art.era_zh,
        "carrier_zh": art.carrier_zh,
        "paragraphs": paragraphs,
        "linked_refs": linked_refs,
        "related_actors": related_actors,
        "plain_copy_text": plain_copy_text,
    }))
}

/// 供遊戲內廣場石碑與酒館抄本列出可閱覽之世界內公開篇章目錄（絕不含任何活人私密列傳）
pub fn build_in_world_scroll_bundle_json(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    town_slug: &str,
    mode: &str, // "stele" | "annals"
    selected_id: Option<&str>,
) -> Value {
    let reg_id = town_region_article_id(town_slug);
    let mut catalog: Vec<(String, &'static str, String, String)> = Vec::new();
    if mode == "stele" {
        for a in REGIONAL_GAZETTEERS
            .iter()
            .filter(|a| a.id == reg_id || a.id == "reg:silver_gull_realm")
        {
            catalog.push((
                a.id.to_string(),
                a.volume,
                a.title_zh.to_string(),
                a.era_zh.to_string(),
            ));
        }
        for ann in all_historical_annals() {
            if ann.id.starts_with("ann:epoch_") {
                catalog.push((
                    ann.id.to_string(),
                    ann.volume,
                    ann.title_zh.to_string(),
                    ann.era_zh.to_string(),
                ));
            }
        }
        for dyn_ann in delta.living_chronicle.dynamic_annals_sorted() {
            catalog.push((
                dyn_ann.target_id.clone(),
                "annals",
                dyn_ann.title_zh.clone(),
                dyn_ann.era_zh.clone(),
            ));
        }
    } else {
        for a in REGIONAL_GAZETTEERS
            .iter()
            .filter(|a| a.id == "reg:cangyuan_mainland")
        {
            catalog.push((
                a.id.to_string(),
                a.volume,
                a.title_zh.to_string(),
                a.era_zh.to_string(),
            ));
        }
        for ann in all_historical_annals() {
            if ann.id.starts_with("ann:war_")
                || ann.id.starts_with("ann:calamity_")
                || ann.id.starts_with("ann:smuggle_")
                || ann.id == "ann:epoch_296_strait_tariff"
            {
                catalog.push((
                    ann.id.to_string(),
                    ann.volume,
                    ann.title_zh.to_string(),
                    ann.era_zh.to_string(),
                ));
            }
        }
        for dyn_ann in delta.living_chronicle.dynamic_annals_sorted() {
            catalog.push((
                dyn_ann.target_id.clone(),
                "annals",
                dyn_ann.title_zh.clone(),
                dyn_ann.era_zh.clone(),
            ));
        }
    }

    let pick_id = selected_id
        .and_then(|sid| {
            catalog
                .iter()
                .find(|(id, _, _, _)| id == sid)
                .map(|(id, _, _, _)| id.clone())
        })
        .or_else(|| catalog.first().map(|(id, _, _, _)| id.clone()))
        .unwrap_or_else(|| reg_id.to_string());

    let active_article = build_gazetteer_article_json(doc, delta, &pick_id, false);
    let items: Vec<Value> = catalog
        .into_iter()
        .map(|(id, volume, title_zh, era_zh)| {
            json!({
                "id": id,
                "volume": volume,
                "title_zh": title_zh,
                "era_zh": era_zh,
            })
        })
        .collect();

    json!({
        "ok": true,
        "mode": mode,
        "selected_id": pick_id,
        "catalog": items,
        "article": active_article,
    })
}
