#[derive(Debug, Clone, Copy)]
pub struct ResidentExpeditionDef {
    pub npc_id: &'static str,
    pub goal_kind: &'static str,
    pub target_coord: (i32, i32),
    pub target_name: &'static str,
    pub activity: &'static str,
    pub thought: &'static str,
    pub return_thought: &'static str,
    pub obligation_kind: &'static str,
    pub obligation_desc: &'static str,
}

pub const RESIDENT_EXPEDITIONS: &[ResidentExpeditionDef] = &[
    ResidentExpeditionDef {
        npc_id: "岑拓",
        goal_kind: "caravan_route",
        target_coord: (-22, 0),
        target_name: "西麓林道",
        activity: "押著滿載鐵器與茶磚的雙輪貨車行進在西麓林道上，隨時留意路旁獸蹤",
        thought: "西麓林道今日風乾路實，趁天色尚早把這批貨趕送出去，回程還能收些山貨",
        return_thought: "在城外林道完成了商隊貨物巡察與交易，正準備沿路返回城裡",
        obligation_kind: "stranded_cargo",
        obligation_desc: "押送鐵器與茶磚商隊車巡視西麓林道",
    },
    ResidentExpeditionDef {
        npc_id: "溫晴",
        goal_kind: "wilderness_expedition",
        target_coord: (-20, 1),
        target_name: "西麓林道",
        activity: "背著竹編藥簍在西麓林道旁辨識新冒芽的光葉藥草與山茶籽",
        thought: "百草堂裡的退熱草藥快見底了，趁今日無雨親自出西門採些鮮葉回去炮製",
        return_thought: "在城外林道採齊了所需的藥材與山貨，正準備沿路返回城裡",
        obligation_kind: "restock_goods",
        obligation_desc: "採集光葉藥草與山茶籽以備炮製",
    },
    ResidentExpeditionDef {
        npc_id: "馬行頭",
        goal_kind: "caravan_route",
        target_coord: (-18, 0),
        target_name: "西麓林道",
        activity: "在西城門外的西麓林道起點檢查駝獸蹄鐵與車轍路況",
        thought: "出西門這段林道車轍頗深，得叮囑車夫們過坡時收緊韁繩",
        return_thought: "在城外林道檢查完路面車轍與蹄鐵，正準備沿路返回車馬行",
        obligation_kind: "repair_tool",
        obligation_desc: "檢查西麓林道起點路況與駝獸蹄鐵車軛",
    },
    ResidentExpeditionDef {
        npc_id: "方石",
        goal_kind: "wilderness_expedition",
        target_coord: (0, 18),
        target_name: "翠冠巨木林",
        activity: "在翠冠巨木林邊緣勘查林木生長與地脈礦脈走勢，記錄可供開採之物產",
        thought: "北門外這片巨木林地氣豐沛，得仔細巡看幾處新露頭的礦石與巨木材質",
        return_thought: "巨木林物產勘查完畢，記錄齊全，正準備回坊間核實物料儲備",
        obligation_kind: "repair_tool",
        obligation_desc: "勘查北門外巨木林與礦石分佈記錄",
    },
    ResidentExpeditionDef {
        npc_id: "林山",
        goal_kind: "wilderness_expedition",
        target_coord: (0, 20),
        target_name: "翠冠巨木林",
        activity: "在翠冠巨木林間巡查獵徑與巨木枝葉，留意林間野獸足跡與落葉堆",
        thought: "翠冠巨木林深處獸蹤漸密，得多加留意野豬與山鹿的活動範圍",
        return_thought: "巡林一圈未見異常，帶回了幾份鮮松脂與巡林筆記",
        obligation_kind: "restock_goods",
        obligation_desc: "巡查翠冠巨木林獵徑與林間野獸蹤跡",
    },
    ResidentExpeditionDef {
        npc_id: "高望雲",
        goal_kind: "wilderness_expedition",
        target_coord: (60, 160),
        target_name: "清泉溪谷",
        activity: "在清泉溪谷高處搭設測風儀，記錄高空雲層流向與山谷濕度",
        thought: "半嶺北坡的風向每至午後便有微變，需親自上雲杉林高處核準氣象測值",
        return_thought: "今日雲杉林氣象測值已完整錄下，這週的降水預測有了著落",
        obligation_kind: "repair_tool",
        obligation_desc: "半嶺北坡高空風向與溪谷濕度實地校準",
    },
    ResidentExpeditionDef {
        npc_id: "雷橫山",
        goal_kind: "wilderness_expedition",
        target_coord: (0, -340),
        target_name: "淺澤泥口",
        activity: "在淺澤泥口灘塗邊檢查重型捕獸夾與耐腐竹樁，排除水澤陷坑危險",
        thought: "泥口潮水退去時常有猛獸涉水，得確保捕獸夾與耐腐機關牢固",
        return_thought: "淺澤泥口的獵具機關已全數調校牢固，可保南哨沿線防務無虞",
        obligation_kind: "repair_tool",
        obligation_desc: "巡檢淺澤泥口重型獵具與防禦機關",
    },
    ResidentExpeditionDef {
        npc_id: "雷千鈞",
        goal_kind: "wilderness_expedition",
        target_coord: (0, -340),
        target_name: "淺澤泥口",
        activity: "在淺澤泥口灘塗邊檢查重型捕獸夾與耐腐竹樁，排除水澤陷坑危險",
        thought: "泥口潮水退去時常有猛獸涉水，得確保捕獸夾與耐腐機關牢固",
        return_thought: "淺澤泥口的獵具機關已全數調校牢固，可保南哨沿線防務無虞",
        obligation_kind: "repair_tool",
        obligation_desc: "巡檢淺澤泥口重型獵具與防禦機關",
    },
    ResidentExpeditionDef {
        npc_id: "葉青",
        goal_kind: "wilderness_expedition",
        target_coord: (120, -320),
        target_name: "白茫洲",
        activity: "在白茫洲蘆葦叢深處採集祛瘴白蘚與水蓼鮮葉，並測試水質毒性",
        thought: "白茫洲水草間的白蘚正是藥性最足的時節，多採些回去配製避瘴丸",
        return_thought: "在白茫洲採得新鮮白蘚與清心水草，回去便能開爐煉製避瘴散",
        obligation_kind: "restock_goods",
        obligation_desc: "採集白茫洲祛瘴白蘚與避瘴水草鮮葉",
    },
    ResidentExpeditionDef {
        npc_id: "葉青囊",
        goal_kind: "wilderness_expedition",
        target_coord: (120, -320),
        target_name: "白茫洲",
        activity: "在白茫洲蘆葦叢深處採集祛瘴白蘚與水蓼鮮葉，並測試水質毒性",
        thought: "白茫洲水草間的白蘚正是藥性最足的時節，多採些回去配製避瘴丸",
        return_thought: "在白茫洲採得新鮮白蘚與清心水草，回去便能開爐煉製避瘴散",
        obligation_kind: "restock_goods",
        obligation_desc: "採集白茫洲祛瘴白蘚與避瘴水草鮮葉",
    },
    ResidentExpeditionDef {
        npc_id: "田大犁",
        goal_kind: "wilderness_expedition",
        target_coord: (-180, -195),
        target_name: "長風草甸",
        activity: "在長風草甸開闊處實地測試重型翻土犁刀的入土深度與草根切斷力",
        thought: "草甸的黑土質地緊密，這款改良犁頭還得再調大兩分傾角才能更省獸力",
        return_thought: "草甸犁刀試驗紀錄詳實，回工坊後即可敲定新一批農具規格",
        obligation_kind: "repair_tool",
        obligation_desc: "長風草甸實地試驗改良農具與犁頭深度",
    },
    ResidentExpeditionDef {
        npc_id: "田百稼",
        goal_kind: "wilderness_expedition",
        target_coord: (-180, -195),
        target_name: "長風草甸",
        activity: "在長風草甸開闊處實地測試重型翻土犁刀的入土深度與草根切斷力",
        thought: "草甸的黑土質地緊密，這款改良犁頭還得再調大兩分傾角才能更省獸力",
        return_thought: "草甸犁刀試驗紀錄詳實，回工坊後即可敲定新一批農具規格",
        obligation_kind: "repair_tool",
        obligation_desc: "長風草甸實地試驗改良農具與犁頭深度",
    },
    ResidentExpeditionDef {
        npc_id: "牧野",
        goal_kind: "wilderness_expedition",
        target_coord: (-180, -162),
        target_name: "長風草甸",
        activity: "在長風草甸巡視放牧的長毛牛群，檢查草場豐茂度與飲水坑水質",
        thought: "春季草甸草勢漸盛，得讓牛群往北面多移幾十丈，避開低窪積水坑",
        return_thought: "今日長風草甸牛群健旺無恙，草色正美，心裡格外舒暢",
        obligation_kind: "restock_goods",
        obligation_desc: "巡視長風草甸放牧牛群與草場飲水點",
    },
    ResidentExpeditionDef {
        npc_id: "牧長風",
        goal_kind: "wilderness_expedition",
        target_coord: (-180, -162),
        target_name: "長風草甸",
        activity: "在長風草甸巡視放牧的長毛牛群，檢查草場豐茂度與飲水坑水質",
        thought: "春季草甸草勢漸盛，得讓牛群往北面多移幾十丈，避開低窪積水坑",
        return_thought: "今日長風草甸牛群健旺無恙，草色正美，心裡格外舒暢",
        obligation_kind: "restock_goods",
        obligation_desc: "巡視長風草甸放牧牛群與草場飲水點",
    },
];

