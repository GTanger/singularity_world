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
        look: "等候線用舊旗和石縫分出幾股，往西軸坡街、鑄幣巷口和傭工棚邊的人各站一側；線尾仍有人低聲問該排哪裡。",
        followup_action: "Use",
        followup: "你順著等候線往旁邊讓出一步，旗手立刻把鑄幣巷口和傭工棚邊的人分得更開，廣場中間的推擠短短鬆了一點。",
    },
    SceneObjectText {
        room: "鑄幣巷口",
        id: "m3_020_mint_half_open_door",
        name: "半開鑄幣門",
        look: "半開的門縫裡能看見銅秤、封袋和一點碎銀反光；兌牌人把門縫守得很窄，只讓被叫到的人靠近門檻。",
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
        room: "北岸渡口",
        id: "m3_027_north_bank_reeds",
        name: "河霧蘆叢",
        look: "蘆叢貼著北岸水線長開，霧水壓在葉尖上；短市包貨單催著蘆草墊箱，靠近踏板的一側已有被割過的斜口，濕蘆草仍壓在泥邊等人伸手取下。",
        actions: &[(
            "Gather",
            "你撥開貼岸的河霧，從濕蘆裡割下一束能用的蘆草；斷口上的水珠沿著葉脈落回泥裡。",
        )],
    },
    LifeSceneObjectText {
        room: "北岸渡口",
        id: "m3_027_wet_bench",
        name: "濕長凳",
        look: "長凳靠河的一半沾著霧水，乾的一端被兩只包袱佔住；有個候渡者把身子挪開半寸，像是在等人決定坐不坐。",
        actions: &[(
            "Rest",
            "你把包袱挪到濕痕外坐下，肩背從挑擔聲裡鬆下來；凳腳晃了一下，旁邊人把自己的包袱往回收。",
        )],
    },
    LifeSceneObjectText {
        room: "北岸渡口",
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
        room: "北岸渡口",
        id: "m3_027_cargo_pole",
        name: "貨擔",
        look: "兩副貨擔斜靠在泥地上，麻袋底拖出深痕；短市的人把幾枚鎂壓在手心裡，看誰願意先把它們送到踏板邊。",
        actions: &[(
            "CarryCargo",
            "你把一副貨擔扛到踏板邊，肩上立刻壓出酸意；短市的人塞給你幾枚鎂，船戶看見路口清出一線，朝你短短點頭。",
        )],
    },
    LifeSceneObjectText {
        room: "北岸渡口",
        id: "m3_027_boat_rope",
        name: "船繩",
        look: "濕船繩繃在木樁和船腹之間，繩股裡卡著泥水；船戶每看一眼河面，就把手掌往繩上壓一下，只有看得懂收放手勢的人才靠近搭手。",
        actions: &[(
            "HelpRope",
            "船戶把繩尾遞給你，你跟著他一收一放，記住船腹貼岸前那一下手勢；船身少晃了一下，踏板前的人群跟著讓出一條窄縫。",
        )],
    },
    LifeSceneObjectText {
        room: "北岸渡口",
        id: "m3_027_river_awning",
        name: "避風棚",
        look: "一片舊帆布斜斜壓在木桿上，棚下沒有好座位，只能貼著貨箱站一會兒，把衣角的冷水慢慢晾開。",
        actions: &[(
            "DryOff",
            "你站到舊帆布下，把濕衣角攤在貨箱邊；河風吹不太進來，肩背雖還沉，手腳先緩過一點。",
        )],
    },
    LifeSceneObjectText {
        room: "北岸渡口",
        id: "m3_027_charcoal_brazier",
        name: "炭火盆",
        look: "炭火盆被幾個候渡者圍在中間，火苗不大，煙氣貼著袖口走；靠近的人都只能烘一小會兒便讓位。",
        actions: &[(
            "WarmHands",
            "你把手伸到炭火盆旁，煙味鑽進袖口，指節慢慢有了知覺；旁邊的人給你讓出半掌寬的位置。",
        )],
    },
    LifeSceneObjectText {
        room: "北岸渡口",
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
        room: "岸邊短市",
        id: "m3_027_market_sackcloth_pile",
        name: "舊麻袋堆",
        look: "舊麻袋堆貼著短市棚腳攤開，破口和好布被分成兩邊；搬貨人只剪能再包貨的布片，剩下太濕太爛的麻線被踩進泥裡。",
        actions: &[(
            "Gather",
            "你從棚腳舊麻袋裡剪下一卷還能包貨的麻袋布；旁邊搬貨人照同一堆料挑出幾片，塞到待包的木箱旁。",
        )],
    },
    LifeSceneObjectText {
        room: "岸邊短市",
        id: "m3_027_market_reed_buyer",
        name: "短市包貨單",
        look: "短市包貨單壓在秤邊濕木板上，三行短字分別催蘆草墊箱、麻袋布包貨、麻繩捆箱；看貨人和搬貨人都照這張單子抽料，不讓箱包堵住踏板口。",
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
            "你指給記牌人看剛才清到踏板邊的貨擔痕，對方把那一行劃進倉棚短冊，又補給你一枚尾款鎂。",
        )],
    },
];

