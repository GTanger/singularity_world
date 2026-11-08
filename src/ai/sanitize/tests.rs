#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_prefix_colon() {
        assert_eq!(
            strip_npc_line_speaker_prefix("阿明：你好", &["阿明", "小華"]),
            "你好"
        );
    }

    #[test]
    fn strip_prefix_ascii_colon() {
        assert_eq!(strip_npc_line_speaker_prefix("Bob: hi", &["Bob"]), "hi");
    }

    #[test]
    fn sanitize_trims_corner_quotes() {
        let s = sanitize_npc_dialogue_line("\u{300c}一句話\u{300d}");
        assert_eq!(s, "一句話");
    }

    #[test]
    fn sanitize_strips_stage_direction_and_fixes_temporal_and_currency() {
        assert_eq!(
            sanitize_npc_dialogue_line("笑著搖頭：「這點小事算什麼，我們都是同鄉。」"),
            "這點小事算什麼，我們都是同鄉。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("村長交代：「天黑前回家。」"),
            "天黑前回家。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("靠在案頭喘了口氣，抬眼問：『羅藥師，天氣這样悶，草都晒乾了，你去那邊采草要留神！』"),
            "羅藥師，天氣這樣悶，草都曬乾了，你去那邊採草要留神！"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("點點頭：『喝口涼白開，昨兒給老伴兒帶了點心。』"),
            "喝口涼茶水，昨日給老伴帶了點心。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("濕得像剛泡過沼澤的草，我剛從水車坊出來。"),
            "濕得像才泡過沼澤的草，我才從水車坊出來。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("志剛從北邊來，我剛到，他剛回答我。"),
            "志剛從北邊來，我才到，他才回答我。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("志剛從北邊來，剛剛好我才剛到，他剛回答我。"),
            "志剛從北邊來，恰好我才到，他才回答我。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("當初從初出茅廬到起初上山。"),
            "當初從初出茅廬到起初上山。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("哪怕多花個八銅也得弄到手，提著一銅壺熱水配夾層銅板與紫銅板。"),
            "哪怕多花個八鎂也得弄到手，提著一銅壺熱水配夾層銅板與紫銅板。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("雨氣濺在葉上像碎銀子，懷裡還揣著幾兩碎銀子和銀錢，湯面浮著油花。"),
            "雨氣濺在葉上像碎銀子，懷裡還揣著幾枚鎂幣和鎂幣，湯面浮著油花。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("剛鬃黑山豬撞上青剛櫟木，含剛玉金剛砂與青剛赤鐵毫髮無損。"),
            "剛鬃黑山豬撞上青剛櫟木，含剛玉金剛砂與青剛赤鐵毫髮無損。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("差点滑倒，有点冷，從黑沼生那頭走回在黑沼生鮮水產行買乾燥劑外帶一份。"),
            "差點滑倒，有點冷，從黑沼生鮮水產行那頭走回在黑沼生鮮水產行買防潮草木灰包包一份帶走。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("拿着差官送來的報表核對數據，導能鎂晶已入庫。"),
            "拿著巡守送來的帳冊核對數目，鎂幣已入庫。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("石板切面很平整，路面條石剛鋪好，吃面條配一碗湯面。"),
            "石板切面很平整，路面條石才鋪好，吃麵條配一碗湯麵。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("才從山徑回来，扛完一趨鹽包，腰背松快了，走你。"),
            "才從山徑回來，扛完一趟鹽包，腰背鬆快了，先走一步。"
        );

        // Round 55 dialogue cleanup & regression tests
        assert_eq!(
            sanitize_npc_dialogue_line("這桐油麻線新進的，乾得快，我才拿去修了兩把船鏟"),
            "這桐油麻線新進的，乾得快，我才拿去修了兩把船鏟"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("嗯，我這邊才從霜林口帶回來一捆老蘆草，乾燥得剛好，正好補補藥房的料。"),
            "嗯，我這邊才從霜林口帶回來一捆老蘆草，乾燥得正好，可補補藥房的料。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("霧還継著呢，我才看過潮汐表"),
            "霧還續著呢，我才看過潮信木牌"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("才摘了兩把，我才踩過，我才翻過那片密林，你才說黃冠廷"),
            "才摘了兩把，我才踩過，我才翻過那片密林，你才說黃冠廷"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("省吃儉用才買下來的"),
            "省吃儉用才買下來的"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("花了大力氣才打出來的"),
            "花了大力氣才打出來的"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("天才買主吧"),
            "天才買主吧"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("走半小時才到，這才拿正眼瞧你，這才看向你，人才出眾，患難之中，才見真情"),
            "走半小時才到，這才拿正眼瞧你，這才看向你，人才出眾，患難之中，才見真情"
        );
    }

    #[test]
    fn sanitize_replaces_prose_and_fa_patterns() {
        let raw = "咱們別衝，林邊有腳印像獅子留的，熱乎乎的麥餅，風吹得人髮麻";
        assert_eq!(
            sanitize_npc_dialogue_line(raw),
            "我們別衝，林邊有腳印像石虎留的，熱騰騰的麥餅，風吹得人發麻"
        );
    }

    #[test]
    fn sanitize_rewrites_sentence_final_bu_question() {
        let line = "你手裡有老蘆草綑不？我這邊缺點，能換點砥石不？";
        assert_eq!(
            sanitize_npc_dialogue_line(line),
            "你手裡有老蘆草綑嗎？我這邊缺點，能換點砥石嗎？"
        );
        let keep_qi = "這豈不？那能行不？";
        assert_eq!(
            sanitize_npc_dialogue_line(keep_qi),
            "這豈不？那能行嗎？"
        );
    }

    #[test]
    fn sanitize_converts_simplified_and_dialect_and_speaker_prefix() {
        let raw = "吕芳英：刚从海泉井喝完白开，今夜风不小，海雾飘得像棉絮，你那边船工们可有见著浪头变大？";
        let stripped = strip_npc_line_speaker_prefix(raw, &["呂芳英"]);
        let out = sanitize_npc_dialogue_line(&stripped);
        assert_eq!(
            out,
            "才從海泉井喝完涼茶水，今夜風不小，海霧飄得像棉絮，你那邊船工們可有見著浪頭變大？"
        );
    }

    #[test]
    fn strip_npc_line_speaker_prefix_generic_pattern() {
        let raw = "張三：今天天氣真好。";
        assert_eq!(strip_npc_line_speaker_prefix(raw, &[]), "今天天氣真好。");
        let dialogue = "笑著說：今天天氣真好。";
        assert_eq!(strip_npc_line_speaker_prefix(dialogue, &[]), "笑著說：今天天氣真好。");
    }

    #[test]
    fn sanitize_round26_dialect_cai_and_commodity_replacements() {
        assert_eq!(
            sanitize_npc_dialogue_line("順路也想瞧瞧新收的野草種子長得咋樣。"),
            "順路也想瞧瞧新收的野草種子長得怎樣。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("這邊的白蘚藥性正勁，要不要一起再采幾把？"),
            "這邊的白蘚藥性正勁，要不要一起再採幾把？"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("我早先在南哨買肥皂花十七鎂。"),
            "我前面在南哨買茶籽皂塊花十七鎂。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("嗯，才窯溫下來，火候穩。"),
            "嗯，窯溫才降下來，火候穩。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("白采妮今日神采奕奕，他風采一如當年，風采果然不凡，神采了得，生活多采多姿。"),
            "白采妮今日神采奕奕，他風采一如當年，風采果然不凡，神采了得，生活多采多姿。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("冒著寒風採藥，趁著晨光採茶。"),
            "冒著寒風採藥，趁著晨光採茶。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("剛好我看了下獸跡，咱倆別走太遠，天才放晴，冷開水才舀好，倒了一碗白開水，泉水涼得跟才冒的霧一樣。"),
            "正好我看了下獸跡，我們倆別走太遠，天色才放晴，涼茶水才舀好，倒了一碗溫茶水，泉水涼得跟新起的山霧一樣。"
        );
    }

    #[test]
    fn sanitize_round28_replacements() {
        // 伙 group-noun fixes vs keep 伙食/伙夫/開伙/動伙
        assert_eq!(
            sanitize_npc_dialogue_line("那伙人跟這伙人是一伙人，同伙合伙入伙散伙，大伙兒跟大伙還有伙計，小伙子與小伙，好家伙老家伙真家伙。"),
            "那夥人跟這夥人是一夥人，同夥合夥入夥散夥，大夥跟大夥還有夥計，年輕人與年輕人，好傢伙老傢伙真傢伙。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("今晚伙食由伙夫張羅，廚房準備開伙，不可隨意動伙。"),
            "今晚伙食由伙夫張羅，廚房準備開伙，不可隨意動伙。"
        );
        assert_eq!(
            normalize_simplified_to_traditional_zh("那伙人同伙合伙入伙散伙大伙兒大伙伙計小伙子小伙好家伙老家伙家伙"),
            "那夥人同夥合夥入夥散夥大夥大夥夥計年輕人年輕人好傢伙老傢伙傢伙"
        );
        assert_eq!(
            normalize_simplified_to_traditional_zh("伙食伙夫開伙動伙"),
            "伙食伙夫開伙動伙"
        );

        // Modern/corporate slang & clock-hour replacements
        assert_eq!(
            sanitize_npc_dialogue_line("都加班了，巡守隊正加班，碼頭加班趕工，工坊也在加班。"),
            "都連夜加派人手了，巡守隊正連夜巡守，碼頭連夜趕工，工坊也在連夜趕工。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("這事得先報備，壞掉的石磨要報修。"),
            "這事得先知會一聲，壞掉的石磨要送去修整。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("這叫效率，我們講究效率，做事效率高，講究效率。"),
            "這叫手腳俐落，我們講究手腳俐落，做事手腳俐落，講究手腳俐落。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("等了半個鐘頭，歇了半個鐘，走了一個鐘頭，花了兩個鐘頭，耗了幾個鐘頭。"),
            "等了半小時，歇了半小時，走了一小時，花了兩小時，耗了好幾小時。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("等了半個時辰，走了一個時辰，花了兩個時辰，耗了幾個時辰。"),
            "等了半小時，走了一小時，花了兩小時，耗了好幾小時。"
        );

        // Legacy non-canonical settlement/zone phrases
        assert_eq!(
            sanitize_npc_dialogue_line("守在黑沼關隘，路過鹽田澳，走過青籠關，剛從青籠回，往青籠走，打算去青籠，還在青籠待著。"),
            "守在下營，路過烏石鼻，走過下營，才從下營回，往下營走，打算去下營，還在下營待著。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("巨藻淺灣林木封山，巨藻淺灣封山，淺澤泥口林木封山，淺澤泥口封山，黑石灣林木封山，驚濤岩岸林木封山。"),
            "巨藻淺灣休漁養息，巨藻淺灣休漁養息，淺澤泥口休漁養息，淺澤泥口休漁養息，黑石灣休漁養息，驚濤岩岸休漁養息。"
        );

    }

    #[test]
    fn sanitize_round29_replacements() {
        // Work shift and overtime terms
        assert_eq!(
            sanitize_npc_dialogue_line("昨夜加了班，今晚加個班，再加點班，準時上個班，等等下個班，上下班辛苦，準時上班，準時下班。"),
            "昨夜連夜巡守，今晚連夜趕工，再多趕一陣工，準時當值上工，等等收工歇息，上工收工辛苦，準時上工，準時收工。"
        );

        // Dialect sun, armor, noodles, water, places
        assert_eq!(
            sanitize_npc_dialogue_line("大陽頭曬得慌，防爆防風斗笠，來碗陽春粗麵或陽春麵，水缸裡的開水，喝口開水，來碗滾開水、熱開水、溫開水、冷開水、白開水或開水。"),
            "大日頭曬得慌，防風斗笠，來碗清湯粗麵或清湯粗麵，水缸裡的熱茶水，喝口熱茶水，來碗滾水、熱茶水、溫茶水、涼茶水、溫茶水或熱茶水。"
        );

        // Emergence, frost, fog, wind, snow, rain (including 剛 -> 才 post-pass)
        assert_eq!(
            sanitize_npc_dialogue_line("草芽剛冒出來，才冒出來，剛冒出，才冒出，剛結的霜，才結的霜，剛結霜，才結霜，剛起的霧，才起的霧，剛起的風，才起的風，剛下的雪，才下的雪，剛下的雨，才下的雨。"),
            "草芽新長出來，新長出來，才冒出，才冒出，新結的霜，新結的霜，新結霜，新結霜，新起的霧，新起的霧，新起的風，新起的風，新落的雪，新落的雪，新落的雨，新落的雨。"
        );

        // Mountain closure and zone name replacements
        assert_eq!(
            sanitize_npc_dialogue_line("聽說古河林礦脈封山了，去半嶺避雪，走入黑沼林澤。"),
            "聽說古河巨木林封山休養了，去半嶺鎮避雪，走入水尾寮。"
        );
    }

    #[test]
    fn sanitize_round54_replacements() {
        assert_eq!(
            sanitize_npc_dialogue_line("吃完面去煮面，吃面配熱湯面，端上一碗面。"),
            "吃完麵去煮麵，吃麵配熱湯麵，端上一碗麵。"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("天才買主吧"),
            "天才買主吧"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("花了大力氣才打出來的純淨金屬"),
            "花了大力氣才打出來的純淨金屬"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("大戰了三百回合才打下來的"),
            "大戰了三百回合才打下來的"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("省吃儉用才買下來的"),
            "省吃儉用才買下來的"
        );
    }

    #[test]
    fn test_round56_cai_guo_replacements() {
        assert_eq!(
            replace_clause_initial_temporal_cai("才過瞭望所，樹影動得怪"),
            "才過瞭望所，樹影動得怪"
        );
        assert_eq!(
            replace_clause_initial_temporal_cai("等風浪停了，才過得去"),
            "等風浪停了，才過得去"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("才過瞭望所，樹影動得怪"),
            "才過瞭望所，樹影動得怪"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("等風浪停了，才過得去"),
            "等風浪停了，才過得去"
        );
    }

    #[test]
    fn test_round57_soak_dialogue_temporal_cai_and_colloquial_guards() {
        assert_eq!(
            sanitize_npc_dialogue_line("嗯，我也早先挑了點菜，早先在水尾寮聽說"),
            "嗯，我也先挑了點菜，前面在水尾寮聽說"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("歇了會兒，曬了會兒，坐了會兒，等了會兒，聊了會兒"),
            "歇了一下，曬了一下，坐了一下，等了一下，聊了一下"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("好一會兒沒見，等了一會兒，這會兒有空"),
            "好一陣子沒見，等了一下，現在有空"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("船工活兒也是手頭活兒，這活兒真不好幹，幹活兒得賣力，粗活兒細活兒都要做"),
            "船工活計也是手頭活計，這差事真不好幹，做工得賣力，粗活細活都要做"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("去攞貨還是幫你攞？刮得真勁，刮得勁，風勁得站不住，幹得得勁"),
            "去提貨還是幫你提？刮得真猛，刮得猛，風猛得站不住，幹得痛快"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("這批防水組件包含組件若乾，換上拖鞋拿著火柴起子和鋼筆"),
            "這批防浪油布包含構件若乾，換上便鞋拿著火籤旋鑿和硬筆"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("穿上風衣在黑板上記錄分紅，用放大鏡看著淋浴沐浴露牙刷濕巾平底鍋面膜"),
            "穿上防雨長衫在漆板上記錄分成，用助視鏡看著沖沐沐浴膏潔齒刷濕布巾平底煎鍋敷面膏"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("晨霧才散，濃霧才散，山霧才散，雨勢才收"),
            "晨霧才散，濃霧才散，山霧才散，雨勢才收"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("穿著風衣拿鋼筆，用平底鍋熱油"),
            "穿著防雨長衫拿硬筆，用平底煎鍋熱油"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("去攞防水組件，換了拖鞋洗了淋浴抹了面膜"),
            "去提防浪油布，換了便鞋洗了沖沐抹了敷面膏"
        );
    }

    #[test]
    fn test_round58_soak_dialogue_and_colloquial_guards() {
        assert_eq!(
            sanitize_npc_dialogue_line("今早海霧湧得厲，衣衫都被濕透了"),
            "今早海霧湧得厲害，衣衫都濕透了"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("今早海霧湧得厲，衣衫被濕透"),
            "今早海霧湧得厲害，衣衫濕透了"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("點點頭，說著也往林道走，天色暗了"),
            "天色暗了"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("估摸著海風吹得碼頭發騷，門口松了，這會子怕不是要堵車，真拼"),
            "估量著海風吹得碼頭呼呼作響，門口鬆了，這陣子怕是要車隊受阻，真賣力"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("剛喘，剛割，剛拾，剛確，剛量，剛泡，剛用，剛剝，剛瞧，剛磨，剛補"),
            "才喘，才割，才拾，才確認，才量，才泡，才用，新剝，才瞧，才磨，才補"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("早先才到，護身符，平安符，符紙，買一送一，買二送一，概不退換"),
            "前面才到，辟瘴香囊，祈安木牌，草藥帖，買一贈一，買二贈一，離手不退"
        );
    }

    #[test]
    fn test_round59_soak_dialogue_and_chemistry_guards() {
        assert_eq!(
            sanitize_npc_dialogue_line("天光才亮，水尾澤的霧還在邊坡轉"),
            "天光才亮，水尾澤的霧還在邊坡轉"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("崖壁上那片藥草該是才冒頭"),
            "崖壁上那片藥草該是才冒頭"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("我早先看到一隻小刺猬在滾草堆，你準備啥用？"),
            "我前面看到一隻小刺蝟在滾草堆，你準備做什麼用？"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("我早先就收了蓋布，怕氧化變色，想先借四鎂過個週"),
            "我前面就收了蓋布，怕受潮走色，想先借四鎂周轉幾日"
        );
        assert_eq!(
            fix_self_surname_vocative_in_pair(
                "嗯，沈師傅你那邊魚油熬得怎麼樣？",
                "沈長青",
                "劉依婷"
            ),
            "嗯，劉師傅你那邊魚油熬得怎麼樣？"
        );
    }

    #[test]
    fn test_round60_soak_dialogue_gang_name_and_slang_guards() {
        assert_eq!(
            sanitize_npc_dialogue_line("我聽說蕭景耀剛走過，說是貨倉見底"),
            "我聽說蕭景耀才走過，說是貨倉見底"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("這邊水氣重，你那狗嗅到啥味沒？我才盯到一串腳印"),
            "這邊水氣重，你那狗嗅到什麼氣味沒？我才盯到一串腳印"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("我才從農業商行回來，鋪子還缺啥？我也才去看了"),
            "我才從農產行回來，鋪子還缺什麼？我也才去看了"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("才過山茶籽叢那邊，路上碰見誰有現成的，咱就換著用"),
            "才過山茶籽叢那邊，路上碰見誰有現成的，我們就換著用"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("橋頭日晷初轉過卯時，傘骨廠的活計都快收尾了"),
            "橋頭日晷初轉過卯時，傘骨作坊的活計都快收尾了"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("這溪旁草灘新長的野草，嫩得能當柴燒，你去得開嗎？"),
            "這溪旁草灘新長的野草，長得肥嫩茂密，你走得開嗎？"
        );
    }

    #[test]
    fn test_round61_soak_dialogue_erhua_and_weather_guards() {
        assert_eq!(
            sanitize_npc_dialogue_line("河灣那頭才傳說有隻雲豹進了密林，我才喝完水"),
            "河灣那頭才傳說有隻雲豹進了密林，我才喝完水"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("海風才轉涼，再等會兒就能封甕，你有看天氣預報嗎？"),
            "海風才轉涼，再稍後就能封甕，你有看風候天色嗎？"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("我才在修船塢閉眼喘了會兒，海味食堂才幫人熬了鍋海苔粥"),
            "我才在修船塢閉眼喘了一下，海味食堂才幫人熬了鍋海苔粥"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("才從車廠出來，貨車還在廠棚裡待著，手腳麻利點"),
            "才從大車坊出來，貨車還在工棚裡待著，手腳俐落點"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("這玩意兒我門兒清，待會兒大夥兒一塊兒去農業商行"),
            "這物件我門路清，待會大夥一同去農產行"
        );
    }

    #[test]
    fn test_round62_soak_dialogue_root_erhua_and_factory_guards() {
        assert_eq!(
            sanitize_npc_dialogue_line("我正往貨運行去，修鞍子的活兒才完，也不覺得冷"),
            "我正往貨運行去，修鞍子的工作才忙完，也不覺得冷"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("早先從水尾寮回來，我才盤貨也聽說這事"),
            "前面從水尾寮回來，我才盤貨也聽說這事"
        );
        assert_eq!(
            sanitize_npc_dialogue_line(
                "這風真帶勁，木雕跟活的似的，巡守隊正加強 ，大夥兒戴上老花鏡去鋸木廠與造船廠"
            ),
            "這風真猛烈，木雕如同活物一般，巡守隊正加強巡防，大夥戴上助視鏡去鋸木坊與造船塢"
        );
    }

    #[test]
    fn test_round63_modern_titles_rooms_and_colloquial_guards() {
        assert_eq!(
            sanitize_npc_dialogue_line("他剛從秦氏會計事務所出來，正往商務會計教室去"),
            "他才從秦氏算學居出來，正往百工商算學舍去"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("會計師與編輯助理、紀錄員、護理員、外務員都在會計事務所"),
            "帳房師傅與編修幫手、記冊員、醫館幫手、跑街夥計都在算學舍"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("民事調解員在陸老師宿舍旁，老師宿舍裡放著寒水玉保鮮櫃與保鮮櫃"),
            "街坊調解人在陸先生住處旁，先生住處裡放著寒水玉儲食石櫃與儲食冰櫃"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("別顯擺又憋屈，扯淡說八卦鏡能治染寒熱，還賣防感冒草藥"),
            "別炫耀又悶氣，胡扯說避邪銅鏡能治發熱，還賣驅寒草藥"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("系學生與巡防課研究員別得瑟嘚瑟，窩囊又磨嘰"),
            "學院學徒與巡防處研習員別張揚張揚，沒出息又拖拉"
        );
    }

    #[test]
    fn test_round64_soak_wt_code_demonstrative_cai_and_vocab_guards() {
        assert_eq!(
            sanitize_npc_dialogue_line("這望遠鏡架才從老船工那邊換的，我背架裡還留著WT-0893北嶺釘鞋"),
            "這望遠鏡架才從老船工那邊換的，我背架裡還留著北嶺釘鞋"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("店長在鍋爐房拿了保溫桶與擔架，準備送到教室"),
            "掌事在炭火氣釜房拿了保溫木桶與抬架，準備送到講堂"
        );
        assert_eq!(
            strip_wt_catalog_codes("這望遠鏡架才從老船工那邊換的，我背架裡還留著-0893北嶺釘鞋"),
            "這望遠鏡架才從老船工那邊換的，我背架裡還留著北嶺釘鞋"
        );
    }

    #[test]
    fn test_round65_cai_wt_code_and_vocab_guards() {
        assert_eq!(
            replace_clause_initial_temporal_cai("海鹽堆邊的六分儀才被吹歪了，你有帶-1024嗎？"),
            "海鹽堆邊的六分儀才被吹歪了，你有帶-1024嗎？"
        );
        assert_eq!(
            replace_clause_initial_temporal_cai("六分儀的游標才調好"),
            "六分儀的游標才調好"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("海鹽堆邊的六分儀才被吹歪了，你有帶-1024嗎？"),
            "海鹽堆邊的六分儀才被吹歪了，你有帶六分儀嗎？"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("你有帶WT-1024嗎？"),
            "你有帶六分儀嗎？"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("帶上WT-0893去巡查"),
            "帶上硬木藤背架去巡查"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("六分儀的游標才調好，警衛在值班室看著病榻與瓷磚車廂"),
            "六分儀的游標才調好，哨衛在值班房看著病床與陶磚車斗"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("這張明信片印著輪胎井蓋幫浦藍圖，別想糊弄擦屁股背黑鍋"),
            "這張風景畫片印著車輪箍水渠石蓋抽水筒工造圖樣，別想敷衍收拾爛攤子代人受過"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("那窮光蛋問清潔工與科研流體力學，發條鐵青蛙在剪板機下課後給員工"),
            "那窮小子問清掃工與研習水道工學，發條木雀在裁板機散學後給夥計"
        );
        assert_eq!(
            sanitize_npc_dialogue_line("拂曉這會兒我才在小市集把蘆葦賣給行腳客，這就回水尾寮安歇，測溫銅管剛換過，我剛聽人說"),
            "拂曉這時候我才在市集把蘆葦賣給外地客商，這就回水尾寮休息，測溫銅管才換過，我才聽人說"
        );
    }

    #[test]
    fn test_phase2_ssot_urban_renewal_obligations_actor_card_and_sanitizer() {
        // 1. Multi-slot obligation cache: repay_debt is never overwritten by routine errands or wiped by empty kind
        let test_npc = "test_urban_renewal_debtor";
        crate::store::open_active_npc_obligation(test_npc, "repay_debt", "creditor:韓鐵山");
        crate::store::open_active_npc_obligation(test_npc, "restock_goods", "西門車馬聯運總站");
        let all_obs = crate::store::get_all_active_npc_obligations(test_npc);
        assert!(all_obs.iter().any(|(k, _)| k == "repay_debt"));
        assert!(all_obs.iter().any(|(k, _)| k == "restock_goods"));
        // Highest priority active obligation is repay_debt
        let top_ob = crate::store::get_active_npc_obligation(test_npc).expect("top obligation");
        assert_eq!(top_ob.0, "repay_debt");
        // Wildcard empty clear only clears non-debt routine obligations
        crate::store::clear_active_npc_obligation(test_npc, "");
        let after_wildcard = crate::store::get_active_npc_obligation(test_npc).expect("repay_debt survives wildcard clear");
        assert_eq!(after_wildcard.0, "repay_debt");
        // Explicit repay_debt clear settles it
        crate::store::clear_active_npc_obligation(test_npc, "repay_debt");
        assert!(crate::store::get_active_npc_obligation(test_npc).is_none());

        // 2. Third-person actor card for NPC-to-NPC dialogue has zero second-person framing or player summary leak
        let card = crate::db::build_third_person_actor_card("韓鐵山", "先開口角色甲", "陳萬福");
        assert!(card.contains("先開口角色甲：韓鐵山"));
        assert!(card.contains("西門・韓氏總站宅院"));
        assert!(card.contains("西門車馬聯運總站"));
        assert!(!card.contains("你是"));
        assert!(!card.contains("與來客過往互動印象"));
    }

    #[test]
    fn test_strip_immediate_temporal_words_and_sky_time_sanitization() {
        // 驗證 strip_immediate_temporal_words 不會產生疊詞或非正典冗贅短語
        let input = "我剛吃完麵，剛檢查完門栓，剛巡完索具，剛吃飽";
        let stripped = strip_immediate_temporal_words(input);
        assert_eq!(stripped, "我才吃完麵，才檢查完門栓，才巡完索具，才吃飽");
        assert!(!stripped.contains("吃過完"));
        assert!(!stripped.contains("吃過飽"));
        assert!(!stripped.contains("檢視過查"));
        assert!(!stripped.contains("巡過完"));

        // 驗證夜深時段問候對齊
        let late_greeting = align_dialogue_sky_time("早啊！請問你這批貨怎麼賣？", 2);
        assert_eq!(late_greeting, "夜深了！請問你這批貨怎麼賣？");
        let early_greeting = align_dialogue_sky_time("早安！路上小心。", 20);
        assert_eq!(early_greeting, "夜安！路上小心。");
        let day_greeting = align_dialogue_sky_time("早啊！請問你這批貨怎麼賣？", 8);
        assert_eq!(day_greeting, "早啊！請問你這批貨怎麼賣？");
    }
}
