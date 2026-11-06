fn resolve_room_venue_object_p1(combined: &str, room_tags: &[String]) -> Option<RoomVenueObject> {
    if (room_tags.iter().any(|t| matches!(canonical_room_tag(t), "clinic" | "herb"))
        || ["診所", "醫館", "醫師"].iter().any(|k| combined.contains(k)))
        && !combined.contains("教務長")
    {
        return Some(RoomVenueObject {
            venue_kind: "醫館藥局",
            default_host_title: "當值醫師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正於青石藥案前調配青鹽薄荷潔牙粉與水苔敷藥",
            on_duty_demeanor: "身形纖細而步履輕快，神情溫柔專注，隨身配掛分格藥囊與百草溫灸盒，衣袍間常帶著淡淡的山風草與清涼薄荷香氣。",
            lendable_tool: Some((
                "wt_cat_1057",
                "溫氏導能溫石百草溫灸盒",
                "診所常備的「溫石百草溫灸盒」借你隨身攜帶溫熨穴位，出入風露寒濕之地可保氣血安和，用畢記得歸還。",
            )),
            counter_trade_offers: &[
                ("wt_cat_1065", "雪蓮活絡跌打膏", 8),
                ("wt_cat_1064", "巡山獵人隨身急救皮囊", 12),
            ],
            trade_talk_pitch: "「隨身防潮急救皮囊裡配齊了止血水苔、止痛敷散與辟穢草油，出門必備，這兒隨時可替你備上一副。」",
            on_duty_talk_lines: &[
                "「請慢些坐下。容我先看一眼舌苔與脈息……氣血略有虧虛，可是在外奔波受了山風涼氣？」",
                "「山風草配北山清泉熬成的舒筋藥膏最能拔除關節寒濕。凡進山涉水之人，隨身備上一盒總有益處。」",
                "「採藥要懂天時，雨前收葉、霜後挖根。診所裡的藥材皆是在北嶺和黑沼外緣採集精選的。」",
            ],
        });
    }

    // 2. 木作/巧木/傢俱
    if ["巧木", "木作", "傢俱", "木匠"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "木作工坊",
            default_host_title: "當值木匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正用省力導能刨刀推平松香木板",
            on_duty_demeanor: "身型精瘦的老木工師傅，穿著沾有細碎木屑的耐磨工作服，神情專注親切，隨身工具皮帶上插著各式精巧的直角尺與導能木刨。",
            lendable_tool: Some((
                "wt_cat_0956",
                "柯氏松木家用工具箱",
                "這口「松木家用工具箱」裡頭鑿子、羊角錘與木尺齊全，你先提去修繕器物，用完擦淨送回工坊便是。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0559", "寒水玉雙層保鮮石櫃", 35),
                ("wt_cat_0723", "旋蓋夜光貝母調光台燈", 15),
                ("wt_cat_0795", "烏木黑子白貝圍棋盤", 20),
                ("wt_cat_0622", "防靜電松香木梳", 5),
            ],
            trade_talk_pitch: "「寒水玉雙層保鮮石櫃、旋蓋夜光貝母台燈、烏木白貝圍棋盤與松香木梳都在架上，防潮防蠹，看中自可帶走。」",
            on_duty_talk_lines: &[
                "「咔嗒！聽這榫卯合榫的一聲脆響，嚴絲合縫，不著一釘一鉚，這便是百年木器的底氣！」",
                "「北山雲杉木性柔韌、松香良木氣味避蟲，打成箱櫃百年生香。挑木料如相人，木紋正，器型才立得正。」",
                "「做傢俱急不得，木頭需陰乾三年才可下刨。摸一摸就知含水幾分。」",
            ],
        });
    }

    // 3. 火磚/測紋/器脈工坊（不得以「分家巷」街名通吃整條街的武館、雜貨與染坊）
    if ["測紋", "火磚", "器脈工坊"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "導能器脈工坊",
            default_host_title: "當值匠師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正用游標卡尺測量導能火磚嵌合縫隙",
            on_duty_demeanor: "年輕幹練的器脈技師，穿著耐磨工作短衫，體態勻稱結實，神情專注認真，腰間別著黃銅測尺與導能紋板。",
            lendable_tool: Some((
                "wt_cat_0735",
                "導能測紋石板",
                "這塊「導能測紋石板」嵌有微光標準刻度，借你拿去比對器物與火磚紋路，切記輕拿輕放。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0611", "岑氏合金黃銅自來水栓", 12),
                ("wt_cat_0956", "柯氏松木家用工具箱", 14),
                ("wt_cat_1010", "卡式導能便攜爐", 16),
                ("wt_cat_0735", "導能測紋石板", 15),
            ],
            trade_talk_pitch: "「前案擺著新造的導能測紋石板、卡式便攜爐、黃銅水栓與松木家用工具箱，皆是民生百工合用器物，隨意挑選。」",
            on_duty_talk_lines: &[
                "「器物脈絡精密入微。卡尺多移一絲、刻紋深淺錯一毫，導能火磚的蓄熱便差之千里。」",
                "「民生所用器具最講究耐用可靠。每一組水栓與便攜爐在出坊前都需經過三道冷熱測壓。」",
                "「鑽研器物手藝看似枯燥，實則處處玄機。若是對器脈結構有興趣，多看看這塊測紋石板自能心領神會。」",
            ],
        });
    }

    // 4. 鍛造/鐵匠鋪
    if room_tags.iter().any(|t| matches!(canonical_room_tag(t), "blacksmith" | "forge"))
        || ["第一鐵坊", "百鍛", "鍛造", "鐵匠", "打鐵"].iter().any(|k| combined.contains(k))
    {
        let is_apprentice_or_tech = combined.contains("青年") || combined.contains("技師") || combined.contains("學徒");
        let offers: &'static [(&'static str, &'static str, i32)] = if is_apprentice_or_tech {
            &[
                ("wt_cat_0611", "岑氏合金黃銅自來水栓", 12),
                ("wt_cat_0956", "柯氏松木家用工具箱", 14),
                ("wt_cat_1010", "卡式導能便攜爐", 16),
                ("wt_cat_0735", "導能測紋石板", 15),
            ]
        } else {
            &[
                ("wt_cat_1021", "岩羊角手柄折疊獵刀", 18),
                ("wt_cat_0957", "鍛鋼小羊角錘", 10),
            ]
        };
        let demeanor = if is_apprentice_or_tech {
            "年輕幹練的鍛造技師，穿著耐磨工作短衫，體態勻稱結實，神情專注認真，眉宇間帶著鑽研新器物的朝氣與執著，指節滿是敲打金屬的厚繭。"
        } else {
            "滿面虯髯、粗獷豪邁的大匠師，露出結實粗壯的手臂，繫著厚牛皮防燙圍裙，眼神如爐火般熾烈堅定，渾身散發著炭火與金屬的沉雄氣息。"
        };
        let pitch = if is_apprentice_or_tech {
            "「前案擺著新造的導能測紋石板、卡式便攜爐、黃銅水栓與松木家用工具箱，皆是民生百工合用器物，隨意挑選。」"
        } else {
            "「精鍛護手獵刀與羊角鍛錘就擺在木架上，刀身冷光如霜，拿在手裡沉實合手，相中哪件儘管開口！」"
        };
        let lines: &'static [&'static str] = if is_apprentice_or_tech {
            &[
                "「這柄獵刀特意加了雙道放血槽，護手用黃銅配重，劈砍時手腕極省力。你覺得握感如何？」",
                "「做鐵匠看似粗笨，實則心細如髮。淬火時水溫多燙一度、火色淺淡一分，成品的硬度與韌性便差之千里。」",
                "「前街新運來的合金黃銅水栓已按標準修磨完成，出水順暢不漏滴，是每戶家屋修繕必備之物。」",
            ]
        } else {
            &[
                "「呼！火爐炭氣嗆人，退後兩步！三號坑精鐵質地堅硬，需借萬斤水錘反覆折疊鍛打，才能逼出雜質。」",
                "「這座鍛造爐打過百工翻土犁，也鍛過各鎮獵隊的斬木刀。手藝不欺人，鐵砧上見真章！」",
                "「打鐵講求火候精準，新式水槽淬火鍛出的獵刀韌性倍增，百煉精鋼才經得起山林考驗。」",
            ]
        };
        return Some(RoomVenueObject {
            venue_kind: "百鍛鐵坊",
            default_host_title: "當值鐵匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正指導學徒查看高溫紅鐵爐的火色",
            on_duty_demeanor: demeanor,
            lendable_tool: Some((
                "wt_cat_0957",
                "鍛鋼小羊角錘",
                "這柄「鍛鋼小羊角錘」錘頭厚實趁手，你先借去敲打修整，莫讓雨水鏽了錘面。",
            )),
            counter_trade_offers: offers,
            trade_talk_pitch: pitch,
            on_duty_talk_lines: lines,
        });
    }

    // 5. 鐘樓/司鐘
    if ["鐘樓", "司鐘"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "傳訊鐘樓",
            default_host_title: "當值司鐘",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正校對旋轉日規與報時青銅巨鐘並核驗快騎郵袋",
            on_duty_demeanor: "年過半百的鐘樓司鐘長，穿著深褐色耐磨工作衫，身形挺拔，斑白短髮梳理得一絲不苟，神情嚴謹，口袋裡放著一枚校時極準的黃銅發條懷錶。",
            lendable_tool: Some((
                "wt_cat_0729",
                "校準隨身黃銅懷錶",
                "這枚「校準隨身黃銅懷錶」已對準鐘樓正午日影，借你隨身掌握時辰，莫誤了潮汐與車班。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0729", "校準隨身黃銅懷錶", 18),
                ("wt_cat_0728", "岑氏黃銅發條座鐘", 24),
                ("wt_cat_0794", "紅松脂封蠟行會委託單", 6),
            ],
            trade_talk_pitch: "「案頭備有三水口校時黃銅懷錶、岑氏黃銅發條座鐘與紅松脂封蠟委託單，皆走時極準，任君挑選。」",
            on_duty_talk_lines: &[
                "「鐘樓一日鳴鐘六次，辰正開市、戌正收更，半刻不容差池。全城作息信賴此音，守這口銅鐘已歷三十載。」",
                "「樓下快騎驛使日夜馳騁，一袋緊急手信關係百里外農桑商訊。凡入鐘樓者，須知惜時如金。」",
                "「發條齒輪與旋轉日晷乃鐘樓之骨髓，日日需添山茶油養護，才能轉動無聲、報時準確。」",
            ],
        });
    }

    // 6. 藏書閣/印書/書局
    if !combined.contains("契書")
        && ["藏書閣", "印書", "旬報", "書局", "主編"].iter().any(|k| combined.contains(k))
    {
        return Some(RoomVenueObject {
            venue_kind: "藏書印書館",
            default_host_title: "當值編修",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正校勘新印出的《白塔風物誌》與《半島旬報》",
            on_duty_demeanor: "清麗素雅的年輕編修，身穿立領長版長衫，神情專注沉靜，眉眼溫婉明慧，袖口微挽，指尖帶著校對書稿與排印圖冊留下的淡淡油墨香。",
            lendable_tool: Some((
                "wt_cat_0784",
                "《白塔風物誌》",
                "這冊《白塔風物誌》詳載半島四鎮十九獵場山川風物，借你展讀參照，切勿折損書頁。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0784", "《白塔風物誌》", 8),
                ("wt_cat_0786", "每旬發行《半島旬報》", 3),
                ("wt_cat_0780", "彈性精鋼尖蘸水硬筆", 10),
                ("wt_cat_0781", "竹綿紙牛皮記事手札", 8),
            ],
            trade_talk_pitch: "「門前書案備有竹綿紙精印的《白塔風物誌》與每旬發行之《半島旬報》，亦有黃銅蘸水硬筆，皆可按公價結算。」",
            on_duty_talk_lines: &[
                "「你也是來查閱古卷的麼？閣內收羅了先祖渡海以來的四方碑拓與各地採風錄，請輕聲慢步，莫驚動滿室書香。」",
                "「新一期《白塔風物誌》新編纂完稿，記載了北嶺地脈與白茫洲水文，讀來頗多意趣。」",
                "「字墨有靈，能載山川興衰。若你在外鄉探尋到奇聞異草或古碑拓片，隨時可送來藏書閣一同校勘。」",
            ],
        });
    }

    // 7. 算學/商算/會計
    if ["算學", "商算", "會計"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "商算學館",
            default_host_title: "當值算師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正操作隨身計數板核算大宗行單",
            on_duty_demeanor: "精明幹練的百工核算師，穿著剪裁合身的深灰棉布外套，面容清瘦，眼神銳利，腰間掛著硬革資料袋，隨身攜帶一具精準計數板。",
            lendable_tool: Some((
                "wt_cat_0788",
                "烏木便攜計數板",
                "這塊「烏木便攜計數板」算珠滑順分明，借你外出盤點貨數與帳目，核完記得歸還。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0788", "烏木便攜計數板", 14),
                ("wt_cat_0781", "竹綿紙牛皮記事手札", 8),
                ("wt_cat_0790", "三水總商行鎂幣分格存摺", 12),
            ],
            trade_talk_pitch: "「工坊精製的計數板與帳本清冊皆在架上，手感極佳，算師學徒皆好此物，需要便取去。」",
            on_duty_talk_lines: &[
                "「精準計算分毫不爽，百工商算學舍講求的是實數實利。左手出海、右手歸倉，帳目清爽，心神始得安定。」",
                "「各工坊每季器物耗料與出貨工錢皆在此匯總。數字不會騙人，經營之道全在精細籌謀。」",
                "「若想研習行商簿記要覽，先學會心口如一、落筆有節。核算嚴謹，始見功底深厚。」",
            ],
        });
    }

    // 8. 契書局 / 房地契 / 戶政 / 屋契
    if ["契書", "房地契", "戶政", "屋契"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "地籍契書司",
            default_host_title: "當值錄事",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正於案前核對《半島全戶總譜》與紅印屋契",
            on_duty_demeanor: "端正嚴謹的青年地政登記員，身穿深藍色當值夾袍，腰板挺直，行事一絲不苟，袖口整齊扣好，桌邊整齊擺放著紅印屋契與吸墨器。",
            lendable_tool: Some((
                "wt_cat_0816",
                "雙面雕刻銅製驗金天平",
                "這具「雙面雕刻銅製驗金天平」分毫不差，借你核驗大宗交割成色，用畢務必歸還公案。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0793", "公議堂紅印屋契與租約摺", 25),
                ("wt_cat_0821", "玻璃螺紋瓶裝黑墨水", 6),
            ],
            trade_talk_pitch: "「案前備有標準公議堂紅印屋契與租約摺，亦有專用螺紋瓶黑墨水，字跡歷數十年不退色，隨時可辦理開具。」",
            on_duty_talk_lines: &[
                "「請出示身份憑信與定居條目。凡落籍文曜坊市者，必先登載於《半島全戶總譜》，朱墨畫押，留底備查。」",
                "「屋契乃立身安居之憑券。加蓋公議堂硃砂鋼印與騎縫花押，受全城通事保護，不可偽冒，不可毀損。」",
                "「契約條款皆白紙黑字寫得明白。田土面積、界石坐標、租期稅賦，一筆不差，此乃文曜立城之基石。」",
            ],
        });
    }

    // 9. 公議堂（不含契書）
    if combined.contains("公議堂") {
        return Some(RoomVenueObject {
            venue_kind: "公議堂民政處",
            default_host_title: "當值調解員",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正手持木牌巡看街坊水渠與公議告示",
            on_duty_demeanor: "辦事幹練圓融的公議堂民務通事，身穿體面的深赭色排扣外套，留著整齊短鬚，笑容可掬，口袋裡常備著幾份街區公告與隨身記事本。",
            lendable_tool: Some((
                "wt_cat_0816",
                "雙面雕刻銅製驗金天平",
                "這具「雙面雕刻銅製驗金天平」分毫不差，借你核驗大宗交割成色，用畢務必歸還公案。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0793", "公議堂紅印屋契與租約摺", 25),
                ("wt_cat_0786", "每旬發行《半島旬報》", 3),
            ],
            trade_talk_pitch: "「公議堂紅印屋契與租約摺、每旬發行之《半島旬報》皆在案頭，手續合規即可開印簽發。」",
            on_duty_talk_lines: &[
                "「哎呀你好！在文曜安家落籍或有公事調解，找公議堂就對了！四坊巷陌每塊界石、每張房契，我都熟記於心。」",
                "「若是看中了西門或分家巷的石砌家屋，先簽了紅印屋契，領取黃銅門牌鑰匙，隨時便能生火入厝！」",
                "「各坊若有修路引水或市集攤位爭執，皆在公議堂擺平調和。和氣生財，安居樂業最重要。」",
            ],
        });
    }

    // 10. 總商行 / 商行 / 通兌
    if ["總商行", "商行", "通兌"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "商行通兌總處",
            default_host_title: "當值掌事",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正於樟木掌事台前核驗跨鎮通兌匯票",
            on_duty_demeanor: "老掌事年事已高卻精神矍鑠，穿著整潔挺括的正裝外套，神色和藹沉穩，眉宇間透著數十年商界歷練的從容大氣，手握一枚黃銅印章。",
            lendable_tool: Some((
                "wt_cat_0816",
                "雙面雕刻銅製驗金天平",
                "這具「雙面雕刻銅製驗金天平」分毫不差，借你核驗大宗交割成色，用畢務必歸還公案。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0790", "三水總商行鎂幣分格存摺", 12),
                ("wt_cat_0892", "軟牛皮分格零錢夾", 8),
            ],
            trade_talk_pitch: "「櫃房新印了一批加厚牛皮存摺與防水鎂幣夾，皆蓋有總商行鋼印，經久耐磨，隨身攜帶最是妥帖。」",
            on_duty_talk_lines: &[
                "「年輕人有幹勁。三水口匯聚半島四方客商，總商行運轉數十年，最看重的便是『信義』二字。坐下喝杯茶吧。」",
                "「總商行發行之分格存摺與防偽匯票，在半島四鎮分號皆通存通兌。手頭有餘錢不妨在此開戶，出門在外才能踏實。」",
                "「商貿如春水，流通則活，淤塞則弊。你若在各鎮往來，多留意各地物價物候，處處皆是學問。」",
            ],
        });
    }

    // 11. 蒙學/講舍/講習
    if ["蒙學", "講舍", "講習"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "蒙學學舍",
            default_host_title: "當值教習",
            has_counter_restricted_zone: false,
            on_duty_fixture_activity: "正敲響響板帶領學生朗讀基礎教材",
            on_duty_demeanor: "面相斯文溫和的塾館先生，穿著深青色剪裁外套，短髮梳理整齊，身形清瘦挺拔，語調溫厚平和，看著學舍裡的學童時目光充滿期許。",
            lendable_tool: Some((
                "wt_cat_0782",
                "蒙學基礎識字課本",
                "這本「蒙學基礎識字課本」字跡工整清晰，借你拿去溫習識字，讀完記得放回講堂書架。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0782", "蒙學基礎識字課本", 6),
                ("wt_cat_0781", "竹綿紙牛皮記事手札", 8),
                ("wt_cat_0780", "彈性精鋼尖蘸水硬筆", 10),
            ],
            trade_talk_pitch: "「堂內備有蒙學基礎識字課本、竹綿紙牛皮記事手札與精鋼尖蘸水硬筆，字跡端正明晰，文友若需自可兌購。」",
            on_duty_talk_lines: &[
                "「基礎教育，乃立世之基。引導孩童端正執筆、朗讀讀本，看著那一張張純真面龐漸知明理，便是我平生最大快事。」",
                "「文曜乃半島文脈所繫，學童自幼誦讀渡海先烈開闢山川之誌，知曉粒米皆出稼穡之辛，長大後才能成為社會棟樑。」",
                "「你看起來見多識廣，若願在堂前為學童講述幾句外鄉山川險奇、行路見聞，必能開拓孩子們的視野。」",
            ],
        });
    }

    // 12. 文曜學院/講堂/教務長（非蒙學/講舍）
    if ["學院", "講堂", "文曜學院", "教務長"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "文曜講堂",
            default_host_title: "當值教習",
            has_counter_restricted_zone: false,
            on_duty_fixture_activity: "正於講堂案前批閱學生呈交的脈絡札記",
            on_duty_demeanor: "面容清瘦斯文的長者，鬢髮微霜，穿著水墨月白色剪裁外套，步履沉穩優雅，目光溫和睿智，展現出主持學院教務的學者氣度。",
            lendable_tool: Some((
                "wt_cat_1057",
                "溫氏導能溫石百草溫灸盒",
                "講堂常備的「溫石百草溫灸盒」可供長坐讀書者溫熨穴位，舒緩筋絡，閱畢記得歸還原處。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0784", "《白塔風物誌》", 8),
                ("wt_cat_0780", "彈性精鋼尖蘸水硬筆", 10),
            ],
            trade_talk_pitch: "「學院墨寶皆用竹綿紙印刷，亦有學生常備之黃銅蘸水硬筆與溫潤墨水，文雅精美，可隨緣取閱。」",
            on_duty_talk_lines: &[
                "「學院講究的是把原理弄懂、落實到生活裡。半島立城以來，正是靠著公開教學與技術累積，社會才能一直進步。」",
                "「學生如果只會背書而不懂實際產銷與實務，終究只是空談。在藏書閣編纂風物誌、在診所看診，都是學以致用。」",
                "「初來文曜嗎？如果對學院的基礎課程或各地人文有興趣，隨時可以詢問。」",
            ],
        });
    }

    // 13. 修傘/補靴
    if ["修傘", "補靴"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "修傘補靴鋪",
            default_host_title: "當值鞋傘匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正在硬木楦頭上釘補防浸皮靴並為竹骨傘刷上熟桐油",
            on_duty_demeanor: "寡言沉穩的修配老匠師，圍著浸過蜂蠟與桐油的厚帆布工作圍裙，雙手厚實有力，腿上墊著修鞋木楦頭，專注地修補每一雙皮靴與晴雨摺疊傘。",
            lendable_tool: Some((
                "wt_cat_0896",
                "韌竹骨桐油晴雨摺疊傘",
                "外頭山海天候多變，這把刷足桐油的「韌竹骨桐油晴雨摺疊傘」借你遮風擋雨，回城時順路還來即可。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0896", "韌竹骨桐油晴雨摺疊傘", 8),
                ("wt_cat_0843", "熟成防水牛皮短靴", 15),
            ],
            trade_talk_pitch: "「厚魚油防浸牛皮行路靴與韌竹骨桐油摺疊晴雨傘皆是現貨，尺碼齊備，過客穿上即可啟程，防風防雨保平安。」",
            on_duty_talk_lines: &[
                "「鞋底磨薄了可不行。西門出去走長路，碎石灘和泥濘路最傷腳骨，加釘一層熟牛皮掌，穩健耐磨能多走千里路。」",
                "「傘骨要選北山老竹，浸透桐油的厚棉紙經得起狂風掀扯。晴可遮烈日、雨可擋暴注，這才叫晴雨傘。」",
                "「這蜂蠟松脂防水膏是老配方，靴縫抹上一圈，踩進積水窪半點不滲。手藝人不做虛活。」",
            ],
        });
    }

    // 14. 柑仔店/雜貨
    if ["柑仔店", "雜貨"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "街角雜貨鋪",
            default_host_title: "當值掌事",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正擦拭廣口玻璃柑仔糖罐並清點紅磷火籤與蚊香",
            on_duty_demeanor: "慈眉善目的白髮阿婆，穿著寬鬆舒適的碎花短衫，坐在藤椅上微笑，眼神溫和，手裡拿著蒲扇，身邊散發著陳皮、麥芽糖與曬乾艾草的熟悉香氣。",
            lendable_tool: Some((
                "wt_cat_0898",
                "防潮紅磷引火籤盒",
                "這盒「防潮紅磷引火籤盒」封蠟完好，出門在外升火照明最要緊，先拿去應急吧，店裡備得多得是！",
            )),
            counter_trade_offers: &[
                ("wt_cat_0514", "柑仔店古早黑糖麥芽糖", 3),
                ("wt_cat_0898", "防潮紅磷引火籤盒", 3),
            ],
            trade_talk_pitch: "「廣口玻璃罐裡的酸梅糖、三號坑防潮紅磷引火籤盒、夏季驅蚊艾草蚊香盤都在這，零錢隨意付，都是街坊價。」",
            on_duty_talk_lines: &[
                "「哎唷，年輕人肚子餓了沒？來含一顆金桔酸梅糖，酸甜生津，解渴又提神，自己用土冰糖醃的哩！」",
                "「這條巷子看了幾十年囉，誰家添丁、誰家置產過契，心裡像明鏡似的。歇歇腳再走吧！」",
                "「出遠門隨身裝一盒引火籤，夜裡露營不怕潮氣。缺零碎什物儘管來找阿婆。」",
            ],
        });
    }

    // 15. 旅社/茶屋/茶館
    if ["旅社", "茶屋", "茶館"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "茶屋旅社",
            default_host_title: "當值茶博士",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正於臨水茶席沏起頭道山風草茶並擺設棋局",
            on_duty_demeanor: "眉目清俊、氣質溫和的旅社負責人，穿著雪青色休閒外套，神態安詳從容，說話不疾不徐，泡茶手法俐落熟練。",
            lendable_tool: Some((
                "wt_cat_0723",
                "旋蓋夜光貝母調光台燈",
                "夜路與房內照明少不了燈火，這盞「旋蓋夜光貝母調光台燈」柔潤不刺眼，借你掌燈夜讀或巡行。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0500", "山風草炭焙烘青茶", 5),
                ("wt_cat_0501", "穩火慢煮醇厚鮮奶茶", 5),
                ("wt_cat_0723", "旋蓋夜光貝母調光台燈", 15),
            ],
            trade_talk_pitch: "「東橋茶屋特選山風草烘青茶葉包、熱煮鮮奶茶皆可裝壺帶走，客房旋蓋夜光貝母調光台燈亦有全新行裝版可售。」",
            on_duty_talk_lines: &[
                "「請入座。這壺鍋煮鮮奶茶用了牛埔原牛乳與霜林烘青茶，火候慢煨，茶香醇厚奶香濃郁，最宜撫慰羈旅之情。」",
                "「東橋乃文曜咽喉，南北商旅皆在此歇腳。一局圍棋、半盞清茶，天下山海風物盡在言笑之中。」",
                "「夜深若要展讀或寫信，櫃檯備有旋蓋夜光台燈，光線柔和，不擾同歇客旅。」",
            ],
        });
    }

    // 16. 大食堂/食堂/廚房
    if room_tags.iter().any(|t| matches!(canonical_room_tag(t), "canteen" | "hot_food"))
        || ["大食堂", "食堂", "廚房", "大廚", "主廚"].iter().any(|k| combined.contains(k))
    {
        return Some(RoomVenueObject {
            venue_kind: "大食堂",
            default_host_title: "食堂主廚",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正守在紅鐵大釜前慢火熬煮原牛肉湯",
            on_duty_demeanor: "體態圓潤的食堂主廚，臉上總是掛著熱情的笑容，繫著寬大的白色廚師圍裙，捲起袖子手持大湯勺，周身帶著原牛高湯與蔥香熱油的濃郁香氣。",
            lendable_tool: Some((
                "wt_cat_0897",
                "薄層重礦烏木膽隨身保溫瓶",
                "這只「薄層重礦烏木膽隨身保溫瓶」能蓄熱六個時辰，借你裝熱湯熱水帶在路上喝，回來再洗淨還我。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0444", "厚釜慢燉原牛肉湯", 5),
                ("wt_cat_0388", "現磨溫米漿", 3),
                ("wt_cat_0390", "黑糖薑汁熱豆花", 4),
            ],
            trade_talk_pitch: "「大釜熬原牛骨高湯、現磨溫米漿、木桶鹽滷嫩豆花皆滾燙備妥，亦可打入保溫瓶帶走，幾枚鎂幣管飽管暖！」",
            on_duty_talk_lines: &[
                "「哈哈！快往裡坐！聞到這鑊氣沒有？紅鐵厚底大釜慢火煨燉的大角原牛骨高湯，整整滾了一晝夜！」",
                "「開食堂四十年，本店就信一條：做飯要捨得下料，米要新米、肉要鮮肉，街坊行商吃得滿意，生意自然紅火！」",
                "「清晨出發前喝一碗現磨溫米漿，胃裡踏實暖和，頂風冒雪走幾十里路都不打哆嗦！」",
            ],
        });
    }

    // 17. 觀星台/司天
    if ["觀星台", "司天"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "觀星測候台",
            default_host_title: "當值司辰",
            has_counter_restricted_zone: false,
            on_duty_fixture_activity: "正撥轉三百六十星軌銅儀測算星度交角與節氣日影",
            on_duty_demeanor: "身形修長的觀星台司天掌度，身穿星青色觀測大衣，神色沉靜專注，雙眼明亮有神，手裡拿著一具摺疊式黃銅量測儀，專注於記錄天象與氣象變化。",
            lendable_tool: Some((
                "wt_cat_0732",
                "外澳夜光貝領航羅盤",
                "這枚「外澳夜光貝領航羅盤」指針穩定、夜裡亦能辨位，借你入山出海辨識北辰度數。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0732", "外澳夜光貝領航羅盤", 15),
            ],
            trade_talk_pitch: "「台上校準的黃銅隨身星象指北羅盤尚有餘珍，嵌夜光貝母刻度，長途遠行不迷北辰度數，可為你結緣。」",
            on_duty_talk_lines: &[
                "「三百六十星軌銅儀測天地度數，觀星望氣，始知四時循環。星宿雖遠，其度可推；人事雖繁，其理如一。」",
                "「白塔雪巔雲霧晨聚暮散，其間流光往往映照半島風候潮汐。細推其規律，自能趨吉避凶。」",
                "「登高望遠，胸襟自朗。若在此台仰望星河，心中紛擾雜念自會消解無形。」",
            ],
        });
    }

    // 18. 習武堂/武館
    if ["習武堂", "武館"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "演武堂",
            default_host_title: "當值武師",
            has_counter_restricted_zone: false,
            on_duty_fixture_activity: "正手持包革木刀指點學徒格擋防身步法",
            on_duty_demeanor: "體格健碩沉著的武術教練，身穿黑色訓練服與厚牛皮護腰，濃眉有神，動作俐落流暢，雙腕戴著防扭傷護腕，神態沉穩自持。",
            lendable_tool: Some((
                "wt_cat_1015",
                "包革練習木刀",
                "這柄「包革練習木刀」重心沉穩又不傷筋骨，借你演練步法拆招，練完放回兵架即可。",
            )),
            counter_trade_offers: &[
                ("wt_cat_1015", "包革練習木刀", 14),
                ("wt_cat_0846", "卸力厚牛皮護腕", 12),
            ],
            trade_talk_pitch: "「架上備有熟牛皮裹革的練習重木刀與熟銅卸力護腕，耐打耐磨，初習身法必不可少，作價皆公道。」",
            on_duty_talk_lines: &[
                "「筋骨舒展、沉肩墜肘！習武並非好勇鬥狠，而在於調和周身氣脈、遇力卸力，此乃半島先民安身之本。」",
                "「分家巷習武堂重在根基紮實。一柄重木刀每日揮擊五百回，手穩心定，才能在山林曠野中應對萬全。」",
                "「看你步履沉穩，是個可造之材。若有餘暇，不妨換上軟革護腕，下場對拆幾招。」",
            ],
        });
    }
    None
}
