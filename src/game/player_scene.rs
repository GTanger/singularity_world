//! 玩家層場景措辭：只描述玩家能看見的公共身份與微動作。

use std::collections::HashMap;

use crate::model::RoomObject;

struct PolishedSceneText {
    name: &'static str,
    role: &'static str,
    crowd: &'static str,
    entry: &'static str,
    reentry: &'static str,
    observation: &'static str,
    micro: &'static [&'static str],
}

struct SceneObjectText {
    room: &'static str,
    id: &'static str,
    name: &'static str,
    look: &'static str,
    followup_action: &'static str,
    followup: &'static str,
}

struct LifeSceneObjectText {
    room: &'static str,
    id: &'static str,
    name: &'static str,
    look: &'static str,
    actions: &'static [(&'static str, &'static str)],
}

const M3_020_SCENE_OBJECTS: &[SceneObjectText] = &[
    SceneObjectText {
        room: "石壘門",
        id: "m3_020_stone_gate_pass_tags",
        name: "通行牌",
        look: "通行牌按牲口、商隊和工坊送件分成幾疊，邊角被手汗磨亮；門衛只把下一疊推到石案前，沒有讓人自己翻找。",
        followup_action: "Read",
        followup: "你看清最上面幾張通行牌：牧地牲口先驗繩扣，工坊送件要留牌號，商隊則等門衛點到旗石廣場的隊列。",
    },
    SceneObjectText {
        room: "旗石廣場",
        id: "m3_020_flagstone_waiting_lines",
        name: "等候線",
        look: "等候線用舊旗和石縫分出幾股，往西門坡道街、鑄幣巷口和傭工棚邊的人各站一側；線尾仍有人低聲問該排哪裡。",
        followup_action: "Use",
        followup: "你順著等候線往旁邊讓出一步，旗手立刻把鑄幣巷口和傭工棚邊的人分得更開，廣場中間的推擠短短鬆了一點。",
    },
    SceneObjectText {
        room: "鑄幣巷口",
        id: "m3_020_mint_half_open_door",
        name: "半開鑄幣門",
        look: "半開的門縫裡能看見銅秤、封袋和一點散鎂反光；兌牌人把門縫守得很窄，只讓被叫到的人靠近門檻。",
        followup_action: "Open",
        followup: "你剛靠近門縫，兌牌人便用手掌把門壓回半開，只留下銅秤聲和封袋摩擦聲；沒被叫到的人仍得退回廣場側線。",
    },
    SceneObjectText {
        room: "傭工棚邊",
        id: "m3_020_hire_short_work_boards",
        name: "短工牌",
        look: "短工牌掛在棚柱外側，牌面只寫活計方向和大略時段；棚頭人每翻下一片牌，等工的人就往前擠半步。",
        followup_action: "Read",
        followup: "你讀到幾片短工牌：商隊卸貨要熟手，牧地補柵要兩人同行，工坊搬料只收半日；棚頭人把最急的一片翻到最外側。",
    },
    SceneObjectText {
        room: "議棚前階",
        id: "m3_020_council_notice_strip",
        name: "議棚告示",
        look: "告示貼在階邊木板上，欠糧、傭工糾紛和通行牌申訴分成三列；記牌人用手掌壓著邊角，不讓圍觀的人撕下來傳看。",
        followup_action: "Read",
        followup: "你讀完告示上最外側幾行：欠糧先找記牌人押名，傭工糾紛等棚內叫號，通行牌申訴不得堵在坡街正中。",
    },
];

const M3_027_LIFE_OBJECTS: &[LifeSceneObjectText] = &[
    LifeSceneObjectText {
        room: "半嶺水岸渡口",
        id: "m3_027_north_bank_reeds",
        name: "河霧蘆叢",
        look: "蘆叢貼著北岸水線長開，霧水壓在葉尖上；市集包貨單催著蘆草墊箱，靠近踏板的一側已有被割過的斜口，濕蘆草仍壓在泥邊等人伸手取下。",
        actions: &[(
            "Gather",
            "你撥開貼岸的河霧，從濕蘆裡割下一束能用的蘆草；斷口上的水珠沿著葉脈落回泥裡。",
        )],
    },
    LifeSceneObjectText {
        room: "半嶺水岸渡口",
        id: "m3_027_wet_bench",
        name: "濕長凳",
        look: "長凳靠河的一半沾著霧水，乾的一端被兩只包袱佔住；有個候渡者把身子挪開半寸，像是在等人決定坐不坐。",
        actions: &[(
            "Rest",
            "你把包袱挪到濕痕外坐下，肩背從挑擔聲裡鬆下來；凳腳晃了一下，旁邊人把自己的包袱往回收。",
        )],
    },
    LifeSceneObjectText {
        room: "半嶺水岸渡口",
        id: "m3_027_hot_soup_stall",
        name: "熱湯攤",
        look: "湯桶蓋剛掀開，熱氣貼著河霧往上冒；攤主一邊收錢盛滿碗，一邊把等分湯的人壓在桶側短線後面。",
        actions: &[
            (
                "BuySoup",
                "你交出兩枚鎂，接過一碗滿湯；碗沿燙得指節發紅，第一口鹹熱很快把腹中空意壓下去。",
            ),
            (
                "QueueBroth",
                "你退到桶側短線後面等分湯，輪到你時只剩半碗淡湯；它不頂飽，至少讓胃裡有了熱氣。",
            ),
        ],
    },
    LifeSceneObjectText {
        room: "半嶺水岸渡口",
        id: "m3_027_cargo_pole",
        name: "貨擔",
        look: "兩副貨擔斜靠在泥地上，麻袋底拖出深痕；市集的人把幾枚鎂壓在手心裡，看誰願意先把它們送到踏板邊。",
        actions: &[(
            "CarryCargo",
            "你把一副貨擔扛到踏板邊，肩上立刻壓出酸意；市集的人塞給你幾枚鎂，船戶看見路口清出一線，朝你短短點頭。",
        )],
    },
    LifeSceneObjectText {
        room: "半嶺水岸渡口",
        id: "m3_027_boat_rope",
        name: "船繩",
        look: "濕船繩繃在木樁和船腹之間，繩股裡卡著泥水；船戶每看一眼河面，就把手掌往繩上壓一下，只有看得懂收放手勢的人才靠近搭手。",
        actions: &[(
            "HelpRope",
            "船戶把繩尾遞給你，你跟著他一收一放，記住船腹貼岸前那一下手勢；船身少晃了一下，踏板前的人群跟著讓出一條窄縫。",
        )],
    },
    LifeSceneObjectText {
        room: "半嶺水岸渡口",
        id: "m3_027_river_awning",
        name: "避風棚",
        look: "一片舊帆布斜斜壓在木桿上，棚下沒有好座位，只能貼著貨箱站一會兒，把衣角的冷水慢慢晾開。",
        actions: &[(
            "DryOff",
            "你站到舊帆布下，把濕衣角攤在貨箱邊；河風吹不太進來，肩背雖還沉，手腳先緩過一點。",
        )],
    },
    LifeSceneObjectText {
        room: "半嶺水岸渡口",
        id: "m3_027_charcoal_brazier",
        name: "炭火盆",
        look: "炭火盆被幾個候渡者圍在中間，火苗不大，煙氣貼著袖口走；靠近的人都只能烘一小會兒便讓位。",
        actions: &[(
            "WarmHands",
            "你把手伸到炭火盆旁，煙味鑽進袖口，指節慢慢有了知覺；旁邊的人給你讓出半掌寬的位置。",
        )],
    },
    LifeSceneObjectText {
        room: "半嶺水岸渡口",
        id: "m3_027_waiting_line_chat",
        name: "候渡隊列",
        look: "候渡隊列在湯攤和踏板之間折了一道彎，熟門路的人低聲交換船期、湯價、谷地路口近況和哪個船戶肯讓人搭手；排到木牌內側的人，才會被船戶看作下一輪上船的人。",
        actions: &[
            (
                "TalkQueue",
                "你沿著隊列和人搭了幾句話，問到下一班船大概還要等多久；話說多了有些耗神，但不再像一個人站在霧裡。",
            ),
            (
                "QueueFerry",
                "你站到隊列折角外側，前面的候渡者照木牌往踏板挪半步；後面的人把你身後的空隙補上，船戶抬手認住你這個位置。",
            ),
        ],
    },
    LifeSceneObjectText {
        room: "水尾寮渡頭市集",
        id: "m3_027_market_sackcloth_pile",
        name: "舊麻袋堆",
        look: "舊麻袋堆貼著市集棚腳攤開，破口和好布被分成兩邊；搬貨人只剪能再包貨的布片，剩下太濕太爛的麻線被踩進泥裡。",
        actions: &[(
            "Gather",
            "你從棚腳舊麻袋裡剪下一卷還能包貨的麻袋布；旁邊搬貨人照同一堆料挑出幾片，塞到待包的木箱旁。",
        )],
    },
    LifeSceneObjectText {
        room: "水尾寮渡頭市集",
        id: "m3_027_market_reed_buyer",
        name: "市集包貨單",
        look: "市集包貨單壓在秤邊濕木板上，三行短字分別催蘆草墊箱、麻袋布包貨、麻繩捆箱；看貨人和搬貨人都照這張單子抽料，不讓箱包堵住踏板口。",
        actions: &[
            (
                "SellReeds",
                "你把一束蘆草交到秤邊，收牌人掂了掂濕重，數出幾枚鎂；旁邊貨籃立刻有人抽走草束去墊箱角。",
            ),
            (
                "SellSackcloth",
                "你把麻袋布交到包貨單旁，看貨人沿布邊摸過一圈後數出幾枚鎂；搬貨人立刻拿它裹住一只磨角木箱。",
            ),
            (
                "SellRope",
                "你把麻繩卷放到濕木板上，收牌人拉開繩頭試了試韌度後數出幾枚鎂；踏板口那排箱包很快被重新捆緊。",
            ),
        ],
    },
    LifeSceneObjectText {
        room: "渡船踏板",
        id: "m3_027_plank_wet_rope_coil",
        name: "濕繩圈",
        look: "濕繩圈掛在踏板木樁內側，舊繩和可用繩被船戶分開；候船人經過時會避開腳邊那圈能重新捆貨的麻繩。",
        actions: &[(
            "Gather",
            "你從木樁內側理出一卷還能捆箱的麻繩；船戶看見少了一圈可用繩，順手把剩下的繩頭往樁後收緊。",
        )],
    },
    LifeSceneObjectText {
        room: "渡船踏板",
        id: "m3_027_plank_boarding_marker",
        name: "踏板木牌",
        look: "木牌插在踏板口，船戶只認排到牌內側的人；牌腳旁壓著剛清出的貨擔痕，踏板上的濕泥被來回腳步磨得發亮。",
        actions: &[(
            "BoardFerry",
            "船戶看見你已被隊列認住，抬手讓你踩到踏板內側；後面的人跟著收成一線，船身在河霧裡貼岸等下一步。",
        )],
    },
    LifeSceneObjectText {
        room: "南岸泥坡",
        id: "m3_027_south_landing_rope",
        name: "上岸繩",
        look: "粗繩從船頭拉到泥坡木樁，繩上沾著被人握過的濕泥；南岸的人按繩位上坡，讓剛下船的人先踩穩再分往倉棚和谷地路。",
        actions: &[(
            "StepAshore",
            "你扶著上岸繩踩進南岸泥坡，鞋底陷下一寸又拔出來；後面的人照同一條繩位上來，沒有把你擠回踏板邊。",
        )],
    },
    LifeSceneObjectText {
        room: "倉棚前場",
        id: "m3_027_warehouse_tally_board",
        name: "倉棚點貨牌",
        look: "點貨牌掛在倉棚門邊，麻袋、木箱和渡口來的短貨各壓一行；記牌人只把已送到踏板邊的貨擔劃入下一列。",
        actions: &[(
            "CheckCargo",
            "你指給記牌人看方前清到踏板邊的貨擔痕，對方把那一行劃進倉棚短冊，又補給你一枚尾款鎂。",
        )],
    },
];

const FINAL_MAP_SCENE_TEXTS: &[PolishedSceneText] = &[
    PolishedSceneText {
        name: "文曜鎮口",
        role: "守門人",
        crowd: "鎮口人群",
        entry: "你走到文曜鎮口。鎮牌樓把谷地路收進陰影裡，守門人讓候檢車馬慢下來，東市街和倉場方向的人聲從門內分開。",
        reentry: "你回到文曜鎮口。路旗下又聚起一小段候檢腳步，守門人把剛放行的車馬引向東市街那邊。",
        observation: "{pronoun}按著門邊路牌，抬手讓一輛車先停在牌樓陰影下。",
        micro: &[
            "守門人把路牌往牌樓下挪了半步，候檢的人聲立刻低下去。",
            "有人把車轅拉正，等東市街那頭讓出一段路再進鎮。",
            "鎮口短短響起木牌碰柱聲，谷地路的腳步被門影吞進去。",
        ],
    },
    PolishedSceneText {
        name: "文曜東市街",
        role: "看貨人",
        crowd: "市集人群",
        entry: "你走進文曜東市街。布棚沿街排開，看貨人把價碼壓在攤布下，井邊巷、旅社前廊和倉場門口的人流在街心交會。",
        reentry: "你回到文曜東市街。攤布前換了一撥看貨人，剛從倉場門口送來的箱包被推到街側等價。",
        observation: "{pronoun}掀起攤布一角看貨色，低聲把問到的價碼傳給旁邊的人。",
        micro: &[
            "看貨人把貨樣放回籃裡，眼神仍留在倉場門口那邊的箱包上。",
            "有人在布棚下短短議價，又回頭問旅社前廊前是否尚有外來商隊。",
            "東市街短短響起秤錘碰木聲，井邊巷的取水聲被壓到後面。",
        ],
    },
    PolishedSceneText {
        name: "文曜旅社前廊",
        role: "接待員",
        crowd: "旅社人群",
        entry: "你走到文曜旅社前廊。長凳、熱水壺和掛包木鉤貼著廊柱排開，接待員替剛入鎮的人分辨東市街、井邊巷和百工坊的去處。",
        reentry: "你回到文曜旅社前廊。廊下包袱換了位置，接待員把新添的熱水壺推到長凳旁。",
        observation: "{pronoun}把熱水壺放到廊桌上，側身聽東市街那邊是否尚有人找宿。",
        micro: &[
            "接待員把長凳下的包袱往內推，給從東市街來的人讓出廊邊。",
            "有人在廊柱旁問今晚尚有無空鋪，聲音很快被街上議價蓋住。",
            "旅社前廊短短響起碗底碰桌聲，百工坊那邊的敲打聲靠近一點。",
        ],
    },
    PolishedSceneText {
        name: "文曜講堂前坪",
        role: "傳話人",
        crowd: "廟前人群",
        entry: "你走到文曜講堂前坪。香灰貼著石凳邊緣，告示板下有人低聲傳話，把鎮口、井邊巷和東市街的消息接在一起。",
        reentry: "你回到文曜講堂前坪。告示板旁多了幾道新摺痕，傳話人把剛聽到的鎮口消息壓低聲說出。",
        observation: "{pronoun}在告示板前停住，低聲把剛問到的事轉給等在石凳旁的人。",
        micro: &[
            "傳話人把告示紙角壓平，讓後面的人先看清日期。",
            "有人在石凳旁合掌停了一下，轉身就把消息送往井邊巷。",
            "講堂前坪短短安靜了一息，東市街的人聲又從牌樓那邊湧回來。",
        ],
    },
    PolishedSceneText {
        name: "文曜井邊巷",
        role: "取水人",
        crowd: "井邊人群",
        entry: "你走進文曜井邊巷。井繩濕痕拖到巷口，取水人和跑腿孩子在住戶牆影下穿過，讓東市街後面有了日常內側。",
        reentry: "你回到文曜井邊巷。石圈旁的水痕又深了一圈，取水人把剛打滿的桶讓到巷壁邊。",
        observation: "{pronoun}把水桶挪到牆影下，抬頭看百工坊那邊是否尚有人要水。",
        micro: &[
            "取水人把井繩往石圈上多繞一圈，水滴沿巷口慢慢拖開。",
            "跑腿孩子抱著小桶穿過人縫，很快又被東市街的聲音吞沒。",
            "井邊巷短短響起水桶碰石聲，講堂前坪的低語跟著停了一下。",
        ],
    },
    PolishedSceneText {
        name: "文曜百工坊",
        role: "學徒",
        crowd: "工坊人群",
        entry: "你走到文曜百工坊。木屑、鐵件和修具架排在半開門面下，學徒把東市街送來的活計分進各家。",
        reentry: "你回到文曜百工坊。門面下又多了幾件待修工具，學徒把剛問到的料價寫到木牌邊。",
        observation: "{pronoun}抱著一捆待修件停在門面下，等裡頭讓出修具架。",
        micro: &[
            "學徒把木屑掃到門檻內側，讓井邊巷過來的人能貼牆走過。",
            "有人把斷扣和小刀放到修具架前，低聲問今天能不能趕好。",
            "百工坊短短響起敲鐵聲，旅社前廊下的談話跟著壓低。",
        ],
    },
    PolishedSceneText {
        name: "文曜車馬行",
        role: "車夫",
        crowd: "車馬人群",
        entry: "你走到文曜車馬行。車轅和草料筐壓在鎮口南側，車夫等倉場門口放出手勢，再把渡口貨路來的車聲接進鎮裡。",
        reentry: "你回到文曜車馬行。車轍又被壓亮了一截，車夫把候卸貨包往倉場方向重新排開。",
        observation: "{pronoun}扶著車轅看向倉場門口，等下一個可卸貨的手勢。",
        micro: &[
            "車夫把草料筐踢到車輪旁，讓從貨路外彎來的空車先靠邊。",
            "有人把貨包重新綁緊，抬頭等倉場門口那邊喊號。",
            "車馬行短短響起車軸聲，鎮口牌樓下的人群跟著讓開一段。",
        ],
    },
    PolishedSceneText {
        name: "文曜倉場門口",
        role: "倉管",
        crowd: "倉場人群",
        entry: "你走到文曜倉場門口。木箱、糧袋和封牌排在門檻陰影裡，倉管把車馬行送來的貨分成入倉、轉市與暫候三路。",
        reentry: "你回到文曜倉場門口。封牌位置換過一輪，倉管把要送去東市街的箱包推到外側。",
        observation: "{pronoun}按著冊簿分貨，抬手讓一批箱包先停在門檻外側。",
        micro: &[
            "倉管把冊簿邊角壓平，示意車馬行再等一輛車。",
            "有人把糧袋從木箱旁分開，低聲問這批要進倉還是轉市。",
            "倉場門口短短響起木箱落地聲，東市街的議價聲立刻靠近。",
        ],
    },
    PolishedSceneText {
        name: "南城外驛道",
        role: "驛夫",
        crowd: "驛道人群",
        entry: "你走上南城外驛道。糧倉外秤的秤聲落在身後，車轍、路旗和候檢行囊一路壓向南城牆影。",
        reentry: "你回到南城外驛道。進城長線又往前挪了幾步，驛夫把從糧倉那邊來的車隊排到路旗旁。",
        observation: "{pronoun}扶著路旗看向南城甕門，把剛到的人流分進車隊與步行兩列。",
        micro: &[
            "驛夫把一面濕路旗扯正，等候進城的人跟著往前貼了一步。",
            "有人把行囊挪到車轍外側，讓從糧倉外秤來的空車先過。",
            "南城外驛道短短響起車鈴聲，高牆陰影在路面上壓得更近。",
        ],
    },
    PolishedSceneText {
        name: "南城甕門",
        role: "門卒",
        crowd: "甕門人群",
        entry: "你走到南城甕門。高牆把外驛道的濕熱壓低，門卒按著通行木牌，讓車隊和步行人分道入城。",
        reentry: "你回到南城甕門。門洞陰影裡換了一批候檢腳步，門卒把木牌重新推到桌邊。",
        observation: "{pronoun}抬手攔住一輛車，低聲核對木牌後才讓它往門內集散廣場去。",
        micro: &[
            "門卒把木牌翻過一面，甕門裡的人聲立刻低了一層。",
            "有人在門洞下擦去額角汗水，等前面的車轍空出半步。",
            "南城甕門短短響起木門軸聲，外驛道的喧聲被高牆切開。",
        ],
    },
    PolishedSceneText {
        name: "門內集散廣場",
        role: "引路人",
        crowd: "廣場人群",
        entry: "你走進門內集散廣場。旗桿、候車牌和臨時貨架把入城人流拆成幾股，引路人站在石階邊指向中軸大街。",
        reentry: "你回到門內集散廣場。廣場上的人流換了方向，引路人把剛入城的車隊引到不擋主街的位置。",
        observation: "{pronoun}站在石階邊抬手指路，讓迷路的人先沿中軸大街南段往裡走。",
        micro: &[
            "引路人把候車牌往旗桿旁挪開，給從甕門進來的人讓出一條線。",
            "有人在臨時貨架前停住，又被人流慢慢推向中軸大街。",
            "門內集散廣場短短響起旗繩聲，南城甕門的腳步從身後湧進來。",
        ],
    },
    PolishedSceneText {
        name: "中軸大街南段",
        role: "攤主",
        crowd: "大街人群",
        entry: "你走上中軸大街南段。街聲沿石面拉長，攤主、挑夫和旗影把門內廣場的人流帶向更深處。",
        reentry: "你回到中軸大街南段。攤位前換了幾個問路的人，挑夫把貨擔貼著外區坊門那側慢慢挪過。",
        observation: "{pronoun}把攤布一角壓住，抬眼看門內集散廣場那邊又放進多少人。",
        micro: &[
            "攤主把濕布蓋回貨樣上，街上的腳步沒有停，只是分得更細。",
            "挑夫貼著外區坊門讓過一隊進城車馬，扁擔短短碰到牆邊。",
            "中軸大街南段短短響起叫賣聲，三水總商行長廊的低語跟著靠近。",
        ],
    },
    PolishedSceneText {
        name: "中軸大街中段",
        role: "巡街人",
        crowd: "大街人群",
        entry: "你走到中軸大街中段。兩側樓影把街聲壓得更密，巡街人沿旗繩下的陰影慢走，前方大十字通衢的車鈴聲一陣陣傳來。",
        reentry: "你回到中軸大街中段。街面上的水痕被腳步踩散，巡街人把擠到路心的人往側邊引。",
        observation: "{pronoun}沿著街側慢走，抬手讓挑擔人不要堵住往大十字通衢的路。",
        micro: &[
            "巡街人把旗繩下的路線讓出來，街面上的人流跟著分成兩股。",
            "有人在樓影下停了半息，又被後方腳步推向大十字通衢。",
            "中軸大街中段短短響起車鈴聲，南段的攤販聲被拋在後頭。",
        ],
    },
    PolishedSceneText {
        name: "外區坊門",
        role: "坊門看守",
        crowd: "坊門人群",
        entry: "你走到外區坊門。門牌、窄巷和曬衣竿把大城住戶日常藏在中軸大街一側，坊門看守讓熟路的人貼牆往裡進。",
        reentry: "你回到外區坊門。坊門下的影子換了角度，住戶把門板拉開一線又很快合上。",
        observation: "{pronoun}扶著坊門木柱，低聲問來人要找哪一條巷。",
        micro: &[
            "坊門看守把門牌擦亮一角，讓問路的人不要擋住大街。",
            "有人從窄巷裡探頭看了一眼，又把門板輕輕合回去。",
            "外區坊門短短飄出飯香，中軸大街的車鈴聲立刻把它蓋過。",
        ],
    },
    PolishedSceneText {
        name: "三水總商行長廊",
        role: "商會書手",
        crowd: "長廊人群",
        entry: "你走到三水總商行長廊。木欄後擺著貨樣匣、印章盒和長桌，商會書手把外城來的貨單收進陰影裡。",
        reentry: "你回到三水總商行長廊。長桌上的貨單換了一疊，商會書手把剛蓋好的印章推到一側風乾。",
        observation: "{pronoun}按著貨單低聲核對，偶爾抬頭看中軸大街南段的人流。",
        micro: &[
            "商會書手把印章盒合上半截，讓送單的人先站到木欄旁。",
            "有人把貨樣匣推近長桌，低聲問這批貨能不能轉進倉區。",
            "三水總商行長廊短短響起紙角聲，中軸大街的叫賣聲被木欄隔開。",
        ],
    },
    PolishedSceneText {
        name: "大十字通衢",
        role: "指路人",
        crowd: "通衢人群",
        entry: "你走到大十字通衢。車鈴、旗影和遠處官署鼓聲從四面交疊，指路人只把你帶到這裡，更多城區仍在街聲深處。",
        reentry: "你回到大十字通衢。十字口的人流又換了方向，指路人把猶豫的人先帶回中軸大街中段邊上。",
        observation: "{pronoun}站在十字口邊緣指路，沒有讓人群完全堵住中軸大街。",
        micro: &[
            "指路人把一隊挑擔人引到側邊，讓車馬先從路心慢慢過去。",
            "遠處官署鼓聲短短壓過街聲，十字口的人都抬頭聽了一息。",
            "大十字通衢短短響起四面回聲，更多城區仍被人潮藏在深處。",
        ],
    },
    PolishedSceneText {
        name: "城邦外坡道",
        role: "牽獸人",
        crowd: "坡道人群",
        entry: "你走上城邦外坡道。獸醫棚的藥草味退到身後，牽獸人把石粉、蹄印和車轍間的隊列排向石壘門。",
        reentry: "你回到城邦外坡道。候檢車隊又往坡上挪了一段，牽獸人把牧地來的牲口壓慢，免得撞上門前隊列。",
        observation: "{pronoun}按著牲口韁繩看向石壘門，讓候檢車隊先貼著坡面往前排。",
        micro: &[
            "牽獸人把韁繩繞緊一圈，坡道上的蹄聲跟著低下去。",
            "有人把車輪推離濕石粉，等石壘門那邊放出下一列。",
            "城邦外坡道短短響起車轍刮石聲，獸醫棚的低語被拋在後頭。",
        ],
    },
    PolishedSceneText {
        name: "石壘門",
        role: "門衛",
        crowd: "門前人群",
        entry: "你走到石壘門。矮厚石牆把坡道風聲壓住，門衛按著通行牌，將商隊、牧人和工坊送件分成幾列。",
        reentry: "你回到石壘門。鐵環和獸欄繩又被挪過位置，門衛把剛查過的木牌推向門內券道。",
        observation: "{pronoun}把通行牌翻到手邊，抬眼看坡道上下一列是否已排好。",
        micro: &[
            "門衛把鐵環從門柱上取下又掛回，候檢人群立刻安靜一截。",
            "有人把獸欄繩遞到門側，等門內券道讓出下一段陰影。",
            "石壘門短短響起木牌碰石聲，外坡道的車轍聲停了一息。",
        ],
    },
    PolishedSceneText {
        name: "門內券道",
        role: "引路人",
        crowd: "券道人群",
        entry: "你穿過門內券道。低牆掛著修補過的盾牌和候審木籤，引路人把過門的人流推向旗石廣場。",
        reentry: "你回到門內券道。券道陰影裡換了一批腳步，引路人把剛過門的車隊壓在牆邊慢行。",
        observation: "{pronoun}站在牆邊指向旗石廣場，讓初入城邦的人不要堵在券道口。",
        micro: &[
            "引路人把候審木籤往牆邊收，給石壘門進來的人讓出窄路。",
            "有人摸過牆上的舊盾牌，很快又被後方腳步推往廣場。",
            "門內券道短短回響一陣腳步，石壘門外的聲音被壓成低嗡。",
        ],
    },
    PolishedSceneText {
        name: "旗石廣場",
        role: "旗手",
        crowd: "廣場人群",
        entry: "你走進旗石廣場。舊旗把等候線分成幾股，旗手把門內券道來的人流指向西門坡道街、鑄幣巷口和傭工棚邊。",
        reentry: "你回到旗石廣場。旗桿下的隊列換了方向，旗手把剛到的商隊先壓到廣場側邊。",
        observation: "{pronoun}扶著旗桿指路，讓往坡街和傭工棚的人不要互相擠住。",
        micro: &[
            "旗手把舊旗扯直，廣場上的等候線跟著重新分開。",
            "有人從鑄幣巷口退回廣場側邊，低聲問下一步該排哪列。",
            "旗石廣場短短響起旗繩聲，門內券道的人流又湧進一小段。",
        ],
    },
    PolishedSceneText {
        name: "西門坡道街",
        role: "坡街販",
        crowd: "坡街人群",
        entry: "你走上西門坡道街。攤布、鐵件和馬具沿低檐掛開，坡街販把旗石廣場來的消息壓低聲傳向議棚前階。",
        reentry: "你回到西門坡道街。坡街上換了一撥問價的人，坡街販把剛聽見的議棚消息藏進攤布下。",
        observation: "{pronoun}按住攤布一角，側耳聽議棚前階那邊是否開始叫號。",
        micro: &[
            "坡街販把一串馬具掛回低檐下，讓往議棚前階的人貼牆走過。",
            "有人在坡街中段問價，又把目光轉回旗石廣場的隊列。",
            "西門坡道街短短響起鐵件碰撞聲，更深處的城邦低語貼著坡面傳來。",
        ],
    },
    PolishedSceneText {
        name: "議棚前階",
        role: "記牌人",
        crowd: "議棚人群",
        entry: "你走到議棚前階。木棚裡傳出低低議事聲，欠糧、傭工和通行牌告示貼在階邊，記牌人讓等候申訴的人站到側線。",
        reentry: "你回到議棚前階。階邊木牌換過一輪，記牌人把剛到的人名壓進短冊裡。",
        observation: "{pronoun}按著短冊看向坡街，抬手讓下一個等候者站到階側。",
        micro: &[
            "記牌人把木牌翻過一面，議棚前的人群立刻壓低聲音。",
            "有人看完欠糧告示，退到階側等棚內傳出下一句話。",
            "議棚前階短短響起木牌落桌聲，坡街上的腳步跟著慢下來。",
        ],
    },
    PolishedSceneText {
        name: "鑄幣巷口",
        role: "兌牌人",
        crowd: "巷口人群",
        entry: "你走到鑄幣巷口。半扇門內一閃而過銅秤、散鎂和封袋，兌牌人把要換票的人攔在旗石廣場側邊。",
        reentry: "你回到鑄幣巷口。門後銅秤聲又輕輕響了一下，兌牌人把新到的人往廣場線外推開。",
        observation: "{pronoun}守在半開門前，低聲核對牌號後才讓人靠近門檻。",
        micro: &[
            "兌牌人把封袋往門內推深，沒有讓排隊的人看清裡面的數目。",
            "有人在巷口摺好舊票，退回旗石廣場邊等下一次叫號。",
            "鑄幣巷口短短響起銅秤聲，半扇門又很快合回陰影裡。",
        ],
    },
    PolishedSceneText {
        name: "傭工棚邊",
        role: "棚頭人",
        crowd: "傭工人群",
        entry: "你走到傭工棚邊。短工牌和舊繩結掛在棚柱上，棚頭人把商隊、牧地和工坊活計拆成幾支小隊。",
        reentry: "你回到傭工棚邊。棚柱下換了一排等工的人，棚頭人把新掛出的短工牌往外推了一點。",
        observation: "{pronoun}站在棚柱旁點人，讓等工者按商隊、牧地和工坊活計分開。",
        micro: &[
            "棚頭人把短工牌翻過一片，等工的人群立刻往前擠了一步。",
            "有人把舊繩結繫回腰間，低聲問這趟是去牧地還是工坊。",
            "傭工棚邊短短響起木牌碰柱聲，旗石廣場那頭又送來一小隊人。",
        ],
    },
    PolishedSceneText {
        name: "邊城外門",
        role: "守門人",
        crowd: "邊門人群",
        entry: "你走到邊城外門。木門半開，通行牌壓在桌邊，守門人把從紅土崁選礦車場來的車隊攔在門影下。",
        reentry: "你回到邊城外門。門影下的商隊又往前挪了一截，守門人把通行牌重新翻到手邊。",
        observation: "{pronoun}按著通行牌，抬眼看商隊和紅土崁選礦車場方向的車輪痕。",
        micro: &[
            "守門人把通行牌翻過一面，示意商隊先停在門影外。",
            "有人把車上貨包往內側推了推，免得擋住哨樓梯下來的人。",
            "邊城外門短短響起木牌碰桌聲，門洞裡的人聲跟著低下去。",
        ],
    },
    PolishedSceneText {
        name: "哨樓梯",
        role: "哨兵",
        crowd: "哨樓人群",
        entry: "你踏上哨樓梯。旗繩貼著木欄抽動，哨兵在半高處看向邊城外門和乾草棚那邊的動靜。",
        reentry: "你回到哨樓梯。旗繩仍在風裡輕響，哨兵把剛收到的口令往樓上遞去。",
        observation: "{pronoun}扶著木欄看向城門外，指尖停在旗繩旁等下一個傳令手勢。",
        micro: &[
            "哨兵把旗繩往欄邊收緊，低聲把門外車隊數成幾段。",
            "有人從樓梯上探頭下望，又把口令傳回糧倉前場方向。",
            "哨樓梯短短響起木階聲，城門下的查驗聲被風切得斷續。",
        ],
    },
    PolishedSceneText {
        name: "青籠關邊貿坪",
        role: "商旅",
        crowd: "邊市人群",
        entry: "你走到青籠關邊貿坪。臨時攤布貼著低牆鋪開，商旅把小件貨和路上消息都壓低聲交出去。",
        reentry: "你回到青籠關邊貿坪。攤布旁換了一撥看貨的人，商旅把剛問到的價碼送往糧倉前場。",
        observation: "{pronoun}把小件貨攤開一角，低聲問城門那邊今天還放不放車隊進來。",
        micro: &[
            "商旅把包袱口解開一半，又立刻用手掌壓住攤布邊。",
            "有人在青籠關邊貿坪比了比價，眼角仍看著邊城外門的查驗。",
            "青籠關邊貿坪短短傳過一陣議價聲，很快被糧倉前場的推車聲壓低。",
        ],
    },
    PolishedSceneText {
        name: "糧倉前場",
        role: "倉管",
        crowd: "糧倉人群",
        entry: "你來到糧倉前場。邊城糧袋靠著牆腳排開，倉管按著冊簿，把補給車往巡邏院方向分出去。",
        reentry: "你回到糧倉前場。糧袋堆又換了高度，倉管把新到的車次補進冊簿邊欄。",
        observation: "{pronoun}按著冊簿核對糧袋，抬手讓一輛補給車先往巡邏院那邊靠。",
        micro: &[
            "倉管把冊簿頁角壓平，示意推車先停在門側。",
            "有人把糧袋重新疊高，讓哨樓梯下來的人能穿過前場。",
            "糧倉前場短短響起推車木輪聲，青籠關邊貿坪的低語被切成幾段。",
        ],
    },
    PolishedSceneText {
        name: "乾草棚",
        role: "馬夫",
        crowd: "乾草棚人群",
        entry: "你走進乾草棚旁。草捆靠著棚柱堆高，馬夫把乾草抖鬆，讓等在邊門外的牲口先安靜下來。",
        reentry: "你回到乾草棚旁。棚柱邊又多了幾束乾草，馬夫把沾土的繩索重新掛好。",
        observation: "{pronoun}抖開一束乾草，側耳聽巡邏院那邊是否又有人牽馬過來。",
        micro: &[
            "馬夫把草捆拖到棚口，給從哨樓梯過來的人讓出窄路。",
            "有人檢查畜繩是否鬆開，乾草屑落在靴邊一圈。",
            "乾草棚短短響起草叉碰柱聲，巡邏院那邊的馬鼻息跟著靠近。",
        ],
    },
    PolishedSceneText {
        name: "巡邏院",
        role: "巡哨",
        crowd: "巡邏人群",
        entry: "你走進巡邏院。兵器架貼著院牆，巡哨把值牌交到下一班手裡，拘留繩掛在陰影處不晃。",
        reentry: "你回到巡邏院。值牌的位置已經換過，巡哨把剛回來的腳步聲收進院牆內。",
        observation: "{pronoun}把值牌掛回木釘旁，回頭看兵器架和乾草棚方向的動靜。",
        micro: &[
            "巡哨把短槍從架上取下又放回，像是在等下一道口令。",
            "有人在院牆邊低聲交班，拘留繩旁的影子短短晃了一下。",
            "巡邏院短短響起值牌碰木聲，糧倉前場的推車聲便低下去。",
        ],
    },
    PolishedSceneText {
        name: "工坊街口",
        role: "學徒",
        crowd: "工坊人群",
        entry: "你走到工坊街口。路牌被灰粉沾暗，學徒抱著料件在鍛造棚、料場秤台和臨河歇腳處之間穿行。",
        reentry: "你回到工坊街口。路牌下又堆了幾件待修工具，學徒把剛問到的料價往鍛造棚送。",
        observation: "{pronoun}抱著一捆料件等路口讓開，眼神在鍛造棚和料場秤台之間來回。",
        micro: &[
            "學徒抱著鐵胚從街口穿過，差點撞上從料場秤台來的買料人。",
            "有人在路牌下短短問價，回頭就把聲音送進鍛造棚。",
            "工坊街口短短響起灰粉腳步聲，臨河歇腳處的人聲被留在後方。",
        ],
    },
    PolishedSceneText {
        name: "鍛造棚",
        role: "鐵匠",
        crowd: "鍛造人群",
        entry: "你走到鍛造棚下。火爐紅光貼著棚柱跳動，鐵匠把鐵胚翻到砧面上，錘聲壓過街口人語。",
        reentry: "你回到鍛造棚下。火爐仍亮著，鐵匠把剛退火的鐵件推到石粉堆那側散熱。",
        observation: "{pronoun}用火鉗翻過鐵胚，等顏色到位才把錘子落到砧面。",
        micro: &[
            "鐵匠把鐵胚夾出爐口，棚下立刻多了一層熱浪。",
            "有人把修車棚送來的輪軸擱到砧旁，等下一輪火候。",
            "鍛造棚短短響起重錘聲，石粉堆那邊的灰粉跟著震落。",
        ],
    },
    PolishedSceneText {
        name: "料場秤台",
        role: "秤手",
        crowd: "料場人群",
        entry: "你來到料場秤台。石材和礦料堆在秤邊，秤手把記帳板壓在臂下，等買料人報出下一車去向。",
        reentry: "你回到料場秤台。秤盤旁多了新灰痕，秤手把一筆料重補到記帳板上。",
        observation: "{pronoun}扶著秤桿看刻痕，讓搬料人先把石材堆到秤台外側。",
        micro: &[
            "秤手把石料推上秤盤，指尖在刻痕旁停了一下。",
            "有人把記帳板翻到新頁，讓修車棚那邊的料車先記下。",
            "料場秤台短短響起秤錘聲，工坊街口的人群讓出半步。",
        ],
    },
    PolishedSceneText {
        name: "修車棚",
        role: "修車工",
        crowd: "修車人群",
        entry: "你走進修車棚前。木輪架佔住半邊棚口，修車工把輪軸墊高，讓外路來的車先卸下破輪。",
        reentry: "你回到修車棚前。輪軸被重新上油，修車工把修好的木輪推回棚外等人領走。",
        observation: "{pronoun}半蹲在車輪旁，拿木槌把輪軸鬆動處重新敲緊。",
        micro: &[
            "修車工把木輪翻到架上，輪緣灰粉落了一地。",
            "有人把油布庫送來的繩圈掛在棚柱旁，等下一輛車進來。",
            "修車棚短短響起輪軸磨聲，鍛造棚的火聲被棚頂壓低。",
        ],
    },
    PolishedSceneText {
        name: "石粉堆",
        role: "搬料人",
        crowd: "石粉堆人群",
        entry: "你停在石粉堆旁。灰白粉塵沿遮布邊緣散開，搬料人把切下的碎料往牧地柵門方向分出一條窄路。",
        reentry: "你回到石粉堆旁。遮布又被風掀開一角，搬料人把新倒下的灰粉壓回堆內。",
        observation: "{pronoun}用鏟背壓住灰粉邊緣，抬手提醒過路的人避開遮布下風口。",
        micro: &[
            "搬料人把灰粉往堆內推回，鞋邊立刻覆上一層白痕。",
            "有人拉住遮布角，讓通往牧地柵門的路先清出來。",
            "石粉堆短短揚起一陣灰霧，鍛造棚那邊的火光變得發白。",
        ],
    },
    PolishedSceneText {
        name: "油布庫",
        role: "看庫人",
        crowd: "油布庫人群",
        entry: "你走到油布庫前。油布捲和繩圈靠牆疊著，看庫人把防雨料分給修車棚，棚內氣味厚而悶。",
        reentry: "你回到油布庫前。牆邊油布捲少了一束，看庫人把剩下的繩圈重新按大小掛起。",
        observation: "{pronoun}把油布捲拍平，低聲核對哪一束要送去修車棚。",
        micro: &[
            "看庫人把繩圈從牆鉤上取下，指尖沾了一點油黑。",
            "有人把剛退回的油布攤開，檢查破口是不是還能補。",
            "油布庫短短響起繩圈擦牆聲，石粉堆的灰味被擋在門外。",
        ],
    },
    PolishedSceneText {
        name: "牧地柵門",
        role: "牧人",
        crowd: "牧地人群",
        entry: "你走到牧地柵門。畜牌掛在欄木上，牧人把柵門開出一條縫，讓牲口從石粉堆那頭慢慢轉進草地。",
        reentry: "你回到牧地柵門。柵欄邊多了幾串泥蹄印，牧人把畜牌重新掛正。",
        observation: "{pronoun}扶著柵門數過畜牌，又回頭看水槽棚那邊是否空出位置。",
        micro: &[
            "牧人把柵門推開半扇，讓一小群牲口先貼著欄木進來。",
            "有人把畜牌換到另一根繩上，免得被水槽棚過來的人碰掉。",
            "牧地柵門短短響起欄木聲，石粉堆的灰味在風裡淡下去。",
        ],
    },
    PolishedSceneText {
        name: "水槽棚",
        role: "牧童",
        crowd: "水槽人群",
        entry: "你走進水槽棚旁。長水槽映著泥蹄印，牧童把水桶拖到槽邊，讓剛進柵門的牲口輪流靠近。",
        reentry: "你回到水槽棚旁。槽邊又多了濕蹄印，牧童把空桶翻過來靠在棚柱下。",
        observation: "{pronoun}把水桶拖到槽邊，伸手攔住太急著擠上來的牲口。",
        micro: &[
            "牧童把水桶抬到槽口，水面立刻被鼻息吹出細紋。",
            "有人拿木刷刮過水槽邊，讓獸醫棚那邊牽來的牲口先喝。",
            "水槽棚短短響起桶底碰木聲，畜欄口的低哞聲跟著靠近。",
        ],
    },
    PolishedSceneText {
        name: "乾草堆",
        role: "搬草人",
        crowd: "乾草堆人群",
        entry: "你走到乾草堆旁。篷布壓住草捆一角，搬草人把乾草拆成小束，送往畜欄口和柵門邊。",
        reentry: "你回到乾草堆旁。篷布被重新壓緊，搬草人把散落草屑掃回堆下。",
        observation: "{pronoun}拆開一束乾草，抬手讓畜欄口那邊先接過去。",
        micro: &[
            "搬草人把草捆放到肩上，草屑一路落到畜欄口方向。",
            "有人把篷布角壓回石塊下，免得風把乾草掀散。",
            "乾草堆短短響起草繩拉緊聲，牧地柵門那頭的牲口安靜些。",
        ],
    },
    PolishedSceneText {
        name: "畜欄口",
        role: "牲口販",
        crowd: "畜欄人群",
        entry: "你來到畜欄口。繩索繞在欄柱上，牲口販摸過畜背和蹄口，低聲把價碼壓到乾草堆那邊。",
        reentry: "你回到畜欄口。欄柱邊換了幾道繩結，牲口販把剛看過的畜牌收回袖口。",
        observation: "{pronoun}拉住繩索看蹄口，側身讓水槽棚牽來的牲口先進欄。",
        micro: &[
            "牲口販把繩索繞回欄柱，低聲說了個價碼又立刻收住。",
            "有人拍了拍畜背，讓買主從季節草場方向繞過來看。",
            "畜欄口短短響起繩扣聲，水槽棚的桶聲被擋在欄外。",
        ],
    },
    PolishedSceneText {
        name: "獸醫棚",
        role: "獸醫",
        crowd: "獸醫棚人群",
        entry: "你走進獸醫棚前。藥箱和繃帶擺在矮桌上，獸醫把受傷的牲口牽到水槽棚旁的陰影裡查看。",
        reentry: "你回到獸醫棚前。矮桌上的藥箱換了位置，獸醫把沾草屑的繃帶捲回箱內。",
        observation: "{pronoun}翻開藥箱，低頭查看牲口腿上的舊傷和新泥印。",
        micro: &[
            "獸醫把藥箱拉近，指尖按過牲口腿上的腫處。",
            "有人把繃帶遞到棚口，等牧場門牌那邊牽來下一匹牲口。",
            "獸醫棚短短響起藥瓶碰箱聲，水槽棚的低哞聲被壓住。",
        ],
    },
    PolishedSceneText {
        name: "季節草場",
        role: "牧人",
        crowd: "草場人群",
        entry: "你走進季節草場。風草貼著低坡起伏，牧人把畜群壓在邊線內，遠哨影子在草色後方一閃。",
        reentry: "你回到季節草場。草線被新蹄印切開，牧人把離隊的牲口慢慢趕回畜欄口方向。",
        observation: "{pronoun}站在風草邊看畜群散向哪裡，手裡的繩結一直沒有鬆開。",
        micro: &[
            "牧人把離隊的牲口攔回草線內，風草在蹄邊翻出亮面。",
            "有人指向遠哨影子，讓畜欄口那邊暫時不要放新群出來。",
            "季節草場短短響起蹄聲，獸醫棚方向的藥箱聲很快遠下去。",
        ],
    },
    PolishedSceneText {
        name: "牧場門牌",
        role: "牧場管事",
        crowd: "牧場人群",
        entry: "你走到牧場門牌下。門牌被馬蹄泥點打暗，牧場管事把進出名冊夾在臂下，看著獸醫棚方向的牽繩。",
        reentry: "你回到牧場門牌下。門牌旁新添了幾道蹄印，牧場管事把名冊翻到下一頁。",
        observation: "{pronoun}按著進出名冊，抬手讓馬欄前場那邊先空出一條路。",
        micro: &[
            "牧場管事把門牌擦出一角，低聲核對剛進來的馬匹。",
            "有人把牽繩掛到門柱旁，等獸醫棚那邊點頭再放行。",
            "牧場門牌短短響起木牌碰柱聲，馬欄前場的水桶聲跟著靠近。",
        ],
    },
    PolishedSceneText {
        name: "馬欄前場",
        role: "馬夫",
        crowd: "馬欄人群",
        entry: "你來到馬欄前場。水桶擺在欄邊，馬夫摸過馬頸和蹄口，讓挑選的人先站在欄外看。",
        reentry: "你回到馬欄前場。水桶邊多了新濕痕，馬夫把一匹躁動的馬牽回欄內。",
        observation: "{pronoun}扶著馬頸讓牠站穩，回頭看獸醫點那邊是否要再檢一次蹄口。",
        micro: &[
            "馬夫把水桶推近欄邊，讓剛牽回來的馬低頭喝水。",
            "有人撫過馬背給買主看，蹄聲在前場短短踏響。",
            "馬欄前場短短響起水桶碰欄聲，牧場門牌下的人群讓出一步。",
        ],
    },
    PolishedSceneText {
        name: "草料棚",
        role: "草料工",
        crowd: "草料棚人群",
        entry: "你走到草料棚。乾草和豆料分成幾堆，草料工把草叉靠在棚柱邊，等商隊泊位報出下一批牲口數。",
        reentry: "你回到草料棚。草料堆被挪開一角，草料工把剛灑出的豆料掃回木斗裡。",
        observation: "{pronoun}用草叉翻開草料，側耳聽邊鎮井口方向是否又有車隊靠近。",
        micro: &[
            "草料工把草叉插進草堆，抖出一片乾響。",
            "有人把豆料木斗推到棚口，等商隊泊位那邊來領。",
            "草料棚短短響起草叉碰柱聲，牧場門牌下的蹄聲低了些。",
        ],
    },
    PolishedSceneText {
        name: "獸醫點",
        role: "獸醫",
        crowd: "獸醫點人群",
        entry: "你走到獸醫點。藥袋掛在欄邊，獸醫低頭查看馬腿，旁邊有人按著繃帶等下一步吩咐。",
        reentry: "你回到獸醫點。欄邊藥袋被打開過，獸醫把新換下的繃帶放到木盆裡。",
        observation: "{pronoun}沿著馬腿摸到蹄口，低聲讓馬夫把繩子放鬆一點。",
        micro: &[
            "獸醫把藥袋打開，從裡頭取出一卷窄繃帶。",
            "有人扶住馬頸，等獸醫點頭後才敢把蹄子放下。",
            "獸醫點短短響起藥瓶碰袋聲，馬欄前場的人聲立刻低了些。",
        ],
    },
    PolishedSceneText {
        name: "馱具修棚",
        role: "修具工",
        crowd: "馱具人群",
        entry: "你走進馱具修棚前。皮帶和銅扣吊在棚梁下，修具工把裂開的鞍帶壓平，等馬夫來試長短。",
        reentry: "你回到馱具修棚前。棚梁下換了一排皮帶，修具工把修好的銅扣推到商隊泊位那側。",
        observation: "{pronoun}把皮帶壓在木墊上，拿銅扣對過孔眼後才重新穿繩。",
        micro: &[
            "修具工把銅扣敲緊，皮帶邊緣短短彈了一下。",
            "有人把商隊泊位送來的斷繩丟到棚口，等下一輪修補。",
            "馱具修棚短短響起皮革擦木聲，馬欄前場的蹄聲被隔在外頭。",
        ],
    },
    PolishedSceneText {
        name: "商隊泊位",
        role: "商隊人",
        crowd: "商隊人群",
        entry: "你走到商隊泊位。馱獸和貨包排在泊位邊，商隊人把繩結重新收緊，等草料棚送來下一批補給。",
        reentry: "你回到商隊泊位。貨包位置換了幾處，商隊人把剛修好的馱具扣回馱獸身上。",
        observation: "{pronoun}拉緊馱獸旁的繩結，抬頭看草料棚那邊補給是否到了。",
        micro: &[
            "商隊人把貨包往馱獸背上推正，繩結被拉得短短作響。",
            "有人在泊位邊清點水囊，等馱具修棚把最後一副皮帶送來。",
            "商隊泊位短短響起貨包落鞍聲，獸醫點那邊的低語被蓋住。",
        ],
    },
    PolishedSceneText {
        name: "邊鎮井口",
        role: "取水人",
        crowd: "井口人群",
        entry: "你走到邊鎮井口。井繩濕亮地垂著，取水人把水桶拉上井沿，路旗在低風裡指向草料棚和門洞。",
        reentry: "你回到邊鎮井口。井沿旁又多了水痕，取水人把滿桶推給剛到的旅人。",
        observation: "{pronoun}把井繩繞到掌心，等水桶穩住後才往井沿上拉。",
        micro: &[
            "取水人把井桶提上來，水聲從桶沿灑到石面。",
            "有人把水囊遞到井邊，回頭看低牆門洞那邊是否還在查人。",
            "邊鎮井口短短響起井繩聲，旅社前廊的人聲跟著靠近。",
        ],
    },
    PolishedSceneText {
        name: "低牆門洞",
        role: "守門人",
        crowd: "門洞人群",
        entry: "你走到低牆門洞。木拒馬卡在門邊，守門人把旅人的水囊和貨繩看過一遍才放人進鎮。",
        reentry: "你回到低牆門洞。拒馬旁多了車轍泥，守門人把剛收下的路牌掛回牆釘。",
        observation: "{pronoun}扶著木拒馬，看過貨繩和路牌後才讓車隊往補給市集移動。",
        micro: &[
            "守門人把木拒馬往旁邊挪開半尺，讓一輛小車慢慢過洞。",
            "有人把路牌遞到門洞陰影裡，等巡哨棚那邊回話。",
            "低牆門洞短短響起木拒馬拖地聲，井口的水聲被擋在牆外。",
        ],
    },
    PolishedSceneText {
        name: "旅社前廊",
        role: "接待員",
        crowd: "旅社人群",
        entry: "你走到旅社前廊。長凳靠著廊柱，接待員把熱水壺放到桌邊，等外路旅人卸下包袱。",
        reentry: "你回到旅社前廊。長凳上換了一撥歇腳的人，接待員把空碗收進門內。",
        observation: "{pronoun}端著熱水壺穿過長凳邊，低聲問旅人要不要先歇腳。",
        micro: &[
            "接待員把熱水壺放到廊桌上，讓剛進鎮的人先暖手。",
            "有人把包袱靠到長凳下，抬頭問補給市集今天還開到多晚。",
            "旅社前廊短短響起碗底碰桌聲，井口那邊的取水聲遠了一點。",
        ],
    },
    PolishedSceneText {
        name: "補給市集",
        role: "小販",
        crowd: "市集人群",
        entry: "你走進補給市集。鹽袋、水囊和乾糧包排在攤邊，小販把外路價碼壓低聲報給車隊場的人。",
        reentry: "你回到補給市集。攤邊水囊少了幾只，小販把新補的鹽袋挪到前排。",
        observation: "{pronoun}把水囊掛到攤前，低聲把鹽袋價碼報給等車的人。",
        micro: &[
            "小販把鹽袋拍實，攤布邊立刻落下一圈白粉。",
            "有人把水囊拿起來掂量，又回頭問車隊場還缺多少補給。",
            "補給市集短短響起乾糧包落桌聲，旅社前廊的談話停了一下。",
        ],
    },
    PolishedSceneText {
        name: "巡哨棚",
        role: "巡哨",
        crowd: "巡哨人群",
        entry: "你走到巡哨棚。槍架靠著棚壁，巡哨把值牌換到腰間，目光一直掃向低牆門洞和車隊場。",
        reentry: "你回到巡哨棚。槍架旁的值牌換了位置，巡哨把剛聽到的外路消息壓低聲傳下去。",
        observation: "{pronoun}把值牌扣回腰間，側身看低牆門洞外是否尚有人滯留。",
        micro: &[
            "巡哨把短槍從架上抬起又放回，棚下的人聲立刻收小。",
            "有人把外路消息說到一半，就被值牌碰木的聲音打斷。",
            "巡哨棚短短響起槍架聲，車隊場那邊的車輪跟著慢下來。",
        ],
    },
    PolishedSceneText {
        name: "車隊場",
        role: "車夫",
        crowd: "車隊人群",
        entry: "你走到車隊場。車轍在場地裡交錯，車夫把貨繩重新收緊，等補給市集把最後一袋鹽送來。",
        reentry: "你回到車隊場。貨繩被重新結過，車夫把下一輛要出低牆門洞的車推到前排。",
        observation: "{pronoun}扶著車轅檢查貨繩，回頭等巡哨棚放出下一道通行手勢。",
        micro: &[
            "車夫把貨繩往車架下收緊，輪旁泥水被踩得發亮。",
            "有人把剛買來的水囊丟上車，等補給市集補齊乾糧。",
            "車隊場短短響起車軸聲，巡哨棚那邊的值牌聲跟著靠近。",
        ],
    },
];

fn final_map_scene_for_room(room_name: &str) -> Option<&'static PolishedSceneText> {
    FINAL_MAP_SCENE_TEXTS
        .iter()
        .find(|scene| room_name.contains(scene.name))
}

pub fn final_map_public_role_for_room(room_name: &str) -> Option<&'static str> {
    final_map_scene_for_room(room_name).map(|scene| scene.role)
}

pub fn final_map_crowd_label_for_room(room_name: &str) -> Option<&'static str> {
    final_map_scene_for_room(room_name).map(|scene| scene.crowd)
}

pub fn final_map_entry_log_for_room(room_name: &str) -> Option<&'static str> {
    final_map_scene_for_room(room_name).map(|scene| scene.entry)
}

pub fn final_map_reentry_log_for_room(room_name: &str) -> Option<&'static str> {
    final_map_scene_for_room(room_name).map(|scene| scene.reentry)
}

pub fn final_map_observation_template_for_room(room_name: &str) -> Option<&'static str> {
    final_map_scene_for_room(room_name).map(|scene| scene.observation)
}

pub fn player_layer_scene_objects_for_room(room_name: &str) -> Vec<RoomObject> {
    let clean_buf = room_name.replace("文曜・文曜", "文曜・").replace("文曜文曜", "文曜");
    let room_name = clean_buf.as_str();
    let is_m3_020 = M3_020_SCENE_OBJECTS.iter().any(|obj| room_name.contains(obj.room));
    let mut objects: Vec<RoomObject> = M3_020_SCENE_OBJECTS
        .iter()
        .filter(|obj| room_name.contains(obj.room))
        .map(|obj| {
            let mut responses = HashMap::new();
            responses.insert("Look".to_string(), obj.look.to_string());
            responses.insert(obj.followup_action.to_string(), obj.followup.to_string());
            let mut sockets = vec!["Look".to_string(), obj.followup_action.to_string()];
            if obj.id == "m3_020_stone_gate_pass_tags" {
                sockets.push("JoinCheck".to_string());
            }
            if obj.id == "m3_020_mint_half_open_door" {
                sockets.push("Queue".to_string());
            }
            if obj.id == "m3_020_hire_short_work_boards" {
                sockets.push("AcceptJob".to_string());
            }
            if obj.id == "m3_020_council_notice_strip" {
                sockets.push("RegisterComplaint".to_string());
            }
            RoomObject {
                id: obj.id.to_string(),
                name: obj.name.to_string(),
                owner: String::new(),
                sockets,
                responses,
                move_to_room_id: String::new(),
            }
        })
        .collect();

    if is_m3_020 {
        return objects;
    }

    objects.extend(
        M3_027_LIFE_OBJECTS
            .iter()
            .filter(|obj| {
                room_name.contains(obj.room)
                    || (obj.room == "半嶺水岸渡口"
                        && (room_name.contains("水岸渡口")
                            || room_name.contains("水尾寮渡口")
                            || room_name.contains("水尾寮渡頭")))
                    || (obj.room == "水尾寮渡頭市集" && (room_name.contains("水尾寮渡頭市集") || room_name.contains("渡頭市集")))
            })
            .map(|obj| {
                let mut responses = HashMap::new();
                responses.insert("Look".to_string(), obj.look.to_string());
                let mut sockets = vec!["Look".to_string()];
                for (action, response) in obj.actions {
                    sockets.push((*action).to_string());
                    responses.insert((*action).to_string(), (*response).to_string());
                }
                RoomObject {
                    id: obj.id.to_string(),
                    name: obj.name.to_string(),
                    owner: String::new(),
                    sockets,
                    responses,
                    move_to_room_id: String::new(),
                }
            }),
    );
    if objects.is_empty() {
        objects = white_tower_peninsula_objects_for_room(room_name);
        objects.truncate(6);
    }

    let is_firewood_venue = room_name.contains("鋸木")
        || room_name.contains("木作")
        || room_name.contains("巧木")
        || room_name.contains("炭窯")
        || room_name.contains("竹木場")
        || room_name.contains("林工")
        || room_name.contains("柴");
    if is_firewood_venue && !objects.iter().any(|o| o.id == "wt_multi_firewood_pile") {
        let mut responses = HashMap::new();
        responses.insert("Look".to_string(), "整齊碼放於簷下或角隅的風乾柴木與硬木料垛，木質乾燥密實，既是用以生火禦寒之良材，亦能順紋劈柴、裁鋸木板、進爐悶燒精炭或車削長柄。".to_string());
        responses.insert("Gather".to_string(), "你自木料垛中細細翻揀，抱出一根紋理密實的伐採原木與兩束乾燥柴枝收整妥當。".to_string());
        responses.insert("MakeFire".to_string(), "你取出乾燥柴枝引燃火星，在空地中央架起通風柴堆，迅速生起一堆溫暖明亮的野地營火。".to_string());
        responses.insert("SplitWood".to_string(), "你運力順著木紋利索剖劈，將原木細細剖分劈成易於引火的乾燥柴枝束。".to_string());
        responses.insert("RefinePlank".to_string(), "你推動長鋸順紋切削，將沉實原木裁鋸打磨為平整規整的樟松木板料。".to_string());
        responses.insert("BurnCharcoal".to_string(), "你將截切硬木置入坑窯泥封悶燒，凝煉出無煙透亮的硬木白炭。".to_string());
        responses.insert("CraftTorch".to_string(), "你挑選油脂豐富的松木枝條塗抹脂膏緊密紮裹，製成一柄照明驅寒的浸脂松明火把。".to_string());
        responses.insert("RepairStructure".to_string(), "你挑選合適的木板料與工具長柄，加固修整破損的欄杆踏板與設施構件。".to_string());

        objects.push(RoomObject {
            id: "wt_multi_firewood_pile".to_string(),
            name: "風乾柴木垛".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Gather".to_string(),
                "MakeFire".to_string(),
                "SplitWood".to_string(),
                "RefinePlank".to_string(),
                "BurnCharcoal".to_string(),
                "CraftTorch".to_string(),
                "RepairStructure".to_string(),
            ],
            responses,
            move_to_room_id: String::new(),
        });
    }

    if let Some(_cf) = crate::game::object_ecology::get_active_campfire(room_name) {
        if !objects.iter().any(|o| o.id == "dyn_campfire") {
            let mut responses = HashMap::new();
            responses.insert("Look".to_string(), "粗壯硬柴交錯架起的野地營火堆正熊熊燃燒，金紅火苗隨風輕躍，散發著令人心神安定的融融暖意與松木焦香。".to_string());
            responses.insert("WarmHands".to_string(), "你湊近營火伸出雙手，融融暖流自掌心順著經脈蔓延至周身四肢，驅散了荒野寒濕（體魄與元氣獲得恢復）。".to_string());
            responses.insert("RoastFood".to_string(), "你取出行囊中的乾糧肉脯架於熾熱柴炭上方炙烤，油脂滋滋作響、肉香四溢，趁熱嚼食腹中溫暖充實（深幅恢復體魄與元氣）。".to_string());
            responses.insert("CraftTorch".to_string(), "你自營火堆中抽出一根引燃的松脂硬木，細心纏裹封固，製成一柄劈啪作響的照明松明火把。".to_string());
            responses.insert("AddFuel".to_string(), "你拾起乾柴添入火堆中央，劈啪脆響中火勢陡然騰起，騰躍的金紅火舌將營地周遭照耀得一片透亮。".to_string());
            responses.insert("CollectCharcoal".to_string(), "你以長枝撥開外層浮灰，將底層悶燒透紅的硬木火炭小心夾出、封土隔氧冷卻，收整出成色極佳的無煙白炭。".to_string());

            objects.push(RoomObject {
                id: "dyn_campfire".to_string(),
                name: "野地營火堆".to_string(),
                owner: String::new(),
                sockets: vec![
                    "Look".to_string(),
                    "WarmHands".to_string(),
                    "RoastFood".to_string(),
                    "CraftTorch".to_string(),
                    "AddFuel".to_string(),
                    "CollectCharcoal".to_string(),
                ],
                responses,
                move_to_room_id: String::new(),
            });
        }
    }
    let field_objs = crate::game::object_ecology::active_field_facility_objects(room_name, crate::game::now_unix());
    for f in field_objs {
        if !objects.iter().any(|o| o.id == f.id) {
            objects.push(f);
        }
    }

    objects
}
pub fn resolve_player_scene_objects(room_name: &str) -> Vec<RoomObject> {
    player_layer_scene_objects_for_room(room_name)
}
pub fn is_wenyao_residential_or_study(room_name: &str) -> bool {
    let mut clean_name = room_name.trim();
    const SETTLEMENT_PREFIXES: &[&str] = &[
        "文曜・", "文曜",
        "風舶角・", "風舶角",
        "風舶嶼・", "風舶嶼",
        "大圳頭・", "大圳頭",
        "半嶺鎮・", "半嶺鎮",
        "南哨・", "南哨",
        "老堰頭・", "老堰頭",
        "北嶺腳・", "北嶺腳",
        "霜林口・", "霜林口",
        "風櫃尾・", "風櫃尾",
        "烏石鼻・", "烏石鼻",
        "牛埔・", "牛埔",
        "紅土崁・", "紅土崁",
        "三號坑口・", "三號坑口",
        "水尾寮・", "水尾寮",
    ];
    let mut changed = true;
    while changed {
        changed = false;
        for p in SETTLEMENT_PREFIXES {
            if let Some(rest) = clean_name.strip_prefix(p) {
                clean_name = rest.trim();
                changed = true;
                break;
            }
        }
    }
    const WARD_PREFIXES: &[&str] = &[
        "講堂外坊・", "講堂外坊", "三水外坊・", "三水外坊",
        "百工外坊・", "百工外坊", "西門外坊・", "西門外坊",
        "講堂街・", "講堂街", "三水口・", "三水口",
        "分家巷・", "分家巷", "西門・", "西門",
    ];
    for p in WARD_PREFIXES {
        if let Some(rest) = clean_name.strip_prefix(p) {
            clean_name = rest.trim();
        }
    }

    const EXCLUDE: &[&str] = &[
        "旅社", "驛館", "客棧", "茶屋", "茶棚", "浴場", "商行", "學舍", "試招院", "公議", "工坊", "作坊",
        "書院", "外院", "醫館", "藥棧", "大埠", "拍賣", "大鍛坊", "鋸木廠", "精煉廠", "木炭窯",
        "磚瓦窯", "工具行", "織造坊", "五金鋪", "造船廠", "竹木場", "大貨棧", "中轉站", "拍賣行",
        "鹽鐵庫", "修造場", "獸醫館", "武備鋪", "酒肆", "印書局", "裝幀鋪", "文具行", "觀測台",
        "傳習所", "哨所", "公所", "診所",
        "鐘樓", "望野樓", "哨樓", "礦務所", "觀測所", "瞭望所", "中繼站", "調配站", "調撥處",
        "集散行", "收購行", "特產行", "物資行", "總社", "公會",
    ];
    if EXCLUDE.iter().any(|ex| clean_name.contains(ex)) {
        return false;
    }

    const CONTAIN: &[&str] = &[
        "住宅", "民宅", "居所", "家屋", "宅邸", "宅院", "民舍", "人家", "小築", "宿所",
        "學寮", "員寮", "山房", "養馬莊", "莊園",
    ];
    if CONTAIN.iter().any(|kw| clean_name.contains(kw)) {
        return true;
    }

    const END_CHARS: &[char] = &['居', '宅', '齋', '廬', '院', '舍', '家', '築', '所', '寮', '房', '莊', '屋', '樓'];
    if let Some(last_ch) = clean_name.chars().last() {
        if END_CHARS.contains(&last_ch) {
            return true;
        }
    }
    false
}

fn make_peninsula_object(
    id: &str,
    name: &str,
    sockets: &[&str],
    responses: &[(&str, &str)],
) -> RoomObject {
    let mut resp_map = std::collections::HashMap::new();
    for (k, v) in responses {
        resp_map.insert((*k).to_string(), (*v).to_string());
    }
    RoomObject {
        id: id.to_string(),
        name: name.to_string(),
        owner: String::new(),
        sockets: sockets.iter().map(|s| (*s).to_string()).collect(),
        responses: resp_map,
        move_to_room_id: String::new(),
    }
}

pub fn is_white_tower_peninsula_room(room_name: &str) -> bool {
    const PENINSULA_PLACES: &[&str] = &[
        "文曜", "風舶", "大圳頭", "半嶺", "南哨", "老堰頭", "北嶺",
        "霜林", "風櫃尾", "烏石鼻", "牛埔", "紅土崁", "三號坑口", "水尾寮",
        "白塔",
    ];
    PENINSULA_PLACES.iter().any(|p| room_name.contains(p))
}

pub fn white_tower_peninsula_objects_for_room(room_name: &str) -> Vec<RoomObject> {
    let clean_buf = room_name.replace("文曜・文曜", "文曜・").replace("文曜文曜", "文曜");
    let room_name = clean_buf.as_str();
    const WILDERNESS_CELL_MARKERS: &[&str] = &[
        "・草原", "・草甸", "・密林", "・山脊", "・幽沼", "・泥澤", "・冰峰", "・雪巔", "・雪林",
        "・海灣", "・深水", "・沙礫", "・巨岩", "・楓林", "・清流", "・草場", "・山隘", "・海岬",
        "・古徑", "・岩岸", "・淺灣", "・礫灣", "・溪谷", "・雲杉", "・林道", "・泥口", "・深潭",
        "・礦谷", "・礦坡", "・峭壁",
    ];
    if WILDERNESS_CELL_MARKERS.iter().any(|m| room_name.ends_with(m)) {
        return Vec::new();
    }

    if room_name == "天然港灣跨海長橋" {
        return vec![make_peninsula_object(
            "wt_bridge_harbor_causeway",
            "防風航標燈",
            &["Look", "Rest", "Operate"],
            &[
                ("Look", "跨海石橋橫臥於碧波海灣之上，厚實防風石欄抵禦著長風與浪沫，石欄轉折處設有旋蓋夜光貝母航標燈，指引往來舟楫與行旅。"),
                ("Rest", "你立於跨海長橋迎風的石欄旁稍作停歇，浩蕩海風迎面拂來，海天相接處波濤湧動，氣象萬千。"),
                ("Operate", "你伸手輕撫航標燈沉厚的銅座與夜光貝罩，防風樞紐運轉自如，散發著穿透夜霧的柔和光暈。"),
            ],
        )];
    }

    let is_trunk_road = !room_name.contains('・')
        && (room_name.ends_with("大道")
            || room_name.ends_with("幹道")
            || room_name.ends_with("山道")
            || room_name.ends_with("險徑")
            || room_name.ends_with("山徑")
            || room_name.ends_with("古徑")
            || room_name.ends_with("牧道")
            || room_name.ends_with("礦道")
            || room_name.ends_with("棧道")
            || room_name.ends_with("軌道")
            || room_name.ends_with("沿海路")
            || room_name.ends_with("海岬路")
            || room_name.ends_with("港灣道")
            || room_name.ends_with("海濱道")
            || room_name.ends_with("潮音道")
            || room_name.ends_with("巡哨路")
            || room_name.ends_with("關隘道")
            || room_name.ends_with("北坡道")
            || room_name.ends_with("天路")
            || room_name.ends_with("冰岸道")
            || room_name.ends_with("橫斷道")
            || room_name.ends_with("林徑"));
    if is_trunk_road {
        return vec![make_peninsula_object(
            "wt_trunk_road_milestone",
            "指路石碑",
            &["Look", "Read", "Rest"],
            &[
                ("Look", "大道旁矗立著風化古樸的四方指路石碑，基座是一方供往來旅客卸擔歇腳的厚重石礎，風塵僕僕中透著安穩。"),
                ("Read", "你俯身撥開碑面上附著的青苔辨讀刻字，半島城鎮間的里數與主幹道走向清晰可辨。"),
                ("Rest", "你在石碑旁的歇腳石礎上落座調勻氣息，就著清涼微風飲下一口水，片刻之後精神重新振奮。"),
            ],
        )];
    }

    let is_peninsula = is_white_tower_peninsula_room(room_name);
    let is_outdoor_road = room_name.contains("石道")
        || room_name.contains("前街")
        || room_name.contains("前巷")
        || room_name.contains("十字口")
        || room_name.contains("學院口")
        || room_name.contains("學院巷")
        || room_name.contains("校舍口")
        || room_name.contains("校舍街")
        || room_name.contains("學舍街")
        || room_name.contains("北段")
        || room_name.contains("中段")
        || room_name.contains("南段")
        || room_name.contains("東段")
        || room_name.contains("西段")
        || room_name.contains("東口")
        || room_name.contains("西口")
        || room_name.contains("北口")
        || room_name.contains("南口")
        || room_name.contains("北巷")
        || room_name.contains("南巷");
    let mut base_objs = if room_name.contains("會計")
        || room_name.contains("算學")
        || room_name.contains("算居")
        || room_name.contains("司書")
        || room_name.contains("商算")
    {
        vec![
            make_peninsula_object(
                "wt_abacus_ebony_desk",
                "烏木計數板",
                &["Look", "Operate", "Use"],
                &[
                    ("Look", "烏木公案上端正擺放著便攜計數板，滑鈕圓潤油亮，旁側立著硃砂墨碟與儲墨鋼筆。"),
                    ("Operate", "你伸手撥動計數滑鈕，上下翻飛發出清脆沉著的扣合聲。"),
                    ("Use", "你提筆在草紙上演算一筆出海貨單，計數板撥轉分毫不差，帳目井然。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_abacus_counters",
                "會計帳冊夾",
                &["Look", "Calculate", "Read"],
                &[
                    ("Look", "條案上擺著硬木封面的試算表夾，內夾整齊的複式帳頁，旁側放著烏木便攜計數板與利率換算表。"),
                    ("Calculate", "你撥動計數板滑鈕逐行覆核試算表，借貸加減縱橫有序，運算如行雲流水，神清氣爽。"),
                    ("Read", "你翻開案頭的會計實務圖表，上面詳載著大宗物產折算與利息攤提之法。"),
                ],
            ),
            make_peninsula_object(
                "wt_abacus_fair_price_ledger",
                "行商價目冊",
                &["Look", "Read"],
                &[
                    ("Look", "厚實的松木帳案上攤開著《各地行商貨價公道帳冊》，墨跡清晰，記載著半島各坊各埠即時市價。"),
                    ("Read", "你細讀帳冊上的公道行情：糧穀、生鐵、海鹽與布疋之平準價目皆有明文，童叟無欺。"),
                ],
            ),
        ]
    } else if room_name.contains("書齋")
        || room_name.contains("小齋")
        || room_name.contains("別室")
        || room_name.contains("山房")
        || room_name.contains("星曆")
        || room_name.contains("讀書")
        || room_name.contains("篆刻")
        || room_name.contains("拓碑")
        || room_name.contains("雕版")
        || room_name.contains("校勘")
        || room_name.contains("墨香")
    {
        vec![
            make_peninsula_object(
                "wt_study_desk_inkstone",
                "烏木書案",
                &["Look", "Use", "Operate", "Read"],
                &[
                    ("Look", "黑沼沉水烏木雕琢的寬厚書案散發著幽靜木香，案上端硯墨色溫潤如玉，水注蓄著山泉，紫毫畫筆與狼毫楷筆整齊懸於紫檀筆架。"),
                    ("Use", "你取墨錠在端硯清泉中徐徐研磨，墨香四溢，提筆蘸墨在宣紙上信手書寫，筆意沉著，心神豁然澄澈。"),
                    ("Operate", "你整理案几上的鎮紙與宣紙摺頁，端整筆洗，室內窗明几淨，文氣盎然。"),
                    ("Read", "你展讀案几上攤開的手抄古卷，筆跡端勁，字句精微，深得先賢格物修齊之旨。"),
                ],
            ),
            make_peninsula_object(
                "wt_study_bamboo_bookshelf",
                "線裝古籍架",
                &["Look", "Use", "Read", "Operate"],
                &[
                    ("Look", "靠牆的高大樟木架上排滿了竹綿紙線裝古籍與輿地總圖，底層整齊碼放著幾隻精編韌竹書簍，分納校勘稿卷。"),
                    ("Use", "你自架上抽出一冊線裝古籍翻閱，紙質柔韌吸墨，墨香沁人心脾，細讀之下頗有領悟。"),
                    ("Read", "你取下一卷半島風物志稿細細研讀，字裡行間詳載水土人文與山川形勝，令人眼界大開。"),
                    ("Operate", "你將翻閱過的文稿重新按部首分類歸入竹編書簍，架欞齊整，井然有序。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_shell_lamp",
                "貝母台燈",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "案角立著造型典雅的旋蓋夜光貝母調光台燈，貝母薄片拼嵌的燈罩泛著溫潤珠光，烏木旋蓋可細調光圈大小。"),
                    ("Use", "你旋動烏木燈蓋調節光暈，柔和清澈的珠白光芒映照書案，滿室皆是清幽安寧之氣。"),
                    ("Operate", "你輕旋貝母燈罩，光暈隨之微調收斂，光斑柔和均勻，極便於夜讀抄寫。"),
                ],
            ),
        ]
    } else if room_name.contains("客房") || room_name.contains("內室") {
        vec![
            make_peninsula_object(
                "wt_reg_wenyao_home",
                "松香木暖床",
                &["Look", "Use", "Rest", "Sleep", "Operate"],
                &[
                    ("Look", "溫暖乾燥的松香木大床上鋪著蓬鬆羽絨暖被與山風草安神枕，榻旁小几上立著一盞旋蓋夜光貝母台燈，光暈柔和溫潤。"),
                    ("Use", "你在松香木大床上舒展四肢，蓬鬆羽絨暖被帶來融融暖意，周身疲勞頓時消散大半，氣血與內息漸漸調和平復。"),
                    ("Rest", "你合衣在柔軟的羽絨大床上躺下歇息，安神草的清香讓緊繃的心神徹底放鬆下來，周身氣血漸復充盈。"),
                    ("Sleep", "你鑽入溫暖蓬鬆的羽絨暖被，旋暗夜光貝母台燈。在靜謐祥和的屋內，你安然入眠，體魄與心神盡皆恢復。"),
                    ("Operate", "你輕輕旋轉床頭夜光貝母台燈的烏木旋蓋，珠白光芒柔和微調，滿室皆是安恬的暖意。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_hotspring_tub",
                "杉木沐浴桶",
                &["Look", "Use", "Bath"],
                &[
                    ("Look", "精工打磨的深腹杉木沐浴桶散發著天然木香，桶畔連通暗渠熱泉注水口，備有乾淨的棉長絨擦巾。"),
                    ("Use", "你扳開注水銅栓，熱氣騰騰的溫泉水汩汩注入杉木桶，滿室蒸騰氤氳暖霧。"),
                    ("Bath", "你浸入溫熱的杉木泉水浴桶中，洗去一身塵泥與筋骨疲憊，周身毛孔舒張，倍感舒泰。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_shell_lamp",
                "貝母台燈",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "几案上立著旋蓋夜光貝母調光台燈，貝母薄片拼嵌的燈罩泛著溫潤珠光，烏木旋蓋可調節明暗。"),
                    ("Use", "你旋亮夜光貝母台燈，珠白光暈柔和灑滿几案與床榻，室內一片溫馨安寧。"),
                    ("Operate", "你旋轉烏木外罩調節進光弧度，光影隨心明暗，極具生活雅趣。"),
                ],
            ),
        ]
    } else if is_wenyao_residential_or_study(room_name) {
        let h = room_name.bytes().fold(5381usize, |h, b| h.wrapping_mul(33).wrapping_add(b as usize));
        match h % 4 {
            0 => vec![
                make_peninsula_object(
                    "wt_home_dining_tea_table",
                    "樟木八仙桌",
                    &["Look", "Use", "Rest"],
                    &[
                        ("Look", "廳堂正中端正擺放著一張打磨光滑的厚實樟木八仙桌，桌上設有青瓷茶具與一壺剛沏好的山風草茶。"),
                        ("Use", "你在八仙桌旁長凳落座，端起青瓷茶杯啜飲一口草茶，茶香清冽回甘，頓解喉燥。"),
                        ("Rest", "你靠在八仙桌旁歇息片刻，廳內窗欞明淨，空氣中飄散著幽淡樟木清香。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_home_shell_lamp",
                    "貝母台燈",
                    &["Look", "Use", "Operate"],
                    &[
                        ("Look", "几案上立著造型典雅的旋蓋夜光貝母調光台燈，貝母薄片拼嵌的燈罩泛著溫潤珠光。"),
                        ("Use", "你旋動烏木燈蓋調節光暈，柔和清澈的珠白光芒映照廳堂，滿室安寧。"),
                        ("Operate", "你輕旋貝母燈罩調節光影明暗，溫潤澄澈。"),
                    ],
                ),
            ],
            1 => vec![
                make_peninsula_object(
                    "wt_reg_wenyao_home",
                    "松香木暖床",
                    &["Look", "Use", "Rest", "Sleep", "Operate"],
                    &[
                        ("Look", "溫暖乾燥的松香木大床上鋪著蓬鬆羽絨暖被與山風草安神枕，榻旁小几上立著一盞旋蓋夜光貝母台燈，光暈柔和溫潤。"),
                        ("Use", "你伸手撫平松香木大床上的厚實棉被，被面乾燥蓬鬆，透著令人安心的松木清香。"),
                        ("Rest", "你坐在床沿卸下隨身行囊，舒展筋骨稍事休憩，滿室寧靜溫暖。"),
                        ("Sleep", "你合衣躺上溫暖的松香木床合眼小憩，羽絨暖被鬆軟溫暖，很快消解了跋涉疲頓。"),
                        ("Operate", "你旋轉榻旁貝母小台燈的旋蓋，微調夜光角度，滿室光暈澄澈柔和。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_home_brass_tap_sink",
                    "黃銅活水栓",
                    &["Look", "Use", "Bath"],
                    &[
                        ("Look", "打磨光滑的黃銅水栓連接著活水管道，下方是耐火磚砌成的溫石蓄熱水槽，槽邊備有山茶松脂皂與棉長絨擦巾。"),
                        ("Use", "你擰開黃銅水栓，清冽純淨的活水流淌而出，隨後轉入溫水檔，溫暖的水流洗去塵泥，草木皂香格外清新。"),
                        ("Bath", "你引入溫石槽加熱的熱水洗手淨面，熱氣騰騰驅散了旅途風塵，倍感舒泰。"),
                    ],
                ),
            ],
            2 => vec![
                make_peninsula_object(
                    "wt_home_ebony_tea_table",
                    "烏木茶盤几",
                    &["Look", "Use", "Rest"],
                    &[
                        ("Look", "窗下置著一張色澤深沉溫潤的沉水烏木茶几，几上平鋪著竹絲編織茶墊與一把細泥紫砂小茶壺。"),
                        ("Use", "你提壺往白瓷茶盅注滿清潤山泉熱茶，細品微苦甘涼的茶湯，神思悠然。"),
                        ("Rest", "你倚在茶几旁的軟墊靠枕上歇息片刻，窗外微風拂竹，室內清雅靜謐。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_home_pine_wardrobe",
                    "松香木衣櫃",
                    &["Look", "Use", "Check"],
                    &[
                        ("Look", "靠牆立著高大的松香木對開門衣櫃，銅合頁打磨細緻，櫃門微敞，飄出乾燥的松脂與樟腦驅蛀清香。"),
                        ("Use", "你拉開衣櫃厚實的木門，將厚重的外袍與隨身行囊整齊掛在木架上，收納妥貼。"),
                        ("Check", "你檢視衣櫃內整齊分格的儲物隔板，各層鋪有防潮麻布，收納井井有條。"),
                    ],
                ),
            ],
            _ => vec![
                make_peninsula_object(
                    "wt_home_bamboo_rocker",
                    "竹編搖椅",
                    &["Look", "Rest", "Sit"],
                    &[
                        ("Look", "向陽窗欞旁安放著一張柔韌寬敞的青竹編織搖椅，弧形椅底打磨得極為光滑，墊著厚實的羊毛坐褥。"),
                        ("Rest", "你仰靠在竹編搖椅上隨意晃動，椅身輕柔搖曳，微風自窗欞拂入，愜意怡然。"),
                        ("Sit", "你在羊毛坐褥上穩穩落座，竹篾貼合背脊自然受力，疲勞隨之一空。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_home_jade_icebox",
                    "寒水玉冰櫃",
                    &["Look", "Use", "Check"],
                    &[
                        ("Look", "角落擺著一座雙層硬木箱櫃，內壁緊密鑲嵌著半嶺寒水玉板，櫃內寒氣繚繞，儲存著新鮮果乾與冰涼井泉水囊。"),
                        ("Use", "你掀開厚實櫃蓋取出一隻冰鎮甘蔗汁小竹筒，清涼甘甜沁入心脾，暑氣立消。"),
                        ("Check", "你查看櫃內寒水玉的凝霜狀況，玉板散發陣陣自然涼意，保鮮極佳。"),
                    ],
                ),
            ],
        }
    } else if room_name.contains("內門坊") {
        vec![
            make_peninsula_object(
                "wt_gate_stone_archway_board",
                "牌坊路引碑",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "巍峨挺拔的青石牌坊橫跨街衢，飛簷挑角雕飾精美，石柱旁立著黃銅包邊的路引告示牌，標注內城四坊與外環諸坊的界域與更次。"),
                    ("Read", "你仰頭研讀告示牌上的通行條例與各坊指引：內連文曜核心樞紐，外接四向繁華市邑與外環關防，車馬行人絡繹不絕。"),
                    ("Operate", "你拂去石牌下端刻石的塵土，石質路標凹刻的方位符號歷歷在目，指引清晰。"),
                ],
            ),
            make_peninsula_object(
                "wt_gate_water_culvert_drum",
                "歇腳石鼓",
                &["Look", "Use", "Rest"],
                &[
                    ("Look", "牌坊兩側鋪設著引自三水川的平整青石導水渠，渠水潺潺清冽；石階兩側立有厚實的雕花歇腳石鼓，供過往挑夫卸擔暫歇。"),
                    ("Use", "你走近導水石渠汲取一捧清涼泉水洗面，清冽的水流滌盡了行走街巷帶來的燥熱。"),
                    ("Rest", "你在石鼓旁卸下行囊坐下稍歇，石面光滑微涼，看著內外城交匯處的熙攘車馬，別有一番市井意趣。"),
                ],
            ),
            make_peninsula_object(
                "wt_gate_patrol_gong_stone",
                "巡更銅鑼",
                &["Look", "Ring", "Read"],
                &[
                    ("Look", "石坊側壁懸掛著巡更差役報時防險的沉厚黃銅響鑼，下方嵌入一塊歷經歲月摩挲的青石指路刻石。"),
                    ("Ring", "你輕叩懸於坊壁的巡更銅鑼，鑼聲沉渾悠長，聲震四街，驚起幾隻棲息在牌坊飛簷上的飛鳥。"),
                    ("Read", "你撫摸指路刻石上的楷書方位：向內直通鐘樓樞紐，向外直達城門外街，條理井然。"),
                ],
            ),
        ]
    } else if room_name.contains("貨船大埠") || room_name.contains("大埠") {
        vec![
            make_peninsula_object(
                "wt_obj_wharf_bollard",
                "生鐵繫纜樁",
                &["Look", "Tie", "LookShip"],
                &[
                    ("Look", "條石駁岸邊深埋著粗壯的生鐵繫纜樁，樁身受粗麻纜繩摩挲得油黑發亮，駁岸沿線排滿厚重防撞松木垛。"),
                    ("Tie", "你將隨身繫繩熟練地在生鐵繫纜樁上打了個牢固的雙套結，纜結吃勁穩當。"),
                    ("LookShip", "你放眼望向泊位，數艘雙桅載重貨船隨著江波起伏，船工正忙碌裝卸貨包。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_wharf_crane",
                "滑輪吊架",
                &["Look", "Operate"],
                &[
                    ("Look", "駁岸邊架著堅固的懸臂式黃銅滑輪吊架，厚木跳板斜搭在貨船甲板與碼頭石階之間。"),
                    ("Operate", "你拉動吊架垂下的麻繩試驗滑輪阻力，黃銅輪軸運轉平滑，省力穩當。"),
                ],
            ),
        ]
    } else if room_name.contains("鮮魚拍賣") || room_name.contains("魚市") {
        vec![
            make_peninsula_object(
                "wt_obj_fish_auction_trough",
                "鮮魚冰槽",
                &["Look", "Weigh", "Inspect"],
                &[
                    ("Look", "平整石板地上整齊排著引寒水玉蓄冷的青石冰槽，槽內盛滿剛出水銀鱗閃爍的大海魚，旁邊立著高大的黃銅懸臂秤。"),
                    ("Weigh", "你撥動懸臂秤上的黃銅大砣校驗斤兩，秤桿平穩起落，顯示魚獲分量沉實。"),
                    ("Inspect", "你湊近冰槽細看，銀鱗大魚鰓色鮮紅、體表光澤，滿是江口活水的清鮮之氣。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_fish_baskets",
                "透水竹魚簍",
                &["Look", "Use"],
                &[
                    ("Look", "冰槽旁堆疊著無數浸泡得深褐發亮的透水大竹簍，案板上整齊排著剖魚刮鱗的厚背弧刃短刀。"),
                    ("Use", "你拿起刮鱗刀在粗石上隨手一磨，刀口鋒利平整，處理魚獲俐落乾脆。"),
                ],
            ),
        ]
    } else if room_name.contains("桅帆") || room_name.contains("造船") || room_name.contains("修船") {
        vec![
            make_peninsula_object(
                "wt_obj_ship_ribs_pitch",
                "造船龍骨架",
                &["Look", "Smell", "Caulking"],
                &[
                    ("Look", "作坊開敞的大棚下支起十多丈長的弧形硬木船肋架，旁側鐵皮火爐上滾沸著填補水密艙縫隙的生桐油松香瀝青鍋。"),
                    ("Smell", "濃烈的松脂焦香與熟桐油氣息撲鼻而來，是江海船匠最熟悉的熱烈味道。"),
                    ("Caulking", "你拿起一把寬頭嵌縫鐵刀在木板試槽刮磨，手感紮實，能使船體水密嚴絲合縫。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_sail_canvas_bench",
                "帆布裁案",
                &["Look", "Touch"],
                &[
                    ("Look", "長條案板上鋪展著厚實堅韌的雙層亞麻帆布，竹筒裡插著數十根浸潤防腐蠟的粗骨針與麻繩卷。"),
                    ("Touch", "你按壓帆布質地，緊密厚實，經得起外海狂風暴雨的強力撕扯。"),
                ],
            ),
        ]
    } else if room_name.contains("漁網") || room_name.contains("曬網") || room_name.contains("繩索工坊") || room_name.contains("河鮮乾貨") || room_name.contains("鹽漬") {
        vec![
            make_peninsula_object(
                "wt_obj_drying_nets_trough",
                "苧麻曬網架",
                &["Look", "Inspect", "Open"],
                &[
                    ("Look", "露天院落中橫架著數十丈長的曬網木架，晾曬著浸染柿漆防腐的深褐色苧麻大網，廊下整齊排列著加蓋重石的鹽漬大陶缸。"),
                    ("Inspect", "你摸了摸懸掛的苧麻漁網，網結緊湊堅韌，透著淡淡的柿漆甘澀香。"),
                    ("Open", "你掀開其中一只大醃缸的竹木蓋，滿缸粗鹽浸漬的青魚散發著鹹香撲鼻的發酵海味。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_rope_twist_wheel",
                "手搖絞繩輪",
                &["Look", "Operate"],
                &[
                    ("Look", "空地上立著三聯手搖木製絞繩輪，麻絲自輪齒穿出絞合為粗壯縴繩，旁側立有刻度拉力測試石柱。"),
                    ("Operate", "你搖動絞繩輪木柄，三股浸油麻線均勻絞合，緊實堅固。"),
                ],
            ),
        ]
    } else if room_name.contains("竹木場") || (room_name.contains("河運") && room_name.contains("倉")) {
        vec![
            make_peninsula_object(
                "wt_obj_timber_reed_stacks",
                "毛竹原木垛",
                &["Look", "Check", "Tap"],
                &[
                    ("Look", "高大通風的倉棚下整齊碼放著以油布覆蓋的防潮糧穀包，場地另一側堆滿通直挺拔的三年生毛竹與直徑數尺的巨杉圓木。"),
                    ("Check", "你翻檢糧垛底層的防潮樟木墊板，乾燥清爽，通風良好。"),
                    ("Tap", "你屈指叩擊粗大毛竹竿，竹管清脆堅挺，正是紮造載重排筏的上等良材。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_steelyard_cart",
                "槓桿大磅秤",
                &["Look", "Operate"],
                &[
                    ("Look", "平整地面停著帶鐵輪的大型移動式槓桿磅秤，旁側備有引導大圓木滾動的斜坡硬木墊條。"),
                    ("Operate", "你推動磅秤上的配重鐵砣，滑動順暢，計重精確。"),
                ],
            ),
        ]
    } else if room_name.contains("水手茶") || room_name.contains("茶棚") || room_name.contains("茶飯館") {
        vec![
            make_peninsula_object(
                "wt_obj_river_tea_kettle",
                "紫銅大茶壺",
                &["Look", "Taste", "WarmHands"],
                &[
                    ("Look", "臨江茶座的炭火紅爐上煨著一把滾水翻騰的大號紫銅茶壺，粗木桌上整齊倒扣著青花粗陶大海碗。"),
                    ("Taste", "你倒了一大碗熱氣蒸騰的濃釅磚茶飲盡，微苦回甘，頓時驅散江風濕寒，通體舒泰。"),
                    ("WarmHands", "你將雙手懸在紫銅大茶壺上方烘烤，熱氣繚繞，暖透指尖。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_river_view_bench",
                "臨江木長凳",
                &["Look", "Rest"],
                &[
                    ("Look", "粗松木長凳依著駁岸欄杆擺放，臨窗正對滔滔江流，江面上白帆點點。"),
                    ("Rest", "你在長凳上坐下倚欄遠眺，聽著江濤拍岸與水手號子，心神開闊。"),
                ],
            ),
        ]
    } else if room_name.contains("清淤") || room_name.contains("巡河") {
        vec![
            make_peninsula_object(
                "wt_obj_patrol_skiff_gear",
                "清淤鐵斗",
                &["Look", "Check"],
                &[
                    ("Look", "哨所石碼頭旁繫泊著兩艘吃水極淺的快船，駁岸邊架著數柄清理河道暗沙淤泥的長柄鋼齒鐵斗與救生浮木。"),
                    ("Check", "你檢查巡河快船上的桐油木槳與水路浮標，裝備齊全，隨時可出航巡視航道。"),
                ],
            ),
        ]
    } else if room_name.contains("宣紙") || room_name.contains("晒紙") || room_name.contains("造紙") {
        vec![
            make_peninsula_object(
                "wt_obj_paper_making_screen",
                "抄紙竹簾槽",
                &["Look", "Operate", "Touch"],
                &[
                    ("Look", "清泉注入的長條抄紙石槽中浮著如雪漿料，抄紙匠手持細密青竹簾輕輕一晃，便蕩出一張厚薄均勻的紙膜；旁側是燒炭增溫的平整烘紙夾石壁。"),
                    ("Operate", "你雙手托起青竹抄紙簾在漿槽中勻速一蕩，水流順簾縫漏下，簾面留下一層纖薄純淨的青檀纖維。"),
                    ("Touch", "你伸手撫摸烘紙石壁上剛揭下的宣紙，紙質綿韌溫熱，如玉生香。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_pulp_vat",
                "木料浸泡池",
                &["Look", "Inspect"],
                &[
                    ("Look", "石砌方池中浸泡著經草木灰水發酵純化後的青檀樹皮，旁邊支著以巨木槓桿加壓的脫水木榨。"),
                    ("Inspect", "你探看池中纖維，色白如玉、柔韌細長，正是成就千年不腐宣紙之根基。"),
                ],
            ),
        ]
    } else if room_name.contains("徽墨") || room_name.contains("精墨") {
        vec![
            make_peninsula_object(
                "wt_obj_ink_soot_mortar",
                "搗墨鐵臼",
                &["Look", "Smell", "Pound"],
                &[
                    ("Look", "暗室中整齊排列著收集上等古松油煙的陶盞，石台上置有沉重的大鐵臼與包革重錘，工匠正千錘百煉將皮膠與松煙捶打成墨坯。"),
                    ("Smell", "空氣中瀰漫著古松燃煙的特有幽香，混雜著麝香與冰片的提神清冽氣息。"),
                    ("Pound", "你握緊搗墨木杵在鐵臼中奮力搗擊數次，墨坯筋道柔韌，烏黑生光。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_ink_mould_drying",
                "雕花墨模架",
                &["Look", "Inspect"],
                &[
                    ("Look", "木架上陳列著精雕山水雲龍紋的硬梨木墨模，旁側通風架上整齊排著以厚灰包裹慢速陰乾的墨錠成品。"),
                    ("Inspect", "你拿起一方成型墨錠端詳，表面紫光油亮、金粉填字，叩之有金石清音。"),
                ],
            ),
        ]
    } else if room_name.contains("活字") || room_name.contains("印書") {
        vec![
            make_peninsula_object(
                "wt_obj_movable_type_case",
                "活字字盤架",
                &["Look", "PickType", "RollInk"],
                &[
                    ("Look", "高聳的斜面字架上排列著按音韻偏旁分門別類的數萬枚錫鉛與硬木活字，工作台上放著蘸滿油亮墨汁的雙手羊毛滾筒與銅夾版框。"),
                    ("PickType", "你自字盤中精準拈出一枚工整反字「道」，鉛錫合金字面光滑如鏡，棱角分明。"),
                    ("RollInk", "你推動羊毛滾筒在字版上均勻滾塗油墨，黑潤發亮，字跡頓時清晰畢現。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_printing_press",
                "螺旋壓印機",
                &["Look", "Operate"],
                &[
                    ("Look", "鑄鐵底座的螺旋式手扳壓印機立在工坊正中，側邊晾架上夾著剛印就的《文曜格物錄》書頁。"),
                    ("Operate", "你扳動螺旋下壓手柄，重板均勻受力壓覆紙面，揭開時字跡黑亮深透，濃淡勻停。"),
                ],
            ),
        ]
    } else if room_name.contains("裝幀") || room_name.contains("線裝") {
        vec![
            make_peninsula_object(
                "wt_obj_book_binding_press",
                "線裝壓書機",
                &["Look", "Operate", "Stitch"],
                &[
                    ("Look", "寬大的實木工作台上固定著旋柄木螺壓平機，旁邊整齊排著裁紙大鍘刀、打孔骨錐、蜂蠟塊與彩色熟絲線。"),
                    ("Operate", "你轉動木螺壓機旋柄，木板緊緊壓實新裝訂的線裝書脊，整飭端莊。"),
                    ("Stitch", "你拿起拋光骨針穿引浸蠟絲線，在紙頁預留的針眼間靈巧穿行，鎖線均勻平整。"),
                ],
            ),
        ]
    } else if room_name.contains("輿圖") || room_name.contains("格物") {
        vec![
            make_peninsula_object(
                "wt_obj_cartography_table",
                "測繪大畫案",
                &["Look", "Measure", "Inspect"],
                &[
                    ("Look", "數丈長的平整大畫案上鋪展著白塔半島全境水陸輿圖草稿，案頭擺著黃銅分規、量角器、等高測量照準儀與各色礦物顏料碟。"),
                    ("Measure", "你拿起精緻的黃銅兩腳分規在輿圖上測量文曜至風舶角的里數，刻度精微，毫釐不差。"),
                    ("Inspect", "你俯身仔細察看圖上繪製的山脈地鳴走向與沿海暗礁標注，測繪嚴謹入微。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_surveyor_theodolite",
                "經緯照準儀",
                &["Look", "Operate"],
                &[
                    ("Look", "畫案旁立著黃銅三腳架經緯照準儀，水晶目鏡擦拭得晶瑩剔透，旁立著黑白分格的測高水準木尺。"),
                    ("Operate", "你調整照準儀水平水泡，指針歸零，望遠鏡筒鎖定遠方地平山稜。"),
                ],
            ),
        ]
    } else if room_name.contains("傳習所") || room_name.contains("會計") || (room_name.contains("算學") && !room_name.contains("院")) {
        vec![
            make_peninsula_object(
                "wt_obj_abacus_counters",
                "會計帳冊夾",
                &["Look", "Calculate", "Read"],
                &[
                    ("Look", "條案上擺著以名貴紅木製作的雙排精密計數板，旁側配有一份工程成本試算表與幾何受力圖譜。"),
                    ("Calculate", "你指尖輕撥計數滑鈕，滑槽發出清脆扣響，運算如行雲流水，神清氣爽。"),
                    ("Read", "你翻開案頭的《工程數學與幾何實務》，上面推演著農水工程與造船載重的幾何受力演算法。"),
                ],
            ),
        ]
    } else if room_name.contains("測風") || room_name.contains("北辰") || room_name.contains("曆法") {
        vec![
            make_peninsula_object(
                "wt_obj_armillary_almanac",
                "定緯銅儀",
                &["Look", "Observe", "Read"],
                &[
                    ("Look", "露天石台中央矗立著多層旋轉銅環交錯的北辰定緯儀，基座四周平嵌著刻滿二十四節氣晷影長短的黑曜石測影日晷。"),
                    ("Observe", "你俯身透過銅儀窺管對準北天極星，重環轉動滑潤無聲，指引天行經緯。"),
                    ("Read", "你俯瞰黑曜石規表上的日影刻度，比對當前時辰與太陽方位，推步精微。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_wind_vane_gauge",
                "相風銅烏",
                &["Look", "Check"],
                &[
                    ("Look", "測候台四角立著展翅相風銅烏，在微風中靈動旋轉；台內置有三級滴漏水鐘，刻箭隨水位均勻浮升。"),
                    ("Check", "你端看滴漏刻箭上的時辰水位刻度，水流均勻如線，記時絲毫不差。"),
                ],
            ),
        ]
    } else if room_name.contains("杏林") || (room_name.contains("醫館") && room_name.contains("百草")) {
        vec![
            make_peninsula_object(
                "wt_obj_herb_cabinet_mortar",
                "百眼藥櫃",
                &["Look", "Weigh", "Pound"],
                &[
                    ("Look", "整面高牆由數百個貼著楷書金漆藥名的抽屜藥櫃組成，長條藥案上擺著磨成光面的紫銅搗藥臼與分釐黃銅戥子秤。"),
                    ("Weigh", "你提起小巧的黃銅戥子秤，精確稱量出三錢清涼薄荷與二錢北山寒水玉細粉。"),
                    ("Pound", "你握起紫銅搗藥杵在藥臼中輕搗乾草藥，微苦回甘的藥香瞬間瀰漫開來。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_acupuncture_pulse_pillow",
                "沉香脈枕",
                &["Look", "Use"],
                &[
                    ("Look", "診案上墊著一具包著軟綾的沉香木切脈枕，旁側錦盒內整齊插著數十根極細的精淬金銀針。"),
                    ("Use", "你手腕輕搭在沉香脈枕上，心跳脈搏沉靜平穩，精神為之一爽。"),
                ],
            ),
        ]
    } else if room_name.contains("採藥棧") || room_name.contains("山林採藥") {
        vec![
            make_peninsula_object(
                "wt_obj_herb_drying_rack",
                "曬藥竹篾架",
                &["Look", "Smell"],
                &[
                    ("Look", "棧房院中支滿了一層層圓形竹篾席，曬著剛自北嶺險峰採回的山風草、石斛與野山參，牆角倒掛著帶防滑鐵爪的登山採藥草鞋。"),
                    ("Smell", "陽光曬透乾草藥散發出的濃郁藥香與泥土芬芳交織，令人提神醒腦。"),
                ],
            ),
        ]
    } else if room_name.contains("文房") || room_name.contains("文具行") {
        vec![
            make_peninsula_object(
                "wt_obj_brush_inkstone_rack",
                "紫檀筆架",
                &["Look", "TestBrush", "GrindInk"],
                &[
                    ("Look", "紫檀雕花筆架上懸掛著數十支自兼毫、純狼毫至紫毫的大中小楷毛筆，案頭臥著一方呵氣生津的端溪青石老硯。"),
                    ("TestBrush", "你取下一支兼毫長鋒在掌心輕輕按壓試筆，彈性十足，蓄墨豐沛。"),
                    ("GrindInk", "你提起松煙墨錠在青石硯台中滴水緩磨，硯面黑亮如漆，墨韻生光。"),
                ],
            ),
        ]
    } else if room_name.contains("水錘") || room_name.contains("大鍛坊") {
        vec![
            make_peninsula_object(
                "wt_obj_trip_hammer_trough",
                "水力重鐵錘",
                &["Look", "Watch", "Quench"],
                &[
                    ("Look", "外河急流驅動的巨木水輪帶動凸輪軸，數千斤重的生鐵大鍛錘以雷霆萬鈞之勢不斷夯擊砧台，砧旁鑿有深廣的循環淬火石槽。"),
                    ("Watch", "你凝神注視燒紅的重型鋼軸在水錘轟擊下火星四濺，每一次撞擊都撼動腳底石板。"),
                    ("Quench", "匠人將鍛打完畢的鋼料浸入冷水槽，頓時蒸騰起滾滾白霧與刺啦劇響。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_bellows_furnace",
                "木風箱高爐",
                &["Look", "Operate"],
                &[
                    ("Look", "耐火磚堆砌的高溫反射爐旁架著丈餘長的雙推桿活塞木風箱，爐口吞吐著熾烈的金白烈焰。"),
                    ("Operate", "你握住風箱推桿來回一推，風道轟然呼嘯，爐膛烈火猛地拔高數尺。"),
                ],
            ),
        ]
    } else if room_name.contains("鋸木") || room_name.contains("大木") {
        vec![
            make_peninsula_object(
                "wt_obj_waterwheel_sawframe",
                "水動排鋸台",
                &["Look", "Smell", "Measure"],
                &[
                    ("Look", "水輪傳動皮帶驅動數排鋒利長鋸上下往復飛速拉動，整根參天巨木被精準剖切成筆直板材，刨木台上積了厚厚一層噴香刨花。"),
                    ("Smell", "剛剖切開的百年雲杉木脂香氣撲面而來，清新宜人。"),
                    ("Measure", "你以曲尺檢驗剖切出的大梁木料，斷面平直方正，嚴合營造法度。"),
                ],
            ),
        ]
    } else if room_name.contains("切割") || room_name.contains("石材") {
        vec![
            make_peninsula_object(
                "wt_obj_stone_gang_saw",
                "切石框鋸台",
                &["Look", "Touch"],
                &[
                    ("Look", "粗壯鐵架上懸掛著加重無齒金剛砂框鋸，細水伴著鋼砂在巨石槽中慢慢切割，四周堆滿了平整如削的城防條石與石雕柱礎。"),
                    ("Touch", "你撫摸切割完畢的青石板側面，細密平整，嚴密無縫。"),
                ],
            ),
        ]
    } else if room_name.contains("精煉") || room_name.contains("青銅器皿") {
        vec![
            make_peninsula_object(
                "wt_obj_bronze_crucible",
                "熔銅坩堝爐",
                &["Look", "Inspect"],
                &[
                    ("Look", "鼓風皮囊呼呼作響，高溫爐膛中石墨坩堝內熔化著金紅透亮的銅錫合金液，旁邊地面整齊排列著加固木製翻砂模具。"),
                    ("Inspect", "你檢視翻砂模具中的浮雕紋飾，饕餮雲紋深峻清晰，成型精準。"),
                ],
            ),
        ]
    } else if room_name.contains("木炭窯") || (room_name.contains("炭窯") && !room_name.contains("院")) {
        vec![
            make_peninsula_object(
                "wt_obj_charcoal_kiln",
                "黏土悶炭窯",
                &["Look", "Knock"],
                &[
                    ("Look", "依山構築的半球形黏土炭窯窯門以濕泥封堵，窯頂煙囪正吐著淡淡青煙，棚下整齊碼放著敲擊如鐵金之聲的頂級硬木白炭。"),
                    ("Knock", "你拿起兩條黑亮白炭互敲，聲音清脆悠揚宛若玉石，足見炭質堅密無渣。"),
                ],
            ),
        ]
    } else if room_name.contains("磚瓦窯") || room_name.contains("官窯") || room_name.contains("燒造") {
        vec![
            make_peninsula_object(
                "wt_obj_long_kiln_firebox",
                "龍窯火膛",
                &["Look", "Check"],
                &[
                    ("Look", "依山勢斜臥的數十丈長條龍窯火眼遍布，火膛內火焰翻騰泛白，外側曬場上排滿了陰乾完畢等待入窯的青筒瓦坯與釉陶大缸。"),
                    ("Check", "你查看窯溫觀火孔，火色金白耀眼，正是瓷胎玻化凝固之關鍵火候。"),
                ],
            ),
        ]
    } else if room_name.contains("工具行") || (room_name.contains("農具") && room_name.contains("總")) {
        vec![
            make_peninsula_object(
                "wt_obj_farm_tool_rack",
                "鐵農具木架",
                &["Look", "TestEdge"],
                &[
                    ("Look", "牆上排滿了雙齒鎬、開荒大板鋤、月牙割麥鐮與重型碎土鐵耙，案頭擺著浸在礦物油中的雙面天然油石。"),
                    ("TestEdge", "你伸指輕彈割麥鐮刀背，鋼聲清越悠長，刃口經油石研磨鋒利如剃。"),
                ],
            ),
        ]
    } else if room_name.contains("織造坊") || room_name.contains("大染坊") || (room_name.contains("機杼") && room_name.contains("染")) {
        vec![
            make_peninsula_object(
                "wt_obj_loom_dye_vats",
                "踏板高織機",
                &["Look", "Touch"],
                &[
                    ("Look", "高大的立式踏板織機梭子飛擲，機旁數口齊腰深的大陶缸盛滿天然發酵群青染液，挑染出的深藍布疋迎風招展如雲霞。"),
                    ("Touch", "你摸了摸新織成的厚帆布料，經緯交錯緊實密致，具備絕佳抗風抗撕拉韌性。"),
                ],
            ),
        ]
    } else if room_name.contains("五金鋪") || room_name.contains("車輪鑲鐵") {
        vec![
            make_peninsula_object(
                "wt_obj_hardware_bins",
                "五金分格櫃",
                &["Look", "Check"],
                &[
                    ("Look", "鐵皮壁櫃上百餘個木匣盛滿各型鉚釘、螺栓、合頁與銷釘，地坪上置有一圈圓形冷水槽，專供燒紅車輪鋼箍套木輪後急速冷縮咬合。"),
                    ("Check", "你翻看鐵匣中打製好的精鋼鉚釘，釘體圓整，毫無毛刺。"),
                ],
            ),
        ]
    } else if room_name.contains("烈酒大飯鋪") || (room_name.contains("工匠") && room_name.contains("食堂")) {
        vec![
            make_peninsula_object(
                "wt_obj_forge_mess_cauldron",
                "大灶鐵炒鑊",
                &["Look", "Taste"],
                &[
                    ("Look", "泥石大灶上架著一口能容數百斤飯食的厚鑄鐵大鑊，熱油爆炒蔥薑大塊原牛肉香氣熏天，牆角立著數只大號土陶老白乾酒罈。"),
                    ("Taste", "你舀了一小勺濃油赤醬的牛肉燴菜品嚐，鹹香濃烈，極為開胃下飯。"),
                ],
            ),
        ]
    } else if room_name.contains("駝隊大驛站") || room_name.contains("駝馬大驛館") || room_name.contains("駝馬大客棧") || room_name.contains("客商大旅社") {
        vec![
            make_peninsula_object(
                "wt_obj_caravan_hitch_trough",
                "拴馬長木欄",
                &["Look", "Feed", "Inspect"],
                &[
                    ("Look", "寬闊的露天驛場四周立著一人多高的結實榆木栓馬欄，兩排長條青石水槽盛滿自來活水，長槽中堆著混合豆餅的切細苜蓿草。"),
                    ("Feed", "你抓起一把清香甘甜的豆餅乾草餵給槽邊的原牛駝獸，駝獸溫順地咀嚼，眼眸溫和。"),
                    ("Inspect", "你檢驗木欄上的韁繩繫扣，粗大牢實，能承受數十頭大牲口同時拉扯。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_caravan_bell_rack",
                "鍛銅駝鈴架",
                &["Look", "Shake"],
                &[
                    ("Look", "廊下懸掛著成串音階各異的鍛銅駝鈴，長凳上擺著塗抹牛油熟化、防磨厚實的羊毛鞍墊。"),
                    ("Shake", "你輕搖銅駝鈴串，渾厚悠揚的鈴聲在開闊驛站迴盪，傳出數里之遙。"),
                ],
            ),
        ]
    } else if room_name.contains("皮毛大市") || room_name.contains("皮毛大貨棧") {
        vec![
            make_peninsula_object(
                "wt_obj_fur_baling_press",
                "生皮壓捆機",
                &["Look", "Touch", "Weigh"],
                &[
                    ("Look", "開闊的棚場裡立著重型螺旋木製壓捆機，數十張整理平整的厚牛皮被牢牢壓成緊實貨立方，上方橫樑懸掛著能稱數千斤的重鐵大桿秤。"),
                    ("Touch", "你伸手摩挲剛熟化好的優等原牛皮，皮板柔軟厚重，毛色油潤光潔。"),
                    ("Weigh", "你推動大桿秤秤砣，秤桿平穩懸浮，分量十足。"),
                ],
            ),
        ]
    } else if (room_name.contains("糧草") && room_name.contains("倉")) || room_name.contains("中轉站") {
        vec![
            make_peninsula_object(
                "wt_obj_ration_crate_stacks",
                "防潮糧草箱",
                &["Look", "Inspect"],
                &[
                    ("Look", "厚條石修築的防潮高台庫房內，整齊堆滿了塗有桐油的防雨樟木大箱，牆邊以十字交錯法碼放著壓成硬塊的高營養乾草飼料磚。"),
                    ("Inspect", "你撬開一只樟木箱封角查看，內裡防潮油紙完好，儲存的炒麥麵散發著烘烤麥香。"),
                ],
            ),
        ]
    } else if room_name.contains("大車廠") || room_name.contains("修造場") {
        vec![
            make_peninsula_object(
                "wt_obj_wagon_jack_wheel",
                "修車頂升機",
                &["Look", "Operate", "Inspect"],
                &[
                    ("Look", "工場中央以重型木質螺桿千斤頂將一輛雙輪大車懸空支起，修車匠正以大鐵錘敲擊校正包鐵硬木輪輻，砧台上放著淬火車軸。"),
                    ("Operate", "你握住千斤頂橫木搖柄轉動半圈，沉重的車架穩步抬升，機構沉穩咬合。"),
                    ("Inspect", "你彎腰細看重型車輪的包鐵厚度，足有半指之厚，足以在荒原碎石路輾壓千里。"),
                ],
            ),
        ]
    } else if room_name.contains("拍賣行") || room_name.contains("牲畜") {
        vec![
            make_peninsula_object(
                "wt_obj_livestock_auction_ring",
                "牲口競價欄",
                &["Look", "Watch"],
                &[
                    ("Look", "環形露天木圍欄中央是平整開闊的展示沙地，四周設有階梯看台，一頭毛色鋥亮的大角原牛正由牽引繩引導繞場踱步展示體格。"),
                    ("Watch", "圍欄周邊行商目光如炬，暗暗比劃手指商定起拍叫價。"),
                ],
            ),
        ]
    } else if room_name.contains("獸醫") {
        vec![
            make_peninsula_object(
                "wt_obj_veterinary_box",
                "修蹄木架",
                &["Look", "Inspect"],
                &[
                    ("Look", "藥鋪內設有固定牲口蹄足的木架，案上擺滿了特大號除石鐵鉤、切蹄刀與銅烙鐵，旁邊石臼裡盛滿了以蜂蜜調和的健胃消脹大草藥丸。"),
                    ("Inspect", "你拿起一把沉重光潔的切蹄刀試了試刃口，弧度合手，利於修整駝獸硬蹄。"),
                ],
            ),
        ]
    } else if room_name.contains("生鐵庫") || room_name.contains("粗鹽") {
        vec![
            make_peninsula_object(
                "wt_obj_iron_salt_stacks",
                "生鐵錠垛",
                &["Look", "Tap"],
                &[
                    ("Look", "高敞乾燥的石庫內，重達數十斤一塊的梯形生鐵錠像磚牆般整齊交錯碼放，另一側防潮木架上整齊堆著密封嚴密的青鹽大包。"),
                    ("Tap", "你手指扣擊底層生鐵錠，鏗鏘作響，質地堅密純淨。"),
                ],
            ),
        ]
    } else if room_name.contains("武備") || room_name.contains("護運行") || room_name.contains("鏢局") {
        vec![
            make_peninsula_object(
                "wt_obj_weapon_rack_stone",
                "護行兵器架",
                &["Look", "Sharpen", "Inspect"],
                &[
                    ("Look", "堅實木架上整齊插放著防身硬木長棍、包銅重皮盾與開山厚背獵刀，長凳上擺著盛滿菜籽油的細砂油石槽。"),
                    ("Sharpen", "你抽出一把厚背獵刀在油石槽上緩緩推磨，刀刃在油光映射下泛出冷冽寒光。"),
                    ("Inspect", "你端起包銅硬皮盾審視，盾面以生牛皮三重疊壓包覆，能有效抵擋荒原野獸撕咬。"),
                ],
            ),
        ]
    } else if room_name.contains("車行") {
        vec![
            make_peninsula_object(
                "wt_obj_weighbridge_milepost",
                "西道里程碑",
                &["Look", "Read"],
                &[
                    ("Look", "鋪面門前青石地嵌著一方重型過磅鐵台，側立一尊高大花崗岩石碑，鐫刻著通往長風草甸、老堰頭與大圳頭的準確里程與地勢險隘。"),
                    ("Read", "你俯身端讀里程路石刻字：西出城門五里為平川，十里有避風坡與取水深井，標記周詳。"),
                ],
            ),
        ]
    } else if room_name.contains("羊肉湯") || room_name.contains("酒肆") {
        vec![
            make_peninsula_object(
                "wt_obj_mutton_broth_pot",
                "羊肉大湯鍋",
                &["Look", "Taste", "Smell"],
                &[
                    ("Look", "粗麻避風棚下擺滿了寬厚木桌長凳，大灶上一口深腹鐵鍋翻滾著奶白色的原羊骨高湯，大蔥花與白胡椒粉香氣四溢。"),
                    ("Taste", "你端起粗瓷碗喝了一大口濃滾羊湯，湯鮮味濃，胡椒辛香熱辣直透四肢百骸。"),
                    ("Smell", "羊肉脂肪甘香混著麵香在空氣中熱烈翻滾，令人饞涎欲滴。"),
                ],
            ),
        ]
    } else if room_name.contains("食堂") {
        vec![
            make_peninsula_object(
                "wt_dining_rice_milk_stall",
                "溫米漿桶",
                &["Look", "Taste"],
                &[
                    ("Look", "老堰頭水磨溫米漿散發著醇厚米香，大杉木桶裡盛著風舶苦滷點製的熱豆花，攤旁蒸籠疊著剛出鍋的蔥香米烙餅與牛埔鮮肉包。"),
                    ("Taste", "你啜飲了一口熱米漿，濃郁米香混著微甘暖透入胃，豆花滑嫩微燙，帶著純粹的黃豆清香。"),
                ],
            ),
            make_peninsula_object(
                "wt_dining_iron_cauldron",
                "紅鐵燉肉釜",
                &["Look", "Taste", "WarmHands"],
                &[
                    ("Look", "紅鐵厚底大釜慢燉著大角原牛肉與甜白蘿蔔，肉香四溢；旁邊紫銅鴛鴦鍋滾水翻騰，備有切仔湯麵配滷蛋與大圳頭溫米酒。"),
                    ("Taste", "你舀起一勺慢燉的原牛肉湯送入口中，肉塊軟爛透味，湯頭鮮濃醇厚，整個人精神一振。"),
                    ("WarmHands", "你把雙手靠近大釜底座的耐火磚緣，溫熱厚重的火氣烘退了手掌上的水氣與寒意。"),
                ],
            ),
            make_peninsula_object(
                "wt_dining_cold_jade_cabinet",
                "寒水玉冰櫃",
                &["Look", "Open"],
                &[
                    ("Look", "雙層松香木隔熱櫃內壁鑲滿北山寒水玉薄板，櫃身透著天然的清涼寒氣，冰鎮著板豆腐、鮮牛乳與寒水玉冰鎮仙草凍。"),
                    ("Open", "你拉開沉沉的櫃門，一股幽涼白霧迎面而來，裡面整齊碼放著白瓷罐裝的鮮牛乳與浸在井泉裡的涼仙草凍。"),
                ],
            ),
            make_peninsula_object(
                "wt_dining_sauce_rack",
                "醬料陶缸架",
                &["Look", "Smell"],
                &[
                    ("Look", "竹木架上整齊排著大圳頭壺底黑豆醬油、純米醋、脆瓜醬菜、豆腐乳與風櫃尾海味提鮮粉，陶缸口皆封著潔淨的白綿布蓋。"),
                    ("Smell", "你掀開黑豆醬油缸蓋深嗅，古法日曬釀造的濃醇豆香中透著微甘，令人胃口大開。"),
                ],
            ),
        ]
    } else if room_name.contains("旅社") || room_name.contains("茶") {
        vec![
            make_peninsula_object(
                "wt_inn_milk_tea_stove",
                "煮茶泥爐",
                &["Look", "Taste"],
                &[
                    ("Look", "銅皮泥爐上坐著一把長嘴鐵壺，穩火慢煮著霜林烘青茶與牛埔鮮乳交融的鍋煮鮮奶茶；竹盤裡盛著冰鎮甘蔗檸檬汁，配黑糖米香、奶油酥餅與鹽烤海苔片。"),
                    ("Taste", "你捧起陶杯細品熱奶茶，醇厚乳香托著烘青茶的微澀回甘，甘蔗檸檬的酸甜則沁人心脾。"),
                ],
            ),
            make_peninsula_object(
                "wt_inn_ebony_shell_chess",
                "烏木棋盤",
                &["Look", "Operate"],
                &[
                    ("Look", "黑沼沉水烏木雕琢的棋盤線道深沉分明，兩只棋罐各盛著溫潤厚重的烏木黑子與烏石鼻厚海貝白子，茶客對弈殺局難解難分。"),
                    ("Operate", "你拈起一枚沉甸甸的白貝棋子扣在棋盤交叉處，發出清脆沉實的篤響，盤中局勢頓時生出新意。"),
                ],
            ),
            make_peninsula_object(
                "wt_inn_gazette_stand",
                "旬報木報架",
                &["Look", "Read"],
                &[
                    ("Look", "松木斜面報架上夾著講堂街印行館竹綿紙印發的《半島旬報》，底層木格裡收著幾本供人消遣的白話故事小冊。"),
                    ("Read", "你翻開《半島旬報》，頭版記載著老堰頭水利翻修進展、風舶深水碼頭貨船期程，副刊刊著西南原秋收米價與林地軼聞。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_feather_bed_lamp",
                "客房羽絨榻",
                &["Look", "Rest", "Sleep", "Operate"],
                &[
                    ("Look", "獨立銅鎖客房內鋪著蓬鬆羽絨暖被、細編蘆葦涼席與山風草安神枕，床頭立著一盞旋蓋夜光貝母調光台燈。"),
                    ("Rest", "你合衣躺上床榻，蓬鬆柔軟的羽絨與安神草清香迅速舒緩了周身筋骨，睡意悄然湧上。"),
                    ("Sleep", "你鑽入蓬鬆羽絨暖被，旋暗貝母燈，聽著窗外隱約微風沉沉入睡，醒來後體魄與精神盡復。"),
                    ("Operate", "你轉動床頭夜光貝母台燈的烏木旋蓋，外露的柔和珠光自幽微漸轉明亮，將床邊照得一片溫馨澄澈。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && (room_name.contains("浴場") || room_name.contains("浴堂") || room_name.contains("石湯")) {
        vec![
            make_peninsula_object(
                "wt_bath_jade_pool",
                "恆溫熱湯池",
                &["Look", "Rest"],
                &[
                    ("Look", "寬闊的大石池以半嶺寒水玉鋪底，池壁嵌有岑氏恆溫導能火磚，池中浸泡著溫氏祛寒草木湯包，水色泛著微翠草露之潤。"),
                    ("Rest", "你浸入溫潤清透的熱湯中，草木藥氣滲透肌膚脈絡，周身毛孔舒張，奔波積累的疲乏頓時消散殆盡。"),
                ],
            ),
            make_peninsula_object(
                "wt_bath_mirror_washstand",
                "銀鏡盥洗台",
                &["Look", "Use"],
                &[
                    ("Look", "光潔明亮的亮銀玻璃大鏡前，整齊備有軟毛竹柄牙刷、青鹽薄荷潔牙粉、山茶松脂香皂、蜂格棉面巾與松香木梳。"),
                    ("Use", "你取了潔牙粉與軟毛牙刷洗漱，青鹽薄荷的清涼瞬間沁透齒頰，再用山茶松脂香皂掬水洗面，容顏頓覺煥然一新。"),
                ],
            ),
            make_peninsula_object(
                "wt_bath_pine_charcoal_dryer",
                "松炭烘衣櫥",
                &["Look", "DryOff"],
                &[
                    ("Look", "厚實的松香木大立櫥內裝多層橫木桿，底部設導能溫石與松炭烘槽，正散發著乾燥溫純的木質暖風。"),
                    ("DryOff", "你把微潮的衣物掛入櫥中關好木門，片刻後取出，衣物已被烘得乾爽蓬鬆，散發著好聞的松木陽光氣息。"),
                ],
            ),
        ]
    } else if room_name.contains("傢俱") || room_name.contains("巧木") {
        vec![
            make_peninsula_object(
                "wt_wood_planing_bench",
                "刨木工作台",
                &["Look", "Operate"],
                &[
                    ("Look", "厚重工匠台上架著導能刨刀、拉鋸與木鑿，台面落滿薄如蟬翼的松木刨花，木香撲鼻。"),
                    ("Operate", "你握起導能推刨在松木料上平推而過，木片應手捲起均勻刨花，木面光滑平整。"),
                ],
            ),
            make_peninsula_object(
                "wt_wood_bed_and_cabinet",
                "松木彈力床",
                &["Look", "Sit", "Open"],
                &[
                    ("Look", "展示區陳設著繃好高彈棕網的松香木彈力大床，以及內嵌北山寒水玉薄板的保鮮石櫃。"),
                    ("Sit", "你在松香木大床上坐下試壓，棕繩床網承托沉穩富有回彈，坐感舒坦自然。"),
                    ("Open", "你拉開保鮮石櫃木門，夾層中的寒水玉散發清涼氣息，櫃格密合嚴整。"),
                ],
            ),
            make_peninsula_object(
                "wt_wood_display_shelf",
                "烏木茶盤几",
                &["Look", "Use"],
                &[
                    ("Look", "展示架上陳列著黑沼沉水烏木精雕茶盤、烏木筷、松香木梳與小巧置物木匣。"),
                    ("Use", "你端起沉水烏木茶盤撫摸，質地沉重溫潤，水漬不滲，顯出精湛木工手藝。"),
                ],
            ),
        ]
    } else if room_name.contains("服飾") || room_name.contains("織造") || room_name.contains("補給行") {
        vec![
            make_peninsula_object(
                "wt_cloth_garment_rack",
                "防雨風衣架",
                &["Look", "Use"],
                &[
                    ("Look", "松木衣架上整齊掛著塗有黑沼樹膠的防雨風衣、透氣棉麻短衫與北山雪狼絨冬衣，做工扎實。"),
                    ("Use", "你取下一件防雨風衣披在身上試穿，衣料厚韌輕便，領口銅扣封得嚴實，能擋狂風急雨。"),
                ],
            ),
            make_peninsula_object(
                "wt_cloth_bag_umbrella_counter",
                "帆布包傘櫃",
                &["Look", "Open"],
                &[
                    ("Look", "櫃台陳列著雙扣帆布斜背包、軟牛皮零錢夾、竹骨晴雨摺疊傘、軟底居家布拖鞋與隨手五色針線盒。"),
                    ("Open", "你拉開展示櫃門查看，隨手五色針線盒內整齊排著鋼針與彩線，斜背包分格實用耐磨。"),
                ],
            ),
            make_peninsula_object(
                "wt_cloth_tailor_sewing_bench",
                "裁縫量身几",
                &["Look", "Use"],
                &[
                    ("Look", "量身木几上擺著軟皮尺、裁布銅剪與小巧便攜的隨手五色針線盒，方便隨時度量修改身形尺寸。"),
                    ("Use", "你抽出軟皮尺比對衣物袖長，拿出一枚細針將袖口微鬆的線頭俐落收好，熨貼合身。"),
                ],
            ),
        ]
    } else if room_name.contains("鍛造") || room_name.contains("鐵器") || room_name.contains("鑿石") {
        vec![
            make_peninsula_object(
                "wt_forge_furnace_anvil",
                "耐火鍛造台",
                &["Look", "Operate", "WarmHands"],
                &[
                    ("Look", "以紅土耐火磚築成的高爐炭火正熾，爐口火光通明，精鋼鍛台旁擺著導能冷卻水槽與各色鍛錘。"),
                    ("Operate", "你握住重鐵錘在燒紅鐵坯上試敲一記，火星迸濺，金鐵交鳴聲清亮悠長。"),
                    ("WarmHands", "你伸開十指靠近鍛爐熱氣，滾燙厚重的熱浪烘走掌心寒氣，筋骨舒張。"),
                ],
            ),
            make_peninsula_object(
                "wt_forge_brass_hardware_rack",
                "黃銅器物架",
                &["Look", "Use"],
                &[
                    ("Look", "展示架上排著拋光自來水黃銅水栓、輕便隨身保溫瓶、黃銅懷錶、精鋼摺疊剃刀與黃銅彈子門鎖。"),
                    ("Use", "你展開精鋼摺疊剃刀撫看刀口，刃口研磨得鋒銳無匹，轉軸銅釘開闔嚴密順滑。"),
                ],
            ),
            make_peninsula_object(
                "wt_forge_quench_trough",
                "精鋼淬火槽",
                &["Look", "Use"],
                &[
                    ("Look", "盛滿深井泉水與淬火油的大石槽，槽邊備有打磨精鋼刀具與門鎖配件的雙面細磨石。"),
                    ("Use", "你舀水淋在細磨石上推磨鐵件，金屬毛刺漸去，刃口泛起雪亮微光。"),
                ],
            ),
        ]
    } else if room_name.contains("觀星") {
        vec![
            make_peninsula_object(
                "wt_cat_0734",
                "黃銅星軌儀",
                &["Look", "Operate", "Read"],
                &[
                    ("Look", "高聳於白石基座上的精緻銅儀，層層同心環刻劃著半島二十四節氣、晨昏潮位與天象經緯刻度，銅環轉動輕盈無聲。"),
                    ("Operate", "你輕推星軌儀外環，銅齒輪咬合轉動，星象指針精準對向今夜將在東天升起的導航主星。"),
                    ("Read", "你辨讀底座銅盤銘刻的星曆推算表，上面詳載著半島歷代學者觀測潮汐漲落與季風轉折的心血記錄。"),
                ],
            ),
            make_peninsula_object(
                "wt_obs_tide_weather_table",
                "潮汐觀測案",
                &["Look", "Read", "Use"],
                &[
                    ("Look", "厚重的老樟木長案上整齊攤開著半島氣象水文走勢圖、多層手繪潮汐升降曲線與防風墨水盒，供學者評議研討。"),
                    ("Read", "你細讀長案上留存的近期海灣潮位紀錄，字跡密密麻麻，詳盡標註著湧浪高度與洋流變化趨勢。"),
                    ("Use", "你提筆在長案的備忘薄頁上記下今日天色與風向，供下一輪評議學者對照參考。"),
                ],
            ),
            make_peninsula_object(
                "wt_obs_brass_telescope_rack",
                "青銅窺管架",
                &["Look", "Operate"],
                &[
                    ("Look", "架設於觀景石台邊緣的重型青銅窺管，配備精密螺紋微調旋鈕，旁側木架整齊插著數十卷絲帛手繪星圖。"),
                    ("Operate", "你俯身湊近青銅窺管的目鏡，微調螺紋對焦，遠方蒼穹中閃爍的繁星與雲層邊緣清晰呈現在眼前。"),
                ],
            ),
        ]
    } else if room_name.contains("契書局") || room_name.contains("房地契") {
        vec![
            make_peninsula_object(
                "wt_deed_cadastral_map_desk",
                "四坊地籍案",
                &["Look", "Read", "Use"],
                &[
                    ("Look", "寬闊平整的核契楠木長案上平鋪著文曜四坊地籍疆界全圖，一旁陳設著玻璃螺紋墨水瓶與弧形吸墨器。"),
                    ("Read", "你俯身查看四坊地籍圖冊，文曜城中各街各巷街屋的界址、戶號與公產邊界皆以硃砂與濃墨標記分明。"),
                    ("Use", "你執起木柄弧形吸墨器在草擬文契上輕輕滾過，剛寫就的黑墨油光瞬間被乾淨吸附，字跡烏黑硬朗。"),
                ],
            ),
            make_peninsula_object(
                "wt_deed_household_registry",
                "樟木戶籍櫃",
                &["Look", "Read", "Open"],
                &[
                    ("Look", "頂天立地的高大樟木檔案櫃，以生鐵包角加固，櫃門貼著分類封籤，保存著《半島全戶總譜》與歷任戶籍謄本。"),
                    ("Read", "你查閱外格貼附的戶籍檔案檢索目錄，各鎮居民落籍與各坊家戶丁冊皆按年月編次，巨細靡遺。"),
                    ("Open", "你拉開沉重的鐵扣樟木抽屜，一陣防蠹樟腦香撲面而來，厚重的羊皮紙總譜冊頁陳列井然。"),
                ],
            ),
            make_peninsula_object(
                "wt_cat_0793",
                "紅印屋契摺",
                &["Look", "Read", "Acquire"],
                &[
                    ("Look", "厚實木盤中擺著蓋有文曜公議堂硃砂官印的正本屋契與租約摺。依半島律例，凡立契買下街屋者可獲發大門銅鎖鑰匙，支付 25 鎂即可辦理置產。"),
                    ("Read", "你細閱屋契條款，公議堂硃砂大印鮮明，清楚載明產權過戶、房屋修繕與大門配匙登錄章程，童叟無欺。"),
                    ("Acquire", "你向書吏出具購屋憑證並繳清規費，在紅印屋契與租約摺上鄭重畫押，順利領取屬於自己的置產契書與門牌底冊。"),
                ],
            ),
        ]
    } else if room_name.contains("公議")
        || room_name.contains("公所")
        || room_name.contains("公會")
        || room_name.contains("礦務所")
        || room_name.contains("公事房")
        || room_name.contains("商盟")
    {
        vec![
            make_peninsula_object(
                "wt_hall_council_table",
                "厚木公議桌",
                &["Look", "Sit", "Read"],
                &[
                    ("Look", "廳堂中央擺設著長條大木會議桌，桌面整齊放著公議議程冊、公章墨盒與簽名木籌。"),
                    ("Sit", "你在厚重的長背木椅上坐定，廳內氣氛莊重嚴明，公議秩序井然。"),
                    ("Read", "你翻開議程冊閱覽，上面記錄著即將商討的供水調配方案與各區工料評定細則。"),
                ],
            ),
            make_peninsula_object(
                "wt_hall_bulletin_board",
                "民務告示欄",
                &["Look", "Read"],
                &[
                    ("Look", "牆側立著高大的厚木告示欄，張貼著新頒定的水額管理辦法、工坊評定標準與季節巡防名單。"),
                    ("Read", "你逐條細讀告示內容，公約條款字跡剛勁端正，旨在保障水土民生休養生息。"),
                ],
            ),
            make_peninsula_object(
                "wt_hall_seal_archive",
                "雙鎖公文櫃",
                &["Look", "Open"],
                &[
                    ("Look", "靠牆安放著帶雙銅鎖的公文卷櫃，案上備有用於封存公函的專用紅陶印泥與封條紙。"),
                    ("Open", "你拉開已解鎖的文卷櫃抽屜，歷年公議決議文冊皆按年號編目整齊，保存極為完好。"),
                ],
            ),
        ]
    } else if room_name.contains("藏書閣")
        || room_name.contains("印行館")
        || room_name.contains("書局")
        || room_name.contains("圖誌館")
    {
        vec![
            make_peninsula_object(
                "wt_book_printing_shelf",
                "風物誌書架",
                &["Look", "Read"],
                &[
                    ("Look", "高大書架上整齊排著竹綿紙印製的《白塔風物誌》、每旬《半島旬報》與白話故事小冊，滿室紙墨芬芳。"),
                    ("Read", "你取下一卷《白塔風物誌》閱讀，書中記載著老堰頭水網修築沿革與寒水玉開採考證。"),
                ],
            ),
            make_peninsula_object(
                "wt_book_map_reading_table",
                "全圖閱覽桌",
                &["Look", "Read", "Open"],
                &[
                    ("Look", "長木桌上平鋪著精細手繪的白塔半島山海全圖，配備著黃銅放大鏡與精準標尺。"),
                    ("Read", "你細看圖上標註的水道、礦脈與城鎮驛站路線，對半島各地山海形勢瞭然於胸。"),
                    ("Open", "你翻開桌側的地圖抽屜，取出周邊海域潮汐水深航圖細細對照。"),
                ],
            ),
            make_peninsula_object(
                "wt_book_pen_notebook_desk",
                "沾水筆書案",
                &["Look", "Use"],
                &[
                    ("Look", "寬大書案上鋪著氈布，銅筆筒裡插著鋼尖沾水筆，案旁置有墨水瓶與竹綿紙記事手札。"),
                    ("Use", "你提起鋼尖沾水筆蘸墨落筆，筆尖在竹綿紙上順滑游走，字跡清晰深沉，墨乾極速。"),
                ],
            ),
        ]
    } else if !is_outdoor_road
        && (room_name.contains("學院")
            || room_name.contains("講習")
            || room_name.contains("國小")
            || room_name.contains("教室")
            || room_name.contains("校舍")
            || room_name.contains("學舍")
            || (room_name.contains("講堂")
                && !room_name.contains("講堂街")
                && !room_name.contains("講堂外坊")
                && !room_name.contains("講堂內環")
                && !room_name.contains("講堂外環")))
    {
        vec![
            make_peninsula_object(
                "wt_academy_fountain_pen",
                "黃銅儲墨筆",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "烏木教案的正中央擱著一支溫潤沉甸的黃銅桿儲墨鋼筆，筆尖以精煉銥金打磨得細膩微光，旁側襯著吸墨氈布與寫滿批註的竹綿紙講義。"),
                    ("Use", "你執起教案上的黃銅鋼筆，銥金筆尖觸及竹綿紙面，黑墨勻稱流暢地順著筆鋒沁入紙紋，留下一行端勁清晰的手跡。"),
                    ("Operate", "你輕旋黃銅筆桿檢視儲墨囊，墨色飽滿澄澈，筆帽卡榫嚴絲合縫，足見文曜百工製筆之精巧。"),
                ],
            ),
            make_peninsula_object(
                "wt_book_printing_shelf",
                "風物誌書架",
                &["Look", "Read"],
                &[
                    ("Look", "高大書架上整齊排著竹綿紙印製的《白塔風物誌》、每旬《半島旬報》與白話故事小冊，滿室紙墨芬芳。"),
                    ("Read", "你取下一卷《白塔風物誌》閱讀，書中記載著老堰頭水網修築沿革與寒水玉開採考證。"),
                ],
            ),
            make_peninsula_object(
                "wt_book_pen_notebook_desk",
                "沾水筆書案",
                &["Look", "Use"],
                &[
                    ("Look", "寬大書案上鋪著氈布，銅筆筒裡插著鋼尖沾水筆，案旁置有墨水瓶與竹綿紙記事手札。"),
                    ("Use", "你提起鋼尖沾水筆蘸墨落筆，筆尖在竹綿紙上順滑游走，字跡清晰深沉，墨乾極速。"),
                ],
            ),
        ]
    } else if room_name.contains("診所") {
        vec![
            make_peninsula_object(
                "wt_clinic_herb_cabinet",
                "百草丸藥櫃",
                &["Look", "Smell", "Use"],
                &[
                    ("Look", "百草藥櫃抽屜密布，桌上整齊排著黑沼止血水苔敷藥、避瘴清心丸、祛寒草木湯包與青鹽薄荷潔牙粉。"),
                    ("Smell", "你低頭深嗅藥盤，薄荷清涼、艾草微辛與止血水苔的草香交織，胸悶頓解。"),
                    ("Use", "你取了一小撮避瘴清心丸服下，清涼藥力入喉，周身氣血頓時平順舒暢。"),
                ],
            ),
            make_peninsula_object(
                "wt_clinic_acupoint_bed",
                "竹席診脈榻",
                &["Look", "Rest"],
                &[
                    ("Look", "診榻鋪著細編竹席與山風草安神枕，榻邊點著驅蚊艾草香盤，靜謐祥和。"),
                    ("Rest", "你躺上竹席診榻靜臥歇息，草木香氣寧神靜氣，緊繃的身心逐漸放鬆下來。"),
                ],
            ),
            make_peninsula_object(
                "wt_clinic_decoction_stove",
                "煎藥小銅爐",
                &["Look", "Taste", "WarmHands"],
                &[
                    ("Look", "小銅爐上砂鍋文火慢熬著溫氏祛寒草木湯包，蒸汽裊裊，飄出濃郁草木溫熱氣息。"),
                    ("Taste", "你啜飲一小口熬好的草木溫湯，微苦回甘，暖流順著喉嚨流向四肢百骸。"),
                    ("WarmHands", "你把手掌靠在銅爐外罩旁，受溫熱藥香蒸汽烘暖，指頭血氣活絡。"),
                ],
            ),
        ]
    } else if room_name.contains("石磨") {
        vec![
            make_peninsula_object(
                "wt_mill_waterwheel_stone",
                "水輪玄武磨",
                &["Look", "Operate"],
                &[
                    ("Look", "老堰頭引水渠衝擊巨木水輪帶動齒輪轉動，兩合巨大玄武岩磨盤正飛速研磨新熟米穀。"),
                    ("Operate", "你扳動水流分水閘柄調節水勢，水輪轉速隨之放緩，磨盤研磨出的白米粉更顯細密勻淨。"),
                ],
            ),
            make_peninsula_object(
                "wt_mill_flour_sieve",
                "竹絲篩粉斗",
                &["Look", "Use"],
                &[
                    ("Look", "細如青絲的密編竹篩斗分出粗米糠與細米粉，篩下積著雪白如霜的純米粉堆。"),
                    ("Use", "你雙手晃動懸掛的竹篩斗，雪白米粉簌簌落下，粗糠被乾淨留在篩網上方。"),
                ],
            ),
            make_peninsula_object(
                "wt_mill_grain_hopper",
                "杉木儲糧斗",
                &["Look", "Open"],
                &[
                    ("Look", "高懸的倒梯形杉木儲料斗連接著下料槽，旁邊架著點驗原糧重量的大秤。"),
                    ("Open", "你拉開料斗下方的調節插板，顆粒飽滿的金色稻穀嘩啦啦滑向磨眼，源源不斷。"),
                ],
            ),
        ]
    } else if room_name.contains("鋸木") {
        vec![
            make_peninsula_object(
                "wt_sawmill_hydraulic_saw",
                "水動排鋸架",
                &["Look", "Operate"],
                &[
                    ("Look", "水流帶動的長排精鋼鋸條轟鳴起伏，平穩將整根百年雲杉原木剖切成均勻厚板。"),
                    ("Operate", "你推動送木台車向前咬緊鋸齒，鋼鋸飛速下切，鋸路筆直順暢，木屑紛飛。"),
                ],
            ),
            make_peninsula_object(
                "wt_sawmill_timber_bench",
                "原木檢寸台",
                &["Look", "Use"],
                &[
                    ("Look", "鐵木長台上刻有度量標尺，擺著散發濃烈松脂香氣的試切木塊與標記石筆。"),
                    ("Use", "你拿起石筆比對標尺在板材邊緣畫下一道規整的裁切基線，下料尺寸分毫不大。"),
                ],
            ),
            make_peninsula_object(
                "wt_sawmill_woodchip_bin",
                "乾燥刨花桶",
                &["Look", "Smell", "Gather"],
                &[
                    ("Look", "高大的松木大桶盛滿剛刨下的乾燥木片與刨花，這是燒製上等松炭的上乘原料。"),
                    ("Smell", "你捧起一把乾燥刨花深嗅，濃郁清冽的松脂油脂香撲面而來，乾爽宜人。"),
                    ("Gather", "你順手抓起一紮結實乾燥的松木刨花紮好，可作為生火引柴的上好引信。"),
                ],
            ),
        ]
    } else if room_name.contains("切石") {
        vec![
            make_peninsula_object(
                "wt_stonecut_wire_saw",
                "金剛砂拉鋸",
                &["Look", "Operate"],
                &[
                    ("Look", "工匠引細密合金絲拉鋸帶著濕潤金剛砂緩緩剖開巨大的北山寒水玉原石，石皮下透出溫潤玉光。"),
                    ("Operate", "你輕轉注水輪維持金剛砂泥漿順暢流入切割線，金屬絲勻速拉動，石料緩緩剖為兩半。"),
                ],
            ),
            make_peninsula_object(
                "wt_stonecut_jade_display",
                "寒水玉料架",
                &["Look", "Use"],
                &[
                    ("Look", "展示架上整齊排著剖切平整的寒水玉薄板，手觸如浸涼泉，是打造保鮮石櫃的極品材料。"),
                    ("Use", "你伸手撫摸一片寒水玉石板，指掌頓感沁涼透骨，板面質地堅密細潤無瑕。"),
                ],
            ),
            make_peninsula_object(
                "wt_stonecut_grind_basin",
                "拋光琢石盆",
                &["Look", "Use"],
                &[
                    ("Look", "引流活水的不鏽石盆中浸著各目數的磨石與羊毛拋光氈，專用於精細琢磨石板邊角。"),
                    ("Use", "你取了一塊切好的寒水玉小塊在細石上加水研磨，石角稜邊很快變得圓潤光滑。"),
                ],
            ),
        ]
    } else if room_name.contains("觀測") {
        vec![
            make_peninsula_object(
                "wt_observatory_anemometer",
                "黃銅風向儀",
                &["Look", "Operate", "Read"],
                &[
                    ("Look", "露台上架著精巧的八方黃銅風向羽與四轉杯，精準指引著半島上空雲霧與氣流動向。"),
                    ("Operate", "你輕推校正旋鈕調整齒輪平衡，指針隨風微微擺動，指向西北風向。"),
                    ("Read", "你查看儀器底盤上的刻度，風速平緩穩定，近期半島沿岸無暴風驟雨之虞。"),
                ],
            ),
            make_peninsula_object(
                "wt_observatory_cloud_log",
                "天象觀測冊",
                &["Look", "Read"],
                &[
                    ("Look", "厚實的竹綿紙記錄冊詳實抄錄著半嶺與四方海灣十年間的節氣、降雨與霜凍日曆。"),
                    ("Read", "你翻閱記錄冊最近數頁，上面載明今日雲氣漸聚於北山山脊，傍晚或有山林微雨。"),
                ],
            ),
            make_peninsula_object(
                "wt_observatory_brass_telescope",
                "黃銅望遠鏡",
                &["Look", "Use"],
                &[
                    ("Look", "架設於石基轉軸上的長筒黃銅望遠鏡，鏡頭打磨純淨，筒身嵌有夜光貝校準螺絲。"),
                    ("Use", "你伏身湊近目鏡遠眺，數十里外的海灣帆影與山原林色歷歷在目，極目千里。"),
                ],
            ),
        ]
    } else if room_name.contains("試招") {
        vec![
            make_peninsula_object(
                "wt_arena_testing_pillar",
                "玄武試勁柱",
                &["Look", "Operate"],
                &[
                    ("Look", "場中央立著取自烏石鼻的沉重玄武岩石柱，柱面留有深淺不一的掌印、拳痕與兵刃劈砍印記。"),
                    ("Operate", "你運勁凝神朝石柱發力擊出一記，拳風激盪石壁發出沉悶轟響，指掌微麻筋骨通暢。"),
                ],
            ),
            make_peninsula_object(
                "wt_arena_wooden_weapon_rack",
                "鐵木兵器架",
                &["Look", "Use"],
                &[
                    ("Look", "架上排滿各型配重勻稱的鐵木長棍、練習單刀、短槍與厚實的原牛皮護胸。"),
                    ("Use", "你抽出一桿沉實的鐵木長棍在手中虛晃挽出幾朵棍花，重心沉著穩當，手感極佳。"),
                ],
            ),
            make_peninsula_object(
                "wt_arena_rest_bench",
                "歇腳粗布凳",
                &["Look", "Rest"],
                &[
                    ("Look", "場邊長凳鋪著耐磨粗布墊，旁邊常備著供習練者洗臉降溫的深井涼泉陶缸。"),
                    ("Rest", "你在粗布長凳上坐下吐納調息，平復躁動血氣，奔馳發力的筋骨漸漸恢復充盈精力。"),
                ],
            ),
        ]
    } else if room_name.contains("農具") {
        vec![
            make_peninsula_object(
                "wt_farm_tool_iron_plow",
                "雙齒精鐵犁",
                &["Look", "Operate"],
                &[
                    ("Look", "為原牛翻耕黏土設計的淬火精鐵犁鏵，泥槽打磨平滑，犁尖銳利厚實。"),
                    ("Operate", "你試抬起犁鏵大轅感受配重，槓桿角度設計精巧，翻開深層黑土輕便省力。"),
                ],
            ),
            make_peninsula_object(
                "wt_farm_tool_bamboo_harrow",
                "竹編平土耙",
                &["Look", "Use"],
                &[
                    ("Look", "排布均勻的竹木平土排耙與輕便鋼口除草鋤，擺在農夫取用順手的高架上。"),
                    ("Use", "你拿起除草鋤試鋤地面浮泥，鋼口切入泥土利索乾脆，雜草根系迎刃而斷。"),
                ],
            ),
            make_peninsula_object(
                "wt_farm_tool_whetstone_trough",
                "活水磨刀槽",
                &["Look", "Use"],
                &[
                    ("Look", "木架凹槽裡嵌著一粗一細兩合磨石，活水自小竹管徐徐滴淋在石面保持濕潤。"),
                    ("Use", "你把身邊的鐵刃貼在磨石上推磨幾回，泥漿流動間，原本微鈍的刃口頓時磨得鋥亮。"),
                ],
            ),
        ]
    } else if room_name.contains("乳酪") || room_name.contains("羊毛") {
        vec![
            make_peninsula_object(
                "wt_cheese_curd_press",
                "螺旋壓酪架",
                &["Look", "Operate", "Taste"],
                &[
                    ("Look", "大木架上設有黃銅螺旋加壓手柄，正將粗濾布中的凝乳塊瀝出金黃乳清，壓製硬乳酪輪。"),
                    ("Operate", "你握住螺旋桿加力旋轉半圈，木板沉重下壓，澄澈的乳清順著凹槽汩汩流出。"),
                    ("Taste", "你嚐了指尖沾上的一滴新鮮乳清，滋味微酸清冽，帶著濃醇的天然奶香。"),
                ],
            ),
            make_peninsula_object(
                "wt_wool_spinning_wheel",
                "手搖木紡車",
                &["Look", "Use"],
                &[
                    ("Look", "案上堆放著洗淨蓬鬆的乳白山羊毛條，竹木紡車轉軸靈動，紡出勻細結實的線團。"),
                    ("Use", "你輕輕搖動紡車木柄，纖細羊毛在引線指引下迅速捻轉成光滑毛線，均勻綿密。"),
                ],
            ),
            make_peninsula_object(
                "wt_cheese_aging_shelf",
                "乳酪熟成架",
                &["Look", "Smell", "Open"],
                &[
                    ("Look", "多層松木架內嵌寒水玉薄板調節溫涼，架上排滿刷洗乾淨、裹著黃蠟的牛埔陳年硬乳酪。"),
                    ("Smell", "你靠近熟成架細聞，發酵成熟的乳香濃郁厚重，混合著天然油脂與微鹹海風韻味。"),
                    ("Open", "你拉開底層遮光木簾，裡面陳列著剛熟成三個月的金黃乳酪輪，成色極佳。"),
                ],
            ),
        ]
    } else if room_name.contains("選礦") {
        vec![
            make_peninsula_object(
                "wt_mine_wash_chute",
                "木製洗礦槽",
                &["Look", "Operate"],
                &[
                    ("Look", "山澗急流順著長木槽奔騰而下，槽底隔木截留住沉重赤鐵礦砂，洗去浮游紅泥。"),
                    ("Operate", "你以長木耙梳理泥槽底層礦石，比重較輕的雜石順水沖走，沉甸甸的鐵礦砂盡收格內。"),
                ],
            ),
            make_peninsula_object(
                "wt_mine_sample_tray",
                "礦石樣品箱",
                &["Look", "Use"],
                &[
                    ("Look", "分格木箱裡擺著高品位重鐵礦石塊與細膩如雪的高嶺瓷土樣粉，標籤字跡清晰。"),
                    ("Use", "你掂起一塊沉甸甸的黑紅赤鐵礦石，質地堅密沉實，正是鍛造上好精鐵的基石。"),
                ],
            ),
            make_peninsula_object(
                "wt_mine_sorting_sieve",
                "分級重鐵篩",
                &["Look", "Operate"],
                &[
                    ("Look", "懸掛在木架下的雙層鐵絲篩網，孔徑由大到小，將開採出的碎礦精確分級。"),
                    ("Operate", "你推動鐵篩手柄用力搖動，小顆粒礦砂簌簌漏下，大塊粗礦穩穩留於上層。"),
                ],
            ),
        ]
    } else if room_name.contains("台車") {
        vec![
            make_peninsula_object(
                "wt_trolley_rail_carriage",
                "雙軸重台車",
                &["Look", "Operate"],
                &[
                    ("Look", "坑道軌道上停著厚重雙軸台車，精鐵鑄造的車輪扣住木軌鐵皮，裝滿粗礦依然平穩。"),
                    ("Operate", "你握住車身旁的機械制動閘手柄扳動，鐵閘緊緊咬住輪緣，發出沉實摩擦聲。"),
                ],
            ),
            make_peninsula_object(
                "wt_trolley_grease_pot",
                "桐油潤滑壺",
                &["Look", "Use"],
                &[
                    ("Look", "油箱邊放著盛滿防鏽桐油脂的長嘴鐵壺，以及用於緊固軌道螺栓的粗重鍛鐵扳手。"),
                    ("Use", "你提起油壺往台車轉軸注入口滴入幾滴桐油脂，原本微澀的輪軸隨即轉動無聲。"),
                ],
            ),
            make_peninsula_object(
                "wt_trolley_brake_winch",
                "鋼索絞盤",
                &["Look", "Operate"],
                &[
                    ("Look", "固定在坑口條石基座上的大型手搖絞盤，上面纏繞著粗大耐磨的精鋼牽引索。"),
                    ("Operate", "你推動棘輪卡榫試轉絞盤，齒輪咬合緊密，能輕鬆將滿載台車拉上陡斜坡道。"),
                ],
            ),
        ]
    } else if room_name.contains("燈塔") {
        vec![
            make_peninsula_object(
                "wt_lighthouse_fresnel_lens",
                "聚光巨鏡組",
                &["Look", "Operate"],
                &[
                    ("Look", "高塔頂層旋轉銅架上鑲嵌著碩大的夜光貝打磨薄片與厚琉璃凸透鏡，聚起刺破黑夜的耀目光柱。"),
                    ("Operate", "你輕推銅架轉輪微調鏡組方位，聚光透鏡緩緩轉動，光束精準掃過遠方險礁水域。"),
                ],
            ),
            make_peninsula_object(
                "wt_lighthouse_whale_oil_burner",
                "海油恆溫爐",
                &["Look", "WarmHands"],
                &[
                    ("Look", "精銅爐膛內燃燒著精煉海獸油，芯線粗實，散發著溫純明亮的橙紅火光與微溫。"),
                    ("WarmHands", "你將雙手覆在燈爐外緣銅罩上，高塔風烈之下的冰冷指尖瞬間被烘得暖和無比。"),
                ],
            ),
            make_peninsula_object(
                "wt_lighthouse_log_desk",
                "夜航日誌案",
                &["Look", "Read"],
                &[
                    ("Look", "書案上整齊擺放著潮汐圖表、銅墨水盒與記載過往船隻燈號答覆的夜航日誌。"),
                    ("Read", "你翻開夜航日誌閱覽，上面工整記著昨夜三艘外澳貨船平安繞過暗礁的航行記錄。"),
                ],
            ),
        ]
    } else if room_name.contains("冷燻") {
        vec![
            make_peninsula_object(
                "wt_smokehouse_kiln",
                "慢燻紅陶灶",
                &["Look", "Smell"],
                &[
                    ("Look", "地面砌出長排低溫煙道陶灶，正以受控的乾燥松木鋸屑慢火逼出濃郁木香冷煙。"),
                    ("Smell", "你深吸一口自陶灶溢出的煙氣，醇厚的松木煙燻香氣純淨甘醇，全無嗆鼻焦味。"),
                ],
            ),
            make_peninsula_object(
                "wt_smokehouse_fish_rack",
                "冷燻海魚架",
                &["Look", "Taste"],
                &[
                    ("Look", "懸架上整齊掛著一排排泛著琥珀油亮金紅光澤的冷燻鯖魚與海苔魚乾，油脂豐厚。"),
                    ("Taste", "你撕下一小片冷燻魚肉放入口中，肉質緊實彈牙，鹹香甘醇中交織著深沉的松煙木香。"),
                ],
            ),
            make_peninsula_object(
                "wt_smokehouse_salt_barrel",
                "海鹽醃魚桶",
                &["Look", "Open"],
                &[
                    ("Look", "杉木大桶盛滿風舶角日曬結晶的潔白粗海鹽，用以在下燻爐前醃透鮮魚肉質。"),
                    ("Open", "你掀開杉木桶蓋，大顆粒海鹽晶瑩剔透，底下浸潤著新近剖好醃漬的鮮美魚肉。"),
                ],
            ),
        ]
    } else if room_name.contains("巨藻") {
        vec![
            make_peninsula_object(
                "wt_kelp_drying_trellis",
                "高架曬藻棚",
                &["Look", "Gather"],
                &[
                    ("Look", "迎著海風搭建的巨型竹棚上懸垂著長達數丈的深海厚巨藻，海風吹拂下漸乾變韌。"),
                    ("Gather", "你從竹架邊緣摘下一束完全曬乾的深黑巨藻片折好，乾脆柔韌，滿是海味精華。"),
                ],
            ),
            make_peninsula_object(
                "wt_kelp_rope_twister",
                "海纜搓繩輪",
                &["Look", "Operate"],
                &[
                    ("Look", "工匠用極富韌性的巨藻纖維與麻絲混合，在木轉輪上捻搓出不懼海水浸蝕的沉重船纜。"),
                    ("Operate", "你推動轉輪搖柄旋轉，條條藻絲纖維被均勻絞緊成粗壯纜繩，結實程度堅如生鐵。"),
                ],
            ),
            make_peninsula_object(
                "wt_kelp_powder_mill",
                "藻粉研石臼",
                &["Look", "Smell", "Taste"],
                &[
                    ("Look", "青石臼中正用石杵搗研烘乾的厚巨藻脆片，研磨出的深翠色細粉即是提鮮神物。"),
                    ("Smell", "你湊近石臼輕聞，濃郁鮮美的天然海潮香氣撲面而來，引人食指大動。"),
                    ("Taste", "你以指尖沾少許海藻提鮮粉嚐了嚐，鹹鮮甘美無比，是半島菜餚不可或缺的調味秘方。"),
                ],
            ),
        ]
    } else if room_name.contains("灰窯") {
        vec![
            make_peninsula_object(
                "wt_lime_kiln_chamber",
                "海貝燃灰窯",
                &["Look", "WarmHands", "Operate"],
                &[
                    ("Look", "烏石鼻以玄武岩壘砌的圓形灰窯，烈火正將堆積的海貝殼高溫煅燒成雪白的建材灰粉。"),
                    ("WarmHands", "你將身子貼近灰窯厚實的外牆，石壁蓄積的渾厚餘熱烘散了海邊帶來的陰冷濕氣。"),
                    ("Operate", "你使用長柄鐵扒撥動窯底通風孔讓空氣流通，爐膛火勢更加旺盛純青。"),
                ],
            ),
            make_peninsula_object(
                "wt_lime_powder_bin",
                "熟貝灰石槽",
                &["Look", "Use"],
                &[
                    ("Look", "大石槽中盛滿過篩後的熟貝灰粉，細膩如白麵，遇水化作黏結力極強的砌石砂漿。"),
                    ("Use", "你取少許熟貝灰加水在陶盆中攪拌，灰漿濃稠細膩，抹在石縫間黏合得天衣無縫。"),
                ],
            ),
            make_peninsula_object(
                "wt_lime_shell_pile",
                "粗貝殼料堆",
                &["Look", "Gather"],
                &[
                    ("Look", "窯外堆積著如小山般的厚重海貝殼，皆是海風大浪自深海衝刷上海灘的天然饋贈。"),
                    ("Gather", "你自料堆拾起幾片質地特別厚重堅硬的大海貝殼，擦淨細沙收於手中。"),
                ],
            ),
        ]
    } else if room_name.contains("獵具") {
        vec![
            make_peninsula_object(
                "wt_hunt_heavy_crossbow",
                "牛角破甲弩",
                &["Look", "Operate"],
                &[
                    ("Look", "工坊以大角原牛角片與堅木多層膠合的守林重弩，弩弦以原牛筋絞就，張力沉雄。"),
                    ("Operate", "你扳動腳蹬絞盤為重弩上弦，機括發出清脆卡響，鎖定弩機扣板，蓄勢待發。"),
                ],
            ),
            make_peninsula_object(
                "wt_hunt_iron_trap",
                "帶齒鋼獸夾",
                &["Look", "Use"],
                &[
                    ("Look", "塗有黑沼天然樹膠防鏽黑漆的踏板精鋼獸夾，彈簧沉重有力，專用於防範猛獸。"),
                    ("Use", "你拿起長鐵枝撥動安全鎖扣試探獸夾靈敏度，踏板微沉立刻金屬閉合，迅猛精準。"),
                ],
            ),
            make_peninsula_object(
                "wt_hunt_tallow_kit",
                "牛油護弦盒",
                &["Look", "Use"],
                &[
                    ("Look", "木盒內收著浸透精煉牛油的布團與蜂蠟塊，是防止弓弦受潮發霉的必備護理之物。"),
                    ("Use", "你取了浸油布團細細擦拭牛筋弩弦，油脂浸入纖維，原本乾燥的弦絲立顯光澤柔韌。"),
                ],
            ),
        ]
    } else if room_name.contains("竹編") || room_name.contains("蘆葦") {
        vec![
            make_peninsula_object(
                "wt_bamboo_weaving_bench",
                "剖竹切篾台",
                &["Look", "Operate"],
                &[
                    ("Look", "平整木剖台上架著圓弧形雙柄剖竹刀，熟練工匠正將青竹劈成薄如蟬翼的均勻竹篾。"),
                    ("Operate", "你雙手握持刀柄順著竹管纖維平推，青竹瞬間應刃破開，剖出兩根粗細一致的平滑竹條。"),
                ],
            ),
            make_peninsula_object(
                "wt_bamboo_woven_crates",
                "雙層竹魚籠",
                &["Look", "Use"],
                &[
                    ("Look", "夾層塗有防水樹膠的密編雙層竹箱與魚籠，輕便結實，通風防潮，是水鄉日用良器。"),
                    ("Use", "你合上手提竹箱的竹編搭扣試提，箱身輕巧挺括，能承受數十斤重物而不變形。"),
                ],
            ),
            make_peninsula_object(
                "wt_bamboo_drying_rack",
                "青竹晾曬架",
                &["Look", "Gather"],
                &[
                    ("Look", "大棚架上整齊排晾著經過熱水蒸煮去糖防蛀的老青竹竿，竹皮碧綠油亮，質地極韌。"),
                    ("Gather", "你從架上挑選了一截粗細勻稱的乾燥老竹管收好，可用於製作器械或竹編配件。"),
                ],
            ),
        ]
    } else if room_name.contains("船塢") || room_name.contains("帆具") || room_name.contains("船材") {
        vec![
            make_peninsula_object(
                "wt_shipyard_dry_dock",
                "雲杉龍骨架",
                &["Look", "Operate"],
                &[
                    ("Look", "斜坡船台架著一整根長逾十丈的厚重雲杉大龍骨，工匠們正以青銅大釘和榫卯拼合肋骨。"),
                    ("Operate", "你拿起大木槌將一枚粗大青銅釘打入龍骨榫卯深處，木料緊密咬合，牢固無比。"),
                ],
            ),
            make_peninsula_object(
                "wt_shipyard_sail_rigging",
                "巨藻船纜堆",
                &["Look", "Use"],
                &[
                    ("Look", "棚內堆放著浸過桐油的防風暴密織重帆布、滑車索具與剛搓好的深海巨藻大絞纜。"),
                    ("Use", "你伸手扯動帆索穿過滑車組試拉，滑車轉動順暢無阻，粗重帆布應聲升起。"),
                ],
            ),
            make_peninsula_object(
                "wt_shipyard_caulking_tar",
                "桐油捻縫鍋",
                &["Look", "WarmHands", "Smell"],
                &[
                    ("Look", "鐵鍋正慢火加熱由桐油、熟貝灰與細麻絲調製的捻縫灰膏，用於填補船板接縫。"),
                    ("WarmHands", "你把雙手靠近鐵鍋周圍的溫熱泥灶，濃稠灰膏散發的熱度將指尖寒意全數烘化。"),
                    ("Smell", "你細聞灰鍋飄出的氣味，桐油的醇厚木香伴著貝灰氣息，是海港船塢熟悉的味道。"),
                ],
            ),
        ]
    } else if room_name.contains("商行") || room_name.contains("契據") {
        vec![
            make_peninsula_object(
                "wt_merchant_ledger_counter",
                "樟木掌事台",
                &["Look", "Read", "Use"],
                &[
                    ("Look", "整塊老樟木打造的掌事長台，台面擺滿銅秤、商盟印鑑、硃砂印泥與出入貨記帳契簿。"),
                    ("Read", "你閱覽櫃台上攤開的貨運契約簿，上面詳細登記著近日運往各鎮的糧油與礦料數量。"),
                    ("Use", "你拿起硃砂印章在契據末尾端正蓋下一記鮮紅印記，商契正式生效立案。"),
                ],
            ),
            make_peninsula_object(
                "wt_merchant_currency_exchange",
                "商盟票據箱",
                &["Look", "Open"],
                &[
                    ("Look", "加固銅角木箱內整齊收存著各商號發行的兌換信票，以及分格盛裝零碎金銀的軟牛皮夾。"),
                    ("Open", "你拉開分格木抽屜查看，各色商行票據分類清晰，印鑑完備，流通無礙。"),
                ],
            ),
            make_peninsula_object(
                "wt_merchant_sample_case",
                "特產陳列櫥",
                &["Look", "Open"],
                &[
                    ("Look", "靠牆玻璃櫥內展示著半島各地的代表物產樣品：紅土鐵錠、牛埔乳酪、霜林茶葉與烏石海貝。"),
                    ("Open", "你輕推櫥門取出霜林茶葉樣品查看，茶葉乾爽翠綠，散發著頂級烘青茶的幽香。"),
                ],
            ),
        ]
    } else if room_name.contains("聯運")
        || room_name.contains("貨運")
        || room_name == "老堰頭・村中泥徑"
        || (room_name.contains("半嶺鎮") && room_name.contains("街"))
        || (room_name.contains("大圳頭") && room_name.contains("街"))
        || (room_name.contains("南哨") && room_name.contains("街"))
        || (room_name.contains("風舶角") && room_name.contains("街"))
    {
        vec![
            make_peninsula_object(
                "wt_caravan_harness_rack",
                "雙輪大貨車",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "大棚下停滿雙輪大貨車，架上排著浸過牛油的厚牛皮馱墊與精鐵挽具。"),
                    ("Use", "你取下原牛挽具檢查皮帶扣環，牛皮堅韌柔軟，鐵扣打磨光滑，毫無磨損隱患。"),
                    ("Operate", "你協同車夫將重貨車大轅抬起卡入牽引插銷，車架結實穩固，準備發車上路。"),
                ],
            ),
            make_peninsula_object(
                "wt_caravan_water_trough",
                "青石飲馬槽",
                &["Look", "Use"],
                &[
                    ("Look", "連通活水渠的深鑿青石長水槽，供來往跋涉的原牛與健壯駝獸低頭痛飲，水質清澈。"),
                    ("Use", "你拔開進水木栓注入新鮮井泉活水，奔波而來的原牛探頭大口痛飲，發出滿足低哼。"),
                ],
            ),
            make_peninsula_object(
                "wt_caravan_ticket_counter",
                "班次木牌案",
                &["Look", "Read", "Use"],
                &[
                    ("Look", "木牌案上每日更換木質班次籤牌，寫明發往大圳頭、風舶角與南哨的車馬出發時辰。"),
                    ("Read", "你細看班次牌，往西南原方向的大車午後即將發班，回程車隊則已進棚卸貨。"),
                    ("Use", "你向車馬行掌櫃交付車資，登車準備前往下一處聯運重鎮。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && (room_name.contains("鐘樓") || room_name.contains("信使") || room_name.contains("瞭望")) {
        vec![
            make_peninsula_object(
                "wt_clocktower_bronze_bell",
                "報時青銅鐘",
                &["Look", "Operate"],
                &[
                    ("Look", "高懸塔頂的厚重青銅巨鐘，垂下粗大的麻繩，每逢正點敲響，鐘聲清越浩蕩傳遍全境。"),
                    ("Operate", "你雙手握住粗麻繩合力一拉，青銅鐘錘重重撞上鐘體，發出一聲渾厚深沉的悠揚巨響。"),
                ],
            ),
            make_peninsula_object(
                "wt_messenger_mailbag_desk",
                "防雨郵袋案",
                &["Look", "Use", "Read"],
                &[
                    ("Look", "案上排滿蓋有半島各鎮印戳的竹綿紙路引，以及供快騎隨身馱帶的雙扣防水郵袋。"),
                    ("Use", "你將封好的信札仔細裝入防雨郵袋扣緊銅扣，妥善交到整裝待發的信使手中。"),
                    ("Read", "你查看案上的郵路派遣日程，各聚落每日皆有快騎往返傳遞急件與民信。"),
                ],
            ),
            make_peninsula_object(
                "wt_messenger_lookout_perch",
                "青銅旋轉晷",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "高台邊緣設有刻劃周密時辰線的青銅日規，晷針在陽光下投射出清晰分明的銳利陰影。"),
                    ("Read", "你俯視日規晷針指向，日影恰好落在午正刻度線上，時辰分秒不差。"),
                    ("Operate", "你輕推日規底盤旋鈕校準微調角度，與遠方主塔旗號完全對齊。"),
                ],
            ),
        ]
    } else if room_name.contains("碼頭")
        || room_name.contains("泊位")
        || room_name.contains("竹筏埠")
        || room_name.contains("渡埠")
        || room_name.contains("水岸")
        || room_name.contains("河畔")
        || room_name.contains("渡口")
        || room_name.contains("竹筏")
    {
        vec![
            make_peninsula_object(
                "wt_dock_riverbank_raft",
                "平底竹筏",
                &["Look", "Operate", "Use"],
                &[
                    ("Look", "水岸石砌埠頭邊立著長年受流水沖刷的繫船青石樁，石孔穿過粗大桐油麻繩，平穩繫泊著數艘雙層防浸平底竹筏，筏上置有推水長竹篙與青竹魚簍。"),
                    ("Operate", "你解開繫在青石樁上的桐油麻繩，平底竹筏在川流水波間輕輕蕩漾，吃水甚穩。"),
                    ("Use", "你手持長竹篙在石階輕撐借力，竹筏滑入川流清澈碧流，沿岸水草豐美、水鳥掠波。"),
                ],
            ),
            make_peninsula_object(
                "wt_dock_stone_mooring_bollard",
                "繫船玄武樁",
                &["Look", "Operate"],
                &[
                    ("Look", "碼頭邊緣深埋地底的圓頭玄武岩石樁，樁身被粗大的巨藻船纜磨得油亮光滑。"),
                    ("Operate", "你接過水手拋來的粗纜繩，俐落打了一個雙套結套在石樁上，船身穩穩貼岸。"),
                ],
            ),
            make_peninsula_object(
                "wt_dock_cargo_gangplank",
                "雲杉卸貨板",
                &["Look", "Use"],
                &[
                    ("Look", "連接貨船與石砌棧橋的厚實跳板，表面刻有縱橫防滑凹槽，方便水手挑擔搬運重貨。"),
                    ("Use", "你踏上平穩結實的木跳板大步走上棧橋，腳下厚板紋絲不晃，安穩可靠。"),
                ],
            ),
            make_peninsula_object(
                "wt_dock_tide_pole",
                "青銅驗潮尺",
                &["Look", "Read"],
                &[
                    ("Look", "嵌在防波石堤側面的青銅驗潮尺，刻著醒目的深淺刻痕，潮水在其下輕輕湧動拍擊。"),
                    ("Read", "你低頭查看水線刻度，此時正值滿潮平水之際，各型吃水深的大船皆能順利進港泊靠。"),
                ],
            ),
        ]
    } else if room_name.contains("市集")
        || room_name.contains("特產行")
        || room_name.contains("水產")
        || room_name.contains("物資行")
        || room_name.contains("素材")
        || room_name.contains("調配")
        || room_name.contains("調撥")
    {
        vec![
            make_peninsula_object(
                "wt_market_produce_stand",
                "山海乾貨攤",
                &["Look", "Taste", "Smell"],
                &[
                    ("Look", "攤位上滿是紅土崁黑糖、酸甜山果乾、大圳頭米香、鹽烤海苔片與風櫃尾冷燻魚乾，琳瑯滿目。"),
                    ("Taste", "你捻起一塊紅土崁黑糖放入口中細嚼，純粹甘甜中帶著濃郁焦香，滋味醇厚無比。"),
                    ("Smell", "你細聞攤前混和著米香、海苔與果乾的甜美香氣，市集生活氣息分外濃郁。"),
                ],
            ),
            make_peninsula_object(
                "wt_market_scale_counter",
                "黃銅雙盤秤",
                &["Look", "Operate"],
                &[
                    ("Look", "懸在木架上的精準黃銅雙盤大吊秤，配備一套由小至大的青銅砝碼，用於買賣過磅。"),
                    ("Operate", "你將砝碼置於左盤，右盤放入乾貨，秤桿微微擺動後平穩水平，分量精確公平。"),
                ],
            ),
            make_peninsula_object(
                "wt_market_water_tank",
                "活水海鮮盆",
                &["Look", "Use"],
                &[
                    ("Look", "引流海水的長木盆中魚蝦游動，水花翻滾，全為新近捕撈上岸的生猛活海鮮。"),
                    ("Use", "你拿起小竹撈網在水盆中輕輕一撈，網起一尾鱗光耀眼的生猛海魚，活力充沛。"),
                ],
            ),
        ]
    } else if room_name.contains("住宅")
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
    {
        vec![
            make_peninsula_object(
                "wt_home_brass_tap_sink",
                "黃銅活水栓",
                &["Look", "Use", "Bath"],
                &[
                    ("Look", "打磨光滑的黃銅水栓連接著老堰頭活水管道，下方是紅土耐火磚砌成的溫石蓄熱水槽，槽邊擺著草木灰柑橘洗潔皂、軟毛竹柄牙刷與青鹽薄荷潔牙粉。"),
                    ("Use", "你擰開黃銅水栓，清冽純淨的活水流淌而出，隨後轉入溫水檔，溫暖的水流洗去塵泥，柑橘皂香格外清新。"),
                    ("Bath", "你引入溫石槽加熱的熱水浸沐洗面，熱氣騰騰驅散了旅途風塵與肌肉酸脹，倍感舒泰。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_porcelain_lavatory",
                "白瓷沖水池",
                &["Look", "Use"],
                &[
                    ("Look", "紅土崁高嶺白瓷燒製的便池平滑光潔，引老堰頭活水沖洗，淨房牆側掛著柔軟的竹綿衛生草紙。"),
                    ("Use", "你拉動沖水黃銅扳手，活水漩渦利落將便池沖洗得乾淨無痕，屋內潔淨乾燥無異味。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_cold_jade_cabinet",
                "寒水玉冰櫃",
                &["Look", "Open", "Taste"],
                &[
                    ("Look", "小巧的家用石櫃由松香木與北山寒水玉夾層打製，存放著鮮牛乳、板豆腐、脆瓜醬菜與桂花新米糕，旁有白釉瓷碗盤與烏木筷。"),
                    ("Open", "你拉開石櫃門，涼意撲面而來，盤中的米糕與鮮牛乳保存完好，散發著微甘甜香。"),
                    ("Taste", "你取了一塊桂花新米糕送入口中，軟糯香甜，米香與桂花清香在舌尖悠悠化開。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_pearl_lamp_desk",
                "烏木書案",
                &["Look", "Operate", "Read"],
                &[
                    ("Look", "松木書案邊角包著黃銅片，案上置有黃銅發條座鐘、鋼尖沾水筆與竹綿紙記事手札，案角立著旋蓋夜光貝母台燈。"),
                    ("Operate", "你旋轉烏木燈罩調光，夜光貝的珠白光芒自罩隙均勻灑落，柔和澄澈。"),
                    ("Read", "你翻開竹綿紙記事手札，上面工整記錄著日常糧柴開銷、修繕屋瓦的備忘與鄰里往來雜記。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_toolbox_and_mailbox",
                "家用工具箱",
                &["Look", "Open", "Use"],
                &[
                    ("Look", "玄關軟底布拖鞋旁收著松木家用工具箱，門前防雨竹編信箱裡插著新到的書信。"),
                    ("Open", "你掀開工具箱銅扣，裡面分格收著羊角錘、螺絲起子、防潮紅磷火柴與細麻繩。"),
                    ("Use", "你取出一枚防潮紅磷火柴在側邊輕擦，升起穩定微亮的火苗，隨後自信箱取出書信細閱。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_feather_bed_lamp",
                "羽絨暖臥榻",
                &["Look", "Rest", "Sleep", "Operate"],
                &[
                    ("Look", "溫暖乾燥的松木臥榻上鋪著蓬鬆羽絨暖被與山風草安神枕，榻旁小几上立著一盞旋蓋夜光貝母調光台燈，光暈柔和溫潤。"),
                    ("Rest", "你合衣在柔軟的羽絨臥榻上躺下，安神草的清香讓緊繃的神經徹底放鬆下來。"),
                    ("Sleep", "你鑽入溫暖蓬鬆的羽絨暖被，旋暗夜光貝母燈。在靜謐祥和的屋內，你沉沉安睡，體魄與精神盡復。"),
                    ("Operate", "你輕輕旋轉床頭夜光貝母燈的烏木旋蓋，外露的珠白光芒柔和微調，滿室皆是安恬的暖意。"),
                ],
            ),
        ]
    } else if room_name.contains("剪剃") || room_name.contains("理髮") {
        vec![
            make_peninsula_object(
                "wt_barber_folding_razor",
                "摺疊剃刀架",
                &["Look", "Use"],
                &[
                    ("Look", "黃銅盤內整齊排著磨得雪亮的精鋼摺疊剃刀，旁側紫銅熱毛巾架正冒著蒸騰水氣與薄荷清香。"),
                    ("Use", "你取過一把精鋼摺疊剃刀試試鋒刃，刀身微薄而銳利，剃刮修鬚乾淨俐落。"),
                ],
            ),
            make_peninsula_object(
                "wt_barber_copper_chair",
                "洗頭木躺椅",
                &["Look", "Sit", "Rest"],
                &[
                    ("Look", "厚實白蠟木躺椅側邊接連著紫銅溫水管道與陶泥洗頭槽，軟皮頭枕弧度貼合。"),
                    ("Sit", "你在紫銅洗頭椅上躺靠下來，溫熱的水氣自銅管微熱散發，頸肩頓時放鬆。"),
                    ("Rest", "你閉上雙眼在躺椅上小憩片刻，聽著水槽流水汩汩，洗去整日奔波的疲乏。"),
                ],
            ),
        ]
    } else if room_name.contains("會計") || room_name.contains("商算") || room_name.contains("算學") {
        vec![
            make_peninsula_object(
                "wt_abacus_ebony_desk",
                "烏木計數板",
                &["Look", "Operate", "Use"],
                &[
                    ("Look", "烏木公案上端正擺放著便攜計數板，滑鈕圓潤油亮，旁側立著硃砂墨碟與儲墨鋼筆。"),
                    ("Operate", "你伸手撥動計數滑鈕，上下翻飛發出清脆沉著的扣合聲。"),
                    ("Use", "你提筆在草紙上演算一筆出海貨單，計數板撥轉分毫不差，帳目井然。"),
                ],
            ),
            make_peninsula_object(
                "wt_abacus_ledger_board",
                "核算大黑板",
                &["Look", "Read"],
                &[
                    ("Look", "青石牆面懸掛著黑漆大木板，上頭以白堊石筆端楷寫滿三水口各船號的貨品抽成與利息折算例題。"),
                    ("Read", "你仔細閱讀黑板上的算例，條理分明，正是半島商行事務員必修的進階算術。"),
                ],
            ),
        ]
    } else if room_name.contains("習武") || (room_name.contains("試招") && room_name.contains("文曜")) {
        vec![
            make_peninsula_object(
                "wt_martial_padded_wooden_blade",
                "包革木刀架",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "沉重木架上陳列著厚牛皮包覆的練習重木刀與熟銅卸力護腕，刀身刻有岑家分家工坊鋼印。"),
                    ("Use", "你反手拔出一柄包革木刀揮斬兩式，重心沉穩合手，破風聲呼呼作響。"),
                    ("Operate", "你調整護腕上的銅扣束帶，勒緊手腕測試卸力彈性，極利於修習剛猛招法。"),
                ],
            ),
            make_peninsula_object(
                "wt_martial_force_stone",
                "測勁玄武碑",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "院角立著一堵嵌有暗金紋理的厚重玄武石板，表面布滿被勁力拍擊出的微凹掌印與拳痕。"),
                    ("Use", "你運轉周身氣息推按測紋石板，掌力透入石紋，石板微鳴回饋內息剛柔，使你對發力關竅深有所得。"),
                    ("Operate", "你深吸一口氣運勁於掌，拍向測紋石板，石面暗紋隱隱亮起微光，回震沉實。"),
                ],
            ),
        ]
    } else if room_name.contains("柑仔店") || (room_name.contains("雜貨") && room_name.contains("文曜")) {
        vec![
            make_peninsula_object(
                "wt_grocery_match_incense_shelf",
                "艾草蚊香架",
                &["Look", "Use", "Acquire"],
                &[
                    ("Look", "木格貨架上擺滿了成盒的防潮紅磷火柴、一圈圈青綠艾草蚊香盤與居家必備的麻繩蠟燭。"),
                    ("Use", "你輕嗅艾草蚊香盤，清苦幽香沁人，正是驅除弄堂水氣與蚊蟲的良伴。"),
                    ("Acquire", "你伸手挑選了一小匣紅磷火柴與幾卷蚊香，隨時準備帶走備用。"),
                ],
            ),
            make_peninsula_object(
                "wt_grocery_candy_jar",
                "玻璃柑仔罐",
                &["Look", "Taste"],
                &[
                    ("Look", "老舊木櫃檯上放著幾只大號廣口玻璃罐，裡面裝著五顏六色的酸梅糖、黑糖麥芽糖與金桔軟糖。"),
                    ("Taste", "你拈起一顆金桔酸梅糖含在嘴裡，酸酸甜甜的滋味在舌尖化開，滿是市井老滋味。"),
                ],
            ),
        ]
    } else if room_name.contains("修傘") || room_name.contains("補靴") {
        vec![
            make_peninsula_object(
                "wt_repair_umbrella_bench",
                "修傘木砧台",
                &["Look", "Operate", "Use"],
                &[
                    ("Look", "矮木長凳前擺著修傘專用木砧板、幾捲熟桐油棉紙、破開的細韌紫竹篾骨與銅穿線錐。"),
                    ("Operate", "你拿起穿線錐將竹骨關節上的絲線拉緊打結，傘骨開合立即靈活如初。"),
                    ("Use", "你撐開一把剛補好桐油的棉紙晴雨傘試驗，傘面緊繃防雨，桐油幽香撲鼻。"),
                ],
            ),
            make_peninsula_object(
                "wt_repair_boots_last",
                "補靴硬木楦",
                &["Look", "Use"],
                &[
                    ("Look", "結實硬木楦頭上套著一隻抹滿厚魚油的防水皮靴，旁側擺著牛皮掌皮、生鐵鞋釘與補靴小錘。"),
                    ("Use", "你拿起小錘輕敲皮靴後跟的鞋掌，鐵釘吃入堅木與硬皮，底掌牢固無比。"),
                ],
            ),
        ]
    } else if room_name.contains("天井") || room_name.contains("弄堂") {
        vec![
            make_peninsula_object(
                "wt_alley_bamboo_drying_rack",
                "竹竿晾衣架",
                &["Look", "Use"],
                &[
                    ("Look", "天井兩側牽著幾根筆直長竹竿，竿上掛著被海風吹拂飄動的藍染粗布衣物，竹夾子咬得極緊。"),
                    ("Use", "你隨手整理被風吹偏的竹夾，仰頭看著窄仄天頂透下的天光與白雲，格外閒適。"),
                ],
            ),
            make_peninsula_object(
                "wt_alley_stone_well",
                "青石古井",
                &["Look", "Use", "Wash"],
                &[
                    ("Look", "弄堂轉角處有一口被繩索磨出深深凹痕的八角青石古井，井水清澈冰涼，桶索完整。"),
                    ("Use", "你轉動轆轤搖起一桶沁涼甘甜的井水，水珠飛濺，暑氣全消。"),
                    ("Wash", "你舀起一勺清涼井水洗了洗臉與雙手，整個人神清氣爽。"),
                ],
            ),
        ]
    } else if room_name.contains("書齋") || room_name.contains("校勘齋") || room_name.contains("墨香居") || room_name.contains("研墨小齋") || room_name.contains("校勘") {
        vec![
            make_peninsula_object(
                "wt_academy_fountain_pen",
                "黃銅儲墨筆",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "烏木教案的正中央擱著一支溫潤沉甸的黃銅桿儲墨鋼筆，筆尖以精煉銥金打磨得細膩微光，旁側襯著吸墨氈布與寫滿批註的竹綿紙講義。"),
                    ("Use", "你執起教案上的黃銅鋼筆，銥金筆尖觸及竹綿紙面，黑墨勻稱流暢地順著筆鋒沁入紙紋，留下一行端勁清晰的手跡。"),
                    ("Operate", "你輕旋黃銅筆桿檢視儲墨囊，墨色飽滿澄澈，筆帽卡榫嚴絲合縫，足見文曜百工製筆之精巧。"),
                ],
            ),
            make_peninsula_object(
                "wt_study_desk_inkstone",
                "烏木書案",
                &["Look", "Use", "Operate", "Read"],
                &[
                    ("Look", "黑沼沉水烏木雕琢的寬厚書案散發著幽靜木香，案上端硯墨色溫潤如玉，水注蓄著山泉，紫毫畫筆與狼毫楷筆整齊懸於紫檀筆架。"),
                    ("Use", "你取墨錠在端硯清泉中徐徐研磨，墨香四溢，提筆蘸墨在宣紙上信手書寫，筆意沉著，心神豁然澄澈。"),
                    ("Operate", "你整理案几上的鎮紙與宣紙摺頁，端整筆洗，室內窗明几淨，文氣盎然。"),
                    ("Read", "你展讀案几上攤開的手抄古卷，筆跡端勁，字句精微，深得先賢格物修齊之旨。"),
                ],
            ),
            make_peninsula_object(
                "wt_study_bamboo_bookshelf",
                "線裝古籍架",
                &["Look", "Use", "Read", "Operate"],
                &[
                    ("Look", "靠牆的高大樟木架上排滿了竹綿紙線裝古籍與輿地總圖，底層整齊碼放著幾隻精編韌竹書簍，分納校勘稿卷。"),
                    ("Use", "你自架上抽出一冊線裝古籍翻閱，紙質柔韌吸墨，墨香沁人心脾，細讀之下頗有領悟。"),
                    ("Read", "你取下一卷半島風物志稿細細研讀，字裡行間詳載水土人文與山川形勝，令人眼界大開。"),
                    ("Operate", "你將翻閱過的文稿重新按部首分類歸入竹編書簍，架欞齊整，井然有序。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_shell_lamp",
                "貝母台燈",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "案角立著造型典雅的旋蓋夜光貝母調光台燈，貝母薄片拼嵌的燈罩泛著溫潤珠光，烏木旋蓋可細調光圈大小。"),
                    ("Use", "你旋動烏木燈蓋調節光暈，柔和清澈的珠白光芒映照書案，滿室皆是清幽安寧之氣。"),
                    ("Operate", "你輕旋貝母燈罩，光暈隨之微調收斂，光斑柔和均勻，極便於夜讀抄寫。"),
                ],
            ),
        ]
    } else if room_name.contains("曆算") || room_name.contains("星曆") || room_name.contains("推步") {
        vec![
            make_peninsula_object(
                "wt_academy_fountain_pen",
                "黃銅儲墨筆",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "烏木教案的正中央擱著一支溫潤沉甸的黃銅桿儲墨鋼筆，筆尖以精煉銥金打磨得細膩微光，旁側襯著吸墨氈布與寫滿批註的竹綿紙講義。"),
                    ("Use", "你執起教案上的黃銅鋼筆，銥金筆尖觸及竹綿紙面，黑墨勻稱流暢地順著筆鋒沁入紙紋，留下一行端勁清晰的手跡。"),
                    ("Operate", "你輕旋黃銅筆桿檢視儲墨囊，墨色飽滿澄澈，筆帽卡榫嚴絲合縫，足見文曜百工製筆之精巧。"),
                ],
            ),
            make_peninsula_object(
                "wt_astro_brass_armillary",
                "渾天推步儀",
                &["Look", "Use", "Operate", "Read"],
                &[
                    ("Look", "多層同心黃銅環刻劃著十二辰次與周天度數，子午環與赤道環交相輝映，底座刻滿了文曜曆算學士世代推步的星度軌跡。"),
                    ("Use", "你推動星軌儀的精銅外環，齒輪與軸承咬合平順，星軌刻度隨之流轉對齊今日天象，心中豁然開朗。"),
                    ("Operate", "你輕輕旋轉子午度盤微調交角，黃銅星規與指針精準校準，測候律曆之奧妙盡在其中。"),
                    ("Read", "你俯身辨讀星軌儀底座鐫刻的文曜先賢推步法度，字跡端凝入微，深邃博大。"),
                ],
            ),
            make_peninsula_object(
                "wt_math_abacus_desk",
                "黑檀計數板",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "厚重松木長案上平放著一具黑檀木精密計數板，滑鈕圓潤黑亮，旁側鋪著墨跡未乾的算術推演草稿與算籌木匣。"),
                    ("Use", "你信手撥動黑檀計數滑鈕，扣合聲清脆利落，心中速算推演，思維條理頓時無比清晰。"),
                    ("Operate", "你推動計數板橫桿歸零復位，滑鈕整齊列陣，隨手整理案上的推算手札與筆墨。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_shell_lamp",
                "貝母台燈",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "算案一側置有一盞精巧的旋蓋夜光貝母台燈，乳白貝殼透光柔和，照亮密密麻麻的算題推算手稿。"),
                    ("Use", "你轉動烏木燈圈調節明暗，柔和白光灑在計數板與草紙上，省目清心。"),
                    ("Operate", "你微調貝母燈罩的角度使光束聚焦於核算案面，光斑清晰不炫目。"),
                ],
            ),
        ]
    } else if room_name.contains("印行館") || room_name.contains("造紙") || room_name.contains("典藏庫") {
        vec![
            make_peninsula_object(
                "wt_print_movable_type",
                "梨木活字盤",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "數以千計的梨木雕刻活字整齊碼放在分格木盤內，長案上擺著棕櫚墨刷、壓紙平砧與剛刷滿松煙墨的排版木框。"),
                    ("Use", "你拿起棕櫚墨刷在排版活字上勻勻刷上一層松煙墨，覆上宣紙輕輕滾壓，字跡頓時端正顯映於紙上。"),
                    ("Operate", "你手持竹鑷在活字盤中檢選字模並嵌入版框，木塊卡榫緊密堅實，字面齊整如刀削。"),
                ],
            ),
            make_peninsula_object(
                "wt_paper_bamboo_vat",
                "抄紙青石槽",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "青石大槽中浸著雪白的竹漿紙漿，旁邊架著細如青絲的抄紙竹簾，內壁熱氣騰騰的夾道焙紙牆正烘乾著剛起出的薄綿紙。"),
                    ("Use", "你雙手執竹簾浸入石槽，平順蕩料抄起一層細密纖維，揭紙貼於熱牆烘乾，紙面瑩白平滑，工藝之妙令人讚歎。"),
                    ("Operate", "你輕撫焙紙牆上烘乾的竹綿宣紙，纖維綿密細韌，揭下疊入紙捆，整齊平整。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_shell_lamp",
                "貝母台燈",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "排版案頭立著防風夜光貝母台燈，珠母薄片在水汽與墨香中泛著瑩潤清輝。"),
                    ("Use", "你旋亮夜光貝母燈罩，澄澈珠光映照著活字盤，微小筆畫一覽無遺。"),
                    ("Operate", "你調整燈罩遮光板，將清光引向剛拓印完成的書頁進行細緻校對。"),
                ],
            ),
        ]
    } else if room_name.contains("烘藥房") || room_name.contains("百草烘藥") {
        vec![
            make_peninsula_object(
                "wt_herb_drying_kiln",
                "紅泥焙藥爐",
                &["Look", "Use", "Operate", "WarmHands"],
                &[
                    ("Look", "雙層紅泥焙爐中炭火微溫，上下兩層密布通氣孔，溫火正烘烤著當歸、黃耆與山風草，濃郁藥香透滿全室。"),
                    ("Use", "你撥動紅泥爐膛的通氣瓦片微調火候，熱氣蒸騰間草木本質被徐徐焙透，藥性純粹，精神為之一振。"),
                    ("Operate", "你掀開頂層紅泥蓋檢查藥材焙乾程度，藥草微黃酥脆，火候恰到好處。"),
                    ("WarmHands", "你靠近紅泥焙藥爐的側壁烘手，厚重紅陶散發著沉穩持久的溫熱藥氣，周身寒氣盡消。"),
                ],
            ),
            make_peninsula_object(
                "wt_herb_bamboo_sieve",
                "切藥鍘刀案",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "層層竹篩中攤曬著洗淨的鮮藥草，長木案上固定著厚刃生鐵切藥鍘刀，案面滿是整齊切片留下的草木汁痕。"),
                    ("Use", "你手握切藥鍘刀木柄壓下，利刃切開晾乾的草藥根莖，發出清脆嚓響，藥香濃烈撲面。"),
                    ("Operate", "你翻動竹篩中的藥草使其受風均勻，青綠與微褐色澤錯落，草木清香盈溢鼻端。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_shell_lamp",
                "貝母台燈",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "切藥長案角設有一盞防潮夜光貝母調光台燈，光暈溫和不燥，映照著鋪展的藥草竹篩。"),
                    ("Use", "你轉動旋蓋調整燈火光暈，白光籠罩切藥案板，便於辨識藥材切片厚度。"),
                    ("Operate", "你輕輕旋轉貝母外殼調整反光角度，照亮掛滿乾草藥的陰乾竹架。"),
                ],
            ),
        ]
    } else if room_name.contains("北渠") || room_name.contains("東灣") || room_name.contains("水脈") || room_name.contains("河灣") || room_name.contains("竹筏渡") {
        vec![
            make_peninsula_object(
                "wt_reg_wenyao_riverbank",
                "雕花青石欄",
                &["Look", "Use", "Rest", "Operate"],
                &[
                    ("Look", "青石欄杆雕飾著渡海浪花紋樣，橋墩下方垂著粗壯熟鐵繫船環，川流自石階下緩緩匯入水灣，波光瀲灩。"),
                    ("Use", "你走近青石欄杆探看繫船環與泊位，粗壯鐵環深嵌石中，吃水牢靠平穩。"),
                    ("Rest", "你伏在微涼平整的石欄杆上眺望川流與來往竹筏，水風徐來，心曠神怡。"),
                    ("Operate", "你輕撫石欄上斑駁溫潤的浪花雕飾，指尖微涼，遠望水天相映。"),
                ],
            ),
            make_peninsula_object(
                "wt_river_stone_steps",
                "汲水青石階",
                &["Look", "Use", "Wash", "Rest"],
                &[
                    ("Look", "平整青石台階直通向清澈水面，階旁繫著兩只浸在水中的竹編魚簍，活水潺潺流淌。"),
                    ("Use", "你拾級而下至石階盡頭，俯身掬起一捧清泉洗濯面容，水質甘冽沁骨，整個人神清氣爽。"),
                    ("Wash", "你舀起清澈川水洗去雙手與面龐上的微塵，水氣清涼，暑氣與疲倦全消。"),
                    ("Rest", "你在石階最高處席地而坐，望著川流碧波隨風泛起漣漪，片刻安寧。"),
                ],
            ),
            make_peninsula_object(
                "wt_river_bamboo_raft",
                "平底竹筏",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "三水川石砌埠頭邊立著長年受流水沖刷的繫船青石樁，石孔穿過粗大桐油麻繩，平穩繫泊著防浸平底竹筏與長竹篙。"),
                    ("Use", "你手持長竹篙在石階輕撐借力，竹筏滑入三水川清澈碧流，沿岸水草豐美、水鳥掠波。"),
                    ("Operate", "你解開繫在青石樁上的桐油麻繩，平底竹筏在川流水波間輕輕蕩漾，吃水甚穩。"),
                ],
            ),
        ]
    } else if room_name.contains("青竹苑") || room_name.contains("藥圃") || room_name.contains("苗圃") {
        vec![
            make_peninsula_object(
                "wt_reg_wenyao_garden",
                "竹骨引水筧",
                &["Look", "Use", "Smell", "Water"],
                &[
                    ("Look", "依水岸而闢的分畦藥圃內，黑土整飭如墨，種著山風草、紫蘇與清涼薄荷，上方架設著以破半青竹搭建的竹骨引水筧，引清泉潺潺注入畦間。"),
                    ("Use", "你撥動引水筧竹筒引水澆灌畦間藥草，清泉滲入泥土，泥土草木清香頓時撲面而來。"),
                    ("Smell", "深吸一口氣，混雜著濕潤泥土、新鮮山風草與清涼薄荷的草木清香沁人心脾。"),
                    ("Water", "你撥正竹骨引水筧的水流轉向，清涼泉水順著竹管注入畦間泥土，滋養著青翠幼苗。"),
                ],
            ),
            make_peninsula_object(
                "wt_bamboo_tea_table",
                "竹林青石案",
                &["Look", "Use", "Rest", "Taste"],
                &[
                    ("Look", "翠竹掩映下設有一方青石茶几與兩只石凳，几上置有陶泥小火爐與青花瓷茶盞，修竹搖曳，幽靜怡人。"),
                    ("Use", "你在微涼的青石凳上端坐歇息，以竹筧引來的甘泉瀹茶，竹影婆娑入盞，頓覺身心放鬆，塵慮盡消。"),
                    ("Rest", "你坐在石凳上閉目調息，耳畔修竹隨風沙沙作響，清幽靜謐，心境空明。"),
                    ("Taste", "你端起溫熱的茶盞啜飲一口，竹瀝清香夾著焙茶回甘，回味悠長。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_shell_lamp",
                "貝母台燈",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "青石茶案旁修竹幹上掛有一盞防雨夜光貝母燈，微風拂動竹葉，青光與珠光交融成趣。"),
                    ("Use", "你撥動燈籠上的遮罩，柔和珠白光華傾瀉於石案，照亮一壺清茶。"),
                    ("Operate", "你調整竹梢上的懸掛索，使貝母燈在竹風中保持平穩照映。"),
                ],
            ),
        ]
    } else if room_name.contains("樞紐石橋") || room_name.contains("樞紐橋") {
        vec![
            make_peninsula_object(
                "wt_bridge_sundial_pillar",
                "日晷指路柱",
                &["Look", "Read"],
                &[
                    ("Look", "石橋兩端立有堅固花崗岩指路柱，柱頂刻有青銅日晷晷針，陰影清晰指示著當前時辰。"),
                    ("Read", "你走近石柱研讀鐫刻的方位路引：北通三水口北街與總商行，東連講堂街學院主校區與東橋旅社，南往公議堂與分家巷百工，西達傳訊鐘樓與西門聯運。"),
                ],
            ),
            make_peninsula_object(
                "wt_bridge_stone_balustrade",
                "雕花青石欄",
                &["Look", "Rest"],
                &[
                    ("Look", "青石欄杆雕飾著渡海浪花紋樣，橋墩下方垂著粗壯熟鐵繫船環，川流自橋下緩緩匯入港灣。"),
                    ("Rest", "你伏在微涼平整的石欄杆上眺望川流與來往貨船，河風拂面，令人心曠神怡。"),
                ],
            ),
        ]
    } else if room_name.contains("國小") {
        vec![
            make_peninsula_object(
                "wt_obj_school_recess_bell",
                "課間報時鐘",
                &["Look", "Listen"],
                &[
                    ("Look", "國小門廊挑樑下懸掛著一口精銅小撞鐘，鐘身刻有日昇月恆圖紋，垂著一根紅絲編結的鐘繩。"),
                    ("Listen", "微風掠過鐘槌，撞鐘隱約發出輕微清脆的金石幽鳴，悠揚動聽。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_stone_bench",
                "青石長凳",
                &["Look", "Rest"],
                &[
                    ("Look", "平整石道旁設有取自半嶺的青石長凳，已被往來行人的衣角磨得微亮，是歇腳閒談的絕佳所在。"),
                    ("Rest", "你在微涼溫潤的石凳上坐下歇息，看著街巷間往來穿梭的鄰里行客，步調悠閒自在。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
        ]
    } else if room_name.contains("藏書閣東側石巷") {
        vec![
            make_peninsula_object(
                "wt_obj_stone_rubbing_cabinet",
                "碑拓石櫥",
                &["Look", "Check"],
                &[
                    ("Look", "巷壁遮雨深檐下嵌著防潮硬木石櫥，玻璃紗門內整齊陳列著半島古碑拓本與金石拓片。"),
                    ("Check", "你透過紗門審視櫥中拓片，字口清晰遒勁，古意盎然。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_bamboo_stone_bench",
                "修竹石凳",
                &["Look", "Rest"],
                &[
                    ("Look", "深巷夾道萬竿翠竹掩映下，擺放著一組琢磨光滑的長條青石凳，竹葉篩下斑駁竹影。"),
                    ("Rest", "你在修竹石凳上靜坐片刻，耳邊唯聞竹葉沙沙天籟，塵慮頓消。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
        ]
    } else if room_name.contains("觀星台道") {
        vec![
            make_peninsula_object(
                "wt_obj_armillary_almanac",
                "定緯銅儀",
                &["Look", "Observe", "Read"],
                &[
                    ("Look", "露天石台中央矗立著多層旋轉銅環交錯的北辰定緯儀，基座四周平嵌著刻滿二十四節氣晷影長短的黑曜石測影日晷。"),
                    ("Observe", "你凝神推動外環赤道環，黃銅軸心平滑旋轉，指針精準指向北天極星，刻度縝密清晰。"),
                    ("Read", "你研讀日晷台面刻印的七十二候節令注解，上載寒暑推移與日影消長之理，字跡精工嚴謹。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_stair_stone_railing",
                "登階石欄",
                &["Look", "Rest"],
                &[
                    ("Look", "通往觀星台的石階兩側築有層疊漸升的青條石護欄，石面平正厚重，欄柱頂部琢磨成圓潤星珠。"),
                    ("Rest", "你扶著平涼的石欄稍作歇步，放眼回望，城郭屋瓦櫛比鱗次。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
        ]
    } else if room_name.contains("講堂街") || room_name.contains("學院巷") || room_name.contains("書院巷") {
        vec![
            make_peninsula_object(
                "wt_street_news_board",
                "旬報閱報欄",
                &["Look", "Read"],
                &[
                    ("Look", "街邊遮雨棚下立著寬闊的閱報木欄，上面張貼著最新一期的《半島旬報》與學院及國小告示。"),
                    ("Read", "你駐足細讀旬報頭條，上面記載著半島各城鎮的航線調度、礦石產量與學院最新評議文論。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_stone_bench",
                "青石長凳",
                &["Look", "Rest"],
                &[
                    ("Look", "平整石道旁設有取自半嶺的青石長凳，已被往來行人的衣角磨得微亮，是歇腳閒談的絕佳所在。"),
                    ("Rest", "你在微涼溫潤的石凳上坐下歇息，看著街巷間往來穿梭的鄰里行客，步調悠閒自在。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
        ]
    } else if room_name.contains("東門") {
        if room_name.contains("紙坊路口") {
            vec![
                make_peninsula_object(
                    "wt_obj_drying_paper_rack",
                    "曬紙竹簾架",
                    &["Look", "Use"],
                    &[
                        ("Look", "巷口通風處斜倚著數排高大的細篾竹簾架，整齊平鋪著剛剛抄造完成的濕潤竹綿宣紙，在微風中散發清幽竹香。"),
                        ("Use", "你伸手輕撫竹簾邊緣，宣紙纖維均勻平整，簾絲密緻結實。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_stone_bench",
                    "青石長凳",
                    &["Look", "Rest"],
                    &[
                        ("Look", "平整石道旁設有取自半嶺的青石長凳，已被往來行人的衣角磨得微亮，是歇腳閒談的絕佳所在。"),
                        ("Rest", "你在微涼溫潤的石凳上坐下歇息，看著街巷間往來穿梭的鄰里行客，步調悠閒自在。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        } else if room_name.contains("機樞工坊路口") {
            vec![
                make_peninsula_object(
                    "wt_obj_hardware_bins",
                    "五金分格櫃",
                    &["Look", "Check"],
                    &[
                        ("Look", "鐵皮壁櫃上百餘個木匣盛滿各型鉚釘、螺栓、合頁與銷釘，地坪上置有一圈圓形冷水槽，專供燒紅車輪鋼箍套木輪後急速冷縮咬合。"),
                        ("Check", "你隨手拉開幾隻抽屜，各規格榫釘排列井然，鐵脂氣味濃厚。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_caravan_water_trough",
                    "青石飲馬槽",
                    &["Look", "Use"],
                    &[
                        ("Look", "路口橫著三丈長的深青石水槽，槽中滿注引自山澗的活水，供來往駝獸與大車飲用。"),
                        ("Use", "你伸手輕撩清涼槽水洗濯雙手，泉水甘冽澄澈。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        } else if room_name.contains("關前") {
            vec![
                make_peninsula_object(
                    "wt_street_platform_scale",
                    "生鐵地秤",
                    &["Look", "Operate"],
                    &[
                        ("Look", "地面平嵌著一方厚重生鐵地秤台，側邊立著刻度精細的黃銅秤桿與大號滑動砝碼。"),
                        ("Operate", "你踏上地秤推動黃銅砝碼校驗重量，機括平順堅實，精準測算出車隊貨箱的載重斤兩。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_obj_east_road_milestone",
                    "東道里程碑",
                    &["Look", "Read"],
                    &[
                        ("Look", "關前立著一方厚實的青石里程方碑，石面鑿刻著通往風舶角與老堰頭的里數與主驛道轉折標記。"),
                        ("Read", "你俯身辨識石碑上的深深刻痕，各聚落里程字跡清晰端正。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_obj_hitch_wood_fence",
                    "拴馬長木欄",
                    &["Look", "Use"],
                    &[
                        ("Look", "粗大圓木架設的長木欄杆，專供長途駝隊歇鞍時拴繫馱獸，木紋已被韁繩磨得光亮。"),
                        ("Use", "你將隨身繩索繞扣在堅固的原木欄杆上，木欄沉穩受力，極其吃勁。"),
                    ],
                ),
            ]
        } else {
            vec![
                make_peninsula_object(
                    "wt_obj_willow_stone_railing",
                    "垂柳石欄",
                    &["Look", "Rest"],
                    &[
                        ("Look", "街側水渠旁立著雕刻簡樸的平整青石護欄，岸邊垂柳依依，枝條拂過平滑的石欄頂部。"),
                        ("Rest", "你倚靠在微涼的石欄旁歇息片刻，柳風拂面，水波粼粼。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_news_board",
                    "旬報閱報欄",
                    &["Look", "Read"],
                    &[
                        ("Look", "街邊遮雨棚下立著寬闊的閱報木欄，上面張貼著最新一期的《半島旬報》與學院及國小告示。"),
                        ("Read", "你駐足細讀旬報頭條，上面記載著半島各城鎮的航線調度、礦石產量與學院最新評議文論。"),
                    ],
                ),
            ]
        }
    } else if room_name.contains("南門") {
        if room_name.contains("鍛坊路口") {
            vec![
                make_peninsula_object(
                    "wt_forge_quench_trough",
                    "精鋼淬火槽",
                    &["Look", "Use"],
                    &[
                        ("Look", "盛滿深井泉水與淬火油的大石槽，槽邊備有打磨精鋼刀具與門鎖配件的雙面細磨石。"),
                        ("Use", "你伸手試了試槽沿的冷水，水溫冰涼刺骨，正是鍛造淬火的最佳溫度。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_grindstone_trough",
                    "磨刀青石槽",
                    &["Look", "Use"],
                    &[
                        ("Look", "工坊門口砌著一堵長條青石水槽，槽緣擱著幾塊粗細不一的天然油磨石與金剛切石泥。"),
                        ("Use", "你舀了勺水淋在油磨石上，順手將隨身小刃貼著石面輕輕推磨幾下，鋒口立即恢復清亮。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        } else if room_name.contains("車馬行路口") {
            vec![
                make_peninsula_object(
                    "wt_obj_hitch_wood_fence",
                    "拴馬長木欄",
                    &["Look", "Use"],
                    &[
                        ("Look", "粗大圓木架設的長木欄杆，專供長途駝隊歇鞍時拴繫馱獸，木紋已被韁繩磨得光亮。"),
                        ("Use", "你將隨身繩索繞扣在堅固的原木欄杆上，木欄沉穩受力，極其吃勁。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_caravan_water_trough",
                    "青石飲馬槽",
                    &["Look", "Use"],
                    &[
                        ("Look", "街邊橫著三丈長的深青石水槽，槽中滿注引自山澗的活水，供來往駝獸飲用。"),
                        ("Use", "你伸手輕撩清涼槽水洗濯雙手，泉水甘冽澄澈。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        } else if room_name.contains("十字") {
            vec![
                make_peninsula_object(
                    "wt_obj_south_road_milestone",
                    "指路石碑",
                    &["Look", "Read"],
                    &[
                        ("Look", "十字街口立著一座四稜青石指路短碑，四面分別刻有通往西門、南哨與分家巷的方向指引。"),
                        ("Read", "你辨讀指路石碑上的箭頭與工整刻字，市街方位一目了然。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_news_board",
                    "旬報閱報欄",
                    &["Look", "Read"],
                    &[
                        ("Look", "街邊遮雨棚下立著寬闊的閱報木欄，上面張貼著最新一期的《半島旬報》與學院及國小告示。"),
                        ("Read", "你駐足細讀旬報頭條，上面記載著半島各城鎮的航線調度、礦石產量與學院最新評議文論。"),
                    ],
                ),
            ]
        } else {
            vec![
                make_peninsula_object(
                    "wt_obj_welcome_pine_bench",
                    "迎客松石凳",
                    &["Look", "Rest"],
                    &[
                        ("Look", "街角蒼勁舒展的迎客古松下方，設有一圈取自半嶺的磨面弧形青石長凳，濃蔭匝地。"),
                        ("Rest", "你在松蔭下的石凳上落座歇涼，松脂清香令人精神一振。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_reg_wenyao_street",
                    "黃銅告示欄",
                    &["Look", "Read"],
                    &[
                        ("Look", "平整石板街邊立著雕工細緻的黃銅包邊木質告示欄，張貼著文曜各工坊、商行與學院評議會的最新公議告示。"),
                        ("Read", "你走近告示欄細讀通告，字跡工整端麗，上載各城鎮商船班次、工坊學徒招募與物資公道定價。"),
                    ],
                ),
            ]
        }
    } else if room_name.contains("北門") {
        if room_name.contains("船埠路口") {
            vec![
                make_peninsula_object(
                    "wt_obj_wharf_bollard",
                    "生鐵繫纜樁",
                    &["Look", "Tie", "LookShip"],
                    &[
                        ("Look", "條石駁岸邊深埋著粗壯的生鐵繫纜樁，樁身受粗麻纜繩摩挲得油黑發亮，駁岸沿線排滿厚重防撞松木垛。"),
                        ("Tie", "你將隨身繫繩熟練地在生鐵繫纜樁上打了個牢固的雙套結，纜結吃勁穩當。"),
                        ("LookShip", "你放眼望向泊位，數艘雙桅載重貨船隨著江波起伏，船工正忙碌裝卸貨包。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_obj_wharf_pulley_hoist",
                    "滑輪吊架",
                    &["Look", "Operate"],
                    &[
                        ("Look", "埠頭路口架設著以粗大雲杉木與生鐵滑輪組裝的起重吊架，垂掛著多股絞緊的粗麻吊繩。"),
                        ("Operate", "你拉了拉吊架垂落的麻繩，滑輪轉動平順，承重吊鉤牢固可靠。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        } else if room_name.contains("學院分院路口") {
            vec![
                make_peninsula_object(
                    "wt_obj_south_road_milestone",
                    "指路石碑",
                    &["Look", "Read"],
                    &[
                        ("Look", "路口立著一座四稜青石指路短碑，四面分別刻有通往學院各分院與城北碼頭的指引。"),
                        ("Read", "你辨讀指路石碑上的箭頭與工整刻字，分院方位一目了然。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_news_board",
                    "旬報閱報欄",
                    &["Look", "Read"],
                    &[
                        ("Look", "街邊遮雨棚下立著寬闊的閱報木欄，上面張貼著最新一期的《半島旬報》與學院及國小告示。"),
                        ("Read", "你駐足細讀旬報頭條，上面記載著半島各城鎮的航線調度、礦石產量與學院最新評議文論。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        } else if room_name.contains("林蔭") {
            vec![
                make_peninsula_object(
                    "wt_obj_chinaberry_stone_bench",
                    "苦楝樹石凳",
                    &["Look", "Rest"],
                    &[
                        ("Look", "林蔭路旁高大苦楝樹下，整齊安放著平整的條石長凳，樹影婆娑，涼風習習。"),
                        ("Rest", "你在苦楝樹下的石凳上歇息，細碎陽光透落石面，步調安詳寧靜。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_reg_wenyao_street",
                    "黃銅告示欄",
                    &["Look", "Read"],
                    &[
                        ("Look", "平整石板街邊立著雕工細緻的黃銅包邊木質告示欄，張貼著文曜各工坊、商行與學院評議會的最新公議告示。"),
                        ("Read", "你走近告示欄細讀通告，字跡工整端麗，上載各城鎮商船班次、工坊學徒招募與物資公道定價。"),
                    ],
                ),
            ]
        } else {
            vec![
                make_peninsula_object(
                    "wt_obj_drainage_stone_ditch",
                    "排水青石渠",
                    &["Look", "Check"],
                    &[
                        ("Look", "街石兩側鋪設著規整開敞的青石排水明渠，活水自暗涵穿流而過，渠水清澈無濁。"),
                        ("Check", "你俯身查看石渠流水，渠壁平滑無淤，水流暢達排向外城水門。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_obj_hitching_stone_post",
                    "拴馬石樁",
                    &["Look", "Use"],
                    &[
                        ("Look", "道旁埋設著鑿有穿繩圓孔的厚重青石短樁，頂部打磨成圓鼓形，石孔已被麻繩摩得油光鋥亮。"),
                        ("Use", "你伸手試了試拴馬石樁的穩固程度，石樁深植地底，紋絲不動。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        }
    } else if room_name.starts_with("文曜") && (room_name.contains("三水") || room_name.contains("商號街")) {
        if room_name.contains("商號街") {
            vec![
                make_peninsula_object(
                    "wt_abacus_fair_price_ledger",
                    "行商價目冊",
                    &["Look", "Read"],
                    &[
                        ("Look", "厚實的松木帳案上攤開著《各地行商貨價公道帳冊》，墨跡清晰，記載著半島各坊各埠即時市價。"),
                        ("Read", "你細讀價目冊上的物產行市，生鐵、宣紙、海鹽與寒水玉的公道兌換比例一目了然。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_market_scale_counter",
                    "黃銅雙盤秤",
                    &["Look", "Operate"],
                    &[
                        ("Look", "懸在木架上的精準黃銅雙盤大吊秤，配備一套由小至大的青銅砝碼，用於買賣過磅。"),
                        ("Operate", "你輕撥雙盤吊秤的黃銅提紐，兩端精準平衡，指針端直無偏。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        } else if room_name.contains("外環北道") || room_name.contains("外環西道") {
            vec![
                make_peninsula_object(
                    "wt_obj_flood_dike_pile",
                    "防汛石堤樁",
                    &["Look", "Check"],
                    &[
                        ("Look", "外環石堤外緣深打著粗壯整齊的花崗岩防汛石樁，頂部套有生鐵箍，能迎抗大潮衝擊。"),
                        ("Check", "你拍了拍冰冷沉實的石堤樁，樁身受巨浪沖刷多年依舊嚴絲合縫。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_obj_drying_nets_trough",
                    "苧麻曬網架",
                    &["Look", "Inspect", "Open"],
                    &[
                        ("Look", "露天院落中橫架著數十丈長的曬網木架，晾曬著浸染柿漆防腐的深褐色苧麻大網，廊下整齊排列著加蓋重石的鹽漬大陶缸。"),
                        ("Inspect", "你走近翻看懸掛的苧麻大網，網線以熟桐油與柿漆浸透，柔韌緊密，網目結實無損。"),
                        ("Open", "你揭開廊下一隻鹽漬陶缸的木蓋，一股濃郁鮮鹹的醃漬海味混合著粗海鹽氣息迎面撲來。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_bridge_harbor_causeway",
                    "防風航標燈",
                    &["Look", "Rest", "Operate"],
                    &[
                        ("Look", "跨海石橋橫臥於碧波海灣之上，厚實防風石欄抵禦著長風與浪沫，石欄轉折處設有旋蓋夜光貝母航標燈，指引往來舟楫與行旅。"),
                        ("Rest", "你在石橋避風凹處倚欄小憩，海浪拍擊橋墩白浪翻滾，海風拂衣。"),
                        ("Operate", "你轉動夜光貝母燈罩調準射角，柔和珠白強光穿透夜霧，指引遠方船影。"),
                    ],
                ),
            ]
        } else if room_name.contains("內環") {
            vec![
                make_peninsula_object(
                    "wt_obj_sanshui_stone_culvert",
                    "引水青石渠",
                    &["Look", "Check"],
                    &[
                        ("Look", "沿內環街道鋪展的石砌水渠引流三水清波，渠底鋪有細礫，活水長流不息。"),
                        ("Check", "你注視著渠中清澈活水順暢奔流，渠畔水生草葉隨波輕擺。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_obj_willow_stone_seat",
                    "垂柳石凳",
                    &["Look", "Rest"],
                    &[
                        ("Look", "臨渠垂柳樹蔭下安放著青條石座，石面被流水汽息浸染得溫潤生涼。"),
                        ("Rest", "你在垂柳旁的石凳上落座，看著清渠落葉漂流，愜意從容。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        } else {
            vec![
                make_peninsula_object(
                    "wt_river_stone_steps",
                    "汲水青石階",
                    &["Look", "Use", "Wash", "Rest"],
                    &[
                        ("Look", "平整青石台階直通向清澈水面，階旁繫著兩只浸在水中的竹編魚簍，活水潺潺流淌。"),
                        ("Use", "你拾級而下至石階盡頭，俯身掬起一捧清泉洗濯面容，水質甘冽沁骨，整個人神清氣爽。"),
                        ("Wash", "你舀起清澈川水洗去雙手與面龐上的微塵，水氣清涼，暑氣與疲倦全消。"),
                        ("Rest", "你在石階最高處席地而坐，望著川流碧波隨風泛起漣漪，片刻安寧。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_obj_willow_stone_railing",
                    "垂柳石欄",
                    &["Look", "Rest"],
                    &[
                        ("Look", "街側水渠旁立著雕刻簡樸的平整青石護欄，岸邊垂柳依依，枝條拂過平滑的石欄頂部。"),
                        ("Rest", "你倚靠在微涼的石欄旁歇息片刻，柳風拂面，水波粼粼。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        }
    } else if room_name.starts_with("文曜") && (room_name.contains("分家巷") || room_name.contains("百工")) {
        if room_name.contains("百工內環") {
            vec![
                make_peninsula_object(
                    "wt_obj_fire_water_tank",
                    "引水消防槽",
                    &["Look", "Check"],
                    &[
                        ("Look", "百工街口設有一座以青石大板拼砌的蓄水槽，連通水渠暗涵保持常滿，備有取水長柄銅斗。"),
                        ("Check", "你俯看水槽內部，槽水清涼充盈，槽緣擱置的汲水桶擺放井然。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_obj_rest_wood_pavilion",
                    "歇腳木亭",
                    &["Look", "Rest"],
                    &[
                        ("Look", "街心設有四柱尖頂的厚重雲杉涼亭，亭內置有環形靠背木椅，專供匠人勞作間隙歇涼。"),
                        ("Rest", "你在木亭長椅上落座歇息，四面穿堂風吹散周身悶熱，頓感舒暢。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        } else if room_name.contains("百工外環") {
            vec![
                make_peninsula_object(
                    "wt_obj_stone_anvil_platform",
                    "石料砧台",
                    &["Look", "Operate"],
                    &[
                        ("Look", "外環石道邊置有一座沉重平坦的玄武岩砧台，檯面留有鐵鑿與石錘敲擊的斑駁凹痕。"),
                        ("Operate", "你伸手撫過粗礪沉重的石砧檯面，堅實穩固，足以承受千鈞重錘。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_obj_raw_log_stack",
                    "原木排垛",
                    &["Look", "Check"],
                    &[
                        ("Look", "道路空坪上整齊滾排著以鐵索鎖固的原木大垛，根根直徑及抱，木皮粗厚乾燥。"),
                        ("Check", "你巡視木垛兩端的固定楔木，鐵索吃緊牢固，堆置安全平穩。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        } else {
            vec![
                make_peninsula_object(
                    "wt_street_grindstone_trough",
                    "磨刀青石槽",
                    &["Look", "Use"],
                    &[
                        ("Look", "工坊門口砌著一堵長條青石水槽，槽緣擱著幾塊粗細不一的天然油磨石與金剛切石泥。"),
                        ("Use", "你舀了勺水淋在油磨石上，順手將隨身小刃貼著石面輕輕推磨幾下，鋒口立即恢復清亮。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_obj_hardwood_square_timber",
                    "硬木方料垛",
                    &["Look", "Check"],
                    &[
                        ("Look", "工坊門側整齊架起離地的硬木方料垛，墊木通風，木料棱角分明，散發濃郁樟松氣味。"),
                        ("Check", "你伸手敲了敲方料端面，木質乾燥緊密，全無蟲蛀開裂。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        }
    } else if room_name.contains("關前") || room_name.contains("過磅") {
        vec![
            make_peninsula_object(
                "wt_street_platform_scale",
                "生鐵地秤",
                &["Look", "Operate"],
                &[
                    ("Look", "地面平嵌著一方厚重生鐵地秤台，側邊立著刻度精細的黃銅秤桿與大號滑動砝碼。"),
                    ("Operate", "你踏上地秤推動黃銅砝碼校驗重量，機括平順堅實，精準測算出車隊貨箱的載重斤兩。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_weighbridge_milepost",
                "西道里程碑",
                &["Look", "Read"],
                &[
                    ("Look", "關前青石路旁立著一尊高大花崗岩石碑，鐫刻著通往長風草甸、老堰頭與大圳頭的準確里程與地勢險隘。"),
                    ("Read", "你俯身端讀里程路石刻字：西出城門五里為平川，十里有避風坡與取水深井，標記周詳。"),
                ],
            ),
            make_peninsula_object(
                "wt_caravan_water_trough",
                "青石飲馬槽",
                &["Look", "Use"],
                &[
                    ("Look", "驛道旁橫著三丈長的深青石水槽，槽中滿注引自山澗的活水，供來往駝獸與大車飲用。"),
                    ("Use", "你伸手輕撩清涼槽水洗濯雙手，泉水甘冽澄澈。"),
                ],
            ),
        ]
    } else if room_name.contains("貨棧") {
        vec![
            make_peninsula_object(
                "wt_obj_steelyard_cart",
                "推車重木秤",
                &["Look", "Operate"],
                &[
                    ("Look", "高架庫棚下停放著兩部加重雙輪推車，車樑上架著帶有青銅滑動大舵砣的重型木桿秤。"),
                    ("Operate", "你推動舵砣校準木秤零點，沉實木梁起落平穩，刻度分毫不差。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_timber_reed_stacks",
                "毛竹原木垛",
                &["Look", "Check", "Tap"],
                &[
                    ("Look", "貨場一側整齊碼放著以油布覆蓋的貨垛，旁側堆著通直挺拔的毛竹與杉木圓料。"),
                    ("Check", "你細細檢查油布覆蓋的貨垛底部，防潮石墊高整齊，透氣乾燥。"),
                    ("Tap", "你輕叩粗壯的毛竹竿身，發出清脆結實的篤篤回音，質地堅韌。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
        ]
    } else if room_name.contains("車馬前街") {
        vec![
            make_peninsula_object(
                "wt_obj_caravan_hitch_trough",
                "拴馬長木欄",
                &["Look", "Tie", "Water"],
                &[
                    ("Look", "粗大圓木架設的長木欄杆，專供長途駝隊歇鞍時拴繫馱獸，木紋已被韁繩磨得光亮。"),
                    ("Tie", "你將隨身繩索繞扣在堅固的原木欄杆上，木欄沉穩受力，極其吃勁。"),
                    ("Water", "你自石槽打起一桶清水倒進飲水槽，駝獸歡快地甩動鬃毛低頭暢飲。"),
                ],
            ),
            make_peninsula_object(
                "wt_caravan_water_trough",
                "青石飲馬槽",
                &["Look", "Use"],
                &[
                    ("Look", "街邊橫著三丈長的深青石水槽，槽中滿注引自山澗的活水，供來往駝獸飲用。"),
                    ("Use", "你伸手輕撩清涼槽水洗濯雙手，泉水甘冽澄澈。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
        ]
    } else if room_name.contains("車馬街西口") {
        vec![
            make_peninsula_object(
                "wt_obj_weighbridge_milepost",
                "西道里程碑",
                &["Look", "Read"],
                &[
                    ("Look", "路旁立著一尊高大花崗岩石碑，鐫刻著通往長風草甸與老堰頭的準確里程與地勢險隘。"),
                    ("Read", "你俯身端讀里程路石刻字：西出城門五里為平川，十里有避風坡與取水深井，標記周詳。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_caravan_bell_rack",
                "駝鈴排架",
                &["Look", "Listen"],
                &[
                    ("Look", "棚架上懸掛著幾十只大小不一的熟銅駝鈴與原牛皮製鈴帶，在風中輕輕碰撞。"),
                    ("Listen", "微風掠過排架，熟銅鈴鐺激盪出清越悠遠的叮咚金鳴，宛若大漠行商長歌。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
        ]
    } else if room_name.contains("車馬街西段") || room_name.contains("草料") {
        vec![
            make_peninsula_object(
                "wt_obj_wagon_jack_wheel",
                "千斤重木頂",
                &["Look", "Check"],
                &[
                    ("Look", "地坪上整齊堆放著維修重輪大車專用的加厚千斤頂木楔與兩組備用硬木輪圈。"),
                    ("Check", "你查驗木頂機括與硬木輪圈的鐵皮緊固螺栓，咬合嚴密無隙。"),
                ],
            ),
            make_peninsula_object(
                "wt_caravan_water_trough",
                "青石飲馬槽",
                &["Look", "Use"],
                &[
                    ("Look", "車馬街旁橫著深青石水槽，槽中滿注引自山澗的活水，供歇腳駝獸飲用。"),
                    ("Use", "你伸手輕撩清涼槽水洗濯雙手，泉水甘冽澄澈。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
        ]
    } else if room_name.contains("車馬") {
        vec![
            make_peninsula_object(
                "wt_obj_caravan_bell_rack",
                "駝鈴排架",
                &["Look", "Listen"],
                &[
                    ("Look", "棚架上懸掛著幾十只大小不一的熟銅駝鈴與原牛皮製鈴帶，在風中輕輕碰撞。"),
                    ("Listen", "微風掠過排架，熟銅鈴鐺激盪出清越悠遠的叮咚金鳴，宛若大漠行商長歌。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_caravan_hitch_trough",
                "拴馬長木欄",
                &["Look", "Tie", "Water"],
                &[
                    ("Look", "粗大圓木架設的長木欄杆，專供長途駝隊歇鞍時拴繫馱獸，木紋已被韁繩磨得光亮。"),
                    ("Tie", "你將隨身繩索繞扣在堅固的原木欄杆上，木欄沉穩受力，極其吃勁。"),
                    ("Water", "你自石槽打起一桶清水倒進飲水槽，駝獸歡快地甩動鬃毛低頭暢飲。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
        ]
    } else if room_name.contains("十字") && room_name.contains("西門") {
        vec![
            make_peninsula_object(
                "wt_trunk_road_milestone",
                "指路石碑",
                &["Look", "Read"],
                &[
                    ("Look", "十字街口立著一座四稜青石指路方碑，四面分別刻有通往西城門、公議堂與三水口的方向指引。"),
                    ("Read", "你辨讀指路石碑上的箭頭與工整刻字，城西主幹方位一目了然。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_stone_bench",
                "青石長凳",
                &["Look", "Rest"],
                &[
                    ("Look", "平整石道旁設有取自半嶺的青石長凳，已被往來行人的衣角磨得微亮，是歇腳閒談的絕佳所在。"),
                    ("Rest", "你在微涼溫潤的石凳上坐下歇息，看著街巷間往來穿梭的鄰里行客，步調悠閒自在。"),
                ],
            ),
        ]
    } else if room_name.contains("西門") && !room_name.contains("草甸") && !room_name.contains("修補") {
        vec![
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_hitch_wood_fence",
                "拴馬長木欄",
                &["Look", "Use"],
                &[
                    ("Look", "粗大圓木架設的長木欄杆，專供長途駝隊歇鞍時拴繫馱獸，木紋已被韁繩磨得光亮。"),
                    ("Use", "你將隨身繩索繞扣在堅固的原木欄杆上，木欄沉穩受力，極其吃勁。"),
                ],
            ),
            make_peninsula_object(
                "wt_caravan_water_trough",
                "青石飲馬槽",
                &["Look", "Use"],
                &[
                    ("Look", "街邊橫著三丈長的深青石水槽，槽中滿注引自山澗的活水，供來往駝獸飲用。"),
                    ("Use", "你伸手輕撩清涼槽水洗濯雙手，泉水甘冽澄澈。"),
                ],
            ),
        ]
    } else if room_name.contains('橋') || room_name.contains("跳石") || room_name.contains("渡埠") {
        vec![
            make_peninsula_object(
                "wt_bridge_stepping_stones",
                "過溪跳石",
                &["Look", "Use", "Rest"],
                &[
                    ("Look", "平緩清淺水面上鋪設著一連串圓整青石跳石，旁側設有防滑青石護欄與繫船鐵環，清流自石隙潺潺穿過，映著水光。"),
                    ("Use", "你踩著露出水面的青石跳石穩健跨越清溪，腳下清流潺潺，水花濺在石階邊緣，平添幾分渡水之趣。"),
                    ("Rest", "你在跳石橋畔微涼的青石基座上歇坐片刻，聆聽溪水淙淙與微風拂過岸柳之聲，頓覺神清氣爽。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_stone_bench",
                "歇腳石凳",
                &["Look", "Rest"],
                &[
                    ("Look", "平整石道旁設有取自半嶺的青石長凳，已被往來行人的衣角磨得微亮，是歇腳閒談的絕佳所在。"),
                    ("Rest", "你在微涼溫潤的石凳上坐下歇息，看著街巷間往來穿梭的鄰里行客，步調悠閒自在。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
        ]
    } else if room_name.contains("河灣") || room_name.contains("東灣") || room_name.contains("水脈") || room_name.contains("北渠") || room_name.contains("草坪") {
        vec![
            make_peninsula_object(
                "wt_reg_wenyao_riverbank",
                "雕花青石欄",
                &["Look", "Use", "Rest", "Operate"],
                &[
                    ("Look", "青石欄杆雕飾著渡海浪花紋樣，橋墩下方垂著粗壯熟鐵繫船環，川流自石階下緩緩匯入水灣，波光瀲灩。"),
                    ("Use", "你走近青石欄杆探看繫船環與泊位，粗壯鐵環深嵌石中，吃水牢靠平穩。"),
                    ("Rest", "你伏在微涼平整的石欄杆上眺望川流與來往竹筏，水風徐來，心曠神怡。"),
                    ("Operate", "你輕撫石欄上斑駁溫潤的浪花雕飾，指尖微涼，遠望水天相映。"),
                ],
            ),
            make_peninsula_object(
                "wt_river_stone_steps",
                "汲水青石階",
                &["Look", "Use", "Wash", "Rest"],
                &[
                    ("Look", "平整青石台階直通向清澈水面，階旁繫著兩只浸在水中的竹編魚簍，活水潺潺流淌。"),
                    ("Use", "你拾級而下至石階盡頭，俯身掬起一捧清泉洗濯面容，水質甘冽沁骨，整個人神清氣爽。"),
                    ("Wash", "你舀起清澈川水洗去雙手與面龐上的微塵，水氣清涼，暑氣與疲倦全消。"),
                    ("Rest", "你在石階最高處席地而坐，望著川流碧波隨風泛起漣漪，片刻安寧。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
        ]
    } else if room_name.contains("阡陌水田")
        || room_name.contains("平整旱田")
        || room_name.contains("整地空壟")
        || room_name.contains("育苗菜畦")
        || room_name.contains("曬穀石坪")
        || room_name.contains("打穀圓場")
        || room_name.contains("穀倉前坪")
        || room_name.contains("農具棚架")
    {
        vec![
            make_peninsula_object(
                "wt_obj_waterwheel_wooden",
                "引水木水車",
                &["Look", "Operate"],
                &[
                    ("Look", "水渠支流上架著一座緩緩轉動的杉木筒車，水斗依次汲水傾入竹筧，灌溉連綿農田。"),
                    ("Operate", "你注視著水車齒輪帶動軸心勻速運轉，水流滔滔湧向田壟，聲響沉穩綿長。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_grain_drying_mat",
                "竹編曬穀席",
                &["Look", "Use"],
                &[
                    ("Look", "寬敞石坪上鋪展著數張細篾編織的巨幅曬穀席，金黃的稻穀薄薄鋪開，在日光下散發純淨穀香。"),
                    ("Use", "你手持木耙將曬席上的穀粒輕輕翻勻，穀粒沙沙作響，受熱均勻。"),
                ],
            ),
            make_peninsula_object(
                "wt_farm_tool_iron_plow",
                "雙齒精鐵犁",
                &["Look", "Operate"],
                &[
                    ("Look", "為原牛翻耕黏土設計的淬火精鐵犁鏵，泥槽打磨平滑，犁尖銳利厚實。"),
                    ("Operate", "你握住雙齒犁的磨光硬木扶柄試著推動，重心穩健，精鐵犁鏵入土沉實。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_flail_threshing_rack",
                "連枷打穀架",
                &["Look", "Operate"],
                &[
                    ("Look", "場院中央立著硬木搭起的傾斜打穀格架，架旁斜倚著幾柄以白蠟木製成的四節打穀連枷。"),
                    ("Operate", "你提起連枷凌空揮轉，梢竹劃過呼呼風聲，落下沉實有力的擊穀節奏。"),
                ],
            ),
            make_peninsula_object(
                "wt_mill_grain_hopper",
                "杉木儲糧斗",
                &["Look", "Open"],
                &[
                    ("Look", "高懸的倒梯形杉木儲料斗連接著下料槽，旁邊架著點驗原糧重量的大秤。"),
                    ("Open", "你拉開儲料斗下方的調節木插板，金黃飽滿的原糧穀粒順槽嘩啦滑落。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_field_rest_stone",
                "田頭歇腳石",
                &["Look", "Rest"],
                &[
                    ("Look", "田埂交匯的大樟樹蔭下，立著一方天然平整的灰白大條石，石面已被農人磨得平滑微光。"),
                    ("Rest", "你在田頭大石上席地而坐，望著眼前綠意翻滾的無垠田壟，微風拂面神清氣爽。"),
                ],
            ),
        ]
    } else if room_name.contains("分畦藥圃")
        || room_name.contains("高山藥畦")
        || room_name.contains("山草藥畦")
        || room_name.contains("水草藥畦")
        || room_name.contains("防瘴艾圃")
        || room_name.contains("避風花圃")
        || room_name.contains("耐風草圃")
        || room_name.contains("曬藥竹場")
    {
        vec![
            make_peninsula_object(
                "wt_reg_wenyao_garden",
                "竹骨引水筧",
                &["Look", "Use", "Smell", "Water"],
                &[
                    ("Look", "依水岸而闢的分畦藥圃內，黑土整飭如墨，種著山風草、紫蘇與清涼薄荷，上方架設著以破半青竹搭建的竹骨引水筧，引清泉潺潺注入畦間。"),
                    ("Use", "你撥動引水筧竹筒引水澆灌畦間藥草，清泉滲入泥土，泥土草木清香頓時撲面而來。"),
                    ("Smell", "深吸一口氣，混雜著濕潤泥土、新鮮山風草與清涼薄荷的草木清香沁人心脾。"),
                    ("Water", "你撥正竹骨引水筧的水流轉向，清涼泉水順著竹管注入畦間泥土，滋養著青翠幼苗。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_herb_drying_trellis",
                "曬藥竹篾架",
                &["Look", "Turn"],
                &[
                    ("Look", "露天開闊處架著雙層竹篾編製的曬藥架，上面攤鋪著切片烘曬的當歸、黃芪與野生甘草。"),
                    ("Turn", "你輕輕翻動竹篩中的乾燥藥材切片，受光均勻，空氣中飄散著濃郁醇厚的甘草微甜。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_herb_stone_scoop",
                "青石澆水瓢",
                &["Look", "Use"],
                &[
                    ("Look", "蓄水池旁擱著一只以整塊青石刳空雕鑿的水瓢，木柄穿梢牢靠，瓢身沉實盛水不漏。"),
                    ("Use", "你舀起滿瓢清涼泉水，細細淋灑在翠綠的幼嫩藥苗根部，泥土芬芳撲鼻。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_windbreak_low_wall",
                "防風石矮牆",
                &["Look", "Rest"],
                &[
                    ("Look", "藥圃迎風面以天然山石乾砌起半人高的避風短牆，縫隙以泥草填塞，護佑幼嫩藥草免遭狂風侵襲。"),
                    ("Rest", "你背靠避風石牆坐下暫歇，牆背微溫避風，身前花草清香繚繞。"),
                ],
            ),
        ]
    } else if room_name.contains("中央牧坪")
        || room_name.contains("向陽牧坪")
        || room_name.contains("苜蓿草場")
        || room_name.contains("草料堆場")
        || room_name.contains("羊毛曬場")
        || room_name.contains("牧坪中道")
        || room_name.contains("環牧石板道")
        || room_name.contains("牧村出入口道")
    {
        vec![
            make_peninsula_object(
                "wt_caravan_water_trough",
                "青石飲馬槽",
                &["Look", "Use"],
                &[
                    ("Look", "牧道旁橫著三丈長的深青石水槽，槽中滿注引自山澗的活水，供群牛群羊飲用。"),
                    ("Use", "你伸手輕撩清涼槽水洗濯雙手，泉水甘冽澄澈。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_hitch_wood_fence",
                "拴馬長木欄",
                &["Look", "Use"],
                &[
                    ("Look", "粗大圓木架設的長木欄杆，專供放牧歇鞍時拴繫馱獸，木紋已被韁繩磨得光亮。"),
                    ("Use", "你將隨身繩索繞扣在堅固的原木欄杆上，木欄沉穩受力，極其吃勁。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_dry_haystack",
                "防潮乾草垛",
                &["Look", "Check"],
                &[
                    ("Look", "草場地勢隆起處高高堆起覆有油布防雨氈的乾牧草大垛，金黃乾燥的甜草散發清甜麥草香。"),
                    ("Check", "你抽出一根乾草掐斷驗看，草梗清脆乾燥，無半分返潮霉味。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_wool_drying_rack",
                "晾毛木排架",
                &["Look", "Check"],
                &[
                    ("Look", "木欄旁排立著傾斜的松木晾架，洗淨的原牛與原羊白毛攤晾其上，隨風微微蓬鬆翻動。"),
                    ("Check", "你伸手輕撫晾曬的羊毛，手感柔軟蓬鬆，油脂已被泉水漂洗得恰到好處。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_pasture_signpost",
                "牧場指路樁",
                &["Look", "Read"],
                &[
                    ("Look", "草道岔路口立著一根去皮紅松指路方樁，樁頂刻有奔牛與羔羊標記，指明各牧區與飲水塘方向。"),
                    ("Read", "你閱讀指路樁上的清晰刻槽，向陽草場與儲料庫方位一目了然。"),
                ],
            ),
        ]
    } else if room_name.contains("曬網石坪")
        || room_name.contains("錨鏈儲場")
        || room_name.contains("船木晾場")
        || room_name.contains("貝灰曬坪")
        || room_name.contains("纜繩工棚")
        || room_name.contains("望海石欄")
        || room_name.contains("卸貨石坪")
        || room_name.contains("海風茶亭")
        || room_name.contains("海鹽曬場")
        || room_name.contains("船帆晾架")
        || room_name.contains("錨石堆場")
        || room_name.contains("礁石望台")
        || room_name.contains("燈號石坪")
        || room_name.contains("礁石淺灘")
        || room_name.contains("巨藻晾場")
        || room_name.contains("風化礁岩")
        || room_name.contains("避風街")
        || room_name.contains("觀海橫街")
        || room_name.contains("烏石街")
        || room_name.contains("海港直街")
    {
        vec![
            make_peninsula_object(
                "wt_obj_drying_nets_trough",
                "苧麻曬網架",
                &["Look", "Inspect", "Open"],
                &[
                    ("Look", "露天院落中橫架著數十丈長的曬網木架，晾曬著浸染柿漆防腐的深褐色苧麻大網，廊下整齊排列著加蓋重石的鹽漬大陶缸。"),
                    ("Inspect", "你走近翻看懸掛的苧麻大網，網線以熟桐油與柿漆浸透，柔韌緊密，網目結實無損。"),
                    ("Open", "你揭開廊下一隻鹽漬陶缸的木蓋，一股濃郁鮮鹹的醃漬海味混合著粗海鹽氣息迎面撲來。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_basalt_heavy_anchor",
                "玄武重鐵錨",
                &["Look", "Check"],
                &[
                    ("Look", "海灘石坪上斜臥著一具以玄武岩壓心、四爪生鐵鍛鑄的巨型防風鐵錨，錨爪粗壯如柱。"),
                    ("Check", "你伸手敲擊厚重的鐵錨身，發出沉實渾厚的嗡鳴，抓底分量十足。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_spruce_drying_rack",
                "雲杉晾木架",
                &["Look", "Check"],
                &[
                    ("Look", "岸邊通風排棚下，整齊架著用粗雲杉搭建的通風層架，架上鋪滿裁切整齊的造船龍骨板材。"),
                    ("Check", "你查驗架上木材的乾燥色澤，木理通直緊密，海水鹽霧更添硬度。"),
                ],
            ),
            make_peninsula_object(
                "wt_lime_powder_bin",
                "熟貝灰石槽",
                &["Look", "Use"],
                &[
                    ("Look", "大石槽中盛滿過篩後的熟貝灰粉，細膩如白麵，遇水化作黏結力極強的砌石砂漿。"),
                    ("Use", "你拿起木鏟翻了翻熟貝灰粉，粉末乾燥滑膩，是海防築堤與船底防漏之上品。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_rope_twist_wheel",
                "手搖絞繩輪",
                &["Look", "Operate"],
                &[
                    ("Look", "空地上立著三聯手搖木製絞繩輪，麻絲自輪齒穿出絞合為粗壯縴繩，旁側立有刻度拉力測試石柱。"),
                    ("Operate", "你伸手搖動木輪握把，齒輪牽動繩股絞緊，木架嗡嗡震響。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_seawall_stone_railing",
                "望海青石欄",
                &["Look", "Rest"],
                &[
                    ("Look", "海岬高台邊緣修築著連綿厚實的防風浪青石圍欄，石面受海風雕琢呈現溫潤石痕。"),
                    ("Rest", "你扶欄憑高遠眺，碧海奔潮拍擊黑石礁崖，白浪滔天令人胸襟大開。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_sea_salt_crystallizer",
                "海鹽結晶池",
                &["Look", "Check"],
                &[
                    ("Look", "平整海坪上以青石條分割成一方方淺水泥池，清澈鹵水在烈日下蒸發，泛起耀眼的雪白鹽霜。"),
                    ("Check", "你拈起一撮池畔凝結的粗鹽晶，晶體顆粒方正透明，鹹鮮純正。"),
                ],
            ),
            make_peninsula_object(
                "wt_kelp_drying_trellis",
                "高架曬藻棚",
                &["Look", "Gather"],
                &[
                    ("Look", "迎著海風搭建的巨型竹棚上懸垂著長達數丈的深海厚巨藻，海風吹拂下漸乾變韌。"),
                    ("Gather", "你順手翻整竹棚上的乾燥巨藻，藻葉黑褐帶白霜，鹹鮮微甜。"),
                ],
            ),
            make_peninsula_object(
                "wt_bridge_harbor_causeway",
                "防風航標燈",
                &["Look", "Rest", "Operate"],
                &[
                    ("Look", "跨海石橋橫臥於碧波海灣之上，厚實防風石欄抵禦著長風與浪沫，石欄轉折處設有旋蓋夜光貝母航標燈，指引往來舟楫與行旅。"),
                    ("Rest", "你在石橋避風凹處倚欄小憩，海浪拍擊橋墩白浪翻滾，海風拂衣。"),
                    ("Operate", "你轉動夜光貝母燈罩調準射角，柔和珠白強光穿透夜霧，指引遠方船影。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_wharf_bollard",
                "生鐵繫纜樁",
                &["Look", "Tie", "LookShip"],
                &[
                    ("Look", "條石駁岸邊深埋著粗壯的生鐵繫纜樁，樁身受粗麻纜繩摩挲得油黑發亮，駁岸沿線排滿厚重防撞松木垛。"),
                    ("Tie", "你將隨身繫繩熟練地在生鐵繫纜樁上打了個牢固的雙套結，纜結吃勁穩當。"),
                    ("LookShip", "你放眼望向泊位，數艘雙桅載重貨船隨著江波起伏，船工正忙碌裝卸貨包。"),
                ],
            ),
        ]
    } else if room_name.contains("寒水玉採石坑")
        || room_name.contains("切石石坪")
        || room_name.contains("採石坡道")
        || room_name.contains("晾石前坪")
        || room_name.contains("玄武採石場")
        || room_name.contains("礦道石坪")
        || room_name.contains("西窯道")
        || room_name.contains("東選礦道")
        || room_name.contains("紅土磚場")
        || room_name.contains("選礦石坪")
        || room_name.contains("礦車儲坪")
        || room_name.contains("耐火泥池")
        || room_name.contains("耐火磚場")
        || room_name.contains("礦車鐵軌道")
        || room_name.contains("坑口中街")
    {
        vec![
            make_peninsula_object(
                "wt_obj_stone_gang_saw",
                "切石框鋸台",
                &["Look", "Touch"],
                &[
                    ("Look", "粗壯鐵架上懸掛著加重無齒金剛砂框鋸，細水伴著鋼砂在巨石槽中慢慢切割，四周堆滿了平整如削的城防條石與石雕柱礎。"),
                    ("Touch", "你伸手撫摸剛切出的石板切面，石面平直如削，微帶金剛砂水漬的溫涼觸感。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_jade_material_rack",
                "寒水玉料架",
                &["Look", "Check"],
                &[
                    ("Look", "棚架內層層陳列著剛開採出的半嶺寒水玉毛料，玉色青綠微透，石皮帶有一層細膩白霜。"),
                    ("Check", "你伸手摩挲玉料切面，入手冰寒徹骨，質地縝密溫潤。"),
                ],
            ),
            make_peninsula_object(
                "wt_trolley_rail_carriage",
                "雙軸重台車",
                &["Look", "Operate"],
                &[
                    ("Look", "坑道軌道上停著厚重雙軸台車，精鐵鑄造的車輪扣住木軌鐵皮，裝滿粗礦依然平穩。"),
                    ("Operate", "你推動台車煞車鐵桿，生鐵閘瓦咬緊輪軸，發出沉實摩擦聲。"),
                ],
            ),
            make_peninsula_object(
                "wt_mine_wash_chute",
                "木製洗礦槽",
                &["Look", "Operate"],
                &[
                    ("Look", "山澗急流順著長木槽奔騰而下，槽底隔木截留住沉重赤鐵礦砂，洗去浮游紅泥。"),
                    ("Operate", "你拔起一片水閘木板，急流分注沖刷沈澱石泥，露出晶亮重礦砂。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_grade_iron_screen",
                "分級重鐵篩",
                &["Look", "Operate"],
                &[
                    ("Look", "堅固的三層傾斜編鐵粗篩，專供分揀不同粒徑的耐火黏土粒與礦砂。"),
                    ("Operate", "你手搖鐵篩連桿，不同規格的礦石砂粒依序分流滑入不同木斗。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_red_brick_drying_rack",
                "紅磚坯料架",
                &["Look", "Check"],
                &[
                    ("Look", "窯道兩側搭滿長條木架，密密麻麻排滿了脫模陰乾的紅土耐火泥磚坯，棱角分明。"),
                    ("Check", "你抽查泥磚坯的乾燥堅實度，黏土調配得當，磚身毫無裂紋變形。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_steel_cable_winch",
                "鋼索絞盤",
                &["Look", "Operate"],
                &[
                    ("Look", "採石斜坡頂端固定著厚重的生鐵鑄造絞盤，多股鋼絲絞索緊密盤繞於轉軸，備有防倒退棘輪。"),
                    ("Operate", "你扳動絞盤長柄試運轉，棘輪發出清脆卡嗒聲，制動齒輪咬合緊密。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_mine_tunnel_lantern",
                "礦道照明燈",
                &["Look", "Operate"],
                &[
                    ("Look", "石壁鐵托架上嵌著以防爆青銅網罩保護的夜光貝母礦燈，在幽深昏暗的石坪投下清亮冷光。"),
                    ("Operate", "你調整礦燈前端的遮光銅葉，聚光弧束指向前路，幽深坑道頓時清晰。"),
                ],
            ),
        ]
    } else if room_name.contains("山泉清槽")
        || room_name.contains("柴薪儲場")
        || room_name.contains("觀雲石台")
        || room_name.contains("試招石坪")
        || room_name.contains("儲木石坪")
        || room_name.contains("雲杉苗圃")
        || room_name.contains("雲杉林圃")
        || room_name.contains("護林松坡")
        || room_name.contains("嶺腳主徑")
        || room_name.contains("橫嶺山徑")
        || room_name.contains("杉木西巷")
        || room_name.contains("霜林中道")
        || room_name.contains("鋸木溪前街")
        || room_name.contains("杉林北道")
    {
        vec![
            make_peninsula_object(
                "wt_obj_timber_reed_stacks",
                "毛竹原木垛",
                &["Look", "Check", "Tap"],
                &[
                    ("Look", "高大通風的倉棚下整齊碼放著以油布覆蓋的防潮糧穀包，場地另一側堆滿通直挺拔的三年生毛竹與直徑數尺的巨杉圓木。"),
                    ("Check", "你抽驗木料質地，木質密實乾燥，松香氣息撲鼻。"),
                    ("Tap", "你輕叩粗壯的原木端面，聲如洪鐘，質地堅韌無蛀。"),
                ],
            ),
            make_peninsula_object(
                "wt_sawmill_hydraulic_saw",
                "水動排鋸架",
                &["Look", "Operate"],
                &[
                    ("Look", "巨型雙水車連桿驅動的直立鋼鋸架，上下往復飛速鋸切，原木如切泥般剖為平整厚木板。"),
                    ("Operate", "你扳動水車分水導流板，水流激增，排鋸隨之發出均勻剛勁的轟鳴切削聲。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_spring_water_trough",
                "引泉青石槽",
                &["Look", "Use"],
                &[
                    ("Look", "山徑旁依山岩鑿有一方青石泉槽，山頂清澈甘泉順竹筧潺潺注入，溢流順石階漫入草叢。"),
                    ("Use", "你俯身掬起清泉痛飲，泉水冰寒甘洌直透心脾，登山疲渴頓時冰消。"),
                ],
            ),
            make_peninsula_object(
                "wt_arena_testing_pillar",
                "玄武試勁柱",
                &["Look", "Operate"],
                &[
                    ("Look", "場中央立著取自烏石鼻的沉重玄武岩石柱，柱面留有深淺不一的掌印、拳痕與兵刃劈砍印記。"),
                    ("Operate", "你凝神運勁一掌拍上試勁石柱，掌風在石面上激起沉悶回響，柱身厚重不搖。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_mountain_milestone",
                "山道指路碑",
                &["Look", "Read"],
                &[
                    ("Look", "陡坡拐角處矗立著一尊取自青脊山的灰黑巨石碑，正面深刻著通往白塔雪巔與各林場的箭頭路徑。"),
                    ("Read", "你仔細端詳碑文上的山勢地形指引，峰迴路轉的險要坡道清晰在目。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_sheltered_rest_stone",
                "避風歇腳石",
                &["Look", "Rest"],
                &[
                    ("Look", "巨岩凹處背風向陽的一方天然青石台，石面寬厚平整，四周崖壁恰好擋住高嶺寒風。"),
                    ("Rest", "你在石台上盤膝打坐調勻呼吸，背風處暖意微生，漸復體力。"),
                ],
            ),
        ]
    } else if room_name.contains("高棧木廊大道")
        || room_name.contains("北防務石街")
        || room_name.contains("哨鎮東西大街")
        || room_name.contains("棧道巡道")
        || room_name.contains("高架木坪")
        || room_name.contains("獵具檢修坪")
        || room_name.contains("篝火石坪")
        || room_name.contains("獸皮晾架")
        || room_name.contains("巡哨前坪")
        || room_name.contains("南哨沼口門")
        || room_name.contains("哨口木道")
        || room_name.contains("高腳烏木棧橋")
        || room_name.contains("水生木棧道")
        || room_name.contains("蘆花北棧")
        || room_name.contains("澤南橫棧")
        || room_name.contains("蘆葦淺澤")
        || room_name.contains("菱角淺塘")
    {
        vec![
            make_peninsula_object(
                "wt_obj_wormwood_dispelling_pot",
                "防瘴艾草盆",
                &["Look", "Smell"],
                &[
                    ("Look", "木棧道轉角處置有一只厚泥陶盆，盆內文火燃點著乾燥陳艾與雄黃香餅，青煙裊裊升騰。"),
                    ("Smell", "你輕嗅空氣中散發的濃烈艾草苦香，周遭蚊蚋退避，胸口濁氣為之一清。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_ebony_mooring_post",
                "烏木繫船樁",
                &["Look", "Use"],
                &[
                    ("Look", "木棧橋頭深打著數根油黑不腐的沼生千年烏木樁，樁頭刻有水鳥標記，套著浸油防霉纜繩。"),
                    ("Use", "你伸手推了推黑亮沈重的烏木樁，木質堅逾鐵石，吃水極深牢不可撼。"),
                ],
            ),
            make_peninsula_object(
                "wt_dock_riverbank_raft",
                "平底竹筏",
                &["Look", "Operate", "Use"],
                &[
                    ("Look", "水岸石砌埠頭邊立著長年受流水沖刷的繫船青石樁，石孔穿過粗大桐油麻繩，平穩繫泊著數艘雙層防浸平底竹筏，筏上置有推水長竹篙與青竹魚簍。"),
                    ("Operate", "你解開繫在青石樁上的桐油麻繩，平底竹筏在川流水波間輕輕蕩漾，吃水甚穩。"),
                    ("Use", "你手持長竹篙在石階輕撐借力，竹筏滑入川流清澈碧流，沿岸水草豐美、水鳥掠波。"),
                ],
            ),
            make_peninsula_object(
                "wt_bamboo_drying_rack",
                "青竹晾曬架",
                &["Look", "Gather"],
                &[
                    ("Look", "大棚架上整齊排晾著經過熱水蒸煮去糖防蛀的老青竹竿，竹皮碧綠油亮，質地極韌。"),
                    ("Gather", "你撫摸晾乾的青竹竿，竹節筆直勻稱，正是編造水排與造屋之良材。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_iron_bound_barricade",
                "包鐵拒馬欄",
                &["Look", "Check"],
                &[
                    ("Look", "哨口木道中央橫設著以重硬木拼組、四角包釘尖刺鐵皮的雙層拒馬防禦柵欄。"),
                    ("Check", "你審查拒馬欄上的生鐵加固鉚釘與交叉木柱，佈局堅固，能阻猛獸直衝。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_tall_watchtower_ladder",
                "高架瞭望梯",
                &["Look", "Climb"],
                &[
                    ("Look", "高棧坪旁立著一座直通樹冠瞭望台的粗大木梯，階梯踏板包覆防滑麻棕，扶手粗壯結實。"),
                    ("Climb", "你攀上木梯數級放眼環顧，沼澤深處的水網航道與蘆葦蕩漾盡收眼底。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_water_caltrop_skiff",
                "採菱木扁舟",
                &["Look", "Operate"],
                &[
                    ("Look", "棧道下繫著一艘吃水極淺的圓頭杉木小划子，艙內備有採菱竹撈網與幾隻防浸圓木桶。"),
                    ("Operate", "你踩上輕晃的扁舟以竹篙微點棧柱，扁舟輕巧滑入淺澤荷菱之間，游動自如。"),
                ],
            ),
        ]
    } else if room_name.contains("實心城牆")
        || room_name.contains("木柵圍欄")
        || room_name.contains("海防石柵")
        || room_name.contains("山隘石柵")
        || room_name.contains("防木重柵")
    {
        vec![
            make_peninsula_object(
                "wt_obj_stone_crenel_battlement",
                "條石垛口",
                &["Look", "Check"],
                &[
                    ("Look", "城牆外緣整齊排列著厚重規整的玄武岩條石垛口，留有向下俯瞰與開弓射箭的防禦缺口。"),
                    ("Check", "你伏在冰涼的條石垛口向外探視，牆外平原野地一覽無遺，扼守要道防守森嚴。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_patrol_tower_stair",
                "巡防望樓梯",
                &["Look", "Climb"],
                &[
                    ("Look", "依實心石牆內側修築的整齊青石台階直通敵樓高台，階石寬闊堅實，附設鍛鐵扶手。"),
                    ("Climb", "你拾級登梯登上石牆高處，登高憑風，城關防務格局嚴密了然。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_windproof_iron_brazier",
                "防風鐵火盆",
                &["Look", "Operate"],
                &[
                    ("Look", "城垛轉角處置有一座三角鍛鐵厚重火盆，盆身鏤刻避風雲紋，內置燃燒持久的硬木白炭。"),
                    ("Operate", "你用鐵火鉗撥動炭塊，暗紅炭火立刻迸出熾烈微芒，驅散城頭凜冽寒意。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_hardwood_barrier_stake",
                "硬木拒馬樁",
                &["Look", "Check"],
                &[
                    ("Look", "關隘通道兩側斜立著以合抱粗原木削尖、深嵌地面的防衝撞拒馬尖樁，木尖塗覆防腐黑漆。"),
                    ("Check", "你檢視拒馬樁基座的鎖固鐵鏈，排樁緊湊密集，形成阻遏大軍衝鋒的銅牆鐵壁。"),
                ],
            ),
        ]
    } else if (room_name.starts_with("文曜") && room_name.contains("三水")) || room_name.contains("南門") || room_name.contains("納涼") || room_name.contains("市集") || room_name.contains("早市") || room_name.contains("夜市") {
        vec![
            make_peninsula_object(
                "wt_street_stone_bench",
                "青石長凳",
                &["Look", "Rest"],
                &[
                    ("Look", "平整石道旁設有取自半嶺的青石長凳，已被往來行人的衣角磨得微亮，是歇腳閒談的絕佳所在。"),
                    ("Rest", "你在微涼溫潤的石凳上坐下歇息，看著街巷間往來穿梭的鄰里行客，步調悠閒自在。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
        ]
    } else if room_name.contains("市街石道")
        || room_name.contains("鎮口道")
        || room_name.contains("市鎮街道")
        || room_name.contains("村中泥徑")
        || room_name.contains("村口路")
        || (is_peninsula
            && (room_name.contains('街')
                || room_name.contains('道')
                || room_name.contains('橋')
                || room_name.contains('路')
                || room_name.contains('徑')
                || room_name.contains('門')
                || room_name.contains('牆')
                || room_name.contains('欄')
                || room_name.contains('田')
                || room_name.contains('壟')
                || room_name.contains('圃')))
    {
        vec![
            make_peninsula_object(
                "wt_street_stone_bench",
                "青石長凳",
                &["Look", "Rest"],
                &[
                    ("Look", "平整石道旁設有取自半嶺的青石長凳，已被往來行人的衣角磨得微亮，是歇腳閒談的絕佳所在。"),
                    ("Rest", "你在微涼溫潤的石凳上坐下歇息，看著街巷間往來穿梭的鄰里行客，步調悠閒自在。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
        ]
    } else if is_peninsula {
        vec![
            make_peninsula_object(
                "wt_street_stone_bench",
                "青石長凳",
                &["Look", "Rest"],
                &[
                    ("Look", "平整石道旁設有取自半嶺的青石長凳，已被往來行人的衣角磨得微亮，是歇腳閒談的絕佳所在。"),
                    ("Rest", "你在微涼溫潤的石凳上坐下歇息，看著街巷間往來穿梭的鄰里行客，步調悠閒自在。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
        ]
    } else {
        Vec::new()
    };

    if !base_objs.is_empty() {
        if base_objs.len() < 3 {
            for cat_obj in super::peninsula_catalog::catalog_objects_for_room(room_name) {
                if !base_objs.iter().any(|o| o.name == cat_obj.name || o.id == cat_obj.id) {
                    base_objs.push(cat_obj);
                    if base_objs.len() >= 3 {
                        break;
                    }
                }
            }
        }
        if base_objs.len() < 3 {
            if let Some(regional_obj) = regional_specialty_object_for_room(room_name) {
                if !base_objs.iter().any(|o| o.name == regional_obj.name || o.id == regional_obj.id) {
                    base_objs.push(regional_obj);
                }
            }
        }
    }
    base_objs.truncate(3);
    base_objs
}

fn regional_specialty_object_for_room(room_name: &str) -> Option<RoomObject> {
    if room_name.starts_with("風舶角")
        || room_name.starts_with("風舶嶼")
        || room_name.starts_with("風櫃尾")
        || room_name.starts_with("烏石鼻")
    {
        Some(make_peninsula_object(
            "wt_reg_coastal_specialty",
            "夜光貝風鈴",
            &["Look", "Operate", "Use"],
            &[
                ("Look", "屋簷與門廊下懸著烏石鼻海貝與夜光貝母串成的防風貝鈴，牆角立著裝配黃銅捲線輪的韌竹捲線釣竿、風櫃冷燻魚乾甕與防水樹膠雨衣。"),
                ("Operate", "海風拂過廊下，夜光貝母風鈴碰撞出清脆悅耳的叮咚聲，泛起溫潤珠光。"),
                ("Use", "你轉動韌竹釣竿上的黃銅捲線輪試了試手感，齒輪順滑輕巧，隨時能去港灣礁石垂釣。"),
            ],
        ))
    } else if room_name.starts_with("半嶺鎮")
        || room_name.starts_with("老堰頭")
        || room_name.starts_with("北嶺腳")
        || room_name.starts_with("霜林口")
    {
        Some(make_peninsula_object(
            "wt_reg_mountain_specialty",
            "寒水玉茶盅",
            &["Look", "WarmHands", "Use"],
            &[
                ("Look", "松香木架上擺著半嶺寒水玉雕成的恆溫茶盅、松脂暖手爐，旁側掛著進山必備的內襯寒水玉採集背簍、卡式導能便攜爐與黃銅單筒望遠鏡。"),
                ("WarmHands", "你捧起溫潤的寒水玉茶盅與松脂暖手爐，北山特有的恆溫暖意順著掌心傳遍全身。"),
                ("Use", "你舉起黃銅單筒望遠鏡朝窗外遠眺，遠方神山雲杉林與山徑景物清晰如在眼前。"),
            ],
        ))
    } else if room_name.starts_with("大圳頭") || room_name.starts_with("牛埔") {
        Some(make_peninsula_object(
            "wt_reg_plains_specialty",
            "陳年米酒甕",
            &["Look", "Taste", "Smell"],
            &[
                ("Look", "厚實的防潮羊毛氈墊旁，立著大圳頭紅泥封口的陳年米酒甕、牛埔熟成硬乳酪木盤，院門邊還放著看家牧羊犬的食盆與春耕艾草青糰蒸籠。"),
                ("Taste", "你切下一小片牛埔硬乳酪配著溫熱米酒品嚐，濃郁奶香與甘醇酒香交融，回味綿長。"),
                ("Smell", "陣陣大圳頭新米酒香與艾草清香撲鼻而來，滿是西南沃野的富足氣息。"),
            ],
        ))
    } else if room_name.starts_with("紅土崁") || room_name.starts_with("三號坑口") {
        Some(make_peninsula_object(
            "wt_reg_mining_specialty",
            "烤番薯砂鍋",
            &["Look", "Taste", "WarmHands"],
            &[
                ("Look", "耐火紅磚砌成的矮爐上煨著一只紅土炭火砂鍋，爐灰裡埋著紅土崁沙地種出的紅心番薯，牆角還收著導能測紋石板與防潮紅磷火柴盒。"),
                ("Taste", "你剝開一顆剛出爐的烤紅心番薯，金黃番薯肉軟糯流蜜，甜香燙口。"),
                ("WarmHands", "你湊近紅土炭火砂鍋烘手，耐火紅陶散發著持久溫厚的熱力。"),
            ],
        ))
    } else if room_name.starts_with("南哨") || room_name.starts_with("水尾寮") {
        Some(make_peninsula_object(
            "wt_reg_wetland_specialty",
            "艾草驅蚊盤",
            &["Look", "Smell", "Use"],
            &[
                ("Look", "沉水烏木矮几上點著溫氏診所配製的驅蚊避瘴艾草香盤，旁側擺著水尾寮細編蘆葦涼席、雙層竹提籃與防雨油布雙人帳篷。"),
                ("Smell", "艾草與山薄荷的清涼草木香氣在屋內緩緩繚繞，將南境濕熱水氣與蚊蟲全數驅散。"),
                ("Use", "你展開細編蘆葦涼席試摸，竹篾與蘆葦平滑冰涼，正是水鄉避暑的良物。"),
            ],
        ))
    } else if room_name.contains("公議") || room_name.contains("契據") {
        Some(make_peninsula_object(
            "wt_cat_0793",
            "紅印屋契匣",
            &["Look", "Operate", "Read"],
            &[
                ("Look", "紫檀木架上擺著文曜公議堂專用的紅印屋契匣與戶籍租約摺，上面整齊列著白塔半島各市鎮的房產地籍規章。"),
                ("Operate", "你端整公議堂印泥盒並展開屋契摺頁，準備簽章畫押或核對租約地界。"),
                ("Read", "你展閱細讀紅印屋契摺，公議堂戶籍署硃砂大印分明，載明半島產權過戶與承租法度。"),
            ],
        ))
    } else if room_name.contains("學院") && !room_name.contains("觀星") && !room_name.contains("學院口") && !room_name.contains("學院巷") && !room_name.contains("前街") {
        Some(make_peninsula_object(
            "wt_cat_0734",
            "黃銅星軌儀",
            &["Look", "Operate", "Read"],
            &[
                ("Look", "精銅底座上高聳著精密的三百六十度黃銅星軌儀，多層同心環刻有節氣、潮汐與天象經緯刻度。"),
                ("Operate", "你輕輕推動星儀外環，黃銅齒輪與星軌指針順滑流轉，對準了今夜即將升起的海角星宿。"),
                ("Read", "你俯身辨讀星軌儀底盤銘刻的文曜先賢測候律歷，字跡端凝入微。"),
            ],
        ))
    } else if room_name.contains("總商行") || room_name.contains("商盟") {
        Some(make_peninsula_object(
            "wt_cat_0790",
            "分格存摺櫃",
            &["Look", "Operate", "Read"],
            &[
                ("Look", "厚重鐵木所造的分格存摺櫃，抽屜銅扣皆刻有三水口流水徽印，收存著各大商隊與鎮民的兌銀簿冊。"),
                ("Operate", "你拉開其中一格抽屜檢驗分格存摺簿，銅夾緊湊牢固，便於快速盤點收支。"),
                ("Read", "你細讀櫃頂張貼的三水總商行通兌章程，載明各港口通兌手續費與信用票號兌領條款。"),
            ],
        ))
    } else if room_name.starts_with("文曜") {
        if is_wenyao_residential_or_study(room_name)
            || room_name.contains("住宅")
            || room_name.contains("民宅")
            || room_name.contains("居所")
            || room_name.contains("宅邸")
            || room_name.contains("宅院")
            || room_name.contains("民舍")
            || room_name.contains("內室")
            || room_name.contains("客房")
        {
            Some(make_peninsula_object(
                "wt_reg_wenyao_home",
                "松香木暖床",
                &["Look", "Use", "Rest", "Sleep", "Operate"],
                &[
                    ("Look", "溫暖乾燥的松香木大床上鋪著蓬鬆羽絨暖被與山風草安神枕，榻旁小几上立著一盞旋蓋夜光貝母台燈，光暈柔和溫潤。"),
                    ("Use", "你在松香木大床上舒展四肢，蓬鬆羽絨暖被帶來融融暖意，周身疲勞與肌肉酸脹頓時消散大半，氣血與內息漸漸調和平復。"),
                    ("Rest", "你合衣在柔軟的羽絨大床上躺下歇息，安神草的清香讓緊繃的心神徹底放鬆下來，周身氣血漸復充盈。"),
                    ("Sleep", "你鑽入溫暖蓬鬆的羽絨暖被，旋暗夜光貝母台燈。在靜謐祥和的屋內，你安然入眠，體魄與心神盡皆恢復。"),
                    ("Operate", "你輕輕旋轉床頭夜光貝母台燈的烏木旋蓋，珠白光芒柔和微調，滿室皆是安恬的暖意。"),
                ],
            ))
        } else if room_name.contains("渡埠")
            || room_name.contains("水岸")
            || room_name.contains("河畔")
            || room_name.contains("渡口")
            || room_name.contains("泊位")
            || room_name.contains("竹筏")
            || room_name.contains("三水川")
            || room_name.contains("北渠")
            || room_name.contains("東灣")
            || room_name.contains("水脈")
            || room_name.contains("河灣")
        {
            Some(make_peninsula_object(
                "wt_reg_wenyao_riverbank",
                "雕花青石欄",
                &["Look", "Use", "Rest", "Operate"],
                &[
                    ("Look", "青石欄杆雕飾著渡海浪花紋樣，橋墩下方垂著粗壯熟鐵繫船環，川流自石階下緩緩匯入水灣，波光瀲灩。"),
                    ("Use", "你走近青石欄杆探看繫船環與泊位，粗壯鐵環深嵌石中，吃水牢靠平穩。"),
                    ("Rest", "你伏在微涼平整的石欄杆上眺望川流與來往竹筏，水風徐來，心曠神怡。"),
                    ("Operate", "你輕撫石欄上斑駁溫潤的浪花雕飾，指尖微涼，遠望水天相映。"),
                ],
            ))
        } else if room_name.contains("青竹苑")
            || room_name.contains("藥圃")
            || room_name.contains("苗圃")
            || room_name.contains("藥草")
            || room_name.contains("竹苑")
            || room_name.contains("竹林")
        {
            Some(make_peninsula_object(
                "wt_reg_wenyao_garden",
                "竹骨引水筧",
                &["Look", "Use", "Smell", "Water"],
                &[
                    ("Look", "依水岸而闢的分畦藥圃內，黑土整飭如墨，種著山風草、紫蘇與清涼薄荷，上方架設著以破半青竹搭建的竹骨引水筧，引清泉潺潺注入畦間。"),
                    ("Use", "你撥動引水筧竹筒引水澆灌畦間藥草，清泉滲入泥土，泥土草木清香頓時撲面而來。"),
                    ("Smell", "深吸一口氣，混雜著濕潤泥土、新鮮山風草與清涼薄荷的草木清香沁人心脾。"),
                    ("Water", "你撥正竹骨引水筧的水流轉向，清涼泉水順著竹管注入畦間泥土，滋養著青翠幼苗。"),
                ],
            ))
        } else if room_name.contains("庭院")
            || room_name.contains("小院")
            || room_name.contains("院落")
            || room_name.contains("門前")
            || room_name.contains("門首")
            || room_name.contains("天井")
        {
            Some(make_peninsula_object(
                "wt_reg_wenyao_courtyard",
                "防雨竹信箱",
                &["Look", "Use", "Open", "Water"],
                &[
                    ("Look", "院門粉牆邊掛著一只以生桐油浸潤的防雨細密竹編信箱，石階旁安放著紅土崁粗陶花盆，栽植著一株枝葉青翠欲滴的粉白山茶花。"),
                    ("Use", "你駐足端詳院落門前整潔的竹編信箱與山茶花，心神怡然。"),
                    ("Open", "你輕輕掀開竹編信箱的防水翻蓋，內裡乾燥整潔，墊著吸濕的竹綿薄紙。"),
                    ("Water", "你自水缸舀起半瓢清水澆灌在山茶花根部，紅土散發出質樸的泥土芬芳，花葉愈顯精神。"),
                ],
            ))
        } else if room_name.contains("內門坊") {
            Some(make_peninsula_object(
                "wt_reg_wenyao_gate_plaza",
                "石雕拴馬柱",
                &["Look", "Rest", "Operate"],
                &[
                    ("Look", "青石坊門前立有一對雕刻流雲海濤紋樣的石雕望柱，柱底鑿有厚實生鐵栓馬環，過路行商常在此歇鞍飲馬。"),
                    ("Rest", "你倚在青石望柱基座旁放鬆筋骨，看著巡坊差役與往來商旅自牌坊下穿行，井然有序。"),
                    ("Operate", "你拉動生鐵栓馬石環試了試牢固度，鐵環嵌合緊密，堅不可摧。"),
                ],
            ))
        } else if room_name.contains("街") || room_name.contains("巷") || room_name.contains("道") || room_name.contains("橋") {
            Some(make_peninsula_object(
                "wt_reg_wenyao_street",
                "黃銅告示欄",
                &["Look", "Use", "Read", "Operate"],
                &[
                    ("Look", "青石街角立著造型典雅的文曜街燈，燈罩內嵌夜光貝母薄片；燈柱旁設有黃銅鑲邊的公佈欄，張貼著近期的市集行曆與商會通告。"),
                    ("Use", "你走近公佈欄與青石街燈，細細閱覽各項告示與通告。"),
                    ("Read", "你走近公佈欄細閱，上面以工整楷體張貼著四坊更次、各坊供水時間以及近期大角原牛隊通商班次。"),
                    ("Operate", "你擦拭公佈欄上的落塵，字跡分明，一目了然。"),
                ],
            ))
        } else if room_name.contains("工坊")
            || room_name.contains("作坊")
            || room_name.contains("大鍛坊")
            || room_name.contains("廠")
            || room_name.contains("窯")
            || room_name.contains("鋪")
            || room_name.contains("鍛")
        {
            Some(make_peninsula_object(
                "wt_reg_wenyao_workshop_bench",
                "工匠工具架",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "厚木壁板上分門別類掛著精鋼手錘、鉗夾、銼刀與鋼角尺，下方是引自暗渠的活水冷卻石槽，水汽蒸騰。"),
                    ("Use", "你走近活水石槽浸手洗去油灰，清涼活水洗滌疲乏，令人神清氣爽。"),
                    ("Operate", "你伸手整理掛架上的工具，擺放井然有序，刃口鎢鋼冷光內斂。"),
                ],
            ))
        } else if room_name.contains("大埠")
            || room_name.contains("拍賣")
            || room_name.contains("大市")
            || room_name.contains("棧")
            || (room_name.contains("商行")
                || room_name.contains("貨運行")
                || room_name.contains("車馬行")
                || room_name.contains("聯運")
                || room_name.contains("糧行"))
            || (room_name.contains("庫")
                && !room_name.contains("藏書")
                && !room_name.contains("印行")
                && !room_name.contains("典藏"))
        {
            Some(make_peninsula_object(
                "wt_reg_wenyao_market_post",
                "通商貨架棚",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "場地正中立著高大的商會通商立柱，上掛夜光貝防風燈，周圍搭建著堅固的防水油布貨架棚，便於堆疊周轉大宗貨物。"),
                    ("Read", "你走近立柱端讀鐫刻的商貿契稅規章與每日各港口船班、車隊發貨時序。"),
                    ("Operate", "你拉緊貨棚一角的防風挽繩，油布緊繃抗風，遮蔽妥帖。"),
                ],
            ))
        } else if (room_name.contains("院")
            || room_name.contains("館")
            || room_name.contains("局")
            || room_name.contains("台")
            || (room_name.contains("所")
                && !room_name.contains("巡防")
                && !room_name.contains("哨所")
                && !room_name.contains("診所")
                && !room_name.contains("居所")
                && !room_name.contains("宿所")))
            && !room_name.contains("藏書")
            && !room_name.contains("印行")
        {
            Some(make_peninsula_object(
                "wt_reg_wenyao_study_bench",
                "杉木公文案",
                &["Look", "Read", "Use"],
                &[
                    ("Look", "開闊的杉木公案上鋪著白綿吸墨紙，案旁設有整齊的公文插架與黃銅鎮紙，透著嚴謹典雅的學風。"),
                    ("Read", "你翻看案頭的記錄名冊與行事日程，字跡工楷，條分縷析。"),
                    ("Use", "你於案前稍坐端整儀容，案几整潔，心無雜念。"),
                ],
            ))
        } else {
            Some(make_peninsula_object(
                "wt_reg_wenyao_amenities",
                "黃銅活水栓",
                &["Look", "Use", "Bath"],
                &[
                    ("Look", "打磨光潔的文曜黃銅水栓連接著暗渠活水管，下方是一座嵌有半嶺溫石的保溫熱水槽，槽旁備有竹綿長絨擦面巾與松脂皂角。"),
                    ("Use", "你旋動黃銅水栓，清純甘洌的活水嘩嘩流淌而出，洗滌手面，神清氣爽。"),
                    ("Bath", "你引入溫石槽加熱的熱水浸沐洗面，熱氣騰騰驅散了旅途風塵與肌肉酸脹，倍感舒泰。"),
                ],
            ))
        }
    } else {
        None
    }
}

pub fn is_final_map_public_role(s: &str) -> bool {
    FINAL_MAP_SCENE_TEXTS
        .iter()
        .any(|scene| s.contains(scene.role))
}
pub fn public_person_label_for_room(room_name: &str) -> Option<&'static str> {
    if let Some(role) = final_map_public_role_for_room(room_name) {
        Some(role)
    } else if room_name.contains("烘藥房") || room_name.contains("百草烘藥") {
        Some("百草房焙藥匠人")
    } else if room_name.contains("診所") || room_name.contains("藥房") {
        Some("診所司藥學徒")
    } else if room_name.contains("曆算") || room_name.contains("星曆") || room_name.contains("推步") {
        Some("文曜曆算系學生")
    } else if room_name.contains("印行館") || room_name.contains("造紙") || room_name.contains("典藏庫") {
        Some("講堂街印書匠")
    } else if room_name.contains("書齋") || room_name.contains("校勘齋") || room_name.contains("墨香居") {
        Some("講堂街讀書人")
    } else if room_name.contains("習武堂") || room_name.contains("試招院") || room_name.contains("武勇舍") || room_name.contains("習武") || (room_name.contains("試招") && room_name.contains("文曜")) {
        Some("分家巷習武青年")
    } else if room_name.contains("樞紐石橋") || room_name.contains("拱橋") || room_name.contains("竹筏渡") {
        Some("三水口往來行商")
    } else if room_name.contains("藥香巷") || room_name.contains("爐火巷") || room_name.contains("百工石道") {
        Some("分家巷百工匠徒")
    } else if (room_name.starts_with("文曜") && is_wenyao_residential_or_study(room_name))
        || ((room_name.contains("居") || room_name.contains("宅") || room_name.contains("院") || room_name.contains("舍")) && room_name.contains("文曜"))
    {
        Some("文曜街坊住戶")
    } else if room_name.contains("總商行") || room_name.contains("商盟") {
        Some("總商行核算員")
    } else if room_name.contains("食堂") || room_name.contains("大食") {
        Some("三水食堂外場服務員")
    } else if room_name.contains("浴場") || room_name.contains("溫水") {
        Some("泉香浴場湯池管理員")
    } else if room_name.contains("旅社") || room_name.contains("茶屋") {
        Some("東橋茶屋茶點服務員")
    } else if room_name.contains("鍛造") || room_name.contains("打鐵") {
        Some("分家巷司爐鐵匠")
    } else if room_name.contains("巧木") || room_name.contains("傢俱") {
        Some("巧木坊刨木匠人")
    } else if room_name.contains("雲錦") || room_name.contains("織造") || room_name.contains("服飾") {
        Some("雲錦坊織繡女工")
    } else if room_name.contains("聯運") || room_name.contains("車馬") {
        Some("西門聯運趕車腳夫")
    } else if room_name.contains("鐘樓") || room_name.contains("信使") {
        Some("鐘樓快騎信使")
    } else if room_name.contains("習武") || room_name.contains("試招") {
        Some("習武堂教習教頭")
    } else if room_name.contains("剪剃") || room_name.contains("理髮") {
        Some("剪剃鋪修面師傅")
    } else if room_name.contains("柑仔店") {
        Some("柑仔店店員")
    } else if room_name.contains("修傘") || room_name.contains("補靴") {
        Some("修傘補靴手藝匠")
    } else if room_name.contains("講堂街") || room_name.contains("學院巷") || room_name.contains("書院巷") {
        Some("講堂街抱書學生")
    } else if room_name.starts_with("文曜") && room_name.contains("三水") {
        Some("三水口往來行商")
    } else if room_name.starts_with("文曜") && (room_name.contains("分家巷") || room_name.contains("百工")) {
        Some("分家巷百工匠徒")
    } else if room_name.contains("西門") {
        Some("西門行路旅客")
    } else if room_name.contains("水尾寮渡頭市集") || room_name.contains("渡頭市集") {
        Some("看貨人")
    } else if room_name.contains("半嶺水岸渡口") || room_name.contains("水岸渡口") || room_name.contains("水尾寮渡口") || room_name.contains("水尾寮渡頭") || room_name.contains("北岸") || room_name.contains("等候") {
        Some("候渡者")
    } else if room_name.contains("水尾寮水網口") || room_name.contains("水網入口") {
        Some("引船人")
    } else if room_name.contains("湖港木棧") {
        Some("候船者")
    } else if room_name.contains("短船泊位") {
        Some("船戶")
    } else if room_name.contains("魚貨") || room_name.contains("分揀") {
        Some("分揀工")
    } else if room_name.contains("蘆草") || room_name.contains("蘆棚") {
        Some("採集者")
    } else if room_name.contains("水倉") {
        Some("倉管")
    } else if room_name.contains("河口潮泥") || room_name.contains("潮泥碼頭") {
        Some("碼頭工")
    } else if room_name.contains("稅站棚") {
        Some("稅吏")
    } else if room_name.contains("船行門面") {
        Some("船行人")
    } else if room_name.contains("潮線貨場") {
        Some("搬貨人")
    } else if room_name.contains("風舶角港倉門") || room_name.contains("港倉門") {
        Some("倉管")
    } else if room_name.contains("商船候泊線") {
        Some("船員")
    } else if room_name.contains("東南船塢口") {
        Some("船匠")
    } else if room_name.contains("修船架") {
        Some("修船工")
    } else if room_name.contains("補給倉") {
        Some("倉管")
    } else if room_name.contains("碼頭木階") {
        Some("船客")
    } else if room_name.contains("帆布曬場") {
        Some("補帆人")
    } else if room_name.contains("船戶街口") {
        Some("船戶")
    } else if room_name.contains("外貿碼頭") {
        Some("船員")
    } else if room_name.contains("海關前棚") {
        Some("海關吏")
    } else if room_name.contains("商館街口") {
        Some("商館人")
    } else if room_name.contains("風舶角外洋棧倉") || room_name.contains("外洋棧倉") {
        Some("倉管")
    } else if room_name.contains("翻譯桌") {
        Some("翻譯")
    } else if room_name.contains("匯兌小屋") {
        Some("兌票人")
    } else if room_name.contains("風舶角避風港") || room_name.contains("避風港") {
        Some("船戶")
    } else if room_name.contains("港灣貨棚") {
        Some("搬魚人")
    } else if room_name.contains("漁網曬場") {
        Some("補網人")
    } else if room_name.contains("冷霧魚市") {
        Some("魚販")
    } else if room_name.contains("船戶屋排") {
        Some("船戶家屬")
    } else if room_name.contains("烏石鼻守望石階") || room_name.contains("守望石階") {
        Some("守望人")
    } else if room_name.contains("礦坑外棚") {
        Some("礦工")
    } else if room_name.contains("洗礦溝") {
        Some("洗礦工")
    } else if room_name.contains("粗料倉") {
        Some("倉管")
    } else if room_name.contains("工具修棚") {
        Some("修具工")
    } else if room_name.contains("礦渣坡") {
        Some("推車人")
    } else if room_name.contains("紅土崁選礦車場") || room_name.contains("選礦車場") {
        Some("車夫")
    } else if room_name.contains("糧倉外秤") {
        Some("秤手")
    } else if room_name.contains("倉門前場") {
        Some("守倉人")
    } else if room_name.contains("曬場邊") {
        Some("翻穀人")
    } else if room_name.contains("車行口") {
        Some("車夫")
    } else if room_name.contains("守倉棚") {
        Some("倉管")
    } else if room_name.contains("糧車泊位") {
        Some("搬糧人")
    } else if room_name.contains("水尾寮渡頭市集") || room_name.contains("渡頭市集") {
        Some("看貨人")
    } else if room_name.contains("歇腳") {
        Some("歇腳旅人")
    } else if room_name.contains("踏板") {
        Some("候船者")
    } else if room_name.contains("南岸") || room_name.contains("泥坡") {
        Some("上岸客")
    } else if room_name.contains("船戶") || room_name.contains("作業") {
        Some("渡工")
    } else if room_name.contains("倉棚") {
        Some("搬貨人")
    } else {
        None
    }
}
pub fn scene_observation_line(room_name: &str, pronoun: &str) -> Option<String> {
    if room_name.contains("烘藥房") || room_name.contains("百草烘藥") {
        Some(format!("{pronoun}正在雙層紅泥焙藥爐前專注微調通氣瓦片，翻動竹篩中的切片藥草，濃郁焙藥香氣在身側繚繞。"))
    } else if room_name.contains("診所") || room_name.contains("藥房") {
        Some(format!("{pronoun}正手持白瓷藥碾在研缽中研磨草藥粉末，動作沉穩細緻，空氣中散發著清涼的草木薄荷香。"))
    } else if room_name.contains("曆算") || room_name.contains("星曆") || room_name.contains("推步") {
        Some(format!("{pronoun}端坐於黑檀木計數板案前，指尖在滑鈕間飛快撥動，目光凝視著黃銅渾天星軌推步儀的子午刻度。"))
    } else if room_name.contains("印行館") || room_name.contains("造紙") || room_name.contains("典藏庫") {
        Some(format!("{pronoun}手持棕櫚墨刷在排版梨木活字上勻勻刷著松煙墨，隨後利落覆上竹綿宣紙滾壓印製。"))
    } else if room_name.contains("書齋") || room_name.contains("校勘齋") || room_name.contains("墨香居") {
        Some(format!("{pronoun}在沉水烏木書案前執筆抄校線裝古籍，窗外竹影搖曳，室內唯聞筆尖游走宣紙的沙沙微響。"))
    } else if room_name.contains("習武堂") || room_name.contains("試招院") || room_name.contains("武勇舍") || room_name.contains("習武") {
        Some(format!("{pronoun}在練武場青石板上扎穩馬步，手持包革練習木刀專注推演刀式，吐納均勻沉實。"))
    } else if (room_name.starts_with("文曜") && is_wenyao_residential_or_study(room_name))
        || ((room_name.contains("居") || room_name.contains("宅") || room_name.contains("院") || room_name.contains("舍")) && room_name.contains("文曜"))
    {
        Some(format!("{pronoun}在宅院門廊下收拾防雨竹編信箱與山茶盆栽，見有行客路過，和善地點頭致意。"))
    } else if room_name.contains("樞紐石橋") || room_name.contains("拱橋") || room_name.contains("竹筏渡") {
        Some(format!("{pronoun}正倚在雕花青石欄杆旁清點隨身貨單，招呼著川流泊位上的竹筏渡伕。"))
    } else if room_name.contains("藥香巷") || room_name.contains("爐火巷") || room_name.contains("百工石道") {
        Some(format!("{pronoun}正坐在門前條石上磨礪小刀與鑿子，不時抬手拭去額前微汗，神情專注踏實。"))
    } else {
        None
    }
}


pub fn micro_interaction_lines_for_room(room_name: &str) -> &'static [&'static str] {
    let hour = crate::store::current_game_hour();
    if hour >= 22 || hour < 5 {
        let is_daytime_facility = room_name.contains("工坊")
            || room_name.contains("印行")
            || room_name.contains("造紙")
            || room_name.contains("典藏庫")
            || room_name.contains("學院")
            || room_name.contains("講堂")
            || room_name.contains("國小")
            || room_name.contains("教室")
            || room_name.contains("校舍")
            || room_name.contains("學舍")
            || room_name.contains("商行")
            || room_name.contains("商會")
            || room_name.contains("鋪")
            || room_name.contains("鍛造")
            || room_name.contains("木作")
            || room_name.contains("織造")
            || room_name.contains("市集")
            || room_name.contains("藏書閣")
            || room_name.contains("烘藥房")
            || room_name.contains("曆算");
        if is_daytime_facility {
            return &[
                "夜深人靜，廊下防風燈映著緊閉的木門與靜置的案几。",
                "四下萬籟俱寂，夜風吹拂著屋簷銅鈴，屋內陳設在夜色中安寧沉靜。",
                "長夜漫漫，街坊燈火漸熄，唯有案角微弱的貝母珠光靜靜映照著沉睡的館閣。",
            ];
        }
    }
    if let Some(scene) = final_map_scene_for_room(room_name) {
        scene.micro
    } else if room_name.contains("烘藥房") || room_name.contains("百草烘藥") {
        &[
            "焙藥匠人用鐵火鉗撥了撥爐膛炭火，濃郁的草藥香氣隨熱氣漫開。",
            "有人端著裝滿剛切好當歸片的竹篩走過，低聲念著今日的焙乾火候。",
            "微風掠過窗櫺，烘烤中的山風草香氣層層疊疊飄滿了整條街巷。",
        ]
    } else if room_name.contains("曆算") || room_name.contains("星曆") || room_name.contains("推步") {
        &[
            "兩名曆算系學生對照著黃銅星軌儀上的刻度，低聲核算著下個節氣的潮汐。",
            "黑檀木計數板的滑鈕聲清脆連綿，有人在算題草紙上記下一串數目。",
            "微風翻動案几上的天文推步圖譜，室內唯有計數滑鈕起落的利落脆響。",
        ]
    } else if room_name.contains("印行館") || room_name.contains("造紙") || room_name.contains("典藏庫") {
        &[
            "排版匠人手持竹鑷在活字盤中敏捷挑揀字模，木框碰撞出清脆篤響。",
            "抄紙石槽邊水聲輕響，薄如蟬翼的濕紙被平整貼上焙紙熱牆。",
            "剛印出的宣紙散發著新鮮的松煙墨香，工匠小心翼翼將其揭下晾乾。",
        ]
    } else if room_name.contains("書齋") || room_name.contains("校勘齋") || room_name.contains("墨香居") {
        &[
            "讀書人輕輕翻過一頁泛黃古籍，指尖沾著淡淡的松煙墨香。",
            "案几前傳來低低的誦讀聲，案角夜光貝母燈灑下柔和澄澈的清光。",
            "窗外翠竹在微風中沙沙作響，書齋內筆墨安詳，時光悠長。",
        ]
    } else if is_wenyao_residential_or_study(room_name) {
        &[
            "街坊鄰里在宅門石階前互道安好，門前山茶花在微風中輕輕搖曳。",
            "屋內傳來水栓蓄水的清響，晚炊與松香木的溫暖氣息飄出窗櫺。",
            "巷道裡偶爾傳來孩童奔跑的足音，隨後又歸於安寧祥和的市井日常。",
        ]
    } else if room_name.contains("水尾寮渡口") || room_name.contains("水尾寮渡頭") || room_name.contains("北岸") || room_name.contains("等候") {
        &[
            "兩名候渡者低聲確認船班，很快又把聲音壓進人群裡。",
            "有人替旁邊的候渡者扶了一下包袱，目光仍留在渡船踏板那頭。",
            "候渡的人群短短讓出一條縫，又隨船繩聲重新擠回原處。",
        ]
    } else if room_name.contains("水尾寮水網口") || room_name.contains("水網入口") {
        &[
            "引船人把篙尖從水面收回，低聲提醒過棧的人看清腳下。",
            "有人在霧牌旁停了一下，等引船人確認水道裡的來船。",
            "棧板邊短短響起篙聲，濕腳印又往湖港木棧那頭接過去。",
        ]
    } else if room_name.contains("湖港木棧") {
        &[
            "候船者把貨包往身側收緊，給抬魚籃的人讓出一條窄縫。",
            "濕繩圈旁有人低聲問船位，回答很快被木棧下的水聲蓋住。",
            "棧板上幾撥腳步短短交錯，又各自散向泊位和貨棚。",
        ]
    } else if room_name.contains("短船泊位") {
        &[
            "船戶扶著船舷催人站穩，篙影在水面晃了一下。",
            "有人把船槳挪到內側，免得剛上棧的人踢到濕木邊。",
            "泊位旁短短傳來靠船手勢，低語隨水面晃開。",
        ]
    } else if room_name.contains("魚貨") || room_name.contains("分揀") {
        &[
            "分揀工把魚籃推到濕布邊，手勢比說話更快。",
            "有人掀開蓋布看了一眼魚色，又把籃口轉向秤邊。",
            "分揀場裡腥味和低聲短賣混在一起，很快又被木棧腳步切散。",
        ]
    } else if room_name.contains("蘆草") || room_name.contains("蘆棚") {
        &[
            "採集者把蘆草捆靠上曬架，袖口還滴著水。",
            "有人用草繩重新束緊一把蘆草，讓出通往水倉前場的板路。",
            "棚下草屑被腳步帶起，濕痕從水邊一路拖到貨棚陰影裡。",
        ]
    } else if room_name.contains("水倉") {
        &[
            "倉管在棚口比了個手勢，板車便往另一側慢慢挪開。",
            "有人把魚貨和蘆草分到不同棚口，低聲核對下一批入倉順序。",
            "倉門前短短響起木輪壓過濕板的聲音，水聲從地板下回來。",
        ]
    } else if room_name.contains("河口潮泥") || room_name.contains("潮泥碼頭") {
        &[
            "碼頭工把纜繩在木樁上多繞一圈，潮泥被腳跟擠出暗亮水痕。",
            "有人把糧袋和木箱分到兩側，讓從守倉棚過來的人先靠邊等候。",
            "潮泥碼頭短短響起靠船手勢，河貨和海貨的腳步在木樁旁交錯。",
        ]
    } else if room_name.contains("稅站棚") {
        &[
            "稅吏把印章壓上貨單，棚外候查的人群立刻安靜了一小截。",
            "有人把濕了角的貨單攤平，木箱和糧袋暫時停在稅站棚外。",
            "棚下短短響起翻紙聲，碼頭那邊的纜繩聲被壓到低處。",
        ]
    } else if room_name.contains("船行門面") {
        &[
            "船行人抬手指向航線板，幾個等船的人便把目光轉向潮線貨場。",
            "有人把新寫的牌價掛到門面下，鹹風一吹便把木牌撞出輕響。",
            "船行門面前短短傳過雇船低語，很快又被碼頭人聲切開。",
        ]
    } else if room_name.contains("潮線貨場") {
        &[
            "搬貨人趁潮水退開，把墊高的木箱往候泊線那頭推了一段。",
            "有人沿潮印重新擺好箱包，讓要查驗的貨先靠近稅站棚。",
            "貨場裡木箱短短擦過石條，濕氣和低聲催促一同貼到腳邊。",
        ]
    } else if room_name.contains("風舶角港倉門") || room_name.contains("港倉門") {
        &[
            "倉管掀起雨布一角，讓剛放行的箱包先壓到倉門內側。",
            "有人把封牌扶正，低聲把河貨和海貨分到不同門邊。",
            "風舶角港倉門裡短短傳出木箱靠牆的悶響，雨布又垂回來。",
        ]
    } else if room_name.contains("商船候泊線") {
        &[
            "船員把錨繩收成一圈，抬頭看稅站棚那邊是否放行。",
            "有人在候泊牌旁比了個手勢，等潮的船影在外頭輕輕偏轉。",
            "候泊線上短短響起繩結抽緊的聲音，船員低語被潮聲吞下去。",
        ]
    } else if room_name.contains("東南船塢口") {
        &[
            "船匠把半修的船板靠到入口旁，木屑從袖口落到潮線邊。",
            "有人把瀝青桶往修船架那側挪開，給補給倉過來的人讓路。",
            "船塢口短短響起木槌聲，碼頭木階那頭的人聲跟著停了一下。",
        ]
    } else if room_name.contains("修船架") {
        &[
            "修船工沿著肋木敲了一串，低頭看楔子是否咬緊。",
            "有人把瀝青刷擱回桶邊，讓抱木板的人從架下穿過。",
            "修船架下短短落下一陣木屑，敲打聲又往帆布曬場那頭傳。",
        ]
    } else if room_name.contains("補給倉") {
        &[
            "倉管把淡水桶和乾糧箱分到兩堆，示意碼頭木階的人先等一等。",
            "有人扛起油布包往倉門外放，腳步被水桶擠得很慢。",
            "補給倉前短短響起木桶碰撞聲，催裝船的低語又靠近一點。",
        ]
    } else if room_name.contains("碼頭木階") {
        &[
            "船客扶著繩柱下階，鞋底在半濕木階上擦出一聲輕響。",
            "有人把淡水桶從補給倉那邊接過來，沿木階慢慢傳下去。",
            "碼頭木階旁短短一陣晃動，等上船的人立刻收住腳步。",
        ]
    } else if room_name.contains("帆布曬場") {
        &[
            "補帆人把針線穿過舊帆裂口，指尖壓住被風吹起的布角。",
            "有人把曬到一半的帆布翻過一折，讓修船架那邊的人看清補痕。",
            "曬場裡短短響起針線架碰木聲，鹹風把船旗聲帶過來。",
        ]
    } else if room_name.contains("船戶街口") {
        &[
            "船戶在招工牌下比了比工錢，飯攤邊的人把聲音壓低。",
            "有人把鹽味衣物從繩線上收起，給碼頭木階來的人讓出路。",
            "船戶街口短短傳過一陣船期低語，很快被飯攤鍋聲蓋住。",
        ]
    } else if room_name.contains("外貿碼頭") {
        &[
            "船員把外地貨箱從吊繩下扶穩，港旗在頭頂被海風扯直。",
            "有人把繩結重新套上木樁，示意海關前棚那邊可以放行下一箱。",
            "外貿碼頭短短響起滑輪聲，商館街口的人聲跟著往碼頭邊靠近。",
        ]
    } else if room_name.contains("海關前棚") {
        &[
            "海關吏把濕貨單壓平，封條在桌邊被手掌按住。",
            "有人把待查貨箱停在棚外，低聲等翻譯桌那邊說清貨名。",
            "海關前棚短短響起印章聲，排在外頭的人群立刻靜了一截。",
        ]
    } else if room_name.contains("商館街口") {
        &[
            "商館人打開貨樣匣給客人看，外語低語在門口換了幾輪。",
            "有人望向碼頭船旗，又把剛問到的價碼送往匯兌小屋。",
            "商館街口短短傳過一陣陌生口音，很快被貨樣匣合上的聲音截住。",
        ]
    } else if room_name.contains("風舶角外洋棧倉") || room_name.contains("外洋棧倉") {
        &[
            "倉管把異地木箱推到雨布內側，低聲分出已查和待查兩堆。",
            "有人擦過箱面漆印，確認貨名後才讓拖箱聲進到門內。",
            "風舶角外洋棧倉短短響起木箱靠牆的悶聲，雨布又被海風吹落。",
        ]
    } else if room_name.contains("翻譯桌") {
        &[
            "翻譯把合約上的貨名重新說了一遍，指尖壓著兩種文字的邊線。",
            "有人把印章推到桌角，等海關前棚那邊確認下一批箱號。",
            "翻譯桌旁短短靜了一下，接著又響起一串被轉成在地話的貨名。",
        ]
    } else if room_name.contains("匯兌小屋") {
        &[
            "兌票人撥了撥銅秤，讓錢票在賬冊旁重新疊齊。",
            "有人把商館街口報來的價碼寫入欄裡，又抬頭問風舶角外洋棧倉的貨名。",
            "匯兌小屋短短響起銅秤輕碰聲，等價的人把聲音壓得更低。",
        ]
    } else if room_name.contains("風舶角避風港") || room_name.contains("避風港") {
        &[
            "船戶把結霜纜繩在木樁上多繞一圈，避風船在內側輕輕碰岸。",
            "有人抬手看了一眼北風，示意後面的小船先貼著碼頭等。",
            "避風港短短響起船腹碰木聲，港灣貨棚那邊的人聲跟著收低。",
        ]
    } else if room_name.contains("港灣貨棚") {
        &[
            "搬魚人把濕魚籃推到鹽箱旁，棚門下立刻多了一道水痕。",
            "有人把濕木箱墊高，讓剛從碼頭進來的魚貨先靠牆停住。",
            "港灣貨棚短短響起木箱拖地聲，冷霧魚市那頭有人催了一句。",
        ]
    } else if room_name.contains("漁網曬場") {
        &[
            "補網人沿著破口穿線，指尖把半乾濕網壓在木架上。",
            "有人把被海風吹起的網角重新拉平，讓船戶屋排那頭的人能過。",
            "漁網曬場短短響起木架碰聲，冷霧魚市的叫價隔著網眼傳來。",
        ]
    } else if room_name.contains("冷霧魚市") {
        &[
            "魚販把濕布掀起一角，木秤旁立刻聚起低聲問價。",
            "有人把剛送到的魚籃轉向貨棚，讓買貨人先看魚色。",
            "冷霧魚市短短響起秤錘聲，霧氣裡的議價聲又壓低一截。",
        ]
    } else if room_name.contains("船戶屋排") {
        &[
            "船戶家屬在排班板前換下一枚木牌，火盆煙氣貼著屋簷散開。",
            "有人把濕衣從繩上收起，給守望石階下來的人讓出窄路。",
            "船戶屋排短短傳過一陣排班低語，很快被火盆劈啪聲蓋住。",
        ]
    } else if room_name.contains("烏石鼻守望石階") || room_name.contains("守望石階") {
        &[
            "守望人扶著濕石欄看海面顏色，霧牌在風口輕輕晃動。",
            "有人伸手試了試警鈴繩，又轉頭看向船戶屋排的排班板。",
            "守望石階短短響起霧牌碰木聲，港灣人聲在風裡斷了一下。",
        ]
    } else if room_name.contains("礦坑外棚") {
        &[
            "礦工把沾泥的鎬靠到木架旁，吊燈光在坑口晃了一下。",
            "有人在濕支柱下換過肩上的布帶，低聲問下一批粗料走哪邊。",
            "礦坑外棚短短響起換班腳步聲，洗礦溝的水聲跟著靠近。",
        ]
    } else if room_name.contains("洗礦溝") {
        &[
            "洗礦工把石籃壓進冷水槽，混濁水面立刻翻出灰泡。",
            "有人把分出的碎料推向礦渣坡，手背在濕木槽邊擦了一下。",
            "洗礦溝短短響起水槽撞石聲，粗料倉那邊有人催著標記。",
        ]
    } else if room_name.contains("粗料倉") {
        &[
            "倉管把新送來的礦袋推到木牌下，炭線在灰粉上補了一筆。",
            "有人照著標記把粗料分成兩堆，等工具修棚那邊讓出車架。",
            "粗料倉短短響起礦袋落地聲，紅土崁選礦車場的車輪聲接著傳來。",
        ]
    } else if room_name.contains("工具修棚") {
        &[
            "修具工把鬆掉的鐵圈敲回車輪邊，火星在棚口短短一亮。",
            "有人把裂柄鎬換到木架下，等下一個礦工從外棚過來領走。",
            "工具修棚短短響起敲鐵聲，粗料倉的標記牌跟著晃了一下。",
        ]
    } else if room_name.contains("礦渣坡") {
        &[
            "推車人把廢料倒在安全線內，碎石沿新滑坡痕滾下幾步。",
            "有人用木棍撥開鬆動的灰石，示意後面的礦車先別靠太近。",
            "礦渣坡短短響起碎石滑落聲，洗礦溝的水聲變得更悶。",
        ]
    } else if room_name.contains("紅土崁選礦車場") || room_name.contains("選礦車場") {
        &[
            "車夫把繩索掛上車架，稱重木牌在灰粉裡露出半行字。",
            "有人把礦袋推上礦車，車輪壓過舊道口磨亮的車轍。",
            "紅土崁選礦車場短短響起車軸聲，邊城外門那頭有人抬頭張望。",
        ]
    } else if room_name.contains("糧倉外秤") {
        &[
            "秤手把糧袋扶上秤盤，指尖在秤桿刻痕旁停了一下。",
            "有人把剛稱過的糧袋拖到倉門方向，稻殼粉在地上擦出亮痕。",
            "雨棚下短短響起秤錘碰木的聲音，排隊的人聲跟著壓低。",
        ]
    } else if room_name.contains("倉門前場") {
        &[
            "守倉人照著秤單點了點頭，半開的倉門又往內退了一線。",
            "有人把封條貼回木栓旁，示意後面的糧袋先停在門外。",
            "倉門前的腳步短短停住，等裡頭的人把糧袋接進陰影裡。",
        ]
    } else if room_name.contains("曬場邊") {
        &[
            "翻穀人把木耙推過曬席，穀粒乾乾地響了一片。",
            "有人沿曬席邊緣收攏潮濕處，鞋面沾上一層稻殼粉。",
            "曬場邊的木耙聲短短停下，又順著風把穀香推到外秤那頭。",
        ]
    } else if room_name.contains("車行口") {
        &[
            "車夫扶著車轅催前車挪動，麻繩在輪旁繃了一下。",
            "有人把排隊牌往泥邊挪開，讓載糧車慢慢壓進倉區。",
            "車行口短短響起軸木聲，後面的人聲便跟著往前湧。",
        ]
    } else if room_name.contains("守倉棚") {
        &[
            "倉管把秤單壓在冊簿下，抬眼看向倉門前的糧袋。",
            "有人在棚柱旁換下值牌，茶壺旁的低語很快收住。",
            "守倉棚下短短翻過一頁冊簿，門前貨聲便被分成幾路。",
        ]
    } else if room_name.contains("糧車泊位") {
        &[
            "搬糧人從車尾接下一袋糧，肩膀一沉又立刻站穩。",
            "有人把麻繩從車輪旁收起，給下一架糧車讓出泊位。",
            "泊位旁木輪壓過濕土，搬糧聲一前一後接到守倉棚那頭。",
        ]
    } else if room_name.contains("市集") {
        &[
            "兩個看貨的人在價牌前比了比手勢，攤邊很快響起壓低的議價聲。",
            "有人把小包貨物遞給同伴掂量，眼角仍瞄著河邊的渡船。",
            "市集裡一陣貨聲掠過，幾個人同時朝下一班船的方向看去。",
        ]
    } else if room_name.contains("歇腳") {
        &[
            "歇腳的人把茶碗換到另一隻手，替身旁人挪了半條長凳。",
            "長凳旁有人低聲說了兩句，又把包袱往腳邊收緊。",
            "茶桶邊短短靜了一下，只剩河風磨過談話尾音。",
        ]
    } else if room_name.contains("踏板") {
        &[
            "兩名候船者在踏板前互相讓了半步，等船身晃動慢下來。",
            "有人伸手試了試濕木扶手，又提醒身後的人避開踏板邊的積水。",
            "踏板前的人群隨船腹碰岸的聲音停住，片刻後才重新低語。",
        ]
    } else if room_name.contains("南岸") || room_name.contains("泥坡") {
        &[
            "兩名上岸客一前一後踩過泥坡，鞋底拖出濕亮的痕跡。",
            "有人伸手扶了一把提包袱的人，又回頭看渡船是否還靠穩。",
            "坡邊的人短短停住腳步，等下一股水聲拍過才繼續上行。",
        ]
    } else if room_name.contains("船戶") || room_name.contains("作業") {
        &[
            "兩名渡工在木樁邊交接纜繩，低聲確認下一趟靠船的手勢。",
            "有人把撐篙靠回船邊，另一人彎身去看木楔是否卡牢。",
            "船邊短短響起一串分派活計的低語，很快被水聲蓋過。",
        ]
    } else if room_name.contains("倉棚") {
        &[
            "兩名搬貨人在麻袋旁交換手勢，一前一後把木箱挪到棚門邊。",
            "有人拍去袖口的泥點，替旁邊的人扶住快滑落的小包貨。",
            "倉棚前短短響起木箱拖過泥地的聲音，又被河風磨低。",
        ]
    } else {
        &[
            "近旁兩名行人低聲交談了幾句，很快又各自看向別處。",
            "有人朝身旁的人點了點頭，腳步聲在近處短短交錯。",
            "人群裡掠過一陣低語，片刻後又散回原本的動靜。",
        ]
    }
}

/// 根據在場人物狀態動態產出攤販攤位物件。
///
/// 排除移動中（`正在前往` / `正步行前往` / `邁步離開`）與睡眠中（`安歇` / `熟睡` / `就寢`）的角色；
/// 僅當攤販在場且處於擺攤營業狀態時，才動態生成對應攤位物件。
pub fn dynamic_vendor_objects_for_entities(entities: &[crate::entity::Character]) -> Vec<RoomObject> {
    let mut objects = Vec::new();
    for ent in entities {
        let act = ent.current_activity.as_str();
        let title = ent.display_title.as_str();
        let id = ent.id.as_str();

        // 排除移動中或睡眠中的角色
        if act.contains("正在前往")
            || act.contains("正步行前往")
            || act.contains("邁步離開")
            || act.contains("安歇")
            || act.contains("熟睡")
            || act.contains("就寢")
        {
            continue;
        }

        // 僅當處於擺攤營業或工作活動時生效
        let is_vending = act.contains('攤')
            || act.contains("叫賣")
            || act.contains("吆喝")
            || act.contains("招呼")
            || act.contains("楦頭")
            || act.contains("擺攤")
            || act.contains("晴雨傘");
        if !is_vending {
            continue;
        }

        // 1. wt_vendor_roast_beef (炭烤牛肉番薯攤)
        if id == "wt_vendor_roast_beef" || title.contains("炭烤牛肉番薯攤") {
            objects.push(make_peninsula_object(
                "wt_stall_roast_beef",
                "炭烤牛肉攤",
                &["Look", "Taste", "Use", "Smell"],
                &[
                    ("Look", "街角木輪推車上的炭火正旺，鐵網上烤著滋滋冒油的原牛肉串與焦蜜流淌的紅心烤番薯，煙火氣撲鼻。"),
                    ("Taste", "你接過一串剛烤好的焦香原牛肉串咬下一口，肉質勁道多汁，炭烤椒鹽香氣瞬間充盈口腔。"),
                    ("Use", "你走向炭烤牛肉番薯攤，準備向攤主買下一份炭烤牛肉串。"),
                    ("Smell", "你站在下風處細聞烤番薯散發出的甘甜焦香與烤肉煙火氣，頓時食指大動。"),
                ],
            ));
        }
        // 2. wt_vendor_noodle_soup (現煮大骨切仔麵攤)
        else if id == "wt_vendor_noodle_soup" || title.contains("切仔麵攤") {
            objects.push(make_peninsula_object(
                "wt_stall_noodle_soup",
                "大骨切仔麵攤",
                &["Look", "Taste", "Use", "Smell"],
                &[
                    ("Look", "木製麵攤前熱氣氤氳，大銅鍋裡翻滾著乳白濃郁的大骨高湯，竹笊籬整齊掛在木架旁，油蔥酥與芹菜珠香氣四溢。"),
                    ("Taste", "你接過一碗熱騰騰的現煮切仔麵啜飲熱湯，大骨湯頭醇厚回甘，麵條彈牙吸汁，暖胃舒暢。"),
                    ("Use", "你走向切仔麵攤，準備向攤主買下一碗切仔湯麵。"),
                    ("Smell", "濃白大骨高湯與酥炸紅蔥頭的誘人鹹香隨熱氣蒸騰撲面而來，引人垂涎。"),
                ],
            ));
        }
        // 3. wt_vendor_pepper_bun (貼爐蔥香胡椒餅攤)
        else if id == "wt_vendor_pepper_bun" || title.contains("胡椒餅攤") {
            objects.push(make_peninsula_object(
                "wt_stall_pepper_bun",
                "炭烤胡椒餅攤",
                &["Look", "Taste", "Use", "Smell"],
                &[
                    ("Look", "高大的炭火深鐵桶正冒著滾燙熱浪，桶壁內圈整齊貼滿烤得金黃微焦的厚實胡椒餅，芝麻粒粒飽滿。"),
                    ("Taste", "你咬下一口剛出爐的胡椒餅，酥脆外皮應聲碎開，滾燙濃郁的蔥香肉汁與辛香黑胡椒瞬間在舌尖爆發。"),
                    ("Use", "你走向胡椒餅攤，準備向攤主買下一份酥皮鮮肉胡椒餅。"),
                    ("Smell", "烘烤麵皮的麥香、焙烤黑胡椒的辛香與炭火焦香交織在一起，濃烈撲鼻。"),
                ],
            ));
        }
        // 4. wt_vendor_herbal_tea (寒水玉冰鎮仙草凍攤)
        else if id == "wt_vendor_herbal_tea" || title.contains("仙草凍攤") || title.contains("烏梅湯攤") || title.contains("酸梅湯攤") {
            objects.push(make_peninsula_object(
                "wt_stall_herbal_tea",
                "冰鎮仙草凍攤",
                &["Look", "Taste", "Use", "Smell"],
                &[
                    ("Look", "青瓷大陶甕外凝著細密水珠，浸在深井冷泉木盆中，旁邊懸著兩枚光亮的小銅盞，青瓷缽裡盛著晶瑩透亮的仙草凍與清涼百草飲。"),
                    ("Taste", "你接過一盞寒水玉冰鎮黑糖仙草凍舀起一口，滑嫩仙草沁涼透心，手炒黑糖醇厚回甘，滿口清香頓時消暑解膩。"),
                    ("Use", "你走向仙草凍攤，準備向攤主買下一份寒水玉冰鎮黑糖仙草凍。"),
                    ("Smell", "文火慢熬仙草的醇厚草本清香伴著手炒黑糖的焦甜香氣撲鼻而來，令人神清氣爽。"),
                ],
            ));
        }
        // 5. wt_vendor_tofu_pudding (古早味黑糖粉圓豆花攤)
        else if id == "wt_vendor_tofu_pudding" || title.contains("豆花攤") {
            objects.push(make_peninsula_object(
                "wt_stall_tofu_pudding",
                "黑糖豆花攤",
                &["Look", "Taste", "Use", "Smell"],
                &[
                    ("Look", "原木手推攤車上立著光滑溫潤的白鐵豆花大圓桶，旁置一小陶缽晶瑩剔透、裹滿琥珀黑糖漿的慢火粉圓。"),
                    ("Taste", "你舀起一匙溫潤豆花入口，嫩滑如脂的豆花入口即化，伴著Q彈黑糖粉圓與微辛老薑糖水，甜而不膩。"),
                    ("Use", "你走向豆花攤，準備向攤主買下一份黑糖薑汁熱豆花。"),
                    ("Smell", "天然黃豆的醇濃豆香與現炒黑糖的焦甜香氣溫暖撲鼻，令人回味無窮。"),
                ],
            ));
        }
        // 6. wt_vendor_rice_roll (現包紫米飯糰早點攤)
        else if id == "wt_vendor_rice_roll" || title.contains("飯糰早點攤") {
            objects.push(make_peninsula_object(
                "wt_stall_rice_roll",
                "紫米飯糰攤",
                &["Look", "Taste", "Use", "Smell"],
                &[
                    ("Look", "厚實的木蒸桶掀開棉布冒出騰騰白氣，旁邊排著酥脆老油條、炒香酸菜、香脆菜脯與現切滷蛋等豐富料餡。"),
                    ("Taste", "你大口咬下剛捏好的紫米飯糰，糯米粒粒香糯彈牙，內餡老油條香脆無比，鹹香酸脆層次豐富極其飽腹。"),
                    ("Use", "你走向早點攤，準備向攤主買下一份荷葉糯米炊飯。"),
                    ("Smell", "現蒸紫糯米特有的溫潤米香混著炒酸菜與老油條的鹹香，是清晨最踏實的煙火氣息。"),
                ],
            ));
        }
        // 7. wt_vendor_soy_milk (現磨米漿杏仁茶攤)
        else if id == "wt_vendor_soy_milk" || title.contains("米漿杏仁茶攤") {
            objects.push(make_peninsula_object(
                "wt_stall_soy_milk",
                "現磨米漿攤",
                &["Look", "Taste", "Use", "Smell"],
                &[
                    ("Look", "攤前溫著兩口長嘴大銅壺，壺嘴不斷吐出溫熱蒸氣，一旁擺著竹盤盛裝的金黃酥脆現炸油條。"),
                    ("Taste", "你端起熱碗啜飲濃稠醇厚的花生糙米漿，焙烤花生的濃香滑入喉嚨，再蘸上一口酥脆油條，甘醇滿口。"),
                    ("Use", "你走向米漿攤，準備向攤主買下一份老堰頭現磨溫米漿。"),
                    ("Smell", "現磨烘烤花生糙米的濃郁焦香與南杏仁的溫潤甜香隨熱霧四溢，暖人心脾。"),
                ],
            ));
        }
        // 8. wt_vendor_morning_produce (清晨帶露時令蔬果攤)
        else if id == "wt_vendor_morning_produce" || title.contains("蔬果攤") {
            objects.push(make_peninsula_object(
                "wt_stall_morning_produce",
                "時令蔬果攤",
                &["Look", "Taste", "Use", "Smell"],
                &[
                    ("Look", "竹編菜簍裡整齊碼放著清晨方從田畦採摘的新鮮高麗菜、帶泥嫩薑、翠綠青蔥與鮮紅多汁的水果，葉片上猶掛著晶亮晨露。"),
                    ("Taste", "你挑選了一份鮮脆水芹菜咀嚼，清甜脆嫩、滿口鮮綠生機。"),
                    ("Use", "你走向蔬果攤，準備向攤主挑選買下一捆水芹菜。"),
                    ("Smell", "水靈靈的新鮮瓜果與帶泥時蔬散發出泥土與青草的純淨生機芬芳。"),
                ],
            ));
        }
        // 9. wt_vendor_morning_fish (現撈三水河鮮蛤蜊攤)
        else if id == "wt_vendor_morning_fish" || title.contains("河鮮蛤蜊攤") {
            objects.push(make_peninsula_object(
                "wt_stall_morning_fish",
                "三水河鮮攤",
                &["Look", "Taste", "Use", "Smell"],
                &[
                    ("Look", "淺木盆注滿清澄河水，幾尾鮮活溪魚正在盆底輕快擺尾，粗麻布上鋪滿吐淨泥沙的肥美文蛤與青背河蝦。"),
                    ("Taste", "你嚐了嚐攤上剛汆燙開殼的現撈活文蛤，肉質鮮甜嫩滑，滿口清冽河水鮮味。"),
                    ("Use", "你走向河鮮攤，準備向攤主稱上一斤鮮活青殼河蝦。"),
                    ("Smell", "淺木盆與碎冰中散發出三水川清澈活水特有的清涼河鮮水氣，毫無半點腥濁。"),
                ],
            ));
        }
        // 10. wt_npc_ma_buxue (老匠晴雨傘與皮靴修補攤)
        else if id == "wt_npc_ma_buxue"
            || (title.contains("修傘補靴") && (act.contains("楦頭") || act.contains("晴雨傘") || act.contains("修補攤")))
        {
            objects.push(make_peninsula_object(
                "wt_stall_umbrella_boot_repair",
                "修傘補靴攤",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "街角鋪著防潮厚油布，擺著一副磨損鋥亮的硬木鞋楦、鐵砧與各式皮針錐子，旁邊架著幾柄剛刷過防水熟桐油的堅韌竹骨晴雨傘。"),
                    ("Use", "你走向晴雨傘與修補攤，挑選了一柄堅韌的竹骨晴雨摺疊傘買下。"),
                    ("Operate", "你請老匠檢視自己的行囊與行靴，老匠熟練地接過查看針腳皮面，俐落地替你緊固了帶扣。"),
                ],
            ));
        }
    }
    objects
}

/// 根據 ViewEntity 清單動態產出攤位對應的 ViewObject
pub fn dynamic_vendor_view_objects_for_entities(
    entities: &[crate::server::ViewEntity],
) -> Vec<crate::server::ViewObject> {
    let mut view_objects = Vec::new();
    for ent in entities {
        let act = ent.current_activity.as_str();
        let title = ent.display_name.as_str();
        let id = ent.id.as_str();

        if act.contains("正在前往")
            || act.contains("正步行前往")
            || act.contains("邁步離開")
            || act.contains("安歇")
            || act.contains("熟睡")
            || act.contains("就寢")
        {
            continue;
        }

        let is_vending = act.contains('攤')
            || act.contains("叫賣")
            || act.contains("吆喝")
            || act.contains("招呼")
            || act.contains("楦頭")
            || act.contains("擺攤")
            || act.contains("晴雨傘");
        if !is_vending {
            continue;
        }

        let stall_info: Option<(&str, &str, &[&str])> = if id == "wt_vendor_roast_beef" || title.contains("炭烤牛肉番薯攤") {
            Some(("wt_stall_roast_beef", "炭烤牛肉攤", &["Look", "Taste", "Use", "Smell"]))
        } else if id == "wt_vendor_noodle_soup" || title.contains("切仔麵攤") {
            Some(("wt_stall_noodle_soup", "大骨切仔麵攤", &["Look", "Taste", "Use", "Smell"]))
        } else if id == "wt_vendor_pepper_bun" || title.contains("胡椒餅攤") {
            Some(("wt_stall_pepper_bun", "炭烤胡椒餅攤", &["Look", "Taste", "Use", "Smell"]))
        } else if id == "wt_vendor_herbal_tea" || title.contains("仙草凍攤") || title.contains("烏梅湯攤") || title.contains("酸梅湯攤") {
            Some(("wt_stall_herbal_tea", "冰鎮仙草凍攤", &["Look", "Taste", "Use", "Smell"]))
        } else if id == "wt_vendor_tofu_pudding" || title.contains("豆花攤") {
            Some(("wt_stall_tofu_pudding", "黑糖豆花攤", &["Look", "Taste", "Use", "Smell"]))
        } else if id == "wt_vendor_rice_roll" || title.contains("飯糰早點攤") {
            Some(("wt_stall_rice_roll", "紫米飯糰攤", &["Look", "Taste", "Use", "Smell"]))
        } else if id == "wt_vendor_soy_milk" || title.contains("米漿杏仁茶攤") {
            Some(("wt_stall_soy_milk", "現磨米漿攤", &["Look", "Taste", "Use", "Smell"]))
        } else if id == "wt_vendor_morning_produce" || title.contains("蔬果攤") {
            Some(("wt_stall_morning_produce", "時令蔬果攤", &["Look", "Taste", "Use", "Smell"]))
        } else if id == "wt_vendor_morning_fish" || title.contains("河鮮蛤蜊攤") {
            Some(("wt_stall_morning_fish", "三水河鮮攤", &["Look", "Taste", "Use", "Smell"]))
        } else if id == "wt_npc_ma_buxue"
            || (title.contains("修傘補靴") && (act.contains("楦頭") || act.contains("晴雨傘") || act.contains("修補攤")))
        {
            Some(("wt_stall_umbrella_boot_repair", "修傘補靴攤", &["Look", "Use", "Operate"]))
        } else {
            None
        };

        if let Some((s_id, s_name, actions)) = stall_info {
            view_objects.push(crate::server::ViewObject {
                id: s_id.to_string(),
                name: s_name.to_string(),
                object_type: "object".to_string(),
                actions: actions.iter().map(|s| s.to_string()).collect(),
            });
        }
    }
    view_objects
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_labels_cover_m3_027_scenes() {
        assert_eq!(public_person_label_for_room("半嶺水岸渡口"), Some("候渡者"));
        assert_eq!(public_person_label_for_room("水尾寮渡頭市集"), Some("看貨人"));
        assert_eq!(public_person_label_for_room("船戶作業點"), Some("渡工"));
        assert_eq!(public_person_label_for_room("倉棚前場"), Some("搬貨人"));
    }

    #[test]
    fn public_labels_cover_m3_032_scenes() {
        assert_eq!(public_person_label_for_room("水尾寮水網口"), Some("引船人"));
        assert_eq!(public_person_label_for_room("水網入口"), Some("引船人"));
        assert_eq!(public_person_label_for_room("湖港木棧"), Some("候船者"));
        assert_eq!(public_person_label_for_room("短船泊位"), Some("船戶"));
        assert_eq!(public_person_label_for_room("魚貨分揀場"), Some("分揀工"));
        assert_eq!(public_person_label_for_room("蘆草貨棚"), Some("採集者"));
        assert_eq!(public_person_label_for_room("水倉前場"), Some("倉管"));
    }

    #[test]
    fn public_labels_cover_m3_045_scenes() {
        assert_eq!(public_person_label_for_room("糧倉外秤"), Some("秤手"));
        assert_eq!(public_person_label_for_room("倉門前場"), Some("守倉人"));
        assert_eq!(public_person_label_for_room("曬場邊"), Some("翻穀人"));
        assert_eq!(public_person_label_for_room("車行口"), Some("車夫"));
        assert_eq!(public_person_label_for_room("守倉棚"), Some("倉管"));
        assert_eq!(public_person_label_for_room("糧車泊位"), Some("搬糧人"));
    }

    #[test]
    fn public_labels_cover_m3_048_scenes() {
        assert_eq!(public_person_label_for_room("河口潮泥碼頭"), Some("碼頭工"));
        assert_eq!(public_person_label_for_room("稅站棚"), Some("稅吏"));
        assert_eq!(public_person_label_for_room("船行門面"), Some("船行人"));
        assert_eq!(public_person_label_for_room("潮線貨場"), Some("搬貨人"));
        assert_eq!(public_person_label_for_room("風舶角港倉門"), Some("倉管"));
        assert_eq!(public_person_label_for_room("港倉門"), Some("倉管"));
        assert_eq!(public_person_label_for_room("商船候泊線"), Some("船員"));
    }

    #[test]
    fn public_labels_cover_m3_050_scenes() {
        assert_eq!(public_person_label_for_room("東南船塢口"), Some("船匠"));
        assert_eq!(public_person_label_for_room("修船架"), Some("修船工"));
        assert_eq!(public_person_label_for_room("補給倉"), Some("倉管"));
        assert_eq!(public_person_label_for_room("碼頭木階"), Some("船客"));
        assert_eq!(public_person_label_for_room("帆布曬場"), Some("補帆人"));
        assert_eq!(public_person_label_for_room("船戶街口"), Some("船戶"));
    }

    #[test]
    fn public_labels_cover_m3_053_scenes() {
        assert_eq!(public_person_label_for_room("外貿碼頭"), Some("船員"));
        assert_eq!(public_person_label_for_room("海關前棚"), Some("海關吏"));
        assert_eq!(public_person_label_for_room("商館街口"), Some("商館人"));
        assert_eq!(public_person_label_for_room("風舶角外洋棧倉"), Some("倉管"));
        assert_eq!(public_person_label_for_room("外洋棧倉"), Some("倉管"));
        assert_eq!(public_person_label_for_room("翻譯桌"), Some("翻譯"));
        assert_eq!(public_person_label_for_room("匯兌小屋"), Some("兌票人"));
    }

    #[test]
    fn public_labels_cover_m3_006_scenes() {
        assert_eq!(public_person_label_for_room("風舶角避風港"), Some("船戶"));
        assert_eq!(public_person_label_for_room("避風港"), Some("船戶"));
        assert_eq!(public_person_label_for_room("港灣貨棚"), Some("搬魚人"));
        assert_eq!(public_person_label_for_room("漁網曬場"), Some("補網人"));
        assert_eq!(public_person_label_for_room("冷霧魚市"), Some("魚販"));
        assert_eq!(public_person_label_for_room("船戶屋排"), Some("船戶家屬"));
        assert_eq!(public_person_label_for_room("烏石鼻守望石階"), Some("守望人"));
        assert_eq!(public_person_label_for_room("守望石階"), Some("守望人"));
    }

    #[test]
    fn public_labels_cover_m3_009_scenes() {
        assert_eq!(public_person_label_for_room("礦坑外棚"), Some("礦工"));
        assert_eq!(public_person_label_for_room("洗礦溝"), Some("洗礦工"));
        assert_eq!(public_person_label_for_room("粗料倉"), Some("倉管"));
        assert_eq!(public_person_label_for_room("工具修棚"), Some("修具工"));
        assert_eq!(public_person_label_for_room("礦渣坡"), Some("推車人"));
        assert_eq!(public_person_label_for_room("紅土崁選礦車場"), Some("車夫"));
        assert_eq!(public_person_label_for_room("選礦車場"), Some("車夫"));
    }

    #[test]
    fn public_labels_cover_final_map_stub_replacement_scenes() {
        for (room, label) in [
            ("文曜鎮口", "守門人"),
            ("文曜東市街", "看貨人"),
            ("文曜旅社前廊", "接待員"),
            ("文曜講堂前坪", "傳話人"),
            ("文曜井邊巷", "取水人"),
            ("文曜百工坊", "學徒"),
            ("文曜車馬行", "車夫"),
            ("文曜倉場門口", "倉管"),
            ("南城外驛道", "驛夫"),
            ("南城甕門", "門卒"),
            ("門內集散廣場", "引路人"),
            ("中軸大街南段", "攤主"),
            ("中軸大街中段", "巡街人"),
            ("外區坊門", "坊門看守"),
            ("三水總商行長廊", "商會書手"),
            ("大十字通衢", "指路人"),
            ("城邦外坡道", "牽獸人"),
            ("石壘門", "門衛"),
            ("門內券道", "引路人"),
            ("旗石廣場", "旗手"),
            ("西門坡道街", "坡街販"),
            ("議棚前階", "記牌人"),
            ("鑄幣巷口", "兌牌人"),
            ("傭工棚邊", "棚頭人"),
            ("邊城外門", "守門人"),
            ("哨樓梯", "哨兵"),
            ("青籠關邊貿坪", "商旅"),
            ("糧倉前場", "倉管"),
            ("乾草棚", "馬夫"),
            ("巡邏院", "巡哨"),
            ("工坊街口", "學徒"),
            ("鍛造棚", "鐵匠"),
            ("料場秤台", "秤手"),
            ("修車棚", "修車工"),
            ("石粉堆", "搬料人"),
            ("油布庫", "看庫人"),
            ("牧地柵門", "牧人"),
            ("水槽棚", "牧童"),
            ("乾草堆", "搬草人"),
            ("畜欄口", "牲口販"),
            ("獸醫棚", "獸醫"),
            ("季節草場", "牧人"),
            ("牧場門牌", "牧場管事"),
            ("馬欄前場", "馬夫"),
            ("草料棚", "草料工"),
            ("獸醫點", "獸醫"),
            ("馱具修棚", "修具工"),
            ("商隊泊位", "商隊人"),
            ("邊鎮井口", "取水人"),
            ("低牆門洞", "守門人"),
            ("旅社前廊", "接待員"),
            ("補給市集", "小販"),
            ("巡哨棚", "巡哨"),
            ("車隊場", "車夫"),
        ] {
            assert_eq!(public_person_label_for_room(room), Some(label), "{room}");
            assert!(is_final_map_public_role(label), "{label}");
        }
    }

    #[test]
    fn m3_020_scene_objects_are_clickable_player_facing_surface() {
        for (room, expected_name, expected_fragment, expected_action) in [
            ("石壘門", "通行牌", "商隊", "Read"),
            ("旗石廣場", "等候線", "西門坡道街", "Use"),
            ("鑄幣巷口", "半開鑄幣門", "銅秤", "Open"),
            ("傭工棚邊", "短工牌", "活計", "Read"),
            ("議棚前階", "議棚告示", "欠糧", "Read"),
        ] {
            let objects = player_layer_scene_objects_for_room(room);
            assert_eq!(objects.len(), 1, "{room}");
            let obj = &objects[0];
            assert_eq!(obj.name, expected_name, "{room}");
            assert!(obj.sockets.iter().any(|socket| socket == "Look"), "{room}");
            assert!(
                obj.sockets.iter().any(|socket| socket == expected_action),
                "{room}: {expected_action}"
            );
            let look = obj.responses.get("Look").cloned().unwrap_or_default();
            assert!(look.contains(expected_fragment), "{room}: {look}");
            let followup = obj
                .responses
                .get(expected_action)
                .cloned()
                .unwrap_or_default();
            assert!(!followup.is_empty(), "{room}: {expected_action}");
            for text in [look, followup] {
                for forbidden in ["debug", "資料庫", "database", "grid", "scene_stub", "還有"]
                {
                    assert!(!text.contains(forbidden), "{room}: {forbidden}");
                }
            }
        }
        assert!(player_layer_scene_objects_for_room("城邦外坡道").is_empty());
    }

    #[test]
    fn north_bank_objects_are_playable_state_surfaces() {
        let objects = player_layer_scene_objects_for_room("半嶺水岸渡口");
        let expected = [
            ("河霧蘆叢", "Gather", "蘆叢"),
            ("濕長凳", "Rest", "包袱"),
            ("熱湯攤", "BuySoup", "熱氣"),
            ("熱湯攤", "QueueBroth", "熱氣"),
            ("貨擔", "CarryCargo", "踏板"),
            ("船繩", "HelpRope", "木樁"),
            ("避風棚", "DryOff", "舊帆布"),
            ("炭火盆", "WarmHands", "煙氣"),
            ("候渡隊列", "TalkQueue", "船期"),
            ("候渡隊列", "QueueFerry", "木牌"),
        ];

        for (name, action, look_fragment) in expected {
            let obj = objects
                .iter()
                .find(|obj| obj.name == name)
                .unwrap_or_else(|| panic!("missing north bank object: {name}"));
            assert!(obj.sockets.iter().any(|socket| socket == "Look"), "{name}");
            assert!(obj.sockets.iter().any(|socket| socket == action), "{name}");
            let look = obj.responses.get("Look").cloned().unwrap_or_default();
            assert!(look.contains(look_fragment), "{name}: {look}");
            let action_text = obj.responses.get(action).cloned().unwrap_or_default();
            assert!(!action_text.is_empty(), "{name}: {action}");
            for text in [look, action_text] {
                for forbidden in ["debug", "資料庫", "database", "grid", "scene_stub", "還有"]
                {
                    assert!(!text.contains(forbidden), "{name}: {forbidden}");
                }
            }
        }
    }

    #[test]
    fn north_bank_first_screen_objects_cover_m3_028_start_paths() {
        let objects = player_layer_scene_objects_for_room("半嶺水岸渡口");
        let has_action = |name: &str, action: &str| {
            objects
                .iter()
                .any(|obj| obj.name == name && obj.sockets.iter().any(|socket| socket == action))
        };

        assert!(has_action("貨擔", "CarryCargo")); // 賺錢 / 接活
        assert!(has_action("候渡隊列", "QueueFerry")); // 通行
        assert!(has_action("河霧蘆叢", "Gather")); // 材料 / 交易準備
        assert!(has_action("船繩", "HelpRope")); // 工作手勢 / 熟練線索
        assert!(has_action("熱湯攤", "QueueBroth") || has_action("濕長凳", "Rest")); // 歇腳處理

        let rope = objects
            .iter()
            .find(|obj| obj.name == "船繩")
            .expect("船繩應在北岸第一屏");
        let rope_look = rope.responses.get("Look").cloned().unwrap_or_default();
        let rope_action = rope.responses.get("HelpRope").cloned().unwrap_or_default();
        assert!(rope_look.contains("手勢"));
        assert!(rope_action.contains("記住"));
    }

    #[test]
    fn m3_027_cross_scene_objects_are_playable_state_surfaces() {
        for (room, expected_name, expected_action, look_fragment) in [
            ("水尾寮渡頭市集", "舊麻袋堆", "Gather", "包貨"),
            ("水尾寮渡頭市集", "市集包貨單", "SellReeds", "蘆草"),
            ("水尾寮渡頭市集", "市集包貨單", "SellSackcloth", "麻袋布"),
            ("水尾寮渡頭市集", "市集包貨單", "SellRope", "麻繩"),
            ("渡船踏板", "濕繩圈", "Gather", "捆貨"),
            ("渡船踏板", "踏板木牌", "BoardFerry", "排到牌內側"),
            ("南岸泥坡", "上岸繩", "StepAshore", "泥坡木樁"),
            ("倉棚前場", "倉棚點貨牌", "CheckCargo", "已送到踏板邊"),
        ] {
            let objects = player_layer_scene_objects_for_room(room);
            let obj = objects
                .iter()
                .find(|obj| obj.name == expected_name)
                .unwrap_or_else(|| panic!("missing M3-027 object: {room} / {expected_name}"));
            assert!(obj.sockets.iter().any(|socket| socket == "Look"), "{room}");
            assert!(
                obj.sockets.iter().any(|socket| socket == expected_action),
                "{room}: {expected_action}"
            );
            let look = obj.responses.get("Look").cloned().unwrap_or_default();
            assert!(look.contains(look_fragment), "{room}: {look}");
            let action_text = obj
                .responses
                .get(expected_action)
                .cloned()
                .unwrap_or_default();
            assert!(!action_text.is_empty(), "{room}: {expected_action}");
            for text in [look, action_text] {
                for forbidden in ["debug", "資料庫", "database", "grid", "scene_stub", "還有"]
                {
                    assert!(!text.contains(forbidden), "{room}: {forbidden}");
                }
            }
        }
    }

    #[test]
    fn micro_lines_do_not_use_private_names() {
        for room in [
            "半嶺水岸渡口",
            "文曜鎮口",
            "文曜東市街",
            "文曜旅社前廊",
            "文曜講堂前坪",
            "文曜井邊巷",
            "文曜百工坊",
            "文曜車馬行",
            "文曜倉場門口",
            "南城外驛道",
            "南城甕門",
            "門內集散廣場",
            "中軸大街南段",
            "中軸大街中段",
            "外區坊門",
            "三水總商行長廊",
            "大十字通衢",
            "城邦外坡道",
            "石壘門",
            "門內券道",
            "旗石廣場",
            "西門坡道街",
            "議棚前階",
            "鑄幣巷口",
            "傭工棚邊",
            "水尾寮水網口",
            "魚貨分揀場",
            "水倉前場",
            "糧倉外秤",
            "守倉棚",
            "糧車泊位",
            "河口潮泥碼頭",
            "稅站棚",
            "商船候泊線",
            "東南船塢口",
            "修船架",
            "帆布曬場",
            "外貿碼頭",
            "海關前棚",
            "商館街口",
            "翻譯桌",
            "匯兌小屋",
            "風舶角外洋棧倉",
            "風舶角港倉門",
            "風舶角避風港",
            "港灣貨棚",
            "漁網曬場",
            "冷霧魚市",
            "船戶屋排",
            "烏石鼻守望石階",
            "礦坑外棚",
            "洗礦溝",
            "粗料倉",
            "工具修棚",
            "礦渣坡",
            "紅土崁選礦車場",
            "邊城外門",
            "哨樓梯",
            "青籠關邊貿坪",
            "糧倉前場",
            "乾草棚",
            "巡邏院",
            "工坊街口",
            "鍛造棚",
            "料場秤台",
            "修車棚",
            "石粉堆",
            "油布庫",
            "牧地柵門",
            "水槽棚",
            "乾草堆",
            "畜欄口",
            "獸醫棚",
            "季節草場",
            "牧場門牌",
            "馬欄前場",
            "草料棚",
            "獸醫點",
            "馱具修棚",
            "商隊泊位",
            "邊鎮井口",
            "低牆門洞",
            "旅社前廊",
            "客棧前廊",
            "補給市集",
            "巡哨棚",
            "車隊場",
        ] {
            let lines = micro_interaction_lines_for_room(room);
            assert!(lines.iter().all(|line| !line.contains('【')), "{room}");
        }
    }
    #[test]
    fn white_tower_peninsula_rooms_have_rich_playable_objects() {
        let sample_rooms = [
            "文曜・三水大食堂",
            "文曜・泉香溫水浴場",
            "文曜・東橋旅社",
            "文曜・三水口石砌住宅",
            "文曜・雲錦織造與服飾坊",
            "文曜・巧木居所傢俱坊",
            "文曜・岑氏第一鍛造工坊",
            "文曜・藏書閣與風物誌編纂處",
            "文曜・講堂街書局",
            "文曜・學院主校區",
            "文曜・溫氏脈絡診所",
            "老堰頭・水輪石磨工坊",
            "霜林口・原木水車鋸木坊",
            "半嶺鎮・寒水玉切石工坊",
            "半嶺鎮・半嶺氣象觀測所",
            "半嶺鎮・半嶺試招場",
            "大圳頭・西南原農具與百工坊",
            "牛埔・羊毛與乳酪精製坊",
            "紅土崁・紅土崁選礦場",
            "三號坑口・坑口台車維修棚",
            "風櫃尾・風櫃尾燈塔",
            "風櫃尾・水產冷燻工坊",
            "風櫃尾・風櫃尾巨藻晾曬場",
            "烏石鼻・烏石鼻海貝灰窯",
            "南哨・重型獵具工坊",
            "水尾寮・蘆葦與竹編工棚",
            "風舶角・林氏大船塢",
            "文曜・三水總商行",
            "文曜・西門車馬聯運總站",
            "文曜・傳訊鐘樓與信使總站",
            "風舶角・深水泊位與卸貨碼頭",
            "大圳頭・農產與糧油市集",
            "文曜・學院評議會廳",
            "老堰頭・農產公議所",
            "文曜・市街石道",
            "文曜・中心樞紐橋",
            "風舶嶼・木柵圍欄",
            "老堰頭・水渠畔石砌農舍",
            "烏石鼻・玄武岩堆砌漁舍",
            "水尾寮・水尾寮高腳木屋",
            "大圳頭・周家莊園",
            "半嶺鎮・岑家二房別院",
            "文曜・三水口・陸家宅邸",
        ];

        for room in sample_rooms {
            let objs = player_layer_scene_objects_for_room(room);
            assert!(
                objs.len() >= 3,
                "Room {room} should have at least 3 playable objects, got {}",
                objs.len()
            );
            for obj in &objs {
                assert!(obj.sockets.contains(&"Look".to_string()), "Object {} missing Look socket", obj.id);
                assert!(obj.sockets.len() >= 2, "Object {} should have at least 2 sockets", obj.id);
                for socket in &obj.sockets {
                    let resp = obj.responses.get(socket);
                    assert!(resp.is_some(), "Object {} missing response for socket {socket}", obj.id);
                    let resp_text = resp.unwrap();
                    assert!(!resp_text.is_empty(), "Response for socket {socket} in {} is empty", obj.id);
                    for forbidden in ["debug", "資料庫", "database", "grid", "scene_stub", "還有"] {
                        assert!(
                            !resp_text.contains(forbidden),
                            "Object {} response contains forbidden word {forbidden}",
                            obj.id
                        );
                    }
                }
            }
        }

        // Verify non-peninsula room remains empty to preserve existing test invariants
        assert!(player_layer_scene_objects_for_room("城邦外坡道").is_empty());
        assert!(
            crate::game::peninsula_catalog::PENINSULA_WORLD_OBJECTS.len() >= 1100,
            "Peninsula world catalog should have >= 1100 objects, got {}",
            crate::game::peninsula_catalog::PENINSULA_WORLD_OBJECTS.len()
        );
        for entry in crate::game::peninsula_catalog::PENINSULA_WORLD_OBJECTS {
            for forbidden in ["debug", "資料庫", "database", "grid", "scene_stub", "還有"] {
                assert!(!entry.look_text.contains(forbidden));
                assert!(!entry.action_text.contains(forbidden));
            }
        }
    }

    #[test]
    fn wenyao_outer_ward_objects_and_residential_classification() {
        // 1. Residential classification
        let residences = [
            "文曜・講堂外坊・修竹讀書山房",
            "講堂外坊・刻書雕版匠居",
            "文曜・三水外坊・蘆洲漁家小築",
            "三水外坊・老舵手石砌院",
            "文曜・百工外坊・司爐老鐵匠院",
            "百工外坊・天工安居大院",
            "文曜・西門外坊・老鏢頭四合院",
            "西門外坊・駝隊領隊家宅",
        ];
        for r in residences {
            assert!(
                is_wenyao_residential_or_study(r),
                "Should classify {} as residential",
                r
            );
        }

        // 2. Commercial / Academic / Craft exclusions
        let commercial = [
            "文曜・講堂外坊・格物輿圖外院",
            "講堂外坊・活字排版印書局",
            "文曜・三水外坊・北灣貨船大埠",
            "三水外坊・早市鮮魚拍賣場",
            "文曜・百工外坊・重鐵水錘大鍛坊",
            "百工外坊・五金農具總商行",
            "文曜・西門外坊・萬里駝隊大驛站",
            "西門外坊・西南原皮毛大市",
        ];
        for c in commercial {
            assert!(
                !is_wenyao_residential_or_study(c),
                "Should NOT classify {} as residential",
                c
            );
        }

        // 3. Inner Gate Plazas authentic objects and NO bath amenities
        let gate_plazas = ["文曜・北內門坊", "文曜・南內門坊", "文曜・東內門坊", "文曜・西內門坊"];
        for gate in gate_plazas {
            let objs = player_layer_scene_objects_for_room(gate);
            assert!(
                objs.iter().any(|o| o.id == "wt_gate_stone_archway_board"),
                "Gate plaza {} must have stone archway board",
                gate
            );
            assert!(
                !objs.iter().any(|o| o.id == "wt_reg_wenyao_amenities"),
                "Gate plaza {} must NEVER have indoor bath sink",
                gate
            );
            let reg_obj = regional_specialty_object_for_room(gate).expect("gate plaza must have regional obj");
            assert_eq!(reg_obj.id, "wt_reg_wenyao_gate_plaza");
        }

        // 4. Outer landmarks have specialized objects
        let nw_wharf = player_layer_scene_objects_for_room("文曜・三水外坊・北灣貨船大埠");
        assert!(nw_wharf.iter().any(|o| o.id == "wt_obj_wharf_bollard"));

        let ne_academy = player_layer_scene_objects_for_room("文曜・講堂外坊・格物輿圖外院");
        assert!(ne_academy.iter().any(|o| o.id == "wt_obj_cartography_table"));

        let se_forge = player_layer_scene_objects_for_room("文曜・百工外坊・重鐵水錘大鍛坊");
        assert!(se_forge.iter().any(|o| o.id == "wt_obj_trip_hammer_trough"));

        let sw_inn = player_layer_scene_objects_for_room("文曜・西門外坊・萬里駝隊大驛站");
        assert!(sw_inn.iter().any(|o| o.id == "wt_obj_caravan_hitch_trough"));
    }

    #[test]
    fn test_dynamic_vendor_objects_for_entities() {
        use crate::entity::{Character, EntityKind, Gender, MoveState, WalkOrRun};

        let mut vendor = Character {
            id: "wt_vendor_roast_beef".to_string(),
            kind: EntityKind::Npc,
            display_char: String::new(),
            x: 0,
            y: 0,
            move_state: MoveState::Idle,
            target_x: None,
            target_y: None,
            walk_or_run: WalkOrRun::Walk,
            move_started_at: None,
            vit: 10,
            qi: 10,
            dex: 10,
            magnesium: 0,
            last_observed_at: None,
            created_at: 0,
            gender: Some(Gender::M),
            soul_seed: None,
            display_title: "炭烤牛肉番薯攤老闆|鄭火旺".to_string(),
            activated_nodes: "[]".to_string(),
            equipment_slots: String::new(),
            inventory: "[]".to_string(),
            disposition: 0,
            thoughts: vec![],
            current_activity: "正在炭烤牛肉番薯攤前翻動原牛肉串，高聲叫賣".to_string(),
            grid_x: None,
            grid_y: None,
        };

        // 1. 營業中的攤販應動態生成攤位物件
        let objects = dynamic_vendor_objects_for_entities(&[vendor.clone()]);
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].id, "wt_stall_roast_beef");
        assert_eq!(objects[0].name, "炭烤牛肉攤");
        assert!(objects[0].sockets.contains(&"Look".to_string()));
        assert!(objects[0].sockets.contains(&"Taste".to_string()));
        assert!(objects[0].sockets.contains(&"Smell".to_string()));

        // 2. 移動中或睡眠中的攤販不應生成攤位物件
        vendor.current_activity = "正在前往三水口南街".to_string();
        assert!(dynamic_vendor_objects_for_entities(&[vendor.clone()]).is_empty());

        vendor.current_activity = "在屋內安歇".to_string();
        assert!(dynamic_vendor_objects_for_entities(&[vendor]).is_empty());

        // 3. 一般街道不應含有固定的炭烤牛肉番薯攤
        let street_objs = player_layer_scene_objects_for_room("三水口南街");
        assert!(!street_objs.iter().any(|o| o.id == "wt_street_roast_snack_cart" || o.name == "炭烤牛肉攤"));
    }
}