pub fn find_resident_expedition(npc_id: &str) -> Option<&'static ResidentExpeditionDef> {
    RESIDENT_EXPEDITIONS.iter().find(|e| e.npc_id == npc_id)
}

fn initial_thought_for_npc(id: &str, title: &str) -> String {
    if let Some(exp) = find_resident_expedition(id) {
        return exp.thought.to_string();
    }
    if title.contains("聯運") || title.contains("總站") || title.contains("車馬") {
        "各路商隊與駝車進出頻繁，得確保草料充足、車馬蹄鐵安妥".to_string()
    } else if title.contains("總商行") || title.contains("會長") || title.contains("票號") {
        "三水總商行的跨鎮通兌匯票已清點完畢，各口岸船運平穩".to_string()
    } else if title.contains("藏書") || title.contains("主編") || title.contains("風物誌") {
        "古卷殘篇與各鎮新送來的地鳴紀錄對比頗有收穫，得抓緊修纂風物誌".to_string()
    } else if title.contains("水輪") || title.contains("石磨") {
        "聽著堰渠水輪推動大石磨的低沉嗡鳴，心裡盤算著明日送往文曜三水大食堂的現磨溫米漿與大圳頭新穀碾期".to_string()
    } else if title.contains("鍛") || title.contains("鐵") {
        "爐火正旺，這批新出爐的精鐵兵刃得趁熱淬火才能堅韌並濟".to_string()
    } else if title.contains("學院") || title.contains("教務") {
        "文曜學院新一期的格物課綱擬定了大半，得為學子多添幾部古算籍".to_string()
    } else if title.contains("食堂") || title.contains("主廚") {
        "今日鮮魚湯底文火慢煨恰到好處，待會得備好新鮮菜蔬迎客人".to_string()
    } else if title.contains("浴場") || title.contains("溫泉") {
        "泉香浴場水溫適中，寒水玉引流清澈，正適合遠行歸人洗去一身風塵".to_string()
    } else if title.contains("木作") || title.contains("木匠") {
        "這根老料烏木乾燥得宜，正好用來打造大旅社的雕花案桌".to_string()
    } else if title.contains("織") || title.contains("染") || title.contains("服飾") {
        "新染就的青藍織錦光澤均勻，正合適裁剪換季的厚實袍服".to_string()
    } else if title.contains("匠") || title.contains("工") {
        "手頭這批工料正加緊整修，心裡琢磨著今日工序進度".to_string()
    } else if title.contains("醫") || title.contains("診") {
        "藥櫃裡的常備藥材得及早補齊，免得病患急需時周轉不及".to_string()
    } else if title.contains("商") || title.contains("店") || title.contains("攤") {
        "盤算著今日進貨帳目與市面行情，打點鋪面準備開門接客".to_string()
    } else if title.contains("獵") || title.contains("巡") || title.contains("防") {
        "留意著城郊各處道路與荒野動靜，提防野獸與異動".to_string()
    } else {
        "手頭事務料理妥當，心裡踏實沉穩".to_string()
    }
}