const FINAL_MAP_SCENE_TEXTS: &[PolishedSceneText] = &[
    PolishedSceneText {
        name: "中鎮鎮口",
        role: "守門人",
        crowd: "鎮口人群",
        entry: "你走到中鎮鎮口。鎮牌樓把谷地路收進陰影裡，守門人讓候檢車馬慢下來，市集街和倉場方向的人聲從門內分開。",
        reentry: "你回到中鎮鎮口。路旗下又聚起一小段候檢腳步，守門人把剛放行的車馬引向市集街那邊。",
        observation: "{pronoun}按著門邊路牌，抬手讓一輛車先停在牌樓陰影下。",
        micro: &[
            "守門人把路牌往牌樓下挪了半步，候檢的人聲立刻低下去。",
            "有人把車轅拉正，等市集街那頭讓出一段路再進鎮。",
            "鎮口短短響起木牌碰柱聲，谷地路的腳步被門影吞進去。",
        ],
    },
    PolishedSceneText {
        name: "中鎮市集街",
        role: "看貨人",
        crowd: "市集人群",
        entry: "你走進中鎮市集街。布棚沿街排開，看貨人把價碼壓在攤布下，井邊巷、客棧廊和倉場門口的人流在街心交會。",
        reentry: "你回到中鎮市集街。攤布前換了一撥看貨人，剛從倉場門口送來的箱包被推到街側等價。",
        observation: "{pronoun}掀起攤布一角看貨色，低聲把問到的價碼傳給旁邊的人。",
        micro: &[
            "看貨人把貨樣放回籃裡，眼神仍留在倉場門口那邊的箱包上。",
            "有人在布棚下短短議價，又回頭問客棧廊前是否還有外來商隊。",
            "市集街短短響起秤錘碰木聲，井邊巷的取水聲被壓到後面。",
        ],
    },
    PolishedSceneText {
        name: "中鎮客棧廊",
        role: "店伙",
        crowd: "客棧人群",
        entry: "你走到中鎮客棧廊。長凳、熱水壺和掛包木鉤貼著廊柱排開，店伙替剛入鎮的人分辨市集、井邊和工坊街的去處。",
        reentry: "你回到中鎮客棧廊。廊下包袱換了位置，店伙把新添的熱水壺推到長凳旁。",
        observation: "{pronoun}把熱水壺放到廊桌上，側身聽市集街那邊是否還有人找宿。",
        micro: &[
            "店伙把長凳下的包袱往內推，給從市集街來的人讓出廊邊。",
            "有人在廊柱旁問今晚還有沒有空鋪，聲音很快被街上議價蓋住。",
            "客棧廊短短響起碗底碰桌聲，工坊街那邊的敲打聲靠近一點。",
        ],
    },
    PolishedSceneText {
        name: "中鎮廟前小場",
        role: "傳話人",
        crowd: "廟前人群",
        entry: "你走到中鎮廟前小場。香灰貼著石凳邊緣，告示板下有人低聲傳話，把鎮口、井邊巷和市集街的消息接在一起。",
        reentry: "你回到中鎮廟前小場。告示板旁多了幾道新摺痕，傳話人把剛聽到的鎮口消息壓低聲說出。",
        observation: "{pronoun}在告示板前停住，低聲把剛問到的事轉給等在石凳旁的人。",
        micro: &[
            "傳話人把告示紙角壓平，讓後面的人先看清日期。",
            "有人在石凳旁合掌停了一下，轉身就把消息送往井邊巷。",
            "廟前小場短短安靜了一息，市集街的人聲又從牌樓那邊湧回來。",
        ],
    },
    PolishedSceneText {
        name: "中鎮井邊巷",
        role: "取水人",
        crowd: "井邊人群",
        entry: "你走進中鎮井邊巷。井繩濕痕拖到巷口，取水人和跑腿孩子在住戶牆影下穿過，讓市集街後面有了日常內側。",
        reentry: "你回到中鎮井邊巷。石圈旁的水痕又深了一圈，取水人把剛打滿的桶讓到巷壁邊。",
        observation: "{pronoun}把水桶挪到牆影下，抬頭看工坊街那邊是否還有人要水。",
        micro: &[
            "取水人把井繩往石圈上多繞一圈，水滴沿巷口慢慢拖開。",
            "跑腿孩子抱著小桶穿過人縫，很快又被市集街的聲音吞沒。",
            "井邊巷短短響起水桶碰石聲，廟前小場的低語跟著停了一下。",
        ],
    },
    PolishedSceneText {
        name: "中鎮工坊街",
        role: "學徒",
        crowd: "工坊人群",
        entry: "你走到中鎮工坊街。木屑、鐵件和修具架排在半開門面下，學徒把市集街送來的活計分進各家。",
        reentry: "你回到中鎮工坊街。門面下又多了幾件待修工具，學徒把剛問到的料價寫到木牌邊。",
        observation: "{pronoun}抱著一捆待修件停在門面下，等裡頭讓出修具架。",
        micro: &[
            "學徒把木屑掃到門檻內側，讓井邊巷過來的人能貼牆走過。",
            "有人把斷扣和小刀放到修具架前，低聲問今天能不能趕好。",
            "工坊街短短響起敲鐵聲，客棧廊下的談話跟著壓低。",
        ],
    },
    PolishedSceneText {
        name: "中鎮車馬泊位",
        role: "車夫",
        crowd: "車馬人群",
        entry: "你走到中鎮車馬泊位。車轅和草料筐壓在鎮口南側，車夫等倉場門口放出手勢，再把渡口貨路來的車聲接進鎮裡。",
        reentry: "你回到中鎮車馬泊位。車轍又被壓亮了一截，車夫把候卸貨包往倉場方向重新排開。",
        observation: "{pronoun}扶著車轅看向倉場門口，等下一個可卸貨的手勢。",
        micro: &[
            "車夫把草料筐踢到車輪旁，讓從貨路外彎來的空車先靠邊。",
            "有人把貨包重新綁緊，抬頭等倉場門口那邊喊號。",
            "車馬泊位短短響起車軸聲，鎮口牌樓下的人群跟著讓開一段。",
        ],
    },
    PolishedSceneText {
        name: "中鎮倉場門口",
        role: "倉管",
        crowd: "倉場人群",
        entry: "你走到中鎮倉場門口。木箱、糧袋和封牌排在門檻陰影裡，倉管把車馬泊位送來的貨分成入倉、轉市與暫候三路。",
        reentry: "你回到中鎮倉場門口。封牌位置換過一輪，倉管把要送去市集街的箱包推到外側。",
        observation: "{pronoun}按著冊簿分貨，抬手讓一批箱包先停在門檻外側。",
        micro: &[
            "倉管把冊簿邊角壓平，示意車馬泊位再等一輛車。",
            "有人把糧袋從木箱旁分開，低聲問這批要進倉還是轉市。",
            "倉場門口短短響起木箱落地聲，市集街的議價聲立刻靠近。",
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
            "中軸大街南段短短響起叫賣聲，商會長廊的低語跟著靠近。",
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
        name: "商會長廊",
        role: "商會書手",
        crowd: "長廊人群",
        entry: "你走到商會長廊。木欄後擺著貨樣匣、印章盒和長桌，商會書手把外城來的貨單收進陰影裡。",
        reentry: "你回到商會長廊。長桌上的貨單換了一疊，商會書手把剛蓋好的印章推到一側風乾。",
        observation: "{pronoun}按著貨單低聲核對，偶爾抬頭看中軸大街南段的人流。",
        micro: &[
            "商會書手把印章盒合上半截，讓送單的人先站到木欄旁。",
            "有人把貨樣匣推近長桌，低聲問這批貨能不能轉進倉區。",
            "商會長廊短短響起紙角聲，中軸大街的叫賣聲被木欄隔開。",
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
        entry: "你走進旗石廣場。舊旗把等候線分成幾股，旗手把門內券道來的人流指向西軸坡街、鑄幣巷口和傭工棚邊。",
        reentry: "你回到旗石廣場。旗桿下的隊列換了方向，旗手把剛到的商隊先壓到廣場側邊。",
        observation: "{pronoun}扶著旗桿指路，讓往坡街和傭工棚的人不要互相擠住。",
        micro: &[
            "旗手把舊旗扯直，廣場上的等候線跟著重新分開。",
            "有人從鑄幣巷口退回廣場側邊，低聲問下一步該排哪列。",
            "旗石廣場短短響起旗繩聲，門內券道的人流又湧進一小段。",
        ],
    },
    PolishedSceneText {
        name: "西軸坡街",
        role: "坡街販",
        crowd: "坡街人群",
        entry: "你走上西軸坡街。攤布、鐵件和馬具沿低檐掛開，坡街販把旗石廣場來的消息壓低聲傳向議棚前階。",
        reentry: "你回到西軸坡街。坡街上換了一撥問價的人，坡街販把剛聽見的議棚消息藏進攤布下。",
        observation: "{pronoun}按住攤布一角，側耳聽議棚前階那邊是否開始叫號。",
        micro: &[
            "坡街販把一串馬具掛回低檐下，讓往議棚前階的人貼牆走過。",
            "有人在坡街中段問價，又把目光轉回旗石廣場的隊列。",
            "西軸坡街短短響起鐵件碰撞聲，更深處的城邦低語貼著坡面傳來。",
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
        entry: "你走到鑄幣巷口。半扇門內一閃而過銅秤、碎銀和封袋，兌牌人把要換票的人攔在旗石廣場側邊。",
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
        entry: "你走到邊城外門。木門半開，通行牌壓在桌邊，守門人把從舊道裝車點來的車隊攔在門影下。",
        reentry: "你回到邊城外門。門影下的商隊又往前挪了一截，守門人把通行牌重新翻到手邊。",
        observation: "{pronoun}按著通行牌，抬眼看商隊和舊道裝車點方向的車輪痕。",
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
        name: "邊市口",
        role: "商旅",
        crowd: "邊市人群",
        entry: "你走到邊市口。臨時攤布貼著低牆鋪開，商旅把小件貨和路上消息都壓低聲交出去。",
        reentry: "你回到邊市口。攤布旁換了一撥看貨的人，商旅把剛問到的價碼送往糧倉前場。",
        observation: "{pronoun}把小件貨攤開一角，低聲問城門那邊今天還放不放車隊進來。",
        micro: &[
            "商旅把包袱口解開一半，又立刻用手掌壓住攤布邊。",
            "有人在邊市口比了比價，眼角仍看著邊城外門的查驗。",
            "邊市口短短傳過一陣議價聲，很快被糧倉前場的推車聲壓低。",
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
            "糧倉前場短短響起推車木輪聲，邊市口的低語被切成幾段。",
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
            "邊鎮井口短短響起井繩聲，客棧前廊的人聲跟著靠近。",
        ],
    },
    PolishedSceneText {
        name: "低牆門洞",
        role: "守門人",
        crowd: "門洞人群",
        entry: "你走到低牆門洞。木拒馬卡在門邊，守門人把旅人的水囊和貨繩看過一遍才放人進鎮。",
        reentry: "你回到低牆門洞。拒馬旁多了車轍泥，守門人把剛收下的路牌掛回牆釘。",
        observation: "{pronoun}扶著木拒馬，看過貨繩和路牌後才讓車隊往補給小市移動。",
        micro: &[
            "守門人把木拒馬往旁邊挪開半尺，讓一輛小車慢慢過洞。",
            "有人把路牌遞到門洞陰影裡，等巡哨棚那邊回話。",
            "低牆門洞短短響起木拒馬拖地聲，井口的水聲被擋在牆外。",
        ],
    },
    PolishedSceneText {
        name: "客棧前廊",
        role: "店伙",
        crowd: "客棧人群",
        entry: "你走到客棧前廊。長凳靠著廊柱，店伙把熱水壺放到桌邊，等外路旅人卸下包袱。",
        reentry: "你回到客棧前廊。長凳上換了一撥歇腳的人，店伙把空碗收進門內。",
        observation: "{pronoun}端著熱水壺穿過長凳邊，低聲問旅人要不要先歇腳。",
        micro: &[
            "店伙把熱水壺放到廊桌上，讓剛進鎮的人先暖手。",
            "有人把包袱靠到長凳下，抬頭問補給小市今天還開到多晚。",
            "客棧前廊短短響起碗底碰桌聲，井口那邊的取水聲遠了一點。",
        ],
    },
    PolishedSceneText {
        name: "補給小市",
        role: "小販",
        crowd: "小市人群",
        entry: "你走進補給小市。鹽袋、水囊和乾糧包排在攤邊，小販把外路價碼壓低聲報給車隊場的人。",
        reentry: "你回到補給小市。攤邊水囊少了幾只，小販把新補的鹽袋挪到前排。",
        observation: "{pronoun}把水囊掛到攤前，低聲把鹽袋價碼報給等車的人。",
        micro: &[
            "小販把鹽袋拍實，攤布邊立刻落下一圈白粉。",
            "有人把水囊拿起來掂量，又回頭問車隊場還缺多少補給。",
            "補給小市短短響起乾糧包落桌聲，客棧前廊的談話停了一下。",
        ],
    },
    PolishedSceneText {
        name: "巡哨棚",
        role: "巡哨",
        crowd: "巡哨人群",
        entry: "你走到巡哨棚。槍架靠著棚壁，巡哨把值牌換到腰間，目光一直掃向低牆門洞和車隊場。",
        reentry: "你回到巡哨棚。槍架旁的值牌換了位置，巡哨把剛聽到的外路消息壓低聲傳下去。",
        observation: "{pronoun}把值牌扣回腰間，側身看低牆門洞外是否還有人滯留。",
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
        entry: "你走到車隊場。車轍在場地裡交錯，車夫把貨繩重新收緊，等補給小市把最後一袋鹽送來。",
        reentry: "你回到車隊場。貨繩被重新結過，車夫把下一輛要出低牆門洞的車推到前排。",
        observation: "{pronoun}扶著車轅檢查貨繩，回頭等巡哨棚放出下一道通行手勢。",
        micro: &[
            "車夫把貨繩往車架下收緊，輪旁泥水被踩得發亮。",
            "有人把剛買來的水囊丟上車，等補給小市補齊乾糧。",
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
    objects.extend(
        M3_027_LIFE_OBJECTS
            .iter()
            .filter(|obj| room_name.contains(obj.room))
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
    objects
}

pub fn is_final_map_public_role(s: &str) -> bool {
    FINAL_MAP_SCENE_TEXTS
        .iter()
        .any(|scene| s.contains(scene.role))
}

pub fn public_person_label_for_room(room_name: &str) -> Option<&'static str> {
    if let Some(role) = final_map_public_role_for_room(room_name) {
        Some(role)
    } else if room_name.contains("北岸") || room_name.contains("等候") {
        Some("候渡者")
    } else if room_name.contains("水網入口") {
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
    } else if room_name.contains("河海倉門") {
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
    } else if room_name.contains("遠洋倉門") {
        Some("倉管")
    } else if room_name.contains("翻譯桌") {
        Some("翻譯")
    } else if room_name.contains("匯兌小屋") {
        Some("兌票人")
    } else if room_name.contains("北灣避風碼頭") {
        Some("船戶")
    } else if room_name.contains("港灣貨棚") {
        Some("搬魚人")
    } else if room_name.contains("漁網曬場") {
        Some("補網人")
    } else if room_name.contains("冷霧魚市") {
        Some("魚販")
    } else if room_name.contains("船戶屋排") {
        Some("船戶家屬")
    } else if room_name.contains("北灣守望石階") {
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
    } else if room_name.contains("舊道裝車點") {
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
    } else if room_name.contains("短市") {
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

pub fn micro_interaction_lines_for_room(room_name: &str) -> &'static [&'static str] {
    if let Some(scene) = final_map_scene_for_room(room_name) {
        scene.micro
    } else if room_name.contains("北岸") || room_name.contains("等候") {
        &[
            "兩名候渡者低聲確認船班，很快又把聲音壓進人群裡。",
            "有人替旁邊的候渡者扶了一下包袱，目光仍留在渡船踏板那頭。",
            "候渡的人群短短讓出一條縫，又隨船繩聲重新擠回原處。",
        ]
    } else if room_name.contains("水網入口") {
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
    } else if room_name.contains("河海倉門") {
        &[
            "倉管掀起雨布一角，讓剛放行的箱包先壓到倉門內側。",
            "有人把封牌扶正，低聲把河貨和海貨分到不同門邊。",
            "河海倉門裡短短傳出木箱靠牆的悶響，雨布又垂回來。",
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
    } else if room_name.contains("遠洋倉門") {
        &[
            "倉管把異地木箱推到雨布內側，低聲分出已查和待查兩堆。",
            "有人擦過箱面漆印，確認貨名後才讓拖箱聲進到門內。",
            "遠洋倉門短短響起木箱靠牆的悶聲，雨布又被海風吹落。",
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
            "有人把商館街口報來的價碼寫入欄裡，又抬頭問遠洋倉門的貨名。",
            "匯兌小屋短短響起銅秤輕碰聲，等價的人把聲音壓得更低。",
        ]
    } else if room_name.contains("北灣避風碼頭") {
        &[
            "船戶把結霜纜繩在木樁上多繞一圈，避風船在內側輕輕碰岸。",
            "有人抬手看了一眼北風，示意後面的小船先貼著碼頭等。",
            "避風碼頭短短響起船腹碰木聲，港灣貨棚那邊的人聲跟著收低。",
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
    } else if room_name.contains("北灣守望石階") {
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
            "粗料倉短短響起礦袋落地聲，舊道裝車點的車輪聲接著傳來。",
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
    } else if room_name.contains("舊道裝車點") {
        &[
            "車夫把繩索掛上車架，稱重木牌在灰粉裡露出半行字。",
            "有人把礦袋推上礦車，車輪壓過舊道口磨亮的車轍。",
            "舊道裝車點短短響起車軸聲，邊城外門那頭有人抬頭張望。",
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
    } else if room_name.contains("短市") {
        &[
            "兩個看貨的人在價牌前比了比手勢，攤邊很快響起壓低的議價聲。",
            "有人把小包短貨遞給同伴掂量，眼角仍瞄著河邊的渡船。",
            "短市裡一陣貨聲掠過，幾個人同時朝下一班船的方向看去。",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_labels_cover_m3_027_scenes() {
        assert_eq!(public_person_label_for_room("北岸渡口"), Some("候渡者"));
        assert_eq!(public_person_label_for_room("岸邊短市"), Some("看貨人"));
        assert_eq!(public_person_label_for_room("船戶作業點"), Some("渡工"));
        assert_eq!(public_person_label_for_room("倉棚前場"), Some("搬貨人"));
    }

    #[test]
    fn public_labels_cover_m3_032_scenes() {
        assert_eq!(public_person_label_for_room("水網入口棧"), Some("引船人"));
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
        assert_eq!(public_person_label_for_room("河海倉門"), Some("倉管"));
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
        assert_eq!(public_person_label_for_room("遠洋倉門"), Some("倉管"));
        assert_eq!(public_person_label_for_room("翻譯桌"), Some("翻譯"));
        assert_eq!(public_person_label_for_room("匯兌小屋"), Some("兌票人"));
    }

    #[test]
    fn public_labels_cover_m3_006_scenes() {
        assert_eq!(public_person_label_for_room("北灣避風碼頭"), Some("船戶"));
        assert_eq!(public_person_label_for_room("港灣貨棚"), Some("搬魚人"));
        assert_eq!(public_person_label_for_room("漁網曬場"), Some("補網人"));
        assert_eq!(public_person_label_for_room("冷霧魚市"), Some("魚販"));
        assert_eq!(public_person_label_for_room("船戶屋排"), Some("船戶家屬"));
        assert_eq!(public_person_label_for_room("北灣守望石階"), Some("守望人"));
    }

    #[test]
    fn public_labels_cover_m3_009_scenes() {
        assert_eq!(public_person_label_for_room("礦坑外棚"), Some("礦工"));
        assert_eq!(public_person_label_for_room("洗礦溝"), Some("洗礦工"));
        assert_eq!(public_person_label_for_room("粗料倉"), Some("倉管"));
        assert_eq!(public_person_label_for_room("工具修棚"), Some("修具工"));
        assert_eq!(public_person_label_for_room("礦渣坡"), Some("推車人"));
        assert_eq!(public_person_label_for_room("舊道裝車點"), Some("車夫"));
    }

    #[test]
    fn public_labels_cover_final_map_stub_replacement_scenes() {
        for (room, label) in [
            ("中鎮鎮口", "守門人"),
            ("中鎮市集街", "看貨人"),
            ("中鎮客棧廊", "店伙"),
            ("中鎮廟前小場", "傳話人"),
            ("中鎮井邊巷", "取水人"),
            ("中鎮工坊街", "學徒"),
            ("中鎮車馬泊位", "車夫"),
            ("中鎮倉場門口", "倉管"),
            ("南城外驛道", "驛夫"),
            ("南城甕門", "門卒"),
            ("門內集散廣場", "引路人"),
            ("中軸大街南段", "攤主"),
            ("中軸大街中段", "巡街人"),
            ("外區坊門", "坊門看守"),
            ("商會長廊", "商會書手"),
            ("大十字通衢", "指路人"),
            ("城邦外坡道", "牽獸人"),
            ("石壘門", "門衛"),
            ("門內券道", "引路人"),
            ("旗石廣場", "旗手"),
            ("西軸坡街", "坡街販"),
            ("議棚前階", "記牌人"),
            ("鑄幣巷口", "兌牌人"),
            ("傭工棚邊", "棚頭人"),
            ("邊城外門", "守門人"),
            ("哨樓梯", "哨兵"),
            ("邊市口", "商旅"),
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
            ("客棧前廊", "店伙"),
            ("補給小市", "小販"),
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
            ("旗石廣場", "等候線", "西軸坡街", "Use"),
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
        let objects = player_layer_scene_objects_for_room("北岸渡口");
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
        let objects = player_layer_scene_objects_for_room("北岸渡口");
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
            ("岸邊短市", "舊麻袋堆", "Gather", "包貨"),
            ("岸邊短市", "短市包貨單", "SellReeds", "蘆草"),
            ("岸邊短市", "短市包貨單", "SellSackcloth", "麻袋布"),
            ("岸邊短市", "短市包貨單", "SellRope", "麻繩"),
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
            "北岸渡口",
            "中鎮鎮口",
            "中鎮市集街",
            "中鎮客棧廊",
            "中鎮廟前小場",
            "中鎮井邊巷",
            "中鎮工坊街",
            "中鎮車馬泊位",
            "中鎮倉場門口",
            "南城外驛道",
            "南城甕門",
            "門內集散廣場",
            "中軸大街南段",
            "中軸大街中段",
            "外區坊門",
            "商會長廊",
            "大十字通衢",
            "城邦外坡道",
            "石壘門",
            "門內券道",
            "旗石廣場",
            "西軸坡街",
            "議棚前階",
            "鑄幣巷口",
            "傭工棚邊",
            "水網入口棧",
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
            "北灣避風碼頭",
            "港灣貨棚",
            "漁網曬場",
            "冷霧魚市",
            "船戶屋排",
            "北灣守望石階",
            "礦坑外棚",
            "洗礦溝",
            "粗料倉",
            "工具修棚",
            "礦渣坡",
            "舊道裝車點",
            "邊城外門",
            "哨樓梯",
            "邊市口",
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
            "客棧前廊",
            "補給小市",
            "巡哨棚",
            "車隊場",
        ] {
            let lines = micro_interaction_lines_for_room(room);
            assert!(lines.iter().all(|line| !line.contains('【')), "{room}");
        }
    }
}
