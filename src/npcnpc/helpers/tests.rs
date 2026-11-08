#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_anchors_cap_four() {
        let old = vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()];
        let fresh = vec!["f".into()];
        let m = merge_thread_anchors(&old, &fresh);
        assert_eq!(m.len(), 4);
        assert_eq!(m.last().map(String::as_str), Some("f"));
    }

    #[test]
    fn dedupe_lines_casefold() {
        let v = vec!["Hi".into(), "hi".into(), "x".into()];
        assert_eq!(dedupe_social_context_lines(&v).len(), 2);
    }

    #[test]
    fn clean_npc_line_removes_stage_directions_and_substitutes() {
        let line = "靠在案頭喘了口氣，抬眼問：『羅藥師，天氣這样悶，草都晒乾了，你去那邊采草要留神！』";
        assert_eq!(
            clean_npc_line(line),
            "羅藥師，天氣這樣悶，草都曬乾了，你去那邊採草要留神！"
        );
        let line2 = "點點頭：『喝口涼白開，昨兒給老伴兒帶了點心。』";
        assert_eq!(
            clean_npc_line(line2),
            "喝口涼茶水，昨日給老伴帶了點心。"
        );
    }

    #[test]
    fn clean_npc_line_rewrites_sentence_final_bu_question() {
        let line = "你手裡有老蘆草綑不？我這邊缺點，能換點砥石不？";
        assert_eq!(
            clean_npc_line(line),
            "你手裡有老蘆草綑嗎？我這邊缺點，能換點砥石嗎？"
        );
        let keep_qi = "這豈不？那能行不？";
        assert_eq!(
            clean_npc_line(keep_qi),
            "這豈不？那能行嗎？"
        );
    }

    #[test]
    fn clean_npc_line_converts_simplified_and_dialect() {
        let line = "喝完白開，吃啥呢？這會兒店里沒啥事兒。";
        assert_eq!(
            clean_npc_line(line),
            "喝完涼茶水，吃什麼呢？現在店裡沒什麼事情。"
        );
    }

    #[test]
    fn clean_npc_line_round28_replacements() {
        // 伙 group-noun fixes vs keep 伙食/伙夫/開伙/動伙
        assert_eq!(
            clean_npc_line("那伙人跟這伙人是一伙人，同伙合伙入伙散伙，大伙兒跟大伙還有伙計，小伙子與小伙，好家伙老家伙真家伙。"),
            "那夥人跟這夥人是一夥人，同夥合夥入夥散夥，大夥跟大夥還有夥計，年輕人與年輕人，好傢伙老傢伙真傢伙。"
        );
        assert_eq!(
            clean_npc_line("今晚伙食由伙夫張羅，廚房準備開伙，不可隨意動伙。"),
            "今晚伙食由伙夫張羅，廚房準備開伙，不可隨意動伙。"
        );

        // Modern/corporate slang & clock-hour replacements
        assert_eq!(
            clean_npc_line("都加班了，巡守隊正加班，碼頭加班趕工，工坊也在加班。"),
            "都連夜加派人手了，巡守隊正連夜巡守，碼頭連夜趕工，工坊也在連夜趕工。"
        );
        assert_eq!(
            clean_npc_line("這事得先報備，壞掉的石磨要報修。"),
            "這事得先知會一聲，壞掉的石磨要送去修整。"
        );
        assert_eq!(
            clean_npc_line("這叫效率，我們講究效率，做事效率高，講究效率。"),
            "這叫手腳俐落，我們講究手腳俐落，做事手腳俐落，講究手腳俐落。"
        );
        assert_eq!(
            clean_npc_line("等了半個鐘頭，歇了半個鐘，走了一個鐘頭，花了兩個鐘頭，耗了幾個鐘頭。"),
            "等了半小時，歇了半小時，走了一小時，花了兩小時，耗了好幾小時。"
        );
        assert_eq!(
            clean_npc_line("等了半個時辰，走了一個時辰，花了兩個時辰，耗了幾個時辰。"),
            "等了半小時，走了一小時，花了兩小時，耗了好幾小時。"
        );

        // Legacy non-canonical settlement/zone phrases
        assert_eq!(
            clean_npc_line("守在黑沼關隘，路過鹽田澳，走過青籠關，剛從青籠回，往青籠走，打算去青籠，還在青籠待著。"),
            "守在下營，路過烏石鼻，走過下營，才從下營回，往下營走，打算去下營，還在下營待著。"
        );
        assert_eq!(
            clean_npc_line("巨藻淺灣林木封山，巨藻淺灣封山，淺澤泥口林木封山，淺澤泥口封山，黑石灣林木封山，驚濤岩岸林木封山。"),
            "巨藻淺灣休漁養息，巨藻淺灣休漁養息，淺澤泥口休漁養息，淺澤泥口休漁養息，黑石灣休漁養息，驚濤岩岸休漁養息。"
        );


        // Round 55 dialogue cleanup & regression tests
        assert_eq!(
            clean_npc_line("這桐油麻線新進的，乾得快，我才拿去修了兩把船鏟"),
            "這桐油麻線新進的，乾得快，我才拿去修了兩把船鏟"
        );
        assert_eq!(
            clean_npc_line("嗯，我這邊才從霜林口帶回來一捆老蘆草，乾燥得剛好，正好補補藥房的料。"),
            "嗯，我這邊才從霜林口帶回來一捆老蘆草，乾燥得正好，可補補藥房的料。"
        );
        assert_eq!(
            clean_npc_line("霧還継著呢，我才看過潮汐表"),
            "霧還續著呢，我才看過潮信木牌"
        );
        assert_eq!(
            clean_npc_line("才摘了兩把，我才踩過，我才翻過那片密林，你才說黃冠廷"),
            "才摘了兩把，我才踩過，我才翻過那片密林，你才說黃冠廷"
        );
        assert_eq!(
            clean_npc_line("省吃儉用才買下來的"),
            "省吃儉用才買下來的"
        );
        assert_eq!(
            clean_npc_line("花了大力氣才打出來的"),
            "花了大力氣才打出來的"
        );
        assert_eq!(
            clean_npc_line("天才買主吧"),
            "天才買主吧"
        );
        assert_eq!(
            clean_npc_line("走半小時才到，這才拿正眼瞧你，這才看向你，人才出眾，患難之中，才見真情"),
            "走半小時才到，這才拿正眼瞧你，這才看向你，人才出眾，患難之中，才見真情"
        );
    }

    #[test]
    fn quality_gate_round28_forbidden_terms() {
        assert_eq!(quality_gate_npc_line("今晚加班趕工", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("這事要向隊長報備", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("壞掉的水車送去報修", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("那伙人不知道哪來的", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("守在黑沼關隘前", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("走到鹽田澳歇腳", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("歇了半個鐘再走", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("大家手腳麻利點", 40), (false, "forbidden_term"));
    }

    #[test]
    fn clean_npc_line_round29_replacements() {
        // Work shift and overtime terms
        assert_eq!(
            clean_npc_line("昨夜加了班，今晚加個班，再加點班，準時上個班，等等下個班，上下班辛苦，準時上班，準時下班。"),
            "昨夜連夜巡守，今晚連夜趕工，再多趕一陣工，準時當值上工，等等收工歇息，上工收工辛苦，準時上工，準時收工。"
        );

        // Dialect sun, armor, noodles, water, places
        assert_eq!(
            clean_npc_line("大陽頭曬得慌，防爆防風斗笠，來碗陽春粗麵或陽春麵，水缸裡的開水，喝口開水，來碗滾開水、熱開水、溫開水、冷開水、白開水或開水。"),
            "大日頭曬得慌，防風斗笠，來碗清湯粗麵或清湯粗麵，水缸裡的熱茶水，喝口熱茶水，來碗滾水、熱茶水、溫茶水、涼茶水、溫茶水或熱茶水。"
        );

        // Emergence, frost, fog, wind, snow, rain
        assert_eq!(
            clean_npc_line("草芽剛冒出來，才冒出來，剛冒出，才冒出，剛結的霜，才結的霜，剛結霜，才結霜，剛起的霧，才起的霧，剛起的風，才起的風，剛下的雪，才下的雪，剛下的雨，才下的雨。"),
            "草芽新長出來，新長出來，才冒出，才冒出，新結的霜，新結的霜，新結霜，新結霜，新起的霧，新起的霧，新起的風，新起的風，新落的雪，新落的雪，新落的雨，新落的雨。"
        );

        // Mountain closure and zone name replacements
        assert_eq!(
            clean_npc_line("聽說古河林礦脈封山了，去半嶺避雪，走入黑沼林澤。"),
            "聽說古河巨木林封山休養了，去半嶺鎮避雪，走入水尾寮。"
        );
    }

    #[test]
    fn quality_gate_round29_forbidden_terms() {
        assert_eq!(quality_gate_npc_line("昨夜加了班巡守", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("今晚加個班趕工", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("準時上班當差", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("何時下班歇息", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("吃碗陽春麵暖身", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("舀一碗開水解渴", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("帶著防爆防風斗笠", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("穿越黑沼林去鎮上", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("從西南原出發", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("白塔山麓風勢強勁", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("北聖山雪積很深", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("前往西北礦帶探採", 40), (false, "forbidden_term"));
    }

    #[test]
    fn quality_gate_round41_forbidden_terms() {
        assert_eq!(quality_gate_npc_line("這事要向隊長報備", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("壞掉的水車送去報修", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("明日排班如何調配", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("木架上的乾糧發霉了", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("今日出門真是倒霉", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("庫房角落有股霉味", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("這道甜湯老少咸宜", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("這事得報給衙門處置", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("老夫今日有些乏了", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("幾位哥們一塊走吧", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("他是哪個宗門的弟子", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("動用神識探查周圍", 40), (false, "forbidden_term"));
    }

    #[test]
    fn quality_gate_round42_forbidden_terms() {
        assert_eq!(quality_gate_npc_line("天地至純清氣奔湧成靈氣", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("體內靈力充盈激盪", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("閉關修煉數月方成", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("莫非是天上神仙下凡", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("島上各大幫派互不相讓", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("這事被捲入門派紛爭", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("行走江湖全憑義氣", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("這次押送的鏢資甚巨", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("不知何處來的機器人", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("在此處練習瑜伽調息", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("深坑中殘留強烈輻射", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("施展某種玄奇法術", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("這塊晶片不知有何用處", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("儀器裡的電池快耗盡了", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("前方是一座廢棄工廠", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("據說那裡曾是兵工廠", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("跟人說了會兒話", 40), (false, "forbidden_term"));
    }

    #[test]
    fn quality_gate_round44_forbidden_terms() {
        assert_eq!(quality_gate_npc_line("用螺絲固定住木料", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("拿扳手把零件旋緊", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("工坊裡有蒸汽鍛打機", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("軸承上抹了些機油", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("試管中產生化學反應", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("據傳來自少林寺武僧", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("古塔下供奉著舍利子", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("那人被關進了瘋人院", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("桌案上放著金屬訂書機", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("信奉奇點神教的信徒", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("此處便是奇點世界", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("懂得施展玄奇魔法", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("手中握著鑲寶法杖", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("那位牧師正在祝禱", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("鎮上的教堂鐘聲響起", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("懷裡揣著幾張舊鈔票", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("這批貨物作價五十金", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("用不鏽鋼製成水壺", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("不銹鋼打造的刀刃", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("結實的尼龍繩索", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("塑膠水桶放在一旁", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("塑料盒子裝著零件", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("口袋裡的錢幣叮噹響", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("賞了幾枚金幣", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("數著手裡的鎂鈔", 40), (false, "forbidden_term"));
    }

    #[test]
    fn clean_npc_line_round54_replacements() {
        assert_eq!(
            clean_npc_line("吃完面去煮面，吃面配熱湯面，端上一碗面。"),
            "吃完麵去煮麵，吃麵配熱湯麵，端上一碗麵。"
        );
        assert_eq!(
            clean_npc_line("天才買主吧"),
            "天才買主吧"
        );
        assert_eq!(
            clean_npc_line("花了大力氣才打出來的純淨金屬"),
            "花了大力氣才打出來的純淨金屬"
        );
        assert_eq!(
            clean_npc_line("大戰了三百回合才打下來的"),
            "大戰了三百回合才打下來的"
        );
        assert_eq!(
            clean_npc_line("省吃儉用才買下來的"),
            "省吃儉用才買下來的"
        );
    }

    #[test]
    fn clean_npc_line_round56_cai_guo_replacements() {
        assert_eq!(
            replace_clause_initial_temporal_cai("才過瞭望所，樹影動得怪"),
            "才過瞭望所，樹影動得怪"
        );
        assert_eq!(
            replace_clause_initial_temporal_cai("等風浪停了，才過得去"),
            "等風浪停了，才過得去"
        );
        assert_eq!(
            clean_npc_line("才過瞭望所，樹影動得怪"),
            "才過瞭望所，樹影動得怪"
        );
        assert_eq!(
            clean_npc_line("等風浪停了，才過得去"),
            "等風浪停了，才過得去"
        );
    }

    #[test]
    fn test_round57_soak_dialogue_temporal_cai_and_colloquial_guards() {
        assert_eq!(
            clean_npc_line("嗯，我也早先挑了點菜，早先在水尾寮聽說"),
            "嗯，我也先挑了點菜，前面在水尾寮聽說"
        );
        assert_eq!(
            clean_npc_line("歇了會兒，曬了會兒，坐了會兒，等了會兒，聊了會兒"),
            "歇了一下，曬了一下，坐了一下，等了一下，聊了一下"
        );
        assert_eq!(
            clean_npc_line("好一會兒沒見，等了一會兒，這會兒有空"),
            "好一陣子沒見，等了一下，現在有空"
        );
        assert_eq!(
            clean_npc_line("船工活兒也是手頭活兒，這活兒真不好幹，幹活兒得賣力，粗活兒細活兒都要做"),
            "船工活計也是手頭活計，這差事真不好幹，做工得賣力，粗活細活都要做"
        );
        assert_eq!(
            clean_npc_line("去攞貨還是幫你攞？刮得真勁，刮得勁，風勁得站不住，幹得得勁"),
            "去提貨還是幫你提？刮得真猛，刮得猛，風猛得站不住，幹得痛快"
        );
        assert_eq!(
            clean_npc_line("這批防水組件包含組件若乾，換上拖鞋拿著火柴起子和鋼筆"),
            "這批防浪油布包含構件若乾，換上便鞋拿著火籤旋鑿和硬筆"
        );
        assert_eq!(
            clean_npc_line("穿上風衣在黑板上記錄分紅，用放大鏡看著淋浴沐浴露牙刷濕巾平底鍋面膜"),
            "穿上防雨長衫在漆板上記錄分成，用助視鏡看著沖沐沐浴膏潔齒刷濕布巾平底煎鍋敷面膏"
        );
        assert_eq!(
            clean_npc_line("晨霧才散，濃霧才散，山霧才散，雨勢才收"),
            "晨霧才散，濃霧才散，山霧才散，雨勢才收"
        );
        assert_eq!(
            clean_npc_line("穿著風衣拿鋼筆，用平底鍋熱油"),
            "穿著防雨長衫拿硬筆，用平底煎鍋熱油"
        );
        assert_eq!(
            clean_npc_line("去攞防水組件，換了拖鞋洗了淋浴抹了面膜"),
            "去提防浪油布，換了便鞋洗了沖沐抹了敷面膏"
        );
    }

    #[test]
    fn test_round58_soak_dialogue_and_colloquial_guards() {
        assert_eq!(
            clean_npc_line("今早海霧湧得厲，衣衫都被濕透了"),
            "今早海霧湧得厲害，衣衫都濕透了"
        );
        assert_eq!(
            clean_npc_line("今早海霧湧得厲，衣衫被濕透"),
            "今早海霧湧得厲害，衣衫濕透了"
        );
        assert_eq!(
            clean_npc_line("點點頭，說著也往林道走，天色暗了"),
            "天色暗了"
        );
        assert_eq!(
            clean_npc_line("估摸著海風吹得碼頭發騷，門口松了，這會子怕不是要堵車，真拼"),
            "估量著海風吹得碼頭呼呼作響，門口鬆了，這陣子怕是要車隊受阻，真賣力"
        );
        assert_eq!(
            clean_npc_line("剛喘，剛割，剛拾，剛確，剛量，剛泡，剛用，剛剝，剛瞧，剛磨，剛補"),
            "才喘，才割，才拾，才確認，才量，才泡，才用，新剝，才瞧，才磨，才補"
        );
        assert_eq!(
            clean_npc_line("早先才到，護身符，平安符，符紙，買一送一，買二送一，概不退換"),
            "前面才到，辟瘴香囊，祈安木牌，草藥帖，買一贈一，買二贈一，離手不退"
        );
    }

    #[test]
    fn test_round60_soak_dialogue_gang_name_and_slang_guards() {
        assert_eq!(
            clean_npc_line("我聽說蕭景耀剛走過，說是貨倉見底"),
            "我聽說蕭景耀才走過，說是貨倉見底"
        );
        assert_eq!(
            clean_npc_line("這邊水氣重，你那狗嗅到啥味沒？我才盯到一串腳印"),
            "這邊水氣重，你那狗嗅到什麼氣味沒？我才盯到一串腳印"
        );
        assert_eq!(
            clean_npc_line("我才從農業商行回來，鋪子還缺啥？我也才去看了"),
            "我才從農產行回來，鋪子還缺什麼？我也才去看了"
        );
        assert_eq!(
            clean_npc_line("才過山茶籽叢那邊，路上碰見誰有現成的，咱就換著用"),
            "才過山茶籽叢那邊，路上碰見誰有現成的，我們就換著用"
        );
        assert_eq!(
            clean_npc_line("橋頭日晷初轉過卯時，傘骨廠的活計都快收尾了"),
            "橋頭日晷初轉過卯時，傘骨作坊的活計都快收尾了"
        );
        assert_eq!(
            clean_npc_line("這溪旁草灘新長的野草，嫩得能當柴燒，你去得開嗎？"),
            "這溪旁草灘新長的野草，長得肥嫩茂密，你走得開嗎？"
        );
        assert_eq!(quality_gate_npc_line("去傘骨廠拿零件", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("那是私家酒廠", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("剛建好的釀酒廠", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("咱今天去不去", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("吃啥好呢", 40), (false, "forbidden_term"));
    }

    #[test]
    fn test_round61_soak_dialogue_erhua_and_weather_guards() {
        assert_eq!(
            clean_npc_line("河灣那頭才傳說有隻雲豹進了密林，我才喝完水"),
            "河灣那頭才傳說有隻雲豹進了密林，我才喝完水"
        );
        assert_eq!(
            clean_npc_line("海風才轉涼，再等會兒就能封甕，你有看天氣預報嗎？"),
            "海風才轉涼，再稍後就能封甕，你有看風候天色嗎？"
        );
        assert_eq!(
            clean_npc_line("我才在修船塢閉眼喘了會兒，海味食堂才幫人熬了鍋海苔粥"),
            "我才在修船塢閉眼喘了一下，海味食堂才幫人熬了鍋海苔粥"
        );
        assert_eq!(
            clean_npc_line("才從車廠出來，貨車還在廠棚裡待著，手腳麻利點"),
            "才從大車坊出來，貨車還在工棚裡待著，手腳俐落點"
        );
        assert_eq!(
            clean_npc_line("這玩意兒我門兒清，待會兒大夥兒一塊兒去農業商行"),
            "這物件我門路清，待會大夥一同去農產行"
        );
        assert_eq!(quality_gate_npc_line("你有看天氣預報嗎", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("再待會兒就走", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("等一會兒再來", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("這會兒正忙著", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("那會兒風浪大", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("這玩意兒真稀罕", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("我對這裡門兒清", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("做事手腳麻利", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("他在大車廠做工", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("貨物堆在廠棚下", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("穿著白色襯衫", 40), (false, "forbidden_term"));
    }

    #[test]
    fn test_round62_clean_and_quality_gate_root_erhua_and_factory_terms() {
        assert_eq!(
            clean_npc_line("我正往貨運行去，修鞍子的活兒才完，也不覺得冷"),
            "我正往貨運行去，修鞍子的工作才忙完，也不覺得冷"
        );
        assert_eq!(
            clean_npc_line("早先從水尾寮回來，我才盤貨也聽說這事"),
            "前面從水尾寮回來，我才盤貨也聽說這事"
        );
        assert_eq!(
            clean_npc_line(
                "這風真帶勁，木雕跟活的似的，巡守隊正加強 ，大夥兒戴上老花鏡去鋸木廠與造船廠"
            ),
            "這風真猛烈，木雕如同活物一般，巡守隊正加強巡防，大夥戴上助視鏡去鋸木坊與造船塢"
        );
        assert_eq!(quality_gate_npc_line("修鞍子的活兒才完", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("大夥兒都在工棚", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("這點事兒不算啥", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("使點勁兒拉繩子", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("找不到門兒路", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("聞到一股味兒", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("昨兒才下過雨", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("今兒日頭正好", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("戴上老花鏡看帳", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("配一副眼鏡", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("鋸木廠送來木料", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("造船廠在整修碼頭", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("精煉廠出產鎂錠", 40), (false, "forbidden_term"));
    }

    #[test]
    fn test_round63_clean_and_quality_gate_modern_titles_and_colloquial_terms() {
        assert_eq!(
            clean_npc_line("他剛從秦氏會計事務所出來，正往商務會計教室去"),
            "他才從秦氏算學居出來，正往百工商算學舍去"
        );
        assert_eq!(
            clean_npc_line("會計師與編輯助理、紀錄員、護理員、外務員都在會計事務所"),
            "帳房師傅與編修幫手、記冊員、醫館幫手、跑街夥計都在算學舍"
        );
        assert_eq!(
            clean_npc_line("民事調解員在陸老師宿舍旁，老師宿舍裡放著寒水玉保鮮櫃與保鮮櫃"),
            "街坊調解人在陸先生住處旁，先生住處裡放著寒水玉儲食石櫃與儲食冰櫃"
        );
        assert_eq!(
            clean_npc_line("別顯擺又憋屈，扯淡說八卦鏡能治染寒熱，還賣防感冒草藥"),
            "別炫耀又悶氣，胡扯說避邪銅鏡能治發熱，還賣驅寒草藥"
        );
        assert_eq!(
            clean_npc_line("系學生與巡防課研究員別得瑟嘚瑟，窩囊又磨嘰"),
            "學院學徒與巡防處研習員別張揚張揚，沒出息又拖拉"
        );
        assert_eq!(quality_gate_npc_line("會計事務所正在查帳", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("去商務會計教室上課", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("會計師正在算帳", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("編輯助理負責校稿", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("紀錄員正在謄寫", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("護理員正在換藥", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("外務員出發押車", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("民事調解員調解紛爭", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("陸老師宿舍在書院旁", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("老師宿舍整潔清幽", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("保鮮櫃裡有鮮肉", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("別在人前顯擺", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("心裡覺得憋屈", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("淨說些扯淡的話", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("門頭掛著八卦鏡", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("採了些防感冒草藥", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("他是中文系學生", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("巡防課派人查驗", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("研究員在整理文獻", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("別得瑟個沒完", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("嘚瑟什麼勁兒", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("樣子真夠窩囊", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("辦事別太磨嘰", 40), (false, "forbidden_term"));
    }

    #[test]
    fn test_round64_clean_and_quality_gate_vocab_terms() {
        assert_eq!(
            clean_npc_line("這望遠鏡架才從老船工那邊換的，我背架裡還留著WT-0893北嶺釘鞋"),
            "這望遠鏡架才從老船工那邊換的，我背架裡還留著北嶺釘鞋"
        );
        assert_eq!(
            clean_npc_line("這望遠鏡架才從老船工那邊換的，我背架裡還留著-0893北嶺釘鞋"),
            "這望遠鏡架才從老船工那邊換的，我背架裡還留著北嶺釘鞋"
        );
        assert_eq!(
            clean_npc_line("店長在鍋爐房拿了保溫桶與擔架，準備送到教室"),
            "掌事在炭火氣釜房拿了保溫木桶與抬架，準備送到講堂"
        );
        assert_eq!(quality_gate_npc_line("教室裡正在講課", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("鍋爐房熱氣騰騰", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("發放遣散費", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("收費站正在查驗", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("信用額度用盡了", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("店長正在對帳", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("保溫桶裝滿熱湯", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("急救擔架抬了過來", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("重卡開過來了", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("他又去賭波了", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("澡堂子剛換水", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("淨在那裡瞎扯", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("別再瞎折騰了", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("好勒馬上就來", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("要回水尾寮安歇", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("賣給行腳客", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("拂曉眼下才收工", 40), (false, "forbidden_term"));
    }

    #[test]
    fn test_round65_clean_and_quality_gate_vocab_terms() {
        assert_eq!(
            clean_npc_line("海鹽堆邊的六分儀才被吹歪了，你有帶-1024嗎？"),
            "海鹽堆邊的六分儀才被吹歪了，你有帶六分儀嗎？"
        );
        assert_eq!(
            clean_npc_line("六分儀的游標才調好"),
            "六分儀的游標才調好"
        );
        assert_eq!(
            clean_npc_line("你有帶WT-1024嗎？"),
            "你有帶六分儀嗎？"
        );
        assert_eq!(
            clean_npc_line("這件WT-0893要收妥"),
            "這件硬木藤背架要收妥"
        );
        assert_eq!(
            clean_npc_line("海防警衛狼犬幼崽在值班室病榻邊，陶磚上的雙翼翻板日曆發條座鐘響了"),
            "海防哨衛狼犬幼崽在值班房病床邊，陶磚上的雙翼翻板日曆座鐘響了"
        );
        assert_eq!(
            clean_npc_line("車廂壞了，明信片上印著輪胎井蓋幫浦藍圖"),
            "車斗壞了，風景畫片上印著車輪箍水渠石蓋抽水筒工造圖樣"
        );
        assert_eq!(
            clean_npc_line("別想糊弄擦屁股背黑鍋，窮光蛋問清潔工科研流體力學，發條鐵青蛙剪板機下課給員工"),
            "別想敷衍收拾爛攤子代人受過，窮小子問清掃工研習水道工學，發條木雀裁板機散學給夥計"
        );
        assert_eq!(quality_gate_npc_line("警衛在門口", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("值班室很冷", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("病榻上躺著傷者", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("地上鋪了瓷磚", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("車廂裡坐滿了人", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("車輿裡坐滿了人", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("寄了一張明信片", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("輪胎磨損嚴重", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("井蓋鬆脫了", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("幫浦抽不出水", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("看了建築藍圖", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("別想糊弄過去", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("幫人擦屁股", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("他是個窮光蛋", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("他是個赤貧窮漢", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("清潔工在掃地", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("淨街夫在掃地", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("專注科研工作", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("研究流體力學", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("桌上有發條鐵青蛙", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("操作剪板機", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("等下課再說", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("等歇功再說", 40), (false, "forbidden_term"));
        assert_eq!(quality_gate_npc_line("他是工坊員工", 40), (false, "forbidden_term"));
    }
}
